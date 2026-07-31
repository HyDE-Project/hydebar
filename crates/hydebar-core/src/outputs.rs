//! Output management façade, re-exporting the collection state and helpers.

mod blur;
mod config;
pub mod scaling;
mod state;
mod wayland;

pub use scaling::{AutoMetrics, metrics as auto_metrics};
pub use state::{HasOutput, Outputs};

/// How long after a theme change the compositor may still reload.
///
/// A HyDE switch repaints the bar early — the palette lands well before the
/// scripts finish — and ends, seconds later, with a compositor reload that
/// forgets every dynamically stated rule. A single restatement raced that
/// reload and lost.
const RELOAD_TAIL: std::time::Duration = std::time::Duration::from_secs(5);

/// Starts the watcher restating the blur rules the moment they are wiped.
///
/// The compositor forgets every dynamically stated rule when it reloads its
/// configuration, and it announces that reload on its event socket. Answering
/// the announcement is what keeps the bar's blur gap to a moment: the timed
/// tail in [`restate_blur`] alone left the bar bare for however long the
/// heaviest theme took to reach its reload, seconds on some. The tail stays as
/// the backstop for a wipe the socket never announced.
pub fn start_blur_guard() {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return;
    }

    std::thread::spawn(|| {
        loop {
            let mut listener = hyprland::event_listener::EventListener::new();

            listener.add_config_reloaded_handler(|| {
                log::debug!("compositor reloaded, restating the blur rules");
                hydebar_proto::compositor_look::CompositorLook::invalidate();
                blur::request();
            });

            if let Err(err) = listener.start_listener() {
                log::warn!("blur guard lost the compositor socket: {err}");
            }

            std::thread::sleep(RELOAD_TAIL);
        }
    });
}

/// Generation of the latest restatement request.
///
/// Bumped on every call so the tail thread can tell whether another trigger
/// arrived while it slept, and owes the far side of that reload a statement
/// too.
static RESTATE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Whether a tail thread is already awake and will cover new generations.
static TAIL_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// States the blur rules to the compositor again, off the caller's thread.
///
/// The rules are handed over dynamically and the compositor forgets them on
/// every configuration reload — and a HyDE theme switch ends in exactly such a
/// reload. The bar re-states them whenever the desktop theme moved, which is
/// the one wipe it can observe, and once more after [`RELOAD_TAIL`] so the
/// statement lands on the far side of the reload that follows the switch. A
/// bar that only asked at startup keeps its blur until the first theme switch
/// and looks broken ever after.
///
/// A theme switch reloads the configuration several times in a burst, and each
/// reload calls here. One tail thread serves the whole burst: triggers landing
/// while it sleeps bump the generation and the thread keeps restating until a
/// full tail passes with no new trigger, instead of every reload paying for a
/// thread and a dozen compositor round trips of its own.
pub fn restate_blur() {
    use std::sync::atomic::Ordering;

    RESTATE_GENERATION.fetch_add(1, Ordering::SeqCst);

    if TAIL_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        blur::request();

        let mut seen = RESTATE_GENERATION.load(Ordering::SeqCst);

        loop {
            std::thread::sleep(RELOAD_TAIL);
            blur::request();

            let current = RESTATE_GENERATION.load(Ordering::SeqCst);

            if current == seen {
                break;
            }

            seen = current;
        }

        TAIL_RUNNING.store(false, Ordering::SeqCst);

        if RESTATE_GENERATION.load(Ordering::SeqCst) != seen {
            restate_blur();
        }
    });
}
