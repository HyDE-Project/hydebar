mod data;
mod runtime;
mod view;

pub use data::{NetworkData, SystemInfoData, SystemInfoSampler};
use hydebar_proto::config::{Appearance, MemoryFormat, SystemModuleConfig};
use iced::Element;
pub use runtime::REFRESH_INTERVAL;
pub use view::{build_indicator_view, build_menu_view, indicator_elements};

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, components::icons::IconTheme, event_bus::ModuleEvent, format_cycle::FormatCycle
};

/// Messages published by the system information module.
#[derive(Debug, Clone)]
pub enum Message {
    Update,
    /// Switch to the next configured readout, wrapping after the last one.
    NextFormat
}

/// Module responsible for sampling and presenting local system metrics.
pub struct SystemInfo {
    sampler: SystemInfoSampler,
    data:    SystemInfoData,
    polling: runtime::PollingTask,
    format:  FormatCycle
}

impl Default for SystemInfo {
    fn default() -> Self {
        let mut sampler = SystemInfoSampler::new();
        let data = sampler.sample_with_extras();

        Self {
            sampler,
            data,
            polling: runtime::PollingTask::new(),
            format: FormatCycle::new()
        }
    }
}

impl SystemInfo {
    /// React to module messages by updating cached metrics when necessary.
    pub fn update(&mut self, message: Message, config: &SystemModuleConfig) {
        match message {
            Message::Update => {
                self.data = self.sampler.sample_with_extras();
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

    /// Render the menu entry exposing detailed system information.
    pub fn menu_view(&self, icons: &IconTheme) -> Element<'_, Message> {
        view::build_menu_view(&self.data, icons)
    }
}

impl<M> Module<M> for SystemInfo
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = (&'a SystemModuleConfig, &'a Appearance, &'a IconTheme);
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        self.polling.spawn(ctx, sender);

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
