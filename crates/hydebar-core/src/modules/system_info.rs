mod data;
pub mod indicators;
mod runtime;
pub mod sensors;
mod view;
mod window;

pub use data::{DiskData, NetworkData, SystemInfoData, SystemInfoSampler};
use hydebar_proto::config::{Appearance, MemoryFormat, SystemModuleConfig};
use iced::Element;
pub use indicators::{IndicatorStatus, Unavailable};
pub use runtime::REFRESH_INTERVAL;
pub use sensors::{GpuPlacement, GpuReadings, GpuVendor, HardwareSensors};
pub use view::{build_indicator_view, indicator_elements, single_indicator};
pub(crate) use view::{gigabytes, used_of_total};
pub use window::build_menu_view;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    attention::PollSchedule,
    components::icons::{IconTheme, Icons},
    event_bus::ModuleEvent,
    format_cycle::FormatCycle
};

/// Messages published by the system information module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Readouts that differ from the ones currently on screen.
    Sampled(SystemInfoData),
    /// Switch to the next configured readout, wrapping after the last one.
    NextFormat
}

/// Module responsible for sampling and presenting local system metrics.
pub struct SystemInfo {
    data:    SystemInfoData,
    polling: runtime::PollingTask,
    format:  FormatCycle
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            data:    SystemInfoSampler::new().sample_with_extras(),
            polling: runtime::PollingTask::new(),
            format:  FormatCycle::new()
        }
    }
}

impl SystemInfo {
    /// React to module messages by updating cached metrics when necessary.
    pub fn update(&mut self, message: Message, config: &SystemModuleConfig) {
        match message {
            Message::Sampled(data) => {
                self.data = data;
            }
            Message::NextFormat => {
                self.format.advance(&config.memory.format_alt);
            }
        }
    }

    /// Memory readout the active index selects.
    pub fn active_memory_format(&self, config: &SystemModuleConfig) -> MemoryFormat {
        *self
            .format
            .resolve(&config.memory.format, &config.memory.format_alt)
    }

    /// Latest sample, for the thin bar entries and the hover hints that
    /// render from it.
    pub fn data(&self) -> &SystemInfoData {
        &self.data
    }

    /// Width the monitor window asks the screen for.
    ///
    /// Stated by the module so the box hugs its value columns; a stock menu
    /// width left a blank margin beside readouts that cannot grow into it.
    pub fn content_width(font_size: f32) -> f32 {
        window::content_width(font_size)
    }

    /// Height the window needs for the readouts this machine reports.
    pub fn content_height(&self, config: &SystemModuleConfig) -> f32 {
        window::content_height(&self.data, config)
    }

    /// Render the menu entry exposing detailed system information.
    pub fn menu_view(
        &self,
        config: &SystemModuleConfig,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        window::build_menu_view(&self.data, config, icons)
    }

    /// Render the window of the standalone processor entry.
    pub fn cpu_menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        window::build_section_window(window::model::processor_section(&self.data), icons)
    }

    /// Render the window of the standalone memory entry.
    pub fn memory_menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        window::build_section_window(
            window::model::scoped_section(&self.data, Icons::Mem),
            icons
        )
    }

    /// Height the standalone processor window needs.
    pub fn cpu_content_height(&self) -> f32 {
        window::section_window_height(window::model::processor_section(&self.data).as_ref())
    }

    /// Height the standalone memory window needs.
    pub fn memory_content_height(&self) -> f32 {
        window::section_window_height(
            window::model::scoped_section(&self.data, Icons::Mem).as_ref()
        )
    }

    /// Render the window of the standalone processor temperature entry.
    pub fn cpu_temp_menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        window::build_section_window(window::model::cpu_temperature_section(&self.data), icons)
    }

    /// Height the standalone processor temperature window needs.
    pub fn cpu_temp_content_height(&self) -> f32 {
        window::section_window_height(
            window::model::cpu_temperature_section(&self.data).as_ref()
        )
    }

    /// Render the window of the standalone graphics entry.
    pub fn gpu_menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        window::build_section_window(
            window::model::scoped_section(&self.data, Icons::Gpu),
            icons
        )
    }

    /// Height the standalone graphics window needs.
    pub fn gpu_content_height(&self) -> f32 {
        window::section_window_height(
            window::model::scoped_section(&self.data, Icons::Gpu).as_ref()
        )
    }
}

impl<M> Module<M> for SystemInfo
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = (&'a SystemModuleConfig, &'a Appearance, &'a IconTheme);
    type RegistrationData<'a> = (&'a SystemModuleConfig, bool);

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (config, full): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        self.polling
            .spawn(ctx, sender, config.gpu.device.as_deref(), full);

        Ok(())
    }

    /// Stops sampling the machine once the readout leaves the bar.
    ///
    /// Reading every CPU, disk and interface costs real work, so a layout that
    /// dropped the module must not keep paying for it every few seconds.
    fn deregister(&mut self) {
        self.polling.abort();
    }

    /// Refreshes faster only while somebody is looking.
    ///
    /// The module's own task keeps the bar current on the resting cadence;
    /// the attended cadence exists for the window and the hover hints, whose
    /// readouts would otherwise be up to a whole interval old the moment
    /// they open.
    fn poll_schedule(&self) -> Option<PollSchedule> {
        Some(PollSchedule::only_when_attended(runtime::ATTENDED_INTERVAL))
    }

    fn poll(&mut self, _: &ModuleContext) -> Result<(), ModuleError> {
        self.polling.poke();

        Ok(())
    }

    fn view(
        &self,
        (config, appearance, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        view::build_indicator_view(
            &self.data,
            config,
            self.active_memory_format(config),
            appearance,
            icons
        )
    }
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::SystemInfoMemory;

    use super::*;

    fn config(format: MemoryFormat, alternatives: &[MemoryFormat]) -> SystemModuleConfig {
        SystemModuleConfig {
            memory: SystemInfoMemory {
                format,
                format_alt: alternatives.to_vec(),
                ..SystemInfoMemory::default()
            },
            ..SystemModuleConfig::default()
        }
    }

    #[test]
    fn a_press_walks_the_configured_readouts_and_wraps_around() {
        let config = config(MemoryFormat::Bytes, &[MemoryFormat::Percentage]);
        let mut module = SystemInfo::default();

        assert_eq!(module.active_memory_format(&config), MemoryFormat::Bytes);

        module.update(Message::NextFormat, &config);
        assert_eq!(
            module.active_memory_format(&config),
            MemoryFormat::Percentage
        );

        module.update(Message::NextFormat, &config);
        assert_eq!(module.active_memory_format(&config), MemoryFormat::Bytes);
    }

    #[test]
    fn a_module_without_alternatives_keeps_its_readout() {
        let config = config(MemoryFormat::Percentage, &[]);
        let mut module = SystemInfo::default();

        module.update(Message::NextFormat, &config);

        assert_eq!(
            module.active_memory_format(&config),
            MemoryFormat::Percentage
        );
    }
}
