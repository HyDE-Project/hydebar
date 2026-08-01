use iced::Element;

use super::{Module, OnModulePress};
use crate::components::icons::{IconTheme, Icons, icon};

#[derive(Default, Debug, Clone)]
pub struct AppLauncher;

impl<M> Module<M> for AppLauncher
where
    M: 'static + Clone
{
    type ViewData<'a> = (&'a Option<String>, &'a IconTheme);
    type RegistrationData<'a> = ();

    fn view(
        &self,
        (config, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if config.is_some() {
            Some((
                icon(icons, Icons::AppLauncher).into(),
                None // Action handled in GUI layer
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::icons::IconTheme;

    #[test]
    fn view_returns_some_when_config_present() {
        let launcher = AppLauncher;
        let config = Some("wofi".to_string());

        let result =
            <AppLauncher as Module<()>>::view(&launcher, (&config, &IconTheme::default()));
        assert!(result.is_some());

        if let Some((_, action)) = result {
            assert!(action.is_none());
        }
    }

    #[test]
    fn view_returns_none_when_config_absent() {
        let launcher = AppLauncher;
        let config = None;

        let result =
            <AppLauncher as Module<()>>::view(&launcher, (&config, &IconTheme::default()));
        assert!(result.is_none());
    }
}
