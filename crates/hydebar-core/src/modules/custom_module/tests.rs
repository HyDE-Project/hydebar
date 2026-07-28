use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    time::{sleep, timeout}
};

use super::{
    listener::{forward_custom_updates, send_event},
    poller::{real_time_signal, run_custom_poller},
    *
};
use crate::{
    ModuleContext, ModuleEventSender,
    event_bus::{BusEvent, EventBus, ModuleEvent},
    modules::ModuleError,
    services::ServiceEvent
};

#[tokio::test]
async fn send_event_propagates_module_errors() {
    let bus = EventBus::new(NonZeroUsize::new(1).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let module_name: Arc<str> = Arc::from("custom");
    let sender = context.module_sender({
        let module_name = Arc::clone(&module_name);
        move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name),
            message
        }
    });

    let data = CustomListenData {
        alt: String::from("alt"),
        ..CustomListenData::default()
    };

    sender
        .try_send(Message::Event(ServiceEvent::Update(data.clone())))
        .expect("initial send");

    let result = send_event(&sender, ServiceEvent::Update(data));
    assert!(matches!(result, Err(ModuleError::EventBus(_))));
}

#[tokio::test]
async fn forward_custom_updates_delivers_events_and_errors() {
    let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let module_name: Arc<str> = Arc::from("custom");
    let sender = context.module_sender({
        let module_name = Arc::clone(&module_name);
        move |message| ModuleEvent::Custom {
            name: Arc::clone(&module_name),
            message
        }
    });

    let (mut writer, reader) = io::duplex(256);
    writer
        .write_all(
            br#"{"alt":"value","text":"ok"}
invalid
"#
        )
        .await
        .expect("write output");
    writer.shutdown().await.expect("shutdown writer");

    let mut lines = BufReader::new(reader).lines();
    forward_custom_updates(&mut lines, module_name.as_ref(), &sender)
        .await
        .expect("forward updates");

    let mut receiver = bus.receiver();

    let first = receiver
        .try_recv()
        .expect("first event")
        .expect("event present");
    match first {
        BusEvent::Module(ModuleEvent::Custom {
            name,
            message
        }) => {
            assert_eq!(name.as_ref(), "custom");
            match message {
                Message::Event(ServiceEvent::Update(data)) => {
                    assert_eq!(data.alt, "value");
                    assert_eq!(data.text.as_deref(), Some("ok"));
                }
                other => panic!("unexpected message: {other:?}")
            }
        }
        other => panic!("unexpected event: {other:?}")
    }

    let second = receiver
        .try_recv()
        .expect("second event")
        .expect("event present");
    match second {
        BusEvent::Module(ModuleEvent::Custom {
            name,
            message
        }) => {
            assert_eq!(name.as_ref(), "custom");
            match message {
                Message::Event(ServiceEvent::Error(error)) => {
                    assert!(matches!(error, CustomCommandError::Parse(_, _)));
                }
                other => panic!("unexpected message: {other:?}")
            }
        }
        other => panic!("unexpected event: {other:?}")
    }
}

