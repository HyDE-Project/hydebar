//! Module rendered from the output of a user provided command.
//!
//! The listener protocol is a superset of the Waybar custom module contract:
//! the process writes one JSON object per line to standard output and the bar
//! renders it.

mod data;
mod error;
mod listener;
mod menu;
mod poller;
mod state;
mod view;

use iced::Element;

pub use self::{
    data::CustomListenData,
    error::CustomCommandError,
    menu::menu_view,
    state::{Custom, CustomCommandService, Message}
};
use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    components::icons::IconTheme,
    config::{Appearance, CustomModuleDef}
};

impl<M> Module<M> for Custom
where
    M: 'static
{
    type RegistrationData<'a> = &'a CustomModuleDef;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.start_listener(ctx, config);
        Ok(())
    }

    fn deregister(&mut self) {
        self.stop_listener();
    }
}

impl Custom {
    /// The bar entry: whatever the listener last printed, drawn the way the
    /// definition asks.
    ///
    /// Rendered by the module itself, so the bar layer holds no custom module
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        config: &CustomModuleDef,
        appearance: &Appearance,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + Clone
    {
        Some((view::render(self, config, appearance, icons), None))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{num::NonZeroUsize, path::Path, sync::Arc, time::Duration};

    use tokio::{
        io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
        time::{sleep, timeout}
    };

    use super::{
        listener::{forward_custom_updates, run_custom_listener, send_event},
        poller::{real_time_signal, run_custom_poller},
        *
    };
    use crate::{
        ModuleContext, ModuleEventSender,
        event_bus::{BusEvent, EventBus, ModuleEvent},
        services::ServiceEvent
    };

    #[tokio::test]
    async fn send_event_survives_a_full_bus_by_evicting_the_oldest() {
        let bus = EventBus::new(NonZeroUsize::new(1).expect("non-zero"));
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
        let module_name: Arc<str> = Arc::from("custom");
        let sender = context.module_sender({
            let module_name = Arc::clone(&module_name);
            move |message| ModuleEvent::Custom {
                name: Arc::clone(&module_name),
                message
            }
        });

        let stale = CustomListenData {
            alt: String::from("stale"),
            ..CustomListenData::default()
        };
        let fresh = CustomListenData {
            alt: String::from("fresh"),
            ..CustomListenData::default()
        };

        sender.send(Message::Event(ServiceEvent::Update(stale)));
        send_event(&sender, ServiceEvent::Update(fresh));

        let survivor = receiver.try_recv().expect("event present");
        match survivor {
            BusEvent::Module(ModuleEvent::Custom {
                message: Message::Event(ServiceEvent::Update(data)),
                ..
            }) => assert_eq!(data.alt, "fresh"),
            other => panic!("unexpected event: {other:?}")
        }
        assert!(
            receiver.try_recv().is_none(),
            "the stale event must be gone"
        );
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

        let first = receiver.try_recv().expect("event present");
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
                    other @ Message::Event(_) => panic!("unexpected message: {other:?}")
                }
            }
            other => panic!("unexpected event: {other:?}")
        }

        let second = receiver.try_recv().expect("event present");
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
                    other @ Message::Event(_) => panic!("unexpected message: {other:?}")
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

        <Custom as Module<Message>>::register(&mut custom, &context, &first)
            .expect("first register");

        timeout(Duration::from_secs(2), async {
            loop {
                if let Some(event) = receiver.try_recv()
                    && let BusEvent::Module(ModuleEvent::Custom {
                        name,
                        message
                    }) = event
                    && name.as_ref() == "first"
                    && matches!(message, Message::Event(ServiceEvent::Update(_)))
                {
                    break;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("first update");

        while receiver.try_recv().is_some() {}

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

        <Custom as Module<Message>>::register(&mut custom, &context, &second)
            .expect("second register");

        let observed = timeout(Duration::from_secs(2), async {
            let mut alts = Vec::new();
            loop {
                // the listener suppresses repeats, so the replacement
                // publishes its payload once and then stays quiet
                if let Some(event) = receiver.try_recv()
                    && let BusEvent::Module(ModuleEvent::Custom {
                        name,
                        message
                    }) = event
                    && let Message::Event(ServiceEvent::Update(data)) = message
                {
                    alts.push((name, data.alt));
                    break alts;
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
        let color = AppearanceColor::deserialize(StrDeserializer::<serde::de::value::Error>::new(
            "#ffa500"
        ))
        .expect("color");

        #[expect(
            clippy::mutable_key_type,
            reason = "RegexCfg hashes and compares by its pattern text; the regex \
                      interior mutability never feeds the map key"
        )]
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
            while let Some(event) = receiver.try_recv() {
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
        #[expect(
            unsafe_code,
            reason = "raises a signal in this process, which the module under test listens for"
        )]
        let raised = unsafe { libc::raise(signal_number) };

        assert_eq!(raised, 0);

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

        <Custom as Module<Message>>::register(&mut custom, &context, &definition)
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

        <Custom as Module<Message>>::register(&mut custom, &context, &streaming)
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

        <Custom as Module<Message>>::register(&mut custom, &context, &scheduled)
            .expect("scheduled register");

        while receiver.try_recv().is_some() {}

        let polled = timeout(
            Duration::from_secs(5),
            collect_alts(&mut receiver, "reloaded", 2)
        )
        .await
        .expect("two scheduled runs");

        assert_eq!(polled, vec!["1", "2"]);
    }

    /// Builds a command that parks a helper of its own and never finishes.
    ///
    /// The helper is a grandchild of the bar, which is exactly the process a
    /// plain kill of the shell would leave behind. Recording its identifier
    /// lets a test watch for the whole family instead of only the shell.
    fn parking_command(pid_file: &Path) -> String {
        let path = pid_file.display();

        format!("sleep 300 & printf '%s' \"$!\" > {path}.tmp; mv {path}.tmp {path}; wait")
    }

    /// Waits until the command has recorded the identifier of its helper.
    async fn recorded_helper(pid_file: &Path) -> u32 {
        loop {
            if let Ok(contents) = std::fs::read_to_string(pid_file)
                && let Ok(pid) = contents.trim().parse::<u32>()
            {
                return pid;
            }

            sleep(Duration::from_millis(10)).await;
        }
    }

    /// Reports whether the process still exists.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "process identifiers fit in i32 on Linux"
    )]
    fn is_alive(pid: u32) -> bool {
        #[expect(
            unsafe_code,
            reason = "signal zero delivers nothing; it only asks whether the id still exists"
        )]
        let probed = unsafe { libc::kill(pid as i32, 0) };

        probed == 0
    }

    /// Waits until the process is gone for good.
    async fn awaits_death(pid: u32) {
        while is_alive(pid) {
            sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn an_aborted_listener_task_leaves_no_process_behind() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let workdir = tempfile::tempdir().expect("temporary directory");
        let pid_file = workdir.path().join("helper.pid");
        let command = parking_command(&pid_file);

        let listener = tokio::spawn(run_custom_listener(
            Arc::from("streaming"),
            Arc::from(command.as_str()),
            custom_sender(&context, "streaming")
        ));

        let helper = timeout(Duration::from_secs(5), recorded_helper(&pid_file))
            .await
            .expect("the command records its helper");

        listener.abort();

        timeout(Duration::from_secs(5), awaits_death(helper))
            .await
            .expect("the helper dies with the aborted listener");
    }

    #[tokio::test]
    async fn an_aborted_poller_task_leaves_no_process_behind() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let workdir = tempfile::tempdir().expect("temporary directory");
        let pid_file = workdir.path().join("helper.pid");
        let command = parking_command(&pid_file);

        let poller = tokio::spawn(run_custom_poller(
            Arc::from("scheduled"),
            Arc::from(command.as_str()),
            Some(Duration::from_millis(50)),
            None,
            custom_sender(&context, "scheduled")
        ));

        let helper = timeout(Duration::from_secs(5), recorded_helper(&pid_file))
            .await
            .expect("the command records its helper");

        poller.abort();

        timeout(Duration::from_secs(5), awaits_death(helper))
            .await
            .expect("the helper dies with the aborted poller");
    }

    #[tokio::test]
    async fn dropping_the_module_leaves_no_process_behind() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let workdir = tempfile::tempdir().expect("temporary directory");
        let pid_file = workdir.path().join("helper.pid");

        let definition = CustomModuleDef {
            name: String::from("dropped"),
            command: String::new(),
            listen_cmd: Some(parking_command(&pid_file)),
            ..CustomModuleDef::default()
        };

        let mut custom = Custom::default();
        <Custom as Module<Message>>::register(&mut custom, &context, &definition)
            .expect("register");

        let helper = timeout(Duration::from_secs(5), recorded_helper(&pid_file))
            .await
            .expect("the command records its helper");

        drop(custom);

        timeout(Duration::from_secs(5), awaits_death(helper))
            .await
            .expect("the helper dies with the module that started it");
    }

    #[tokio::test]
    async fn re_registering_a_module_leaves_no_process_behind() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let workdir = tempfile::tempdir().expect("temporary directory");
        let pid_file = workdir.path().join("helper.pid");

        let first = CustomModuleDef {
            name: String::from("replaced"),
            command: String::new(),
            listen_cmd: Some(parking_command(&pid_file)),
            ..CustomModuleDef::default()
        };

        let mut custom = Custom::default();
        <Custom as Module<Message>>::register(&mut custom, &context, &first)
            .expect("first register");

        let helper = timeout(Duration::from_secs(5), recorded_helper(&pid_file))
            .await
            .expect("the command records its helper");

        let second = CustomModuleDef {
            name: String::from("replaced"),
            command: String::new(),
            listen_cmd: Some(String::from("sleep 300")),
            ..CustomModuleDef::default()
        };

        <Custom as Module<Message>>::register(&mut custom, &context, &second)
            .expect("second register");

        timeout(Duration::from_secs(5), awaits_death(helper))
            .await
            .expect("the helper of the replaced listener dies");
    }
}
