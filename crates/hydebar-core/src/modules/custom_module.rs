//! Module rendered from the output of a user provided command.
//!
//! The listener protocol is a superset of the Waybar custom module contract:
//! the process writes one JSON object per line to standard output and the bar
//! renders it.

mod data {
    //! Payload produced by a custom module listener process.

    use serde::Deserialize;

    /// One update emitted by a listener process as a single line of JSON.
    ///
    /// The shape is a superset of the Waybar custom module return type, so
    /// scripts written for Waybar work without modification.
    #[derive(Debug, Clone, Deserialize, Default, PartialEq)]
    pub struct CustomListenData {
        /// Alternate state name, matched against the configured icon and alert
        /// patterns.
        #[serde(default)]
        pub alt:        String,
        /// Text rendered next to the icon.
        #[serde(default)]
        pub text:       Option<String>,
        /// Text rendered when the pointer rests on the module.
        #[serde(default)]
        pub tooltip:    Option<String>,
        /// Style class requested by the listener.
        #[serde(default, deserialize_with = "first_class")]
        pub class:      Option<String>,
        /// Progress value in the zero to one hundred range.
        #[serde(default)]
        pub percentage: Option<f32>
    }

    /// Accepts both the single string and the list form Waybar allows for
    /// `class`, keeping the first entry of a list.
    fn first_class<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ClassField {
            One(String),
            Many(Vec<String>)
        }

        Ok(match Option::<ClassField>::deserialize(deserializer)? {
            Some(ClassField::One(value)) => Some(value),
            Some(ClassField::Many(values)) => values.into_iter().next(),
            None => None
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_the_waybar_payload_shape() {
            let parsed: CustomListenData = serde_json::from_str(
            r#"{"text":"42%","alt":"charging","tooltip":"Battery","class":"warning","percentage":42}"#
        )
        .expect("waybar payload");

            assert_eq!(parsed.alt, "charging");
            assert_eq!(parsed.text.as_deref(), Some("42%"));
            assert_eq!(parsed.tooltip.as_deref(), Some("Battery"));
            assert_eq!(parsed.class.as_deref(), Some("warning"));
            assert_eq!(parsed.percentage, Some(42.0));
        }

        #[test]
        fn accepts_a_class_list_and_keeps_the_first_entry() {
            let parsed: CustomListenData =
                serde_json::from_str(r#"{"text":"x","class":["urgent","blinking"]}"#)
                    .expect("class list");

            assert_eq!(parsed.class.as_deref(), Some("urgent"));
        }

        #[test]
        fn tolerates_a_payload_without_any_field() {
            let parsed: CustomListenData = serde_json::from_str("{}").expect("empty payload");

            assert_eq!(parsed, CustomListenData::default());
        }
    }
}
mod error {
    //! Failure modes of a custom module listener process.

    use std::sync::Arc;

    use crate::modules::ModuleError;

    /// Something that went wrong while running or reading a listener process.
    #[derive(Debug, Clone)]
    pub enum CustomCommandError {
        Spawn(Arc<std::io::Error>),
        MissingStdout,
        Read(Arc<std::io::Error>),
        Parse(String, Arc<serde_json::Error>),
        Wait(Arc<std::io::Error>),
        NonZeroExit { status: Option<i32> },
        Signal(u8, Arc<std::io::Error>),
        UnsupportedSignal(u8),
        ChannelClosed
    }

    impl std::fmt::Display for CustomCommandError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Spawn(err) => {
                    write!(f, "failed to spawn custom module listener process: {}", err)
                }
                Self::MissingStdout => write!(f, "custom module listener did not expose stdout"),
                Self::Read(err) => {
                    write!(f, "failed to read line from custom module output: {}", err)
                }
                Self::Parse(snippet, err) => {
                    write!(
                        f,
                        "failed to parse custom module output: {} ({})",
                        snippet, err
                    )
                }
                Self::Wait(err) => write!(f, "failed to wait for custom module process: {}", err),
                Self::NonZeroExit {
                    status
                } => write!(
                    f,
                    "custom module process exited unsuccessfully ({:?})",
                    status
                ),
                Self::Signal(offset, err) => write!(
                    f,
                    "failed to listen for the custom module refresh signal SIGRTMIN+{}: {}",
                    offset, err
                ),
                Self::UnsupportedSignal(offset) => write!(
                    f,
                    "custom module refresh signal SIGRTMIN+{} is outside the real time range",
                    offset
                ),
                Self::ChannelClosed => write!(f, "custom module updates channel closed")
            }
        }
    }

    impl std::error::Error for CustomCommandError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Spawn(err) => Some(err.as_ref()),
                Self::Read(err) => Some(err.as_ref()),
                Self::Parse(_, err) => Some(err.as_ref()),
                Self::Wait(err) => Some(err.as_ref()),
                Self::Signal(_, err) => Some(err.as_ref()),
                _ => None
            }
        }
    }

    impl CustomCommandError {
        /// Short message rendered in place of the module content.
        pub(super) fn to_display_message(&self) -> String {
            match self {
                Self::Parse(snippet, ..) => format!("Invalid output: {snippet}"),
                Self::NonZeroExit {
                    status
                } => match status {
                    Some(code) => format!("Listener exited with status {code}"),
                    None => String::from("Listener exited due to signal")
                },
                Self::Signal(offset, _) => format!("Cannot watch SIGRTMIN+{offset}"),
                Self::UnsupportedSignal(offset) => {
                    format!("Signal SIGRTMIN+{offset} out of range")
                }
                Self::ChannelClosed => String::from("Listener updates queue closed"),
                Self::MissingStdout => String::from("Listener stdout unavailable"),
                Self::Spawn(_) | Self::Read(_) | Self::Wait(_) => {
                    String::from("Listener IO failure")
                }
            }
        }
    }

    /// Trims a listener output line so error messages stay readable.
    pub(super) fn truncate_snippet(line: &str) -> String {
        const MAX_LEN: usize = 120;

        if line.len() <= MAX_LEN {
            return line.to_owned();
        }

        let mut truncated = String::with_capacity(MAX_LEN + 1);
        for (idx, ch) in line.char_indices() {
            if idx >= MAX_LEN {
                truncated.push('…');
                break;
            }
            truncated.push(ch);
        }
        truncated
    }

    /// Error raised by the listener task itself.
    #[derive(Debug, Clone)]
    pub(super) enum CustomListenerError {
        Command(CustomCommandError),
        Module(ModuleError)
    }

    impl std::fmt::Display for CustomListenerError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Command(err) => write!(f, "{}", err),
                Self::Module(err) => write!(f, "{}", err)
            }
        }
    }

    impl std::error::Error for CustomListenerError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Command(err) => Some(err),
                Self::Module(err) => Some(err)
            }
        }
    }
}
mod listener {
    //! Listener task reading newline delimited JSON from an external process.

