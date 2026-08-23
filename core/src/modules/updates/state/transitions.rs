//! How each message folds into what the bar shows.

use log::warn;

use super::{CheckState, Message, Updates};
use crate::{config::UpdatesModuleConfig, outputs::Outputs};

impl Updates {
    /// Folds one message into the entry, and says what follows.
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

    /// Folds everything a check reports into what the bar shows.
    ///
    /// Kept apart from [`Updates::update`] because these are the
    /// transitions that need neither a window to close a menu on
    /// nor a runtime to spawn a command into.
    pub(super) fn observe(&mut self, message: Message) {
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
}
