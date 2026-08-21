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
pub use view::{
    build_indicator_view, gigabytes, indicator_elements, single_indicator, used_of_total
};
pub use window::build_menu_view;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, attention::PollSchedule, components::icons::IconTheme, event_bus::ModuleEvent,
    format_cycle::FormatCycle
};

/// Messages published by the system information module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Readouts that differ from the ones currently on screen.
    Sampled(std::sync::Arc<SystemInfoData>),
    /// Switch to the next configured readout, wrapping after the last one.
    NextFormat
}

/// Module responsible for sampling and presenting local system metrics.
#[derive(Debug)]
pub struct SystemInfo {
    data:    std::sync::Arc<SystemInfoData>,
    polling: runtime::PollingTask,
    format:  FormatCycle
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            data:    std::sync::Arc::new(SystemInfoSampler::new().sample_with_extras()),
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
    #[must_use]
    pub fn active_memory_format(&self, config: &SystemModuleConfig) -> MemoryFormat {
        *self
            .format
            .resolve(&config.memory.format, &config.memory.format_alt)
    }

    /// Latest sample, for the thin bar entries and the hover hints that
    /// render from it.
    #[must_use]
    pub fn data(&self) -> &SystemInfoData {
        &self.data
    }

    /// Width the monitor window asks the screen for.
    ///
    /// Stated by the module so the box hugs its value columns; a stock menu
    /// width left a blank margin beside readouts that cannot grow into it.
    #[must_use]
    pub fn content_width(font_size: f32) -> f32 {
        window::content_width(font_size)
    }

    /// The monitor window and its height, built from one model.
    ///
    /// The model is stated once and both the drawing and the measurement
    /// read it, so a frame costs one build instead of two.
    #[must_use]
    pub fn monitor_window(
        &self,
        config: &SystemModuleConfig,
        icons: &IconTheme
    ) -> (Element<'_, Message>, f32) {
        let sections = window::model::sections(&self.data);
        let footnotes = window::model::footnotes(&self.data, config);
        let height = window::content_height_of(&sections, &footnotes);

        (window::build_menu_view(sections, footnotes, icons), height)
    }

    /// The window of the standalone processor entry, with its height.
    #[must_use]
    pub fn cpu_window(&self, icons: &IconTheme) -> (Element<'_, Message>, f32) {
        Self::section_window(window::model::processor_section(&self.data), icons)
    }

    /// The window of the standalone memory entry, with its height.
    #[must_use]
    pub fn memory_window(&self, icons: &IconTheme) -> (Element<'_, Message>, f32) {
        Self::section_window(window::model::memory_section(&self.data).into(), icons)
    }

    /// The window of the standalone processor temperature entry.
    #[must_use]
    pub fn cpu_temp_window(&self, icons: &IconTheme) -> (Element<'_, Message>, f32) {
        Self::section_window(window::model::cpu_temperature_section(&self.data), icons)
    }

    /// The window of the standalone graphics entry, with its height.
    #[must_use]
    pub fn gpu_window(&self, icons: &IconTheme) -> (Element<'_, Message>, f32) {
        Self::section_window(window::model::graphics_section(&self.data), icons)
    }

    /// One section drawn and measured from a single build.
    fn section_window<'a>(
        section: Option<window::model::Section>,
        icons: &IconTheme
    ) -> (Element<'a, Message>, f32) {
        let height = window::section_window_height(section.as_ref());

        (window::build_section_window(section, icons), height)
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
#[cfg_attr(coverage_nightly, coverage(off))]
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