    use std::{process::Stdio, sync::Arc};

    use log::{error, info};
    use tokio::{
        io::{AsyncBufRead, AsyncBufReadExt, BufReader, Lines},
        process::Command
    };

    use super::{
        Message,
        data::CustomListenData,
        error::{CustomCommandError, CustomListenerError, truncate_snippet}
    };
    use crate::{ModuleEventSender, modules::ModuleError, services::ServiceEvent};

    pub(super) fn send_event(
        sender: &ModuleEventSender<Message>,
        event: ServiceEvent<super::CustomCommandService>
    ) -> Result<(), ModuleError> {
        sender
            .try_send(Message::Event(event))
            .map_err(ModuleError::from)
    }

    /// Publishes every line the process prints, skipping repeats.
    ///
    /// A `listen_cmd` written as a `while :; do …; sleep N; done` loop reprints
    /// the same payload whenever the value it reports has not moved.
    /// Forwarding those repeats costs a full window repaint each time, so
    /// an unchanged payload stops here. A parse failure clears the memo,
    /// because the module state it leaves behind differs from the one the
    /// repeated payload described.
    pub(super) async fn forward_custom_updates<R>(
        reader: &mut Lines<R>,
        module_name: &str,
        sender: &ModuleEventSender<Message>
    ) -> Result<(), CustomListenerError>
    where
        R: AsyncBufRead + Unpin
    {
        let mut published: Option<CustomListenData> = None;

        while let Some(line) = reader
            .next_line()
            .await
            .map_err(|err| CustomListenerError::Command(CustomCommandError::Read(Arc::new(err))))?
        {
            match serde_json::from_str::<CustomListenData>(&line) {
                Ok(event) => {
                    if published.as_ref() == Some(&event) {
                        continue;
                    }

                    published = Some(event.clone());

                    send_event(sender, ServiceEvent::Update(event))
                        .map_err(CustomListenerError::Module)?;
                }
                Err(err) => {
                    published = None;
                    let parse_error =
                        CustomCommandError::Parse(truncate_snippet(&line), Arc::new(err));
                    error!(
                        "Custom module '{module_name}' failed to parse JSON output: {parse_error:?}"
                    );
                    send_event(sender, ServiceEvent::Error(parse_error.clone()))
                        .map_err(CustomListenerError::Module)?;
                }
            }
        }

        Ok(())
    }

