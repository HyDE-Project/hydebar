//! The menu pages whose window height is measured from their content.

use hydebar_core::menu::{MenuSize, MenuType};
use iced::Element;

use super::super::state::{App, Message};

impl App {
    /// Content, width and measured height of a menu sized from its content.
    ///
    /// The arms of [`App::menu_page`] whose windows report a measured height:
    /// the settings and themes panels, the system monitor windows and the
    /// calendar. Any other menu type answers [`None`].
    #[allow(clippy::type_complexity)]
    pub(super) fn measured_page(
        &self,
        menu_type: &MenuType,
        opacity: f32
    ) -> Option<(Element<'_, Message>, MenuSize, Option<f32>)> {
        match menu_type {
            MenuType::Settings => {
                let (window, metrics) =
                    self.settings
                        .window(&self.config, opacity, self.icons(), self.magnification);

                Some((
                    window.map(Message::Settings),
                    MenuSize::Content(metrics.width),
                    Some(metrics.height)
                ))
            }
            MenuType::Themes => {
                let metrics = self.themes.window_metrics(&self.config);

                Some((
                    self.themes
                        .menu_view(&self.config, opacity, metrics.page_width)
                        .map(Message::Themes),
                    MenuSize::Content(metrics.width),
                    Some(metrics.height)
                ))
            }
            MenuType::SystemInfo => {
                let (window, height) = self
                    .system_info
                    .monitor_window(&self.config.system, self.icons());

                Some((
                    window.map(Message::SystemInfo),
                    MenuSize::Content(
                        hydebar_core::modules::system_info::SystemInfo::content_width(
                            self.appearance().font_size_px()
                        )
                    ),
                    Some(height)
                ))
            }
            MenuType::Cpu => {
                let (window, height) = self.system_info.cpu_window(self.icons());

                Some((
                    window.map(Message::SystemInfo),
                    MenuSize::Content(
                        hydebar_core::modules::system_info::SystemInfo::content_width(
                            self.appearance().font_size_px()
                        )
                    ),
                    Some(height)
                ))
            }
            MenuType::Memory => {
                let (window, height) = self.system_info.memory_window(self.icons());

                Some((
                    window.map(Message::SystemInfo),
                    MenuSize::Content(
                        hydebar_core::modules::system_info::SystemInfo::content_width(
                            self.appearance().font_size_px()
                        )
                    ),
                    Some(height)
                ))
            }
            MenuType::CpuTemp => {
                let (window, height) = self.system_info.cpu_temp_window(self.icons());

                Some((
                    window.map(Message::SystemInfo),
                    MenuSize::Content(
                        hydebar_core::modules::system_info::SystemInfo::content_width(
                            self.appearance().font_size_px()
                        )
                    ),
                    Some(height)
                ))
            }
            MenuType::Gpu => {
                let (window, height) = self.system_info.gpu_window(self.icons());

                Some((
                    window.map(Message::SystemInfo),
                    MenuSize::Content(
                        hydebar_core::modules::system_info::SystemInfo::content_width(
                            self.appearance().font_size_px()
                        )
                    ),
                    Some(height)
                ))
            }
            MenuType::Calendar => Some((
                self.calendar.menu_view(self.icons()).map(Message::Calendar),
                MenuSize::Content(hydebar_core::modules::calendar::Calendar::content_width(
                    self.appearance().font_size_px()
                )),
                Some(hydebar_core::modules::calendar::Calendar::content_height())
            )),
            _ => None
        }
    }
}
