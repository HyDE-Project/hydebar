//! The two long commands the menu can start, spawned off the state.

use log::{debug, warn};

use super::{super::commands, Message, Updates};

impl Updates {
    /// Applies the configured update command, narrating into the window.
    pub(super) fn start_package_update(&mut self) {
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
    pub(super) fn start_hyde_update(&mut self) {
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
}