    /// Streams a `listen_cmd` and publishes every payload it prints.
    ///
    /// The command runs in a process group of its own, guarded for as long as
    /// this future lives. Aborting the task — which is how a configuration
    /// reload replaces a module — drops the guard and ends the shell
    /// together with every helper it spawned; releasing the guard only
    /// after the child has been reaped keeps that true even when the abort
    /// lands during the final wait.
    pub(super) async fn run_custom_listener(
        module_name: Arc<str>,
        command: Arc<str>,
        sender: ModuleEventSender<Message>
    ) -> Result<(), CustomListenerError> {
        let mut spawner = Command::new("bash");
        spawner
            .arg("-c")
            .arg(command.as_ref())
            .stdout(Stdio::piped());

        let (mut child, mut guard) = crate::utils::process_group::spawn_guarded(&mut spawner)
            .map_err(|err| {
                CustomListenerError::Command(CustomCommandError::Spawn(Arc::new(err)))
            })?;

        let stdout = child.stdout.take().ok_or(CustomListenerError::Command(
            CustomCommandError::MissingStdout
        ))?;

        let mut reader = BufReader::new(stdout).lines();

        forward_custom_updates(&mut reader, module_name.as_ref(), &sender).await?;

        match child.wait().await {
            Ok(status) => {
                if let Some(guard) = guard.as_mut() {
                    guard.release();
                }

                info!("Custom module '{module_name}' listener exited with status: {status}");
                if status.success() {
                    Ok(())
                } else {
                    Err(CustomListenerError::Command(
                        CustomCommandError::NonZeroExit {
                            status: status.code()
                        }
                    ))
                }
            }
            Err(err) => Err(CustomListenerError::Command(CustomCommandError::Wait(
                Arc::new(err)
            )))
        }
    }
}
mod menu {
    //! Context menu a custom module opens on a right press.
    //!
    //! It is the native counterpart of the Waybar `menu-actions` map: every
    //! entry a definition declares becomes a row running its command,
    //! without the separate GTK menu file Waybar needs to name the rows.

    use iced::{
        Element, Length,
        widget::{Column, button, row}
    };

    use crate::{
        components::{icons::icon_raw, scale, text::text},
        config::{Appearance, CustomMenuEntry, CustomModuleDef},
        style::menu_entry_button_style
    };

    /// Builds the rows of the context menu declared by a custom module.
    ///
    /// `on_select` turns the pressed entry into the message the caller reacts
    /// to, so the module stays unaware of how the command is run and of how
    /// the menu surface is dismissed.
    ///
    /// Entries carry their glyph verbatim rather than through the icon theme:
    /// an entry names the glyph the way the module itself does, so there is
    /// no named icon to resolve.
    ///
    /// Entries missing a label or a command are dropped by
    /// [`CustomModuleDef::menu_entries`], leaving an empty column for a
    /// definition without a usable entry.
    pub fn menu_view<'a, M>(
        definition: &CustomModuleDef,
        appearance: &Appearance,
        opacity: f32,
        on_select: impl Fn(&CustomMenuEntry) -> M + 'a
    ) -> Element<'a, M>
    where
        M: Clone + 'a
    {
        let radius = appearance.pill_radius();
        let gap = appearance.icon_label_gap();

        Column::with_children(
            definition
                .menu_entries()
                .map(|entry| {
                    let label: Element<'a, M> = match entry.icon.as_deref() {
                        Some(glyph) if !glyph.is_empty() => {
                            row![icon_raw(glyph.to_owned()), text(entry.label.clone())]
                                .spacing(gap)
                                .align_y(iced::Alignment::Center)
                                .into()
                        }
                        _ => text(entry.label.clone()).into()
                    };

                    button(label)
                        .padding([scale::scaled(4.0), scale::scaled(12.0)])
                        .width(Length::Fill)
                        .style(menu_entry_button_style(opacity, radius))
                        .on_press(on_select(entry))
                        .into()
                })
                .collect::<Vec<Element<'a, M>>>()
        )
        .width(Length::Fill)
        .spacing(scale::scaled(4.0))
        .into()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn definition() -> CustomModuleDef {
            toml::from_str(
                r#"
            name = "power"
            command = ""

            [[menu]]
            label = "Lock"
            icon = "L"
            command = "hyde-shell lockscreen.sh"

            [[menu]]
            label = "Logout"
            command = "hyprctl dispatch exit 0"
            "#
            )
            .expect("definition")
        }

        #[test]
        fn selecting_an_entry_builds_the_message_of_its_command() {
            let definition = definition();
            let commands = definition
                .menu_entries()
                .map(|entry| entry.command.clone())
                .collect::<Vec<_>>();

            assert_eq!(
                commands,
                vec![
                    String::from("hyde-shell lockscreen.sh"),
                    String::from("hyprctl dispatch exit 0")
                ]
            );
        }

        #[test]
        fn renders_a_row_per_declared_entry() {
            let definition = definition();
            let appearance = Appearance::default();

            let _element: Element<'_, String> =
                menu_view(&definition, &appearance, appearance.menu.opacity, |entry| {
                    entry.command.clone()
                });
        }
    }
}
mod poller {
    //! Scheduled and signal driven execution of a custom module command.
    //!
    //! Where the listener task keeps a single process alive and reads its
    //! output, the poller re-runs a short lived command: once at startup,
    //! then on every interval tick and whenever the configured real time
    //! signal arrives. This is the Waybar `interval` plus `signal`
    //! contract, so scripts written for `pkill -RTMIN+N waybar` work
    //! unchanged.

