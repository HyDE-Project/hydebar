//! The state of the desk itself: its keyboard, its sky, its tray, its look.

use super::super::{Panel, push};
use crate::app::state::App;

/// The keyboard layout in force.
pub fn keyboard(app: &App) -> Option<Panel> {
    let active = app.keyboard_layout.active_layout();

    if active.is_empty() {
        return None;
    }

    let named = app
        .config
        .keyboard_layout
        .labels
        .get(active)
        .cloned()
        .unwrap_or_else(|| active.to_owned());

    Panel::of("keyboard", vec![("layout".to_owned(), named)])
}

/// The sky over the configured place.
pub fn weather(app: &App) -> Option<Panel> {
    let sky = app.weather.data();

    if !sky.has_reading() {
        return None;
    }

    Panel::of(
        "weather",
        vec![
            ("place".to_owned(), sky.location.clone()),
            ("temperature".to_owned(), sky.temperature.clone()),
            ("sky".to_owned(), sky.description.clone()),
            ("humidity".to_owned(), sky.humidity.clone()),
            ("wind".to_owned(), sky.wind_speed.clone()),
        ]
    )
}

/// The applications keeping an icon in the tray.
pub fn tray(app: &App) -> Option<Panel> {
    let service = app.tray.service.as_ref()?;

    if service.data.is_empty() {
        return None;
    }

    let mut rows = vec![("items".to_owned(), service.data.len().to_string())];

    for item in service.data.iter() {
        let name = item
            .name
            .rsplit('/')
            .next()
            .unwrap_or(item.name.as_str())
            .to_owned();

        rows.push((String::new(), name));
    }

    Panel::of("tray", rows)
}

/// The desktop theme in force, and the themes it stands among.
///
/// The look is shown rather than named: `HyDE` keeps a crop of every theme's
/// wallpaper and its own picker draws exactly those, so the block draws them
/// too — the one in force big in the middle of the row, the neighbours it
/// would step to on either side.
pub fn theme(app: &App) -> Option<Panel> {
    let hyde = app.themes.hyde();
    let reel = &app.looks.themes;
    let mut rows = Vec::new();

    push(&mut rows, "in force", hyde.theme.clone());
    push(
        &mut rows,
        "switching to",
        app.themes.switching().map(ToOwned::to_owned)
    );
    rows.push((
        "colours".to_owned(),
        if hyde.wallpaper_colors {
            "from the wallpaper"
        } else {
            "from the theme"
        }
        .to_owned()
    ));
    push(&mut rows, "shader", hyde.shader.clone());
    push(&mut rows, "place", place(reel));

    if reel.is_empty() {
        return Panel::of("theme", rows);
    }

    Some(Panel::drawn(
        "theme",
        rows,
        super::super::Figure::Accordion(reel.clone())
    ))
}

/// The wallpaper on screen, and the ones the theme keeps beside it.
pub fn wallpaper(app: &App) -> Option<Panel> {
    let reel = &app.looks.wallpapers;
    let mut rows = Vec::new();

    push(
        &mut rows,
        "picture",
        reel.in_force().map(ToOwned::to_owned).or_else(|| {
            app.wallpaper_preview.as_ref().and_then(|(path, _)| {
                path.file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
            })
        })
    );
    push(&mut rows, "place", place(reel));

    if reel.is_empty() {
        return match app.wallpaper_preview.as_ref() {
            Some((_, picture)) => Some(Panel::drawn(
                "wallpaper",
                rows,
                super::super::Figure::Picture(picture.clone())
            )),
            None => Panel::of("wallpaper", rows)
        };
    }

    Some(Panel::drawn(
        "wallpaper",
        rows,
        super::super::Figure::Accordion(reel.clone())
    ))
}

/// Where in its reel the one in force stands, when the reel holds more than it.
///
/// A row of pictures is a window onto a longer list, and a window says nothing
/// about how long the list is. One line does.
fn place(reel: &hydebar_core::modules::desk::looks::Reel) -> Option<String> {
    (reel.total > 1).then(|| format!("{} of {}", reel.at, reel.total))
}

/// Who is at the machine and what they are sitting in front of.
///
/// The header of any screen worth the name: a machine is not a list of
/// readings, it is somebody's machine, and the first thing an overview says
/// is whose and where.
pub fn seat(app: &App) -> Option<Panel> {
    let who = hydebar_core::modules::system_info::who::who();
    let mut rows = Vec::new();

    push(
        &mut rows,
        "session",
        who.user.as_ref().map(|user| {
            who.host
                .as_ref()
                .map_or_else(|| user.clone(), |host| format!("{user}@{host}"))
        })
    );
    push(&mut rows, "desktop", who.desktop.clone());
    push(&mut rows, "display", who.seat.clone());
    push(&mut rows, "shell", who.shell.clone());
    push(
        &mut rows,
        "screen",
        app.screen_width
            .zip(app.screen_height)
            .map(|(width, height)| format!("{width:.0} × {height:.0}"))
    );

    Panel::of("seat", rows)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::modules::desk::looks::{Reel, Slide};

    use super::{super::super::Figure, *};
    use crate::app::state::test_support::test_app;

    /// A one pixel picture: what is asserted is which pictures are drawn, not
    /// what is in them.
    fn slide(name: &str, active: bool) -> Slide {
        Slide {
            name: name.to_owned(),
            picture: iced::widget::image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            active
        }
    }

    fn reel() -> Reel {
        Reel {
            shown: vec![
                slide("Gruvbox Retro", false),
                slide("Nordic Blue", true),
                slide("Tokyo Night", false),
            ],
            at:    5,
            total: 12
        }
    }

    /// The look is a picture, and the block draws it rather than spelling it.
    #[test]
    fn the_theme_block_draws_the_themes_it_stands_among() {
        let mut app = test_app();
        app.looks.themes = reel();

        let panel = theme(&app).expect("a theme block");
        let Some(Figure::Accordion(drawn)) = panel.figure else {
            panic!("the theme block draws no reel: {:?}", panel.figure);
        };

        assert_eq!(
            drawn
                .shown
                .iter()
                .map(|slide| slide.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Gruvbox Retro", "Nordic Blue", "Tokyo Night"]
        );
        assert_eq!(drawn.in_force(), Some("Nordic Blue"));
    }

    /// A row of three says nothing about a list of twelve; the line does.
    #[test]
    fn a_reel_says_where_in_the_whole_list_the_one_in_force_stands() {
        let mut app = test_app();
        app.looks.wallpapers = reel();

        let panel = wallpaper(&app).expect("a wallpaper block");

        assert!(
            panel
                .rows
                .contains(&("place".to_owned(), "5 of 12".to_owned())),
            "the place is written out: {:?}",
            panel.rows
        );
        assert!(
            panel
                .rows
                .contains(&("picture".to_owned(), "Nordic Blue".to_owned())),
            "and so is the name of the one in force: {:?}",
            panel.rows
        );
    }

    /// A machine `HyDE` has cached nothing for still has a theme, and the
    /// block still says which.
    #[test]
    fn a_desktop_with_no_cached_pictures_still_names_its_theme() {
        let app = test_app();
        let panel = theme(&app).expect("a theme block");

        assert_eq!(panel.figure, None);
        assert!(!panel.rows.is_empty());
    }

    /// One picture is not a reel: a place reads as a choice, and there is
    /// none.
    #[test]
    fn a_theme_standing_alone_is_given_no_place() {
        assert_eq!(
            place(&Reel {
                shown: vec![slide("Only One", true)],
                at:    1,
                total: 1
            }),
            None
        );
    }
}
