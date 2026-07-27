// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use iced::Point;

    use super::*;
    use crate::config::Config;

    #[test]
    fn toggle_menu_opens_and_closes() {
        let config = Config::default();
        let (mut outputs, _task) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let id = outputs
            .iter_internal()
            .next()
            .unwrap()
            .1
            .as_ref()
            .unwrap()
            .id;

        let button_ref = ButtonUIRef {
            position: Point::new(0.0, 0.0),
            viewport: (0., 0.)
        };
        let _ = outputs.toggle_menu::<()>(id, MenuType::Updates, button_ref, &config);
        assert!(outputs.menu_is_open());

        let _ = outputs.close_menu::<()>(id, &config);
        assert!(!outputs.menu_is_open());
    }

    #[test]
    fn sync_updates_position_internally() {
        let config = Config::default();
        let (mut outputs, _task) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let id = outputs
            .iter_internal()
            .next()
            .unwrap()
            .1
            .as_ref()
            .unwrap()
            .id;

        let mut updated_config = config.clone();
        updated_config.position = match updated_config.position {
            Position::Top => Position::Bottom,
            Position::Bottom => Position::Top
        };

        let _ = outputs.sync::<()>(
            updated_config.appearance.style,
            &updated_config.outputs,
            updated_config.position,
            &updated_config
        );

        assert!(matches!(outputs.has(id), Some(HasOutput::Main)));
    }
}