    use std::{future::pending, process::Stdio, sync::Arc, time::Duration};

    use log::error;
    use tokio::{
        process::Command,
        signal::unix::{Signal, SignalKind, signal},
        time::{Instant, Interval, MissedTickBehavior, interval_at}
    };

    use super::{
        Message,
        data::CustomListenData,
        error::{CustomCommandError, CustomListenerError, truncate_snippet},
        listener::send_event
    };
    use crate::{ModuleEventSender, services::ServiceEvent};

    /// Resolves the real time signal number an offset refers to.
    ///
    /// The offset is relative to `SIGRTMIN`, the same base `pkill -RTMIN+N`
    /// uses, and offsets past `SIGRTMAX` are rejected instead of aliasing
    /// another signal.
    pub(super) fn real_time_signal(offset: u8) -> Option<i32> {
        let raw = libc::SIGRTMIN().checked_add(i32::from(offset))?;

        (raw <= libc::SIGRTMAX()).then_some(raw)
    }

    /// Registers the refresh signal, if the module asked for one.
    fn open_refresh_signal(offset: Option<u8>) -> Result<Option<Signal>, CustomListenerError> {
        let Some(offset) = offset else {
            return Ok(None);
        };

        let raw = real_time_signal(offset).ok_or(CustomListenerError::Command(
            CustomCommandError::UnsupportedSignal(offset)
        ))?;

        signal(SignalKind::from_raw(raw)).map(Some).map_err(|err| {
            CustomListenerError::Command(CustomCommandError::Signal(offset, Arc::new(err)))
        })
    }

    /// Builds the ticker firing after the first period, the initial run
    /// happening before the loop is entered.
    fn open_ticker(period: Option<Duration>) -> Option<Interval> {
        let period = period?;
        let mut ticker = interval_at(Instant::now() + period, period);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Some(ticker)
    }

    /// Waits for the next scheduled tick, or forever when no interval is set.
    async fn next_tick(ticker: Option<&mut Interval>) {
        match ticker {
            Some(ticker) => {
                ticker.tick().await;
            }
            None => pending::<()>().await
        }
    }

    /// Waits for the next refresh signal, or forever when none is registered.
    async fn next_refresh(refresh: Option<&mut Signal>) {
        match refresh {
            Some(refresh) => {
                refresh.recv().await;
            }
            None => pending::<()>().await
        }
    }

