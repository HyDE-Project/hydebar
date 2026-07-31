mod commands {
    use std::process::{ExitStatus, Stdio};

    use tokio::process;

    use super::state::Update;

    /// Errors that can occur while executing an update-related shell command.
    #[derive(Debug)]
    pub(super) enum CommandError {
        /// Failed to spawn the command.
        Io(std::io::Error),
        /// The command exited with a non-zero status.
        Status(ExitStatus)
    }

    impl std::fmt::Display for CommandError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Io(_) => write!(f, "failed to execute command"),
                Self::Status(status) => write!(f, "command exited with failure status: {status}")
            }
        }
    }

    impl std::error::Error for CommandError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Io(err) => Some(err),
                Self::Status(_) => None
            }
        }
    }

    impl From<std::io::Error> for CommandError {
        fn from(err: std::io::Error) -> Self {
            Self::Io(err)
        }
    }

    impl CommandError {
        pub(super) fn or_log(self, context: &str) {
            log::warn!("{context}: {self}");
        }
    }

    /// Why a check could not be trusted.
    #[derive(Debug)]
    pub(super) enum CheckFailure {
        /// The check cannot be run on this machine at all.
        ///
        /// A configuration naming a package manager the machine does not have
        /// is not a fault to report every hour; it means the bar has
        /// nothing to show.
        Unavailable(CommandError),
        /// The check ran but this particular run said nothing usable.
        Transient(CommandError)
    }

    impl std::fmt::Display for CheckFailure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Unavailable(err) | Self::Transient(err) => write!(f, "{err}")
            }
        }
    }

    /// Exit status a shell reports when the command it was asked to run is
    /// missing.
    const COMMAND_NOT_FOUND: i32 = 127;

    /// Asks the package manager what is out of date.
    ///
    /// The command is spawned into a process group of its own, so a check still
    /// talking to a mirror when the schedule is torn down goes with it instead
    /// of outliving the task that started it.
    ///
    /// A failing exit status alone does not make the answer worthless: the
    /// usual check pipelines end in a query that reports "nothing to do" by
    /// failing, and treating that as an error left the bar showing a list
    /// it had already been told was empty. Output that parses is therefore
    /// believed whatever the status, and a status without output is only a
    /// failure when the command also complained about something.
    pub(super) async fn check_for_updates(command: &str) -> Result<Vec<Update>, CheckFailure> {
        let mut spawner = process::Command::new("bash");
        spawner
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = crate::utils::process_group::guarded_output(&mut spawner)
            .await
            .map_err(|err| CheckFailure::Unavailable(CommandError::Io(err)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let updates = parse_updates(stdout.trim_end_matches('\n'));

        classify(&output.status, &updates, &output.stderr)?;

        Ok(updates)
    }

    /// Decides whether a finished check is worth believing.
    ///
    /// Split out of the spawn so the policy can be exercised without a shell.
    fn classify(
        status: &ExitStatus,
        updates: &[Update],
        stderr: &[u8]
    ) -> Result<(), CheckFailure> {
        if status.success() || !updates.is_empty() {
            return Ok(());
        }

        if status.code() == Some(COMMAND_NOT_FOUND) {
            return Err(CheckFailure::Unavailable(CommandError::Status(*status)));
        }

        if stderr.iter().any(|byte| !byte.is_ascii_whitespace()) {
            return Err(CheckFailure::Transient(CommandError::Status(*status)));
        }

        Ok(())
    }

    pub(super) async fn apply_updates(command: &str) -> Result<(), CommandError> {
        let output = process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;

        if !output.success() {
            return Err(CommandError::Status(output));
        }

        Ok(())
    }

    fn parse_updates(output: &str) -> Vec<Update> {
        output.lines().filter_map(parse_update_line).collect()
    }

    fn parse_update_line(line: &str) -> Option<Update> {
        let mut tokens = line.split_whitespace();
        let package = tokens.next()?;
        let from = tokens.next()?;
        let separator = tokens.next()?;
        let to = tokens.next()?;

        if separator != "->" {
            return None;
        }

        Some(Update {
            package: package.to_owned(),
            from:    from.to_owned(),
            to:      to.to_owned()
        })
    }

    #[cfg(test)]
    mod tests {
        use std::os::unix::process::ExitStatusExt;

        use super::*;

        fn status(code: i32) -> ExitStatus {
            ExitStatus::from_raw(code << 8)
        }

        fn one_update() -> Vec<Update> {
            vec![Update {
                package: "pkg".to_owned(),
                from:    "1".to_owned(),
                to:      "2".to_owned()
            }]
        }

        #[test]
        fn a_successful_check_is_believed() {
            assert!(classify(&status(0), &[], b"").is_ok());
        }

        /// `checkupdates; paru -Qua` ends in a query that fails when the AUR
        /// has nothing, and silence from both is the shape of
        /// "everything is current".
        #[test]
        fn a_silent_failure_means_nothing_is_out_of_date() {
            assert!(classify(&status(1), &[], b"\n").is_ok());
        }

        #[test]
        fn output_that_parses_outweighs_a_failing_status() {
            assert!(classify(&status(1), &one_update(), b"mirror is slow").is_ok());
        }

        #[test]
        fn a_missing_command_makes_the_check_unavailable() {
            assert!(matches!(
                classify(
                    &status(COMMAND_NOT_FOUND),
                    &[],
                    b"bash: checkupdates: not found"
                ),
                Err(CheckFailure::Unavailable(_))
            ));
        }

        #[test]
        fn a_complaint_without_output_is_a_passing_failure() {
            assert!(matches!(
                classify(&status(1), &[], b"could not reach the mirror"),
                Err(CheckFailure::Transient(_))
            ));
        }

        #[test]
        fn parse_updates_skips_malformed_lines() {
            let output = "pkg1 1 -> 2\ninvalid line\npkg2 3 -> 4";

            let updates = parse_updates(output);

            assert_eq!(updates.len(), 2);
            assert_eq!(updates[0].package, "pkg1");
            assert_eq!(updates[1].package, "pkg2");
        }

        #[test]
        fn parse_updates_handles_empty_input() {
            let updates = parse_updates("");

            assert!(updates.is_empty());
        }
    }
}
mod state {
    use std::{sync::Arc, time::Duration};

    use iced::{Element, SurfaceId as Id};
    use log::{debug, error, info, warn};
    use tokio::{runtime::Handle, sync::Notify, task::JoinHandle, time::sleep};

    use super::{commands, commands::CheckFailure, view};
    use crate::{
        ModuleContext, ModuleEventSender,
        components::icons::IconTheme,
        config::UpdatesModuleConfig,
        event_bus::ModuleEvent,
        menu::MenuType,
        modules::{Module, ModuleError, OnModulePress},
        outputs::Outputs
    };

    /// Shortest interval a configuration is allowed to ask for.
    ///
    /// Every check spawns a package manager that talks to a mirror. A
    /// configuration asking for one a second would be answered by the mirror
    /// rather than by the bar, so the floor is enforced here instead of
    /// trusting the file.
    const MIN_INTERVAL: Duration = Duration::from_secs(60);

    /// Longest a single check may run before it is given up on.
    ///
    /// A package manager waiting on an unreachable mirror can hang for as long
    /// as its own timeouts allow, and the schedule has only one runner:
    /// without a deadline of its own a single stuck check would silence the
    /// module until the bar restarts.
    const CHECK_TIMEOUT: Duration = Duration::from_secs(300);

    /// Identical failures passed over between two log lines.
    ///
    /// A mirror that is down stays down, and the check meets it once per
    /// interval. Writing the same line every time buries everything else in
    /// the journal, so the repetitions are counted and reported in one line
    /// instead.
    const FAILURE_REPEAT: u32 = 12;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Update {
        pub(super) package: String,
        pub(super) from:    String,
        pub(super) to:      String
    }

    #[derive(Debug, Clone)]
    pub enum Message {
        UpdatesCheckCompleted(Vec<Update>),
        /// A check ran but could not be trusted; what is known already stands.
        CheckFailed,
        /// The configured check cannot be run on this machine.
        UpdatesUnavailable,
        UpdateFinished,
        ToggleUpdatesList,
        CheckNow,
        Update(Id)
    }

    #[derive(Debug, Default, Clone, Eq, PartialEq)]
    pub(crate) enum CheckState {
        #[default]
        Checking,
        Ready,
        /// No check can be run here, so the bar has no update count to show.
        Unavailable
    }

    #[derive(Default)]
    pub struct Updates {
        state:                    CheckState,
        updates:                  Vec<Update>,
        pub is_updates_list_open: bool,
        update_command:           Option<Arc<str>>,
        sender:                   Option<ModuleEventSender<Message>>,
        runtime:                  Option<Handle>,
        schedule:                 Option<Schedule>
    }

    impl Updates {
        /// Hint shown while the pointer rests on the bar entry.
        ///
        /// Nothing is shown where no check can run: the entry itself is absent
        /// from such a bar, so there is nothing to explain.
        #[must_use]
        pub fn tooltip(&self) -> Option<String> {
            match self.state {
                CheckState::Checking => Some("Updates: checking".to_owned()),
                CheckState::Ready => Some(match self.updates.len() {
                    0 => "Updates: none pending".to_owned(),
                    pending => format!("Updates: {pending} pending")
                }),
                CheckState::Unavailable => None
            }
        }
    }

    impl std::fmt::Debug for Updates {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Updates")
                .field("state", &self.state)
                .field("updates", &self.updates)
                .field("is_updates_list_open", &self.is_updates_list_open)
                .field("update_command", &self.update_command)
                .field("sender", &self.sender)
                .field("runtime", &self.runtime)
                .field("schedule", &self.schedule)
                .finish()
        }
    }

    impl Clone for Updates {
        fn clone(&self) -> Self {
            Self {
                state:                self.state.clone(),
                updates:              self.updates.clone(),
                is_updates_list_open: self.is_updates_list_open,
                update_command:       self.update_command.clone(),
                sender:               self.sender.clone(),
                runtime:              self.runtime.clone(),
                schedule:             None
            }
        }
    }

    /// The one task that ever runs the check command.
    ///
    /// It outlives every configuration reload that leaves the check unchanged,
    /// and it is the only place a check is started from: the manual button
    /// wakes it rather than spawning a second runner, so two checks can
    /// never overlap and a reload can never leave a half-finished one
    /// behind.
    struct Schedule {
        /// Check command this schedule was started for.
        command:  Arc<str>,
        /// Time between the end of one check and the start of the next.
        interval: Duration,
        /// Wake-up the manual button rings.
        wake:     Arc<Notify>,
        /// The running task, aborted when this schedule is dropped.
        handle:   JoinHandle<()>
    }

    impl std::fmt::Debug for Schedule {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Schedule")
                .field("command", &self.command)
                .field("interval", &self.interval)
                .finish()
        }
    }

    impl Schedule {
        /// Starts the check loop on `runtime`.
        fn start(
            runtime: &Handle,
            sender: ModuleEventSender<Message>,
            command: Arc<str>,
            interval: Duration
        ) -> Self {
            let wake = Arc::new(Notify::new());
            let loop_wake = Arc::clone(&wake);
            let loop_command = Arc::clone(&command);

            let handle = runtime.spawn(check_loop(sender, loop_command, interval, loop_wake));

            Self {
                command,
                interval,
                wake,
                handle
            }
        }

        /// Reports whether this schedule already does what is being asked for.
        fn matches(&self, command: &str, interval: Duration) -> bool {
            self.command.as_ref() == command && self.interval == interval
        }

        /// Asks for a check as soon as the runner is free.
        ///
        /// Repeated requests collapse into one, and a request arriving during a
        /// check is answered once that check is done rather than beside it.
        fn request_check(&self) {
            self.wake.notify_one();
        }
    }

    impl Drop for Schedule {
        fn drop(&mut self) {
            self.handle.abort();
        }
    }

    /// Runs the check command forever, once per interval and on request.
    async fn check_loop(
        sender: ModuleEventSender<Message>,
        command: Arc<str>,
        interval: Duration,
        wake: Arc<Notify>
    ) {
        let mut failures = FailureLog::default();

        loop {
            let outcome = check_once(command.as_ref(), &mut failures).await;

            if let Err(err) = sender.try_send(outcome) {
                error!("failed to publish the updates check result: {err}");
            }

            tokio::select! {
                () = sleep(interval) => {}
                () = wake.notified() => {}
            }
        }
    }

    /// Runs the check command once and turns whatever happened into a message.
    ///
    /// A failure never leaves the loop: the module reports what it can and the
    /// next interval tries again.
    async fn check_once(command: &str, failures: &mut FailureLog) -> Message {
        match tokio::time::timeout(CHECK_TIMEOUT, commands::check_for_updates(command)).await {
            Ok(Ok(updates)) => {
                failures.clear();

                Message::UpdatesCheckCompleted(updates)
            }
            Ok(Err(CheckFailure::Unavailable(err))) => {
                report(failures, format!("the check command cannot be run: {err}"));

                Message::UpdatesUnavailable
            }
            Ok(Err(CheckFailure::Transient(err))) => {
                report(failures, format!("the check command failed: {err}"));

                Message::CheckFailed
            }
            Err(_) => {
                report(
                    failures,
                    format!("the check command did not finish within {CHECK_TIMEOUT:?}")
                );

                Message::CheckFailed
            }
        }
    }

    /// Writes a failure to the journal unless it is the same one all over
    /// again.
    fn report(failures: &mut FailureLog, reason: String) {
        match failures.record(&reason) {
            Some(1) => warn!("updates check failed: {reason}"),
            Some(count) => warn!("updates check has failed {count} times in a row: {reason}"),
            None => debug!("updates check failed again: {reason}")
        }
    }

    /// Counter that keeps an unchanging failure out of the journal.
    #[derive(Debug, Default)]
    struct FailureLog {
        /// The failure the run before this one reported.
        last:    Option<String>,
        /// How many times it has been reported in a row.
        repeats: u32
    }

    impl FailureLog {
        /// Records a failure, reporting the count when it deserves a log line.
        ///
        /// A failure that differs from the last one is always worth a line; an
        /// identical one only every [`FAILURE_REPEAT`] occurrences, so a
        /// lasting fault is visible without being the only thing in the
        /// journal.
        fn record(&mut self, reason: &str) -> Option<u32> {
            if self.last.as_deref() == Some(reason) {
                self.repeats += 1;

                return self
                    .repeats
                    .is_multiple_of(FAILURE_REPEAT)
                    .then_some(self.repeats);
            }

            self.last = Some(reason.to_owned());
            self.repeats = 1;

            Some(1)
        }

        /// Forgets the last failure after a check that worked.
        fn clear(&mut self) {
            self.last = None;
            self.repeats = 0;
        }
    }

    /// Interval the configuration asks for, never below the floor.
    fn check_interval(config: &UpdatesModuleConfig) -> Duration {
        Duration::from_secs(config.check_interval).max(MIN_INTERVAL)
    }

    impl Updates {
        pub fn update(
            &mut self,
            message: Message,
            _config: &UpdatesModuleConfig,
            outputs: &mut Outputs,
            main_config: &crate::config::Config
        ) {
            match message {
                Message::CheckNow => match self.schedule.as_ref() {
                    Some(schedule) => {
                        self.state = CheckState::Checking;
                        schedule.request_check();
                    }
                    None => warn!("the updates module has no schedule; skipping the manual check")
                },
                Message::Update(id) => {
                    if let (Some(runtime), Some(sender), Some(update_command)) = (
                        self.runtime.clone(),
                        self.sender.clone(),
                        self.update_command.clone()
                    ) {
                        runtime.spawn(async move {
                            if let Err(err) =
                                commands::apply_updates(update_command.as_ref()).await
                            {
                                err.or_log("failed to execute update command");
                            }

                            if let Err(err) = sender.try_send(Message::UpdateFinished) {
                                error!("failed to publish update completion: {err}");
                            }
                        });
                    } else {
                        warn!("updates module is not fully initialised; skipping update command");
                    }

                    let _ = outputs.close_menu_if::<Message>(id, MenuType::Updates, main_config);
                }
                observed => self.observe(observed)
            }
        }

        /// Folds everything a check reports into what the bar shows.
        ///
        /// Kept apart from [`Updates::update`] because these are the
        /// transitions that need neither a window to close a menu on
        /// nor a runtime to spawn a command into.
        fn observe(&mut self, message: Message) {
            match message {
                Message::UpdatesCheckCompleted(updates) => {
                    self.updates = updates;
                    self.state = CheckState::Ready;
                }
                Message::CheckFailed => self.state = CheckState::Ready,
                Message::UpdatesUnavailable => {
                    self.updates.clear();
                    self.state = CheckState::Unavailable;
                }
                Message::UpdateFinished => {
                    self.updates.clear();
                    self.state = CheckState::Ready;
                }
                Message::ToggleUpdatesList => {
                    self.is_updates_list_open = !self.is_updates_list_open;
                }
                Message::CheckNow | Message::Update(_) => {}
            }
        }

        pub fn menu_view(&self, id: Id, opacity: f32, icons: &IconTheme) -> Element<'_, Message> {
            view::menu_view(self, id, opacity, icons)
        }

        pub(crate) fn updates(&self) -> &[Update] {
            &self.updates
        }

        pub(crate) fn is_updates_list_open(&self) -> bool {
            self.is_updates_list_open
        }

        pub(crate) fn state(&self) -> &CheckState {
            &self.state
        }

        /// Ends the schedule and forgets what it was started for.
        fn stop(&mut self) {
            self.schedule = None;
            self.update_command = None;
            self.sender = None;
        }
    }

    impl<M> Module<M> for Updates
    where
        M: 'static + Clone + From<Message>
    {
        type ViewData<'a> = (&'a Option<UpdatesModuleConfig>, &'a IconTheme);
        type RegistrationData<'a> = Option<&'a UpdatesModuleConfig>;

        /// Makes sure exactly one check schedule is running for `config`.
        ///
        /// Registration happens again after every configuration reload, and the
        /// desktop reloads for reasons that have nothing to do with updates. A
        /// schedule already checking the same command on the same interval is
        /// therefore left alone: tearing it down and starting another one would
        /// cancel the check in flight, and a package manager killed halfway
        /// leaves helpers of its own behind, once per reload.
        fn register(
            &mut self,
            ctx: &ModuleContext,
            config: Self::RegistrationData<'_>
        ) -> Result<(), ModuleError> {
            let Some(definition) = config else {
                self.stop();

                return Ok(());
            };

            self.runtime = Some(ctx.runtime_handle().clone());

            let check_command: Arc<str> = Arc::from(definition.check_cmd.as_str());
            let interval = check_interval(definition);

            self.update_command = Some(Arc::from(definition.update_cmd.as_str()));

            if self
                .schedule
                .as_ref()
                .is_some_and(|schedule| schedule.matches(&check_command, interval))
            {
                debug!("the updates schedule outlived a configuration reload");

                return Ok(());
            }

            self.schedule = None;

            let sender = ctx.module_sender(ModuleEvent::Updates);
            self.sender = Some(sender.clone());
            self.schedule = Some(Schedule::start(
                ctx.runtime_handle(),
                sender,
                check_command,
                interval
            ));

            info!("checking for updates every {interval:?}");

            Ok(())
        }

        /// Stops the scheduled check once the indicator leaves the bar.
        ///
        /// Each check spawns a shell command that talks to the package manager,
        /// so an unplaced module would keep forking a process every
        /// interval for a badge nobody renders.
        fn deregister(&mut self) {
            self.stop();
        }

        fn view(
            &self,
            (config, icons): Self::ViewData<'_>
        ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
            if config.is_none() || self.state == CheckState::Unavailable {
                return None;
            }

            Some((
                view::icon(&self.state, self.updates.len(), icons).map(M::from),
                Some(OnModulePress::ToggleMenu(MenuType::Updates))
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{
            fs,
            num::NonZeroUsize,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH}
        };

        use tokio::runtime::Runtime;

        use super::*;
        use crate::event_bus::EventBus;

        fn context(runtime: &Runtime) -> (EventBus, ModuleContext) {
            let bus = EventBus::new(NonZeroUsize::new(16).expect("capacity"));
            let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());

            (bus, ctx)
        }

        fn config(check: &str, interval: u64) -> UpdatesModuleConfig {
            UpdatesModuleConfig {
                check_cmd:      check.to_owned(),
                update_cmd:     ":".to_owned(),
                check_interval: interval
            }
        }

        fn task_id(updates: &Updates) -> tokio::task::Id {
            updates
                .schedule
                .as_ref()
                .expect("a running schedule")
                .handle
                .id()
        }

        #[test]
        fn a_reload_that_changes_nothing_keeps_the_running_check() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();
            let definition = config("sleep 30", 3600);

            <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
                .expect("the first registration succeeds");
            let first = task_id(&updates);

            <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
                .expect("the second registration succeeds");

            assert_eq!(first, task_id(&updates));
        }

        /// The update command may be edited without touching the check, and
        /// that alone must not disturb a check in flight.
        #[test]
        fn a_reload_that_only_changes_the_update_command_keeps_the_check() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();
            let mut definition = config("sleep 30", 3600);

            <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
                .expect("the first registration succeeds");
            let first = task_id(&updates);

            definition.update_cmd = "pacman -Syu".to_owned();
            <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
                .expect("the second registration succeeds");

            assert_eq!(first, task_id(&updates));
            assert_eq!(
                updates
                    .update_command
                    .as_deref()
                    .expect("a recorded update command"),
                "pacman -Syu"
            );
        }

        #[test]
        fn a_changed_check_command_replaces_the_schedule() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 30", 3600))
            )
            .expect("the first registration succeeds");
            let first = task_id(&updates);

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 40", 3600))
            )
            .expect("the second registration succeeds");

            assert_ne!(first, task_id(&updates));
        }

        #[test]
        fn a_changed_interval_replaces_the_schedule() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 30", 3600))
            )
            .expect("the first registration succeeds");
            let first = task_id(&updates);

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 30", 1800))
            )
            .expect("the second registration succeeds");

            assert_ne!(first, task_id(&updates));
        }

        #[test]
        fn a_replaced_schedule_ends_the_task_it_replaced() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 30", 3600))
            )
            .expect("the first registration succeeds");
            let first = updates
                .schedule
                .as_ref()
                .expect("a running schedule")
                .handle
                .abort_handle();

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 40", 3600))
            )
            .expect("the second registration succeeds");

            runtime.block_on(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            });

            assert!(first.is_finished());
        }

        #[test]
        fn a_module_without_a_configuration_schedules_nothing() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();

            <Updates as Module<Message>>::register(&mut updates, &ctx, None)
                .expect("registration succeeds without a configuration");

            assert!(updates.schedule.is_none());
            assert!(updates.update_command.is_none());
        }

        #[test]
        fn losing_the_configuration_ends_the_running_schedule() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 30", 3600))
            )
            .expect("the first registration succeeds");
            let handle = updates
                .schedule
                .as_ref()
                .expect("a running schedule")
                .handle
                .abort_handle();

            <Updates as Module<Message>>::register(&mut updates, &ctx, None)
                .expect("registration succeeds without a configuration");

            runtime.block_on(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            });

            assert!(updates.schedule.is_none());
            assert!(handle.is_finished());
        }

        #[test]
        fn deregistering_ends_the_schedule() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let mut updates = Updates::default();

            <Updates as Module<Message>>::register(
                &mut updates,
                &ctx,
                Some(&config("sleep 30", 3600))
            )
            .expect("registration succeeds");
            let handle = updates
                .schedule
                .as_ref()
                .expect("a running schedule")
                .handle
                .abort_handle();

            <Updates as Module<Message>>::deregister(&mut updates);

            runtime.block_on(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            });

            assert!(updates.schedule.is_none());
            assert!(handle.is_finished());
        }

        /// The runner is single: a check that takes longer than the interval
        /// delays the next one instead of running beside it.
        #[test]
        fn two_checks_never_run_at_once() {
            let runtime = Runtime::new().expect("runtime");
            let (_bus, ctx) = context(&runtime);
            let trace = trace_path();
            let _ = fs::remove_file(&trace);

            let command = format!(
                "printf 'in\\n' >> {trace}; sleep 0.2; printf 'out\\n' >> {trace}",
                trace = trace.display()
            );

            let schedule = Schedule::start(
                ctx.runtime_handle(),
                ctx.module_sender(ModuleEvent::Updates),
                Arc::from(command.as_str()),
                Duration::from_millis(10)
            );

            runtime.block_on(async {
                for _ in 0..20 {
                    schedule.request_check();
                    tokio::time::sleep(Duration::from_millis(45)).await;
                }
            });

            drop(schedule);

            let recorded = fs::read_to_string(&trace).expect("the checks left a trace");
            let _ = fs::remove_file(&trace);

            let mut inside = false;
            let mut runs = 0;

            for line in recorded.lines() {
                match line {
                    "in" => {
                        assert!(!inside, "a check started while another was running");
                        inside = true;
                        runs += 1;
                    }
                    "out" => {
                        assert!(inside, "a check ended without having started");
                        inside = false;
                    }
                    other => panic!("unexpected trace line {other:?}")
                }
            }

            assert!(runs >= 2, "the schedule ran {runs} times, expected repeats");
        }

        fn trace_path() -> PathBuf {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos());

            std::env::temp_dir().join(format!("hydebar-updates-{}-{stamp}", std::process::id()))
        }

        #[test]
        fn the_interval_never_drops_below_the_floor() {
            assert_eq!(check_interval(&config("true", 0)), MIN_INTERVAL);
            assert_eq!(check_interval(&config("true", 1)), MIN_INTERVAL);
            assert_eq!(
                check_interval(&config("true", 7200)),
                Duration::from_secs(7200)
            );
        }

        #[test]
        fn the_first_failure_of_its_kind_is_always_reported() {
            let mut failures = FailureLog::default();

            assert_eq!(failures.record("mirror is down"), Some(1));
            assert_eq!(failures.record("database is locked"), Some(1));
        }

        #[test]
        fn the_same_failure_is_reported_once_in_a_while() {
            let mut failures = FailureLog::default();

            assert_eq!(failures.record("mirror is down"), Some(1));

            for _ in 2..FAILURE_REPEAT {
                assert_eq!(failures.record("mirror is down"), None);
            }

            assert_eq!(failures.record("mirror is down"), Some(FAILURE_REPEAT));
        }

        #[test]
        fn a_check_that_works_forgets_the_failure_before_it() {
            let mut failures = FailureLog::default();

            assert_eq!(failures.record("mirror is down"), Some(1));
            failures.clear();

            assert_eq!(failures.record("mirror is down"), Some(1));
        }

        #[test]
        fn an_unavailable_check_takes_the_indicator_off_the_bar() {
            let mut updates = Updates::default();
            updates.state = CheckState::Unavailable;
            let icons = IconTheme::default();
            let config = Some(config("true", 3600));

            assert!(<Updates as Module<Message>>::view(&updates, (&config, &icons)).is_none());
        }

        #[test]
        fn a_failed_check_keeps_what_the_bar_already_knows() {
            let mut updates = Updates::default();
            updates.updates = vec![Update {
                package: "pkg".to_owned(),
                from:    "1".to_owned(),
                to:      "2".to_owned()
            }];
            updates.state = CheckState::Checking;

            updates.observe(Message::CheckFailed);

            assert_eq!(updates.updates.len(), 1);
            assert_eq!(updates.state, CheckState::Ready);
        }

        #[test]
        fn an_unavailable_check_clears_what_the_bar_knew() {
            let mut updates = Updates::default();
            updates.updates = vec![Update {
                package: "pkg".to_owned(),
                from:    "1".to_owned(),
                to:      "2".to_owned()
            }];

            updates.observe(Message::UpdatesUnavailable);

            assert!(updates.updates.is_empty());
            assert_eq!(updates.state, CheckState::Unavailable);
        }
    }
}
mod view {
    use std::borrow::Cow;

