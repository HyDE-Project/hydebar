use std::sync::Arc;

use iced::{Element, SurfaceId as Id};
use log::{debug, error, info, warn};
use tokio::runtime::Handle;

use super::{commands, view};
use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::IconTheme,
    config::UpdatesModuleConfig,
    event_bus::ModuleEvent,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress},
    outputs::Outputs
};

mod failures;
mod hyde_clone;
mod schedule;

use hyde_clone::find_hyde_clone;
use schedule::{Schedule, check_interval};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub(super) package: String,
    pub(super) from:    String,
    pub(super) to:      String
}

/// What one look at the `HyDE` clone reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydeSnapshot {
    /// Version the clone describes itself as.
    pub(crate) version: String,
    /// Subjects of the upstream commits the clone has not taken yet.
    pub(crate) commits: Vec<String>
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdatesCheckCompleted(Vec<Update>),
    /// A check ran but could not be trusted; what is known already stands.
    CheckFailed,
    /// The configured check cannot be run on this machine.
    UpdatesUnavailable,
    /// The package update ended, well or badly.
    UpdateFinished {
        failed: bool
    },
    /// The last lines the running package update printed.
    UpdateLog(Vec<String>),
    ToggleUpdatesList,
    CheckNow,
    /// Apply the configured update command, narrating into the window.
    Update,
    /// The `HyDE` clone was compared against upstream.
    HydeChecked(HydeSnapshot),
    ToggleHydeList,
    /// Bring the `HyDE` clone up to date, narrating into the window.
    UpdateHyde,
    /// The last lines the running `HyDE` update printed.
    HydeUpdateLog(Vec<String>),
    /// The `HyDE` update ended, well or badly.
    HydeUpdateFinished {
        failed: bool
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum CheckState {
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
    is_hyde_list_open:        bool,
    hyde:                     Option<HydeSnapshot>,
    hyde_clone:               Option<Arc<str>>,
    hyde_branch:              Option<Arc<str>>,
    hyde_updating:            bool,
    hyde_failed:              bool,
    hyde_log:                 Vec<String>,
    applying:                 bool,
    apply_failed:             bool,
    apply_log:                Vec<String>,
    update_command:           Option<Arc<str>>,
    sender:                   Option<ModuleEventSender<Message>>,
    runtime:                  Option<Handle>,
    schedule:                 Option<Schedule>,
    shown_count:              crate::components::crossfade::Crossfade
}

impl Updates {
    /// Hint shown while the pointer rests on the bar entry.
    ///
    /// Nothing is shown where no check can run: the entry itself is absent
    /// from such a bar, so there is nothing to explain.
    #[must_use]
    pub fn tooltip(&self) -> Option<String> {
        let line = match self.state {
            CheckState::Checking => "Updates: checking".to_owned(),
            CheckState::Ready => match self.updates.len() {
                0 => "Updates: none pending".to_owned(),
                pending => format!("Updates: {pending} pending")
            },
            CheckState::Unavailable => return None
        };

        Some(match self.hyde_pending() {
            0 => line,
            behind => format!("{line} · HyDE: {behind} commits behind")
        })
    }

    /// Folds both open lists shut, the state a freshly opened menu shows.
    ///
    /// Leftover narration goes with them: a log that outlived its run was
    /// read in the window that witnessed it, and a window opened anew
    /// should not still be reporting last time's weather. A run still
    /// going keeps its lines.
    pub fn collapse(&mut self) {
        self.is_updates_list_open = false;
        self.is_hyde_list_open = false;

        if !self.applying {
            self.apply_log.clear();
            self.apply_failed = false;
        }

        if !self.hyde_updating {
            self.hyde_log.clear();
            self.hyde_failed = false;
        }
    }

    /// How many upstream commits the `HyDE` clone has not taken yet.
    fn hyde_pending(&self) -> usize {
        self.hyde
            .as_ref()
            .map_or(0, |snapshot| snapshot.commits.len())
    }
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates")
            .field("state", &self.state)
            .field("updates", &self.updates)
            .field("is_updates_list_open", &self.is_updates_list_open)
            .field("is_hyde_list_open", &self.is_hyde_list_open)
            .field("hyde", &self.hyde)
            .field("hyde_clone", &self.hyde_clone)
            .field("hyde_branch", &self.hyde_branch)
            .field("hyde_updating", &self.hyde_updating)
            .field("hyde_failed", &self.hyde_failed)
            .field("hyde_log", &self.hyde_log)
            .field("applying", &self.applying)
            .field("apply_failed", &self.apply_failed)
            .field("apply_log", &self.apply_log)
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
            is_hyde_list_open:    self.is_hyde_list_open,
            hyde:                 self.hyde.clone(),
            hyde_clone:           self.hyde_clone.clone(),
            hyde_branch:          self.hyde_branch.clone(),
            hyde_updating:        self.hyde_updating,
            hyde_failed:          self.hyde_failed,
            hyde_log:             self.hyde_log.clone(),
            applying:             self.applying,
            apply_failed:         self.apply_failed,
            apply_log:            self.apply_log.clone(),
            update_command:       self.update_command.clone(),
            sender:               self.sender.clone(),
            runtime:              self.runtime.clone(),
            schedule:             None,
            shown_count:          self.shown_count.clone()
        }
    }
}
impl Updates {
    pub fn update(
        &mut self,
        message: Message,
        _config: &UpdatesModuleConfig,
        _outputs: &mut Outputs,
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
            Message::Update => {
                if self.applying {
                    debug!("a package update is already running");
                } else if let (Some(runtime), Some(sender), Some(update_command)) = (
                    self.runtime.clone(),
                    self.sender.clone(),
                    self.update_command.clone()
                ) {
                    self.applying = true;
                    self.apply_failed = false;
                    self.apply_log.clear();

                    let log_sender = sender.clone();

                    runtime.spawn(async move {
                        let publish = move |lines| {
                            let _ = log_sender.try_send(Message::UpdateLog(lines));
                        };

                        let failed =
                            match commands::apply_updates(update_command.as_ref(), publish)
                                .await
                            {
                                Ok(()) => false,
                                Err(err) => {
                                    err.or_log("the package update failed");

                                    true
                                }
                            };

                        if let Err(err) = sender.try_send(Message::UpdateFinished {
                            failed
                        }) {
                            error!("failed to publish update completion: {err}");
                        }
                    });
                } else {
                    warn!("updates module is not fully initialised; skipping update command");
                }
            }
            Message::UpdateFinished {
                failed
            } => {
                self.applying = false;
                self.apply_failed = failed;
                self.apply_log.push(
                    if failed {
                        "· the update failed"
                    } else {
                        "· the update finished"
                    }
                    .to_owned()
                );

                match self.schedule.as_ref() {
                    Some(schedule) => {
                        self.state = CheckState::Checking;
                        schedule.request_check();
                    }
                    None => self.state = CheckState::Ready
                }
            }
            Message::UpdateHyde => {
                if self.hyde_updating {
                    debug!("a hyde update is already running");
                } else if let (Some(runtime), Some(sender), Some(clone), Some(branch)) = (
                    self.runtime.clone(),
                    self.sender.clone(),
                    self.hyde_clone.clone(),
                    self.hyde_branch.clone()
                ) {
                    self.hyde_updating = true;
                    self.hyde_failed = false;
                    self.hyde_log.clear();

                    let log_sender = sender.clone();

                    runtime.spawn(async move {
                        let publish = move |lines| {
                            let _ = log_sender.try_send(Message::HydeUpdateLog(lines));
                        };

                        let failed = match commands::update_hyde(
                            clone.as_ref(),
                            branch.as_ref(),
                            publish
                        )
                        .await
                        {
                            Ok(()) => false,
                            Err(err) => {
                                err.or_log("the hyde update failed");

                                true
                            }
                        };

                        if let Err(err) = sender.try_send(Message::HydeUpdateFinished {
                            failed
                        }) {
                            error!("failed to publish the hyde update outcome: {err}");
                        }

                        if let Err(err) = sender.try_send(Message::CheckNow) {
                            error!("failed to ask for a check after the hyde update: {err}");
                        }
                    });
                } else {
                    warn!("no hyde clone is known; skipping the hyde update");
                }
            }
            observed => self.observe(observed)
        }