    /// Runs the command once and publishes whatever it printed.
    ///
    /// The whole standard output is parsed as a single JSON object, matching
    /// the non-continuous Waybar `exec` contract. A failing run reports an
    /// error event so the module can render an alert without tearing the
    /// poller down.
    ///
    /// `published` carries the payload the bar is already showing. A run that
    /// reprints it publishes nothing, since the repaint every event triggers
    /// would produce an identical frame.
    ///
    /// The run happens in a process group of its own so that a reload landing
    /// while the command is still working ends it instead of orphaning it:
    /// a script that blocks on the network, run every few seconds, would
    /// otherwise pile up one stranded copy per reload.
    async fn run_once(
        module_name: &str,
        command: &str,
        sender: &ModuleEventSender<Message>,
        published: &mut Option<CustomListenData>
    ) -> Result<(), CustomListenerError> {
        let mut spawner = Command::new("bash");
        spawner
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = crate::utils::process_group::guarded_output(&mut spawner)
            .await
            .map_err(|err| {
                CustomListenerError::Command(CustomCommandError::Spawn(Arc::new(err)))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let payload = stdout.trim();

        if payload.is_empty() {
            if !output.status.success() {
                let failure = CustomCommandError::NonZeroExit {
                    status: output.status.code()
                };
                error!("Custom module '{module_name}' command failed: {failure:?}");
                *published = None;

                return send_event(sender, ServiceEvent::Error(failure))
                    .map_err(CustomListenerError::Module);
            }

            return Ok(());
        }

        match serde_json::from_str::<CustomListenData>(payload) {
            Ok(data) => {
                if published.as_ref() == Some(&data) {
                    return Ok(());
                }

                *published = Some(data.clone());

                send_event(sender, ServiceEvent::Update(data)).map_err(CustomListenerError::Module)
            }
            Err(err) => {
                let parse_error =
                    CustomCommandError::Parse(truncate_snippet(payload), Arc::new(err));
                error!(
                    "Custom module '{module_name}' failed to parse JSON output: {parse_error:?}"
                );
                *published = None;

                send_event(sender, ServiceEvent::Error(parse_error))
                    .map_err(CustomListenerError::Module)
            }
        }
    }

    /// Drives a custom module by re-running its command.
    ///
    /// The command runs immediately, then on every `period` tick and on every
    /// delivery of the real time signal `signal_offset` refers to. Without
    /// either trigger the command runs exactly once and the task completes.
    pub(super) async fn run_custom_poller(
        module_name: Arc<str>,
        command: Arc<str>,
        period: Option<Duration>,
        signal_offset: Option<u8>,
        sender: ModuleEventSender<Message>
    ) -> Result<(), CustomListenerError> {
        let mut refresh = open_refresh_signal(signal_offset)?;
        let mut ticker = open_ticker(period);
        let mut published = None;

        run_once(
            module_name.as_ref(),
            command.as_ref(),
            &sender,
            &mut published
        )
        .await?;

        if ticker.is_none() && refresh.is_none() {
            return Ok(());
        }

        loop {
            tokio::select! {
                () = next_tick(ticker.as_mut()) => {}
                () = next_refresh(refresh.as_mut()) => {}
            }

            run_once(
                module_name.as_ref(),
                command.as_ref(),
                &sender,
                &mut published
            )
            .await?;
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Distance from `SIGRTMIN` to `SIGRTMAX` on the running kernel.
        fn real_time_span() -> u8 {
            u8::try_from(libc::SIGRTMAX() - libc::SIGRTMIN()).expect("span fits a byte")
        }

        #[test]
        fn a_zero_offset_names_sigrtmin_itself() {
            assert_eq!(real_time_signal(0), Some(libc::SIGRTMIN()));
        }

        #[test]
        fn an_offset_counts_up_from_sigrtmin() {
            assert_eq!(real_time_signal(2), Some(libc::SIGRTMIN() + 2));
        }

        #[test]
        fn the_last_real_time_signal_is_still_accepted() {
            assert_eq!(real_time_signal(real_time_span()), Some(libc::SIGRTMAX()));
        }

        #[test]
        fn an_offset_past_sigrtmax_is_rejected() {
            assert_eq!(real_time_signal(real_time_span() + 1), None);
        }

        #[test]
        fn the_largest_offset_is_rejected() {
            assert_eq!(real_time_signal(u8::MAX), None);
        }

        #[test]
        fn no_interval_means_no_ticker() {
            assert!(open_ticker(None).is_none());
        }

        #[tokio::test]
        async fn a_ticker_keeps_the_configured_period_and_delays_missed_ticks() {
            let ticker = open_ticker(Some(Duration::from_secs(5))).expect("ticker");

            assert_eq!(ticker.period(), Duration::from_secs(5));
            assert_eq!(ticker.missed_tick_behavior(), MissedTickBehavior::Delay);
        }

        #[tokio::test(start_paused = true)]
        async fn the_first_tick_waits_a_full_period() {
            let period = Duration::from_secs(5);
            let start = Instant::now();
            let mut ticker = open_ticker(Some(period)).expect("ticker");

            ticker.tick().await;

            assert_eq!(Instant::now() - start, period);
        }
    }
}
mod state {
    //! Runtime state and listener wiring for a custom module.

    use std::{sync::Arc, time::Duration};

    use hydebar_proto::config::CustomModuleSource;
    use iced::Subscription;
    use log::error;
    use tokio::task::JoinHandle;

    use super::{
        data::CustomListenData,
        error::{CustomCommandError, CustomListenerError},
        listener::{run_custom_listener, send_event},
        poller::run_custom_poller
    };
    use crate::{
        ModuleContext, ModuleEventSender, config::CustomModuleDef, event_bus::ModuleEvent,
        modules::ModuleError, services::ServiceEvent
    };

    /// State of a single custom module instance.
    #[derive(Default, Debug)]
    pub struct Custom {
        pub(super) data:       CustomListenData,
        pub(super) last_error: Option<CustomCommandError>,
        registration:          Option<CustomRegistration>,
        sender:                Option<ModuleEventSender<Message>>,
        listener_task:         Option<JoinHandle<()>>
    }

    #[derive(Debug, Clone)]
    struct CustomRegistration {
        name:   Arc<str>,
        source: RegistrationSource
    }

    /// Producing command of a registered module, together with its schedule.
    #[derive(Debug, Clone)]
    enum RegistrationSource {
        /// One long lived process streaming json lines.
        Stream { command: Arc<str> },
        /// A command re-run on a schedule or when a real time signal arrives.
        Poll {
            command: Arc<str>,
            period:  Option<Duration>,
            signal:  Option<u8>
        }
    }

    impl RegistrationSource {
        fn from_config(source: CustomModuleSource<'_>) -> Self {
            match source {
                CustomModuleSource::Stream {
                    command
                } => Self::Stream {
                    command: Arc::from(command)
                },
                CustomModuleSource::Poll {
                    command,
                    interval,
                    signal
                } => Self::Poll {
                    command: Arc::from(command),
                    period: interval.map(Duration::from_secs),
                    signal
                }
            }
        }
    }

    impl Custom {
        /// Reports whether a producing command is currently feeding the module.
        ///
        /// Registration is what starts the shell behind a custom module, and
        /// the only externally visible trace of it is the task it left
        /// behind. The bar gates registration on the module being drawn
        /// somewhere, so this is the question a caller has to be able
        /// to ask to tell a module that is merely silent from one that
        /// was never started.
        #[must_use]
        pub fn is_listening(&self) -> bool {
            self.registration.is_some()
        }

        fn abort_listener(&mut self) {
            if let Some(handle) = self.listener_task.take() {
                handle.abort();
            }
        }

        pub fn update(&mut self, msg: Message) {
            match msg {
                Message::Event(ServiceEvent::Update(data)) => {
                    self.data = data;
                    self.last_error = None;
                }
                Message::Event(ServiceEvent::Error(error)) => {
                    self.last_error = Some(error);
                }
                Message::Event(ServiceEvent::Init(_)) => {}
            }
        }

        /// Restarts the task feeding the module from the given configuration.
        ///
        /// A definition without a schedule keeps the streaming listener, while
        /// an `interval` or a `signal` switches to the poller. Because
        /// the whole task is torn down first, a configuration reload
        /// that only changes the interval or the signal number restarts
        /// the module on the new schedule.
        pub(super) fn start_listener(
            &mut self,
            ctx: &ModuleContext,
            config: Option<&CustomModuleDef>
        ) -> Result<(), ModuleError> {
            self.abort_listener();
            self.sender = None;
            self.last_error = None;
            self.registration = config.and_then(|definition| {
                definition.source().map(|source| CustomRegistration {
                    name:   Arc::from(definition.name.as_str()),
                    source: RegistrationSource::from_config(source)
                })
            });

            let Some(registration) = self.registration.clone() else {
                return Ok(());
            };

            let module_name_for_sender = Arc::clone(&registration.name);
            let sender = ctx.module_sender(move |message| ModuleEvent::Custom {
                name: Arc::clone(&module_name_for_sender),
                message
            });

            self.sender = Some(sender.clone());
            let module_name = Arc::clone(&registration.name);
            let source = registration.source.clone();
            let error_sender = sender.clone();

            self.listener_task = Some(ctx.runtime_handle().spawn(async move {
                let outcome = match source {
                    RegistrationSource::Stream {
                        command
                    } => run_custom_listener(Arc::clone(&module_name), command, sender).await,
                    RegistrationSource::Poll {
                        command,
                        period,
                        signal
                    } => {
                        run_custom_poller(
                            Arc::clone(&module_name),
                            command,
                            period,
                            signal,
                            sender
                        )
                        .await
                    }
                };

                report_listener_outcome(outcome, &module_name, &error_sender);
            }));

            Ok(())
        }
    }

    fn report_listener_outcome(
        outcome: Result<(), CustomListenerError>,
        module_name: &Arc<str>,
        error_sender: &ModuleEventSender<Message>
    ) {
        match outcome {
            Ok(()) => {}
            Err(CustomListenerError::Command(error)) => {
                error!("Custom module '{module_name}' listener terminated with error: {error:?}");

                if !matches!(error, CustomCommandError::ChannelClosed)
                    && let Err(send_error) = send_event(error_sender, ServiceEvent::Error(error))
                {
                    error!(
                        "Custom module '{module_name}' failed to publish error notification: \
                     {send_error}"
                    );
                }
            }
            Err(CustomListenerError::Module(error)) => {
                error!("Custom module '{module_name}' failed to publish event: {error}");
            }
        }
    }

    impl Drop for Custom {
        fn drop(&mut self) {
            self.abort_listener();
        }
    }

    /// Messages delivered to a custom module.
    #[derive(Debug, Clone)]
    pub enum Message {
        Event(ServiceEvent<CustomCommandService>)
    }

    /// Marker service carrying listener updates through the event bus.
    #[derive(Debug, Clone, Default)]
    pub struct CustomCommandService;

    impl crate::services::ReadOnlyService for CustomCommandService {
        type UpdateEvent = CustomListenData;
        type Error = CustomCommandError;

        fn update(&mut self, _event: Self::UpdateEvent) {}

        fn subscribe() -> Subscription<ServiceEvent<Self>> {
            Subscription::none()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn update_payload(alt: &str) -> CustomListenData {
            CustomListenData {
                alt: String::from(alt),
                ..CustomListenData::default()
            }
        }

        #[test]
        fn an_update_replaces_the_data_and_clears_the_previous_error() {
            let mut module = Custom::default();
            module.update(Message::Event(ServiceEvent::Error(
                CustomCommandError::ChannelClosed
            )));

            module.update(Message::Event(ServiceEvent::Update(update_payload("42"))));

            assert_eq!(module.data.alt, "42");
            assert!(module.last_error.is_none());
        }

        #[test]
        fn an_error_keeps_the_data_already_on_screen() {
            let mut module = Custom::default();
            module.update(Message::Event(ServiceEvent::Update(update_payload("42"))));

            module.update(Message::Event(ServiceEvent::Error(
                CustomCommandError::ChannelClosed
            )));

            assert_eq!(module.data.alt, "42");
            assert!(matches!(
                module.last_error,
                Some(CustomCommandError::ChannelClosed)
            ));
        }

        #[test]
        fn an_init_event_changes_nothing() {
            let mut module = Custom::default();
            module.update(Message::Event(ServiceEvent::Update(update_payload("42"))));
            module.update(Message::Event(ServiceEvent::Error(
                CustomCommandError::ChannelClosed
            )));

            module.update(Message::Event(ServiceEvent::Init(CustomCommandService)));

            assert_eq!(module.data.alt, "42");
            assert!(matches!(
                module.last_error,
                Some(CustomCommandError::ChannelClosed)
            ));
        }

        #[test]
        fn a_module_without_a_registration_is_not_listening() {
            assert!(!Custom::default().is_listening());
        }

        #[test]
        fn a_stream_config_keeps_its_command_verbatim() {
            let source = RegistrationSource::from_config(CustomModuleSource::Stream {
                command: "tail -f log"
            });

            match source {
                RegistrationSource::Stream {
                    command
                } => assert_eq!(command.as_ref(), "tail -f log"),
                other => panic!("unexpected source: {other:?}")
            }
        }

        #[test]
        fn a_poll_config_turns_interval_seconds_into_a_period() {
            let source = RegistrationSource::from_config(CustomModuleSource::Poll {
                command:  "hyde-shell cpuinfo",
                interval: Some(5),
                signal:   Some(20)
            });

            match source {
                RegistrationSource::Poll {
                    command,
                    period,
                    signal
                } => {
                    assert_eq!(command.as_ref(), "hyde-shell cpuinfo");
                    assert_eq!(period, Some(Duration::from_secs(5)));
                    assert_eq!(signal, Some(20));
                }
                other => panic!("unexpected source: {other:?}")
            }
        }

        #[test]
        fn a_poll_config_without_an_interval_has_no_period() {
            let source = RegistrationSource::from_config(CustomModuleSource::Poll {
                command:  "checkupdates",
                interval: None,
                signal:   None
            });

            match source {
                RegistrationSource::Poll {
                    period,
                    signal,
                    ..
                } => {
                    assert!(period.is_none());
                    assert!(signal.is_none());
                }
                other => panic!("unexpected source: {other:?}")
            }
        }
    }
}
mod view {
    //! Bar rendering for modules driven by an external command.

    use iced::{
        Color, Element, Length, Theme,
        mouse::Cursor,
        widget::{
            Stack, canvas,
            canvas::{Cache, Geometry, Path, Program},
            container, row
        }
    };

    use super::Custom;
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon, icon_raw},
            text::text
        },
        config::{Appearance, CustomModuleDef}
    };

    /// Small circle drawn over the icon while the module is in an alert state.
    ///
    /// Carries its radius so the dot follows the themed sizes instead of
    /// staying two pixels on every screen.
    #[derive(Debug, Clone, Copy, Default)]
    pub(super) struct AlertIndicator {
        radius: f32
    }

    impl<Message> Program<Message> for AlertIndicator {
        type State = ();

        fn draw(
            &self,
            _state: &Self::State,
            renderer: &iced::Renderer,
            theme: &Theme,
            bounds: iced::Rectangle,
            _cursor: Cursor
        ) -> Vec<Geometry> {
            let cache = Cache::new();

            vec![cache.draw(renderer, bounds.size(), |frame| {
                let center = frame.center();
                let circle = Path::circle(center, self.radius);
                frame.fill(&circle, theme.palette().danger);
            })]
        }
    }

    /// Diameter of the alert dot, in em of the themed font.
    const ALERT_DOT_EM: f32 = 0.5;

    /// Resolves the color a module paints itself with for the state it reports.
    ///
    /// The alternate state carries the bucket a listener assigns to its
    /// reading, so a temperature readout can shade itself from cold to
    /// critical the way the equivalent Waybar stylesheet does.
    pub(super) fn state_color(module: &Custom, config: &CustomModuleDef) -> Option<Color> {
        let colors = config.colors.as_ref()?;

        colors.iter().find_map(|(pattern, color)| {
            pattern.is_match(&module.data.alt).then(|| color.get_base())
        })
    }

    /// Builds the bar content for a custom module.
    ///
    /// The gap between the icon and its text is derived from the themed font
    /// size carried by `appearance` instead of being fixed in pixels.
    pub(super) fn render<M>(
        module: &Custom,
        config: &CustomModuleDef,
        appearance: &Appearance,
        icons: &IconTheme
    ) -> Element<'static, M>
    where
        M: 'static + Clone
    {
        let state_color = state_color(module, config);

        let mut icon_element = config.icon.as_ref().map_or_else(
            || icon(icons, Icons::None),
            |glyph| icon_raw(glyph.trim().to_owned())
        );

        if let Some(icons_map) = &config.icons {
            for (re, icon_str) in icons_map {
                if re.is_match(&module.data.alt) {
                    icon_element = icon_raw(icon_str.trim().to_owned());
                    break;
                }
            }
        }

        if let Some(color) = state_color {
            icon_element = icon_element.color(color);
        }

        let padded_icon_container = container(icon_element);

        let mut show_alert = false;
        if let Some(re) = &config.alert
            && re.is_match(&module.data.alt)
        {
            show_alert = true;
        }

        if module.last_error.is_some() {
            show_alert = true;
        }

        let icon_with_alert: Element<'static, M> = if show_alert {
            let dot = appearance.spacing(ALERT_DOT_EM);
            let alert_canvas = canvas(AlertIndicator {
                radius: dot / 2.0
            })
            .width(Length::Fixed(dot))
            .height(Length::Fixed(dot));

            let alert_indicator_container = container(alert_canvas)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top);

            Stack::new()
                .push(padded_icon_container)
                .push(alert_indicator_container)
                .into()
        } else {
            padded_icon_container.into()
        };

