//! How the module joins the bar and leaves it: the schedule it keeps
//! running while the layout hosts the entry.

use std::sync::Arc;

use log::{debug, info};

use super::{
    Updates,
    hyde_clone::find_hyde_clone,
    schedule::{Schedule, check_interval}
};
use crate::{
    ModuleContext,
    config::UpdatesModuleConfig,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl Updates {
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
    M: 'static
{
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
}
