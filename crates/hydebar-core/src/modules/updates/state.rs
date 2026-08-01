use std::sync::Arc;

use iced::{Element, SurfaceId as Id};
use log::{debug, info, warn};
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

#[expect(
    clippy::struct_excessive_bools,
    reason = "the open lists and the run outcomes are independent switches, not one state machine"
)]
#[derive(Default)]
pub struct Updates {
    state:                    CheckState,
    pending:                  Vec<Update>,
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
            CheckState::Ready => match self.pending.len() {
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
            .field("pending", &self.pending)
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
            .field("shown_count", &self.shown_count)
            .finish()
    }
}

impl Clone for Updates {
    fn clone(&self) -> Self {
        Self {
            state:                self.state.clone(),
            pending:              self.pending.clone(),
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
            Message::Update => self.start_package_update(),
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
            Message::UpdateHyde => self.start_hyde_update(),
            observed => self.observe(observed)
        }

        self.shown_count.set(
            self.pending.len().to_string(),
            main_config.appearance.animations.enabled
        );
    }

    /// Applies the configured update command, narrating into the window.
    fn start_package_update(&mut self) {
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
                    log_sender.send(Message::UpdateLog(lines));
                };

                let failed = match commands::apply_updates(update_command.as_ref(), publish).await
                {
                    Ok(()) => false,
                    Err(err) => {
                        err.or_log("the package update failed");

                        true
                    }
                };

                sender.send(Message::UpdateFinished {
                    failed
                });
            });
        } else {
            warn!("updates module is not fully initialised; skipping update command");
        }
    }

    /// Brings the `HyDE` clone up to date, narrating into the window.
    fn start_hyde_update(&mut self) {
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
                    log_sender.send(Message::HydeUpdateLog(lines));
                };

                let failed =
                    match commands::update_hyde(clone.as_ref(), branch.as_ref(), publish).await {
                        Ok(()) => false,
                        Err(err) => {
                            err.or_log("the hyde update failed");

                            true
                        }
                    };

                sender.send(Message::HydeUpdateFinished {
                    failed
                });

                sender.send(Message::CheckNow);
            });
        } else {
            warn!("no hyde clone is known; skipping the hyde update");
        }
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
                self.pending = updates;
                self.state = CheckState::Ready;

                if !self.applying && !self.apply_failed {
                    self.apply_log.clear();
                }
            }
            Message::CheckFailed => self.state = CheckState::Ready,
            Message::UpdatesUnavailable => {
                self.pending.clear();
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
        &self.pending
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
        self.hyde_clone = find_hyde_clone().map(|path| Arc::from(path.to_string_lossy().as_ref()));

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
                self.pending.len(),
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
mod tests;
