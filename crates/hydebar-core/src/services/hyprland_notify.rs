//! Notifications shown by the compositor itself, painted with the bar theme.
//!
//! The compositor keeps no style file for its own notifications: colour, icon,
//! duration and text size travel with every single call. Sending them from here
//! is what lets them follow the theme automatically, without anyone editing a
//! second configuration to keep two things looking alike.

use std::process::Command;

use hydebar_proto::config::{AppearanceColor, Config};
use log::warn;

/// How long a notice about a refused desktop action stays on screen, in
/// milliseconds.
///
/// A desktop action is asked for deliberately, so the answer that it did not
/// happen has to outlast a glance away from the screen.
const REFUSAL_DURATION: u32 = 6000;

/// Kind of notice, which decides the glyph the compositor draws beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    Warning,
    Info,
    Hint,
    Error,
    Ok
}

impl Notice {
    /// Number the compositor addresses this glyph by.
    #[must_use]
    pub const fn icon(self) -> i32 {
        match self {
            Notice::Warning => 0,
            Notice::Info => 1,
            Notice::Hint => 2,
            Notice::Error => 3,
            Notice::Ok => 5
        }
    }
}

/// Renders a colour the way the compositor spells colours.
#[must_use]
pub fn compositor_color(color: AppearanceColor) -> String {
    let base = color.get_base();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;

    format!(
        "rgba({:02x}{:02x}{:02x}{:02x})",
        channel(base.r),
        channel(base.g),
        channel(base.b),
        channel(base.a)
    )
}

/// Builds the arguments a notice is sent with.
///
/// The text size is carried inside the message, which is the only way the
/// compositor accepts it, so a themed bar and a themed notice stay the same
/// size.
#[must_use]
pub fn notify_args(
    notice: Notice,
    duration: u32,
    color: &str,
    font_size: f32,
    message: &str
) -> Vec<String> {
    vec![
        "notify".to_owned(),
        notice.icon().to_string(),
        duration.to_string(),
        color.to_owned(),
        format!("fontsize:{} {message}", font_size.round() as u32),
    ]
}

/// Asks the compositor to show `message`, painted with the bar theme.
///
/// Failures are logged rather than propagated: a notice that cannot be shown
/// must not take the bar down with it.
pub fn notify(notice: Notice, duration: u32, color: &str, font_size: f32, message: &str) {
    let args = notify_args(notice, duration, color, font_size, message);

    match Command::new("hyprctl").args(&args).output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => warn!(
            "the compositor refused a notice: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(err) => warn!("the compositor could not be reached for a notice: {err}")
    }
}

/// Puts `message` in front of the user as well as in the log.
///
/// A desktop action the bar could not perform has no other way of being
/// noticed: the surface that asked for it draws the desktop as it is, so a
/// refused change simply leaves that surface unchanged and reads as the press
/// having been missed.
///
/// Which surface it appears on follows the notification source the user chose.
/// Handing every notice to the compositor regardless would put a stray
/// compositor bubble on a desktop that asked for the bar's own popups, or for
/// the daemon it already runs — the bar's own messages have no more right to
/// pick a channel than anyone else's.
pub fn report(config: &Config, message: &str) {
    warn!("{message}");

    if config.notifications.source.hands_to_compositor() {
        notify(
            Notice::Error,
            REFUSAL_DURATION,
            &compositor_color(config.appearance.primary_color.clone()),
            config.appearance.font_size_px(),
            message
        );

        return;
    }

    post_to_bus(message);
}

/// Name the bar posts its own notices under.
const NOTIFIER: &str = "hydebar";

/// Sends `message` to whichever notification service holds the bus.
///
/// That is the bar's own service when the user asked for the bar's popups —
/// the service takes the name for itself rather than yielding to whatever
/// daemon the session started — and the session's daemon otherwise. Either way
/// the notice is drawn by the thing the user chose, which is the whole point of
/// the setting.
fn post_to_bus(message: &str) {
    let sent = Command::new("notify-send")
        .args(["--app-name", NOTIFIER, "--urgency", "critical", message])
        .spawn();

    if let Err(err) = sent {
        warn!("the notice could not be posted to the notification bus: {err}");
    }
}

#[cfg(test)]
mod tests {
    use hex_color::HexColor;

    use super::*;

    #[test]
    fn a_colour_is_rendered_the_way_the_compositor_spells_it() {
        let color = AppearanceColor::Simple(HexColor::rgb(0x1e, 0x1e, 0x2e));

        assert_eq!(compositor_color(color), "rgba(1e1e2eff)");
    }

    #[test]
    fn every_notice_names_its_glyph() {
        assert_eq!(Notice::Warning.icon(), 0);
        assert_eq!(Notice::Info.icon(), 1);
        assert_eq!(Notice::Hint.icon(), 2);
        assert_eq!(Notice::Error.icon(), 3);
        assert_eq!(Notice::Ok.icon(), 5);
    }

    #[test]
    fn the_text_size_travels_inside_the_message() {
        let args = notify_args(Notice::Info, 4000, "rgba(112233ff)", 20.0, "hello");

        assert_eq!(args[0], "notify");
        assert_eq!(args[1], "1");
        assert_eq!(args[2], "4000");
        assert_eq!(args[3], "rgba(112233ff)");
        assert_eq!(args[4], "fontsize:20 hello");
    }

    #[test]
    fn a_fractional_text_size_is_rounded() {
        let args = notify_args(Notice::Ok, 1000, "rgba(00000000)", 17.6, "x");

        assert_eq!(args[4], "fontsize:18 x");
    }
}
