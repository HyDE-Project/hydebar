//! Registration of the tick loop the clock owns.

use super::Clock;
use crate::{
    ModuleContext,
    config::ClockModuleConfig,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for Clock
where
    M: 'static
{
    type RegistrationData<'a> = &'a ClockModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.register(ctx, config);
        Ok(())
    }

    /// Stops the tick loop once the clock leaves the bar.
    ///
    /// A tick repaints every surface the bar owns, which is pure waste when no
    /// section renders the time any more.
    fn deregister(&mut self) {
        self.stop();
    }
}
