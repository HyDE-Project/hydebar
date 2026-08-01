use iced::Element;

use super::{Module, OnModulePress};
use crate::components::icons::{IconTheme, Icons, icon};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl<M> Module<M> for Clipboard
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
                icon(icons, Icons::Clipboard).into(),
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
        let clipboard = Clipboard;
        let config = Some("cliphist".to_string());

        let result = <Clipboard as Module<()>>::view(&clipboard, (&config, &IconTheme::default()));
        assert!(result.is_some());

        if let Some((_, action)) = result {
            assert!(action.is_none());
        }
    }

    #[test]
    fn view_returns_none_when_config_absent() {
        let clipboard = Clipboard;
        let config = None;

        let result = <Clipboard as Module<()>>::view(&clipboard, (&config, &IconTheme::default()));
        assert!(result.is_none());
    }
}