    use iced::{
        Alignment, Element, Length, Padding, SurfaceId as Id,
        alignment::Horizontal,
        widget::{Column, button, column, container, row, rule, scrollable}
    };

    use super::state::{CheckState, Message, Updates};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon as icon_component},
            scale,
            text::text
        },
        style::ghost_button_style
    };

    pub(super) fn menu_view<'a>(
        updates: &'a Updates,
        id: Id,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'a, Message> {
        column!(
            if updates.updates().is_empty() {
                container(text("Up to date ;)"))
                    .padding([scale::scaled(8.0), scale::scaled(8.0)])
                    .into()
            } else {
                build_updates_list(updates, opacity, icons)
            },
            rule::horizontal(1),
            action_button("Update", Message::Update(id), opacity),
            check_now_button(updates, opacity, icons),
        )
        .spacing(scale::scaled(4.0))
        .into()
    }

    pub(super) fn icon(
        state: &CheckState,
        update_count: usize,
        icons: &IconTheme
    ) -> Element<'static, Message> {
        let icon = match state {
            CheckState::Checking => Icons::Refresh,
            CheckState::Unavailable => Icons::NoUpdatesAvailable,
            CheckState::Ready if update_count == 0 => Icons::NoUpdatesAvailable,
            CheckState::Ready => Icons::UpdatesAvailable
        };

        let mut content = row!(container(icon_component(icons, icon)))
            .align_y(Alignment::Center)
            .spacing(scale::icon_gap());

        if update_count > 0 {
            content = content.push(text(update_count));
        }

        content.into()
    }

    fn build_updates_list<'a>(
        updates: &'a Updates,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'a, Message> {
        let mut elements = column!(
            button(row!(
                text(format!("{} Updates available", updates.updates().len())).width(Length::Fill),
                icon_component(
                    icons,
                    if updates.is_updates_list_open() {
                        Icons::MenuClosed
                    } else {
                        Icons::MenuOpen
                    }
                )
            ))
            .style(ghost_button_style(opacity))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
            .on_press(Message::ToggleUpdatesList)
            .width(Length::Fill),
        );

        if updates.is_updates_list_open() {
            elements = elements.push(
                container(scrollable(
                    Column::with_children(
                        updates
                            .updates()
                            .iter()
                            .map(|update| build_update_entry(update))
                            .collect::<Vec<Element<'_, Message>>>()
                    )
                    .padding(Padding::ZERO.right(16))
                    .spacing(scale::scaled(4.0))
                ))
                .padding([scale::scaled(8.0), scale::scaled(0.0)])
                .max_height(300)
            );
        }

        elements.into()
    }

    fn build_update_entry(update: &super::state::Update) -> Element<'_, Message> {
        column!(
            text(update.package.as_str())
                .size(scale::scaled(10.0))
                .width(Length::Fill),
            text(format!(
                "{} -> {}",
                truncated(&update.from, 18),
                truncated(&update.to, 18)
            ))
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .size(scale::scaled(10.0)),
        )
        .into()
    }

    fn action_button<'a>(
        label: &'a str,
        message: Message,
        opacity: f32
    ) -> iced::widget::Button<'a, Message> {
        button(label)
            .style(ghost_button_style(opacity))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
            .on_press(message)
            .width(Length::Fill)
    }

    fn check_now_button(
        updates: &Updates,
        opacity: f32,
        icons: &IconTheme
    ) -> iced::widget::Button<'static, Message> {
        let mut content = row!(text("Check now").width(Length::Fill));

        if matches!(updates.state(), CheckState::Checking) {
            content = content.push(icon_component(icons, Icons::Refresh));
        }

        button(content)
            .style(ghost_button_style(opacity))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
            .on_press(Message::CheckNow)
            .width(Length::Fill)
    }

    fn truncated(value: &str, max: usize) -> Cow<'_, str> {
        if value.chars().count() <= max {
            Cow::Borrowed(value)
        } else {
            Cow::Owned(value.chars().take(max).collect())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn truncated_returns_borrowed_when_short_enough() {
            let value = "short";

            assert!(matches!(truncated(value, 10), Cow::Borrowed("short")));
        }

        #[test]
        fn truncated_returns_owned_when_too_long() {
            let value = "averylongstring";

            let truncated = truncated(value, 5);

            assert!(matches!(truncated, Cow::Owned(ref owned) if owned == "avery"));
        }
    }
}

pub use state::{Message, Updates};
