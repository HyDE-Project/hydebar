//! The one law of registration, and the rosters it is applied over.

use hydebar_core::{ModuleContext, modules};
use hydebar_proto::config::{Config, ModuleName};
use log::error;

use super::super::super::state::Message;

/// Applies the one law of registration: a module is wired while the layout
/// hosts it and released the moment it is not.
///
/// Every module used to restate this law as its own if/else block, and the
/// copies drifted — one dropped its deregister arm, another swallowed its
/// registration error. Stating the law once means a module cannot forget half
/// of it: a failure is logged under the module's label, and an unhosted module
/// always gives its background work back.
pub(super) fn gate<M>(
    label: &str,
    hosted: bool,
    module: &mut M,
    ctx: &ModuleContext,
    data: M::RegistrationData<'_>
) where
    M: modules::Module<Message>
{
    if hosted {
        if let Err(err) = module.register(ctx, data) {
            error!("failed to register {label} module: {err}");
        }
    } else {
        module.deregister();
    }
}

/// Bar entries the control centre services feed.
///
/// The five hardware listeners behind the control centre are shared: the
/// standalone `Audio`, `Network`, `Bluetooth` and `PowerProfile` readouts
/// render from the same state as the full panel, and the battery indicator
/// opens its menu. One of them on the bar is enough to justify the connections.
pub(super) const CONTROL_CENTER_CONSUMERS: [ModuleName; 8] = [
    ModuleName::ControlCenter,
    ModuleName::Audio,
    ModuleName::Network,
    ModuleName::Bluetooth,
    ModuleName::PowerProfile,
    ModuleName::Brightness,
    ModuleName::Settings,
    ModuleName::Battery
];

/// Bar entries the system monitor's sampler feeds.
///
/// The standalone processor and memory readouts render from the same sample
/// as the combined monitor, so the sampler has to run while any of the three
/// is on screen.
pub(super) const SYSTEM_INFO_CONSUMERS: [ModuleName; 5] = [
    ModuleName::SystemInfo,
    ModuleName::Cpu,
    ModuleName::Memory,
    ModuleName::CpuTemp,
    ModuleName::GpuTemp
];

/// Whether the bar itself is the session's notification server.
///
/// The one exception to gating on layout placement: the bar serves the
/// notification bus for its popups even when no bell entry is drawn, and
/// hands the bus back the moment the configuration chooses a separate
/// daemon.
pub(super) const fn notifications_hosted(config: &Config) -> bool {
    config.notifications.source.owns_the_bus()
}