        let maybe_text_element = if let Some(error) = &module.last_error {
            Some(text(error.to_display_message()))
        } else {
            module.data.text.as_ref().and_then(|text_content| {
                let trimmed = text_content.trim();

                if trimmed.is_empty() {
                    None
                } else {
                    Some(text(trimmed.to_owned()))
                }
            })
        };

        let maybe_text_element = maybe_text_element.map(|text_element| match state_color {
            Some(color) => text_element.color(color),
            None => text_element
        });

        let row_content: Element<'static, M> = if let Some(text_element) = maybe_text_element {
            row![icon_with_alert, text_element]
                .spacing(appearance.icon_label_gap())
                .into()
        } else {
            icon_with_alert
        };

        row_content
    }

    impl Custom {
        /// Text the module asks the bar to show while the pointer rests on it.
        ///
        /// The bar surface is only as tall as the bar, so the hint cannot be
        /// drawn as an overlay next to the module without covering it.
        /// It is handed to the tooltip surface instead, which the
        /// compositor lays out beside the bar.
        pub fn tooltip(&self) -> Option<&str> {
            match self.data.tooltip.as_deref() {
                Some(hint) if !hint.is_empty() && self.last_error.is_none() => Some(hint),
                _ => None
            }
        }
    }
}

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
    M: 'static + Clone
{
    type ViewData<'a> = (&'a CustomModuleDef, &'a Appearance, &'a IconTheme);
    type RegistrationData<'a> = Option<&'a CustomModuleDef>;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.start_listener(ctx, config)
    }

    fn view(
        &self,
        (config, appearance, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((view::render(self, config, appearance, icons), None))
    }
}

#[cfg(test)]
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
                        // the listener suppresses repeats, so the replacement
                        // publishes its payload once and then stays quiet
                        if let Message::Event(ServiceEvent::Update(data)) = message {
                            alts.push((name, data.alt));
                            break alts;
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
        let color = AppearanceColor::deserialize(StrDeserializer::<serde::de::value::Error>::new(
            "#ffa500"
        ))
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
    fn is_alive(pid: u32) -> bool {
        unsafe { libc::kill(pid as i32, 0) == 0 }
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
        <Custom as Module<Message>>::register(&mut custom, &context, Some(&definition))
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
        <Custom as Module<Message>>::register(&mut custom, &context, Some(&first))
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

        <Custom as Module<Message>>::register(&mut custom, &context, Some(&second))
            .expect("second register");

        timeout(Duration::from_secs(5), awaits_death(helper))
            .await
            .expect("the helper of the replaced listener dies");
    }
}