#[tokio::test]
async fn re_register_aborts_previous_listener() {
    let bus = EventBus::new(NonZeroUsize::new(32).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let mut custom = Custom::default();

    let mut receiver = bus.receiver();

    let first = CustomModuleDef {
        name: String::from("first"),
        command: String::from("true"),
        icon: None,
        listen_cmd: Some(String::from(
            r#"while true; do printf '{"alt":"first","text":"one"}
'; sleep 0.1; done"#
        )),
        icons: None,
        colors: None,
        alert: None,
        ..CustomModuleDef::default()
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&first))
        .expect("first register");

    timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = receiver.try_recv().expect("receive") {
                if let BusEvent::Module(ModuleEvent::Custom {
                    name,
                    message
                }) = event
                {
                    if name.as_ref() == "first" {
                        if matches!(message, Message::Event(ServiceEvent::Update(_))) {
                            break;
                        }
                    }
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("first update");

    while let Some(Some(_)) = receiver.try_recv().ok() {}

    let second = CustomModuleDef {
        name: String::from("second"),
        command: String::from("true"),
        icon: None,
        listen_cmd: Some(String::from(
            r#"while true; do printf '{"alt":"second","text":"two"}
'; sleep 0.1; done"#
        )),
        icons: None,
        colors: None,
        alert: None,
        ..CustomModuleDef::default()
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&second))
        .expect("second register");

    let observed = timeout(Duration::from_secs(2), async {
        let mut alts = Vec::new();
        loop {
            if let Some(event) = receiver.try_recv().expect("receive") {
                if let BusEvent::Module(ModuleEvent::Custom {
                    name,
                    message
                }) = event
                {
                    if let Message::Event(ServiceEvent::Update(data)) = message {
                        alts.push((name, data.alt));
                        if alts.len() >= 3 {
                            break alts;
                        }
                    }
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("collected updates");

    assert!(
        observed
            .iter()
            .all(|(name, alt)| { name.as_ref() == "second" && alt == "second" })
    );
}

#[test]
fn paints_the_module_with_the_color_matching_the_reported_state() {
    use std::collections::HashMap;

    use hydebar_proto::config::{AppearanceColor, RegexCfg};
    use serde::{Deserialize, de::value::StrDeserializer};

    let pattern = RegexCfg::deserialize(StrDeserializer::<serde::de::value::Error>::new(
        r"^(6\d|7\d)$"
    ))
    .expect("pattern");
    let color =
        AppearanceColor::deserialize(StrDeserializer::<serde::de::value::Error>::new("#ffa500"))
            .expect("color");

    let mut colors = HashMap::new();
    colors.insert(pattern, color);

    let definition = CustomModuleDef {
        name: String::from("cpuinfo"),
        command: String::new(),
        icon: None,
        listen_cmd: None,
        icons: None,
        colors: Some(colors),
        alert: None,
        ..CustomModuleDef::default()
    };

    let mut module = Custom::default();
    module.update(Message::Event(ServiceEvent::Update(CustomListenData {
        alt: String::from("65"),
        ..CustomListenData::default()
    })));

    let painted = view::state_color(&module, &definition).expect("color for a hot reading");
    assert_eq!(painted, iced::Color::from_rgb8(0xff, 0xa5, 0x00));

    module.update(Message::Event(ServiceEvent::Update(CustomListenData {
        alt: String::from("35"),
        ..CustomListenData::default()
    })));

    assert!(view::state_color(&module, &definition).is_none());
}

/// Collects the alternate states a module published, waiting for `count` of
/// them.
async fn collect_alts(
    receiver: &mut crate::event_bus::EventReceiver,
    module_name: &str,
    count: usize
) -> Vec<String> {
    let mut alts = Vec::with_capacity(count);

    loop {
        while let Ok(Some(event)) = receiver.try_recv() {
            if let BusEvent::Module(ModuleEvent::Custom {
                name,
                message: Message::Event(ServiceEvent::Update(data))
            }) = event
                && name.as_ref() == module_name
            {
                alts.push(data.alt);

                if alts.len() >= count {
                    return alts;
                }
            }
        }

        sleep(Duration::from_millis(10)).await;
    }
}

/// Builds a command printing how many times it has been executed so far.
fn counting_command(counter: &std::path::Path) -> String {
    let path = counter.display();

    format!(
        "count=$(( $(cat {path} 2>/dev/null || echo 0) + 1 )); printf '%s' \"$count\" > {path}; \
         printf '{{\"alt\":\"%s\"}}' \"$count\""
    )
}

fn custom_sender(context: &ModuleContext, module_name: &str) -> ModuleEventSender<Message> {
    let module_name: Arc<str> = Arc::from(module_name);

    context.module_sender(move |message| ModuleEvent::Custom {
        name: Arc::clone(&module_name),
        message
    })
}

#[tokio::test]
async fn an_interval_runs_the_command_again_on_every_tick() {
    let bus = EventBus::new(NonZeroUsize::new(64).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let mut receiver = bus.receiver();

    let workdir = tempfile::tempdir().expect("temporary directory");
    let command = counting_command(&workdir.path().join("runs"));

    let poller = tokio::spawn(run_custom_poller(
        Arc::from("cpuinfo"),
        Arc::from(command.as_str()),
        Some(Duration::from_millis(40)),
        None,
        custom_sender(&context, "cpuinfo")
    ));

    let alts = timeout(
        Duration::from_secs(5),
        collect_alts(&mut receiver, "cpuinfo", 3)
    )
    .await
    .expect("three scheduled runs");

    poller.abort();

    assert_eq!(alts, vec!["1", "2", "3"]);
}

#[tokio::test]
async fn a_refresh_signal_runs_the_command_out_of_band() {
    const OFFSET: u8 = 7;

    let bus = EventBus::new(NonZeroUsize::new(64).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let mut receiver = bus.receiver();

    let workdir = tempfile::tempdir().expect("temporary directory");
    let command = counting_command(&workdir.path().join("runs"));

    let poller = tokio::spawn(run_custom_poller(
        Arc::from("updates"),
        Arc::from(command.as_str()),
        None,
        Some(OFFSET),
        custom_sender(&context, "updates")
    ));

    let initial = timeout(
        Duration::from_secs(5),
        collect_alts(&mut receiver, "updates", 1)
    )
    .await
    .expect("initial run");
    assert_eq!(initial, vec!["1"]);

    let signal_number = real_time_signal(OFFSET).expect("real time signal");
    assert_eq!(unsafe { libc::raise(signal_number) }, 0);

    let refreshed = timeout(
        Duration::from_secs(5),
        collect_alts(&mut receiver, "updates", 1)
    )
    .await
    .expect("run triggered by the signal");

    poller.abort();

    assert_eq!(refreshed, vec!["2"]);
}

#[tokio::test]
async fn a_definition_without_a_schedule_keeps_one_streaming_process() {
    let bus = EventBus::new(NonZeroUsize::new(64).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let mut receiver = bus.receiver();
    let mut custom = Custom::default();

    let definition = CustomModuleDef {
        name: String::from("streaming"),
        command: String::new(),
        listen_cmd: Some(String::from(
            r#"count=0; while true; do count=$((count + 1)); printf '{"alt":"%s"}
' "$count"; sleep 0.05; done"#
        )),
        ..CustomModuleDef::default()
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&definition))
        .expect("register");

    let alts = timeout(
        Duration::from_secs(5),
        collect_alts(&mut receiver, "streaming", 3)
    )
    .await
    .expect("three streamed updates");

    assert_eq!(alts, vec!["1", "2", "3"]);
}

#[tokio::test]
async fn changing_the_schedule_restarts_the_module() {
    let bus = EventBus::new(NonZeroUsize::new(64).expect("non-zero"));
    let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
    let mut receiver = bus.receiver();
    let mut custom = Custom::default();

    let workdir = tempfile::tempdir().expect("temporary directory");
    let counter = workdir.path().join("runs");

    let streaming = CustomModuleDef {
        name: String::from("reloaded"),
        command: String::new(),
        listen_cmd: Some(String::from(
            r#"while true; do printf '{"alt":"stream"}
'; sleep 0.05; done"#
        )),
        ..CustomModuleDef::default()
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&streaming))
        .expect("streaming register");

    let streamed = timeout(
        Duration::from_secs(5),
        collect_alts(&mut receiver, "reloaded", 1)
    )
    .await
    .expect("streamed update");
    assert_eq!(streamed, vec!["stream"]);

    let scheduled = CustomModuleDef {
        name: String::from("reloaded"),
        command: String::new(),
        exec: Some(counting_command(&counter)),
        interval: Some(1),
        ..CustomModuleDef::default()
    };

    <Custom as Module<Message>>::register(&mut custom, &context, Some(&scheduled))
        .expect("scheduled register");

    while receiver.try_recv().expect("drain").is_some() {}

    let polled = timeout(
        Duration::from_secs(5),
        collect_alts(&mut receiver, "reloaded", 2)
    )
    .await
    .expect("two scheduled runs");

    assert_eq!(polled, vec!["1", "2"]);
}
