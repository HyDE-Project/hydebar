//! The hint a module publishes while the pointer rests on it.

use hydebar_core::{
    config::ModuleName,
    modules::battery::BatteryData,
    tooltip::{TooltipInfo, tooltip_anchor}
};
use iced::{Element, SurfaceId as Id};

use crate::app::state::{App, Message};

/// States the battery in one line: charge, and whether it is being fed.
fn battery_hint(data: &BatteryData) -> String {
    if data.charging {
        format!("Battery: {}%, charging", data.capacity)
    } else {
        format!("Battery: {}%", data.capacity)
    }
}

impl App {
    /// Hint a module publishes while the pointer rests on it.
    ///
    /// The outer [`Option`] separates a module that never shows a hint, and is
    /// left unwrapped, from one that shows a hint only in some of its states.
    ///
    /// Modules whose facts fit a line state them; the rest state their name,
    /// because a strip of glyphs nobody can name is a bar nobody can learn.
    /// A custom module with no hint of its own states the name it was
    /// configured under, for the same reason. The ones that stay silent
    /// already say everything on the bar itself: workspaces and the window
    /// title are their own text, the tray draws icons the bar does not own,
    /// and the system readouts stand beside their values.
    #[expect(
        clippy::option_option,
        reason = "the outer option marks modules that never hint, the inner a hint absent in the current state"
    )]
    fn module_tooltip(&self, module_name: &ModuleName) -> Option<Option<String>> {
        match module_name {
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                Some(custom.tooltip().map_or_else(
                    || hydebar_proto::bar_layout::display_label(name),
                    str::to_owned
                ))
            }),
            ModuleName::IdleInhibitor => Some(
                self.config
                    .idle_inhibitor
                    .tooltip(self.control_center.is_idle_inhibited())
                    .map(str::to_owned)
            ),
            ModuleName::Battery => Some(self.battery.data().map(battery_hint)),
            ModuleName::Network => Some(Some(
                self.control_center
                    .network_hint()
                    .unwrap_or_else(|| module_name.label().to_owned())
            )),
            ModuleName::Cpu => Some(Some(hydebar_core::modules::cpu::hint(
                self.system_info.data()
            ))),
            ModuleName::Memory => Some(Some(hydebar_core::modules::memory::hint(
                self.system_info.data()
            ))),
            ModuleName::CpuTemp => Some(Some(hydebar_core::modules::cpu_temp::hint(
                self.system_info.data()
            ))),
            ModuleName::GpuTemp => Some(Some(hydebar_core::modules::gpu_temp::hint(
                self.system_info.data()
            ))),
            ModuleName::Clock => Some(Some(self.clock.data().format("%A, %-d %B %Y"))),
            ModuleName::Updates => Some(self.updates.tooltip()),
            ModuleName::KeyboardLayout => Some(Some(format!(
                "{}: {}",
                module_name.label(),
                self.keyboard_layout.active_layout()
            ))),
            ModuleName::Workspaces
            | ModuleName::WindowTitle
            | ModuleName::Tray
            | ModuleName::SystemInfo => None,
            named => Some(Some(named.label().to_owned()))
        }
    }

    /// Wraps a module in the anchor its hover is published from.
    ///
    /// Every module is wrapped, hint or no hint: the pointer resting on a
    /// module is what the bar reads its attention out of, and a module that
    /// shows no tooltip is still something the user can look at. The hint, when
    /// there is one, rides along on the same message rather than on a second
    /// one, because there is only ever one thing being looked at.
    ///
    /// The hint is composed inside the closure on purpose: the closure runs
    /// when the pointer actually enters or leaves, while the wrapper itself
    /// runs for every module on every frame — and a date formatted for a
    /// tooltip nobody hovers is a frame budget spent on nothing.
    pub(crate) fn with_tooltip<'a>(
        &'a self,
        module_name: &'a ModuleName,
        module: Element<'a, Message>,
        id: Id
    ) -> Element<'a, Message> {
        tooltip_anchor(module, move |anchor| Message::ModuleHover {
            surface: id,
            module:  module_name.clone(),
            entered: anchor.is_some(),
            tooltip: anchor
                .zip(anchor.and_then(|_| self.module_tooltip(module_name).flatten()))
                .map(|(anchor, text)| TooltipInfo {
                    text,
                    anchor
                })
        })
        .into()
    }
}