        self.shown_count.set(
            self.updates.len().to_string(),
            main_config.appearance.animations.enabled
        );
    }

    /// Advances the dissolve of the count on the bar.
    pub fn tick_fade(&mut self, elapsed: std::time::Duration) -> bool {
        self.shown_count.advance(elapsed)
    }

    /// Whether the count on the bar is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown_count.is_animating()
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

                if !self.applying && !self.apply_failed {
                    self.apply_log.clear();
                }
            }
            Message::CheckFailed => self.state = CheckState::Ready,
            Message::UpdatesUnavailable => {
                self.updates.clear();
                self.state = CheckState::Unavailable;
            }
            Message::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;
            }
            Message::HydeChecked(snapshot) => {
                self.hyde = Some(snapshot);

                if !self.hyde_updating && !self.hyde_failed {
                    self.hyde_log.clear();
                }
            }
            Message::ToggleHydeList => {
                self.is_hyde_list_open = !self.is_hyde_list_open;
            }
            Message::HydeUpdateLog(lines) => {
                if self.hyde_updating {
                    self.hyde_log = lines;
                }
            }
            Message::UpdateLog(lines) => {
                if self.applying {
                    self.apply_log = lines;
                }
            }
            Message::HydeUpdateFinished {
                failed
            } => {
                self.hyde_updating = false;
                self.hyde_failed = failed;
                self.hyde_log.push(
                    if failed {
                        "· the update failed"
                    } else {
                        "· the update finished"
                    }
                    .to_owned()
                );
            }
            Message::CheckNow
            | Message::Update
            | Message::UpdateFinished {
                ..
            }
            | Message::UpdateHyde => {}
        }
    }

    #[must_use]
    pub fn menu_view(&self, id: Id, opacity: f32, icons: &IconTheme) -> Element<'_, Message> {
        view::menu_view(self, id, opacity, icons)
    }

    pub(crate) fn updates(&self) -> &[Update] {
        &self.updates
    }

    pub(crate) const fn is_updates_list_open(&self) -> bool {
        self.is_updates_list_open
    }

    pub(crate) const fn state(&self) -> &CheckState {
        &self.state
    }

    pub(crate) const fn hyde(&self) -> Option<&HydeSnapshot> {
        self.hyde.as_ref()
    }

    pub(crate) const fn is_hyde_list_open(&self) -> bool {
        self.is_hyde_list_open
    }

    pub(crate) const fn is_hyde_updating(&self) -> bool {
        self.hyde_updating
    }

    pub(crate) fn hyde_log(&self) -> &[String] {
        &self.hyde_log
    }

    pub(crate) const fn is_applying(&self) -> bool {
        self.applying
    }

    pub(crate) fn apply_log(&self) -> &[String] {
        &self.apply_log
    }

    /// Branch the `HyDE` clone is measured against.
    pub(crate) fn hyde_branch_name(&self) -> &str {
        self.hyde_branch.as_deref().unwrap_or("master")
    }

    /// Ends the schedule and forgets what it was started for.
    fn stop(&mut self) {
        self.schedule = None;
        self.update_command = None;
        self.sender = None;
        self.hyde_clone = None;
        self.hyde_branch = None;
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
        self.hyde_clone =
            find_hyde_clone().map(|path| Arc::from(path.to_string_lossy().as_ref()));

        let branch: Arc<str> = Arc::from(definition.hyde_branch.git_name());
        self.hyde_branch = Some(Arc::clone(&branch));

        if self
            .schedule
            .as_ref()
            .is_some_and(|schedule| schedule.matches(&check_command, interval, &branch))
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
            interval,
            self.hyde_clone.clone(),
            branch
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
            view::icon(
                &self.state,
                self.updates.len(),
                self.hyde_pending(),
                self.shown_count.element(crate::components::scale::base()),
                icons
            )
            .map(M::from),
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
        time::{Duration, SystemTime, UNIX_EPOCH}
    };

    use tokio::runtime::Runtime;

    use super::{
        *,
        failures::{FAILURE_REPEAT, FailureLog},
        hyde_clone::clone_path_from,
        schedule::MIN_INTERVAL
    };
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
            check_interval: interval,
            hyde_branch:    Default::default()
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

    /// Picking the other branch must restart the check, or the bar would
    /// keep measuring the clone against the line it just left.
    #[test]
    fn a_changed_branch_replaces_the_schedule() {
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

        let mut definition = config("sleep 30", 3600);
        definition.hyde_branch = hydebar_proto::config::HydeBranch::Dev;
        <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
            .expect("the second registration succeeds");

        assert_ne!(first, task_id(&updates));
        assert_eq!(updates.hyde_branch_name(), "dev");
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
            Duration::from_millis(10),
            None,
            Arc::from("master")
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
    fn a_stale_hyde_clone_reaches_the_menu() {
        let mut updates = Updates::default();

        updates.observe(Message::HydeChecked(HydeSnapshot {
            version: "v25.10.1".to_owned(),
            commits: vec!["fix: one".to_owned()]
        }));

        assert_eq!(updates.hyde_pending(), 1);
    }

    #[test]
    fn collapsing_folds_both_lists_shut() {
        let mut updates = Updates::default();
        updates.is_updates_list_open = true;
        updates.observe(Message::ToggleHydeList);

        updates.collapse();

        assert!(!updates.is_updates_list_open);
        assert!(!updates.is_hyde_list_open);
    }

    #[test]
    fn the_tooltip_names_a_hyde_clone_that_fell_behind() {
        let mut updates = Updates::default();
        updates.state = CheckState::Ready;
        updates.observe(Message::HydeChecked(HydeSnapshot {
            version: "v25.10.1".to_owned(),
            commits: vec!["fix: one".to_owned(), "feat: two".to_owned()]
        }));

        assert_eq!(
            updates.tooltip().expect("a tooltip"),
            "Updates: none pending · HyDE: 2 commits behind"
        );
    }

    #[test]
    fn the_clone_path_is_read_from_the_version_file() {
        let content = "HYDE_BRANCH='master'\nHYDE_CLONE_PATH='/home/user/HyDE'\n";

        assert_eq!(
            clone_path_from(content),
            Some(std::path::PathBuf::from("/home/user/HyDE"))
        );
        assert_eq!(clone_path_from("HYDE_CLONE_PATH=''\n"), None);
        assert_eq!(clone_path_from("nothing here"), None);
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
