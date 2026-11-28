//! The desktop portal settings the bar mirrors.
//!
//! The XDG desktop portal holds a handful of desktop-wide preferences that a
//! bar is expected to honour rather than override. Today the bar reads one:
//! whether the user asked the desktop to reduce motion. The portal is the
//! single place that answer lives across toolkits, so the bar asks it instead
//! of inventing its own switch, and follows it live through the portal's own
//! change signal.

use std::{any::TypeId, hash::Hash};

use iced::{
    Subscription,
    futures::{SinkExt, StreamExt, channel::mpsc::Sender, pin_mut, stream::BoxStream},
    stream::channel
};
use iced_futures::subscription::{self, Recipe, from_recipe};
use log::{debug, error};
use zbus::{Connection, OwnedVariant, Variant, proxy};

/// Portal namespace holding the desktop appearance preferences.
const APPEARANCE: &str = "org.freedesktop.appearance";

/// Portal key carrying the reduced-motion preference within that namespace.
const REDUCED_MOTION: &str = "reduced-motion";

/// The portal's settings store, read-only as far as the bar is concerned.
#[proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait Settings {
    /// Reads one namespaced setting back as a variant value.
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<OwnedVariant>;

    /// Emitted when a namespaced setting changes anywhere on the desktop.
    #[zbus(signal)]
    fn setting_changed(&self, namespace: &str, key: &str, value: OwnedVariant)
    -> zbus::Result<()>;
}

/// Whether a reduced-motion number asks for less motion.
///
/// The portal states the preference as a 32-bit number: `1` means reduce, and
/// anything else — `0`, or a value from a portal newer than this bar — leaves
/// motion alone. Treating the unknown as "do not reduce" keeps the bar from
/// acting on a meaning it has not been told.
#[must_use]
pub const fn reduces_motion(value: u32) -> bool {
    value == 1
}

/// Reads the reduced-motion preference out of a portal variant value.
///
/// The portal hands the setting back wrapped in a variant, so the number is
/// unwrapped first; a value that is not the number the portal promises reads as
/// "do not reduce", the safe default of keeping the bar's animations.
fn reduced_motion_of(value: &OwnedVariant) -> bool {
    let inner = match &**value {
        Variant::Variant(variant) => &**variant,
        other => other
    };

    u32::try_from(inner).is_ok_and(reduces_motion)
}

/// Reports the desktop's reduced-motion preference, now and on every change.
///
/// The subscription reads the current value once, then follows the portal's
/// change signal for as long as the session lasts. A machine with no portal, or
/// with no reduced-motion setting, simply never emits: the bar keeps the
/// animations its configuration asked for.
pub fn reduced_motion_subscription() -> Subscription<bool> {
    from_recipe(ReducedMotionWatcher)
}

struct ReducedMotionWatcher;

impl Recipe for ReducedMotionWatcher {
    type Output = bool;

    fn hash(&self, state: &mut subscription::Hasher) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: subscription::EventStream) -> BoxStream<'static, bool> {
        Box::pin(channel(1, watch_reduced_motion))
    }
}

/// Reads the preference once, then forwards every change until the bar stops
/// listening.
async fn watch_reduced_motion(mut output: Sender<bool>) {
    let Ok(connection) = Connection::session().await else {
        error!("cannot reach the session bus to read the reduced-motion setting");

        return;
    };

    let Ok(settings) = SettingsProxy::new(&connection).await else {
        debug!("no portal settings on this desktop; keeping the bar's animations");

        return;
    };

    if let Ok(value) = settings.read(APPEARANCE, REDUCED_MOTION).await
        && output.send(reduced_motion_of(&value)).await.is_err()
    {
        return;
    }

    let Ok(changes) = settings.receive_setting_changed().await else {
        debug!("cannot watch reduced-motion changes on this desktop");

        return;
    };

    pin_mut!(changes);

    while let Some(signal) = changes.next().await {
        let Ok(args) = signal.args() else {
            continue;
        };

        if args.namespace != APPEARANCE || args.key != REDUCED_MOTION {
            continue;
        }

        if output.send(reduced_motion_of(&args.value)).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    /// Wraps a borrowed value as the owned variant the portal hands back.
    fn owned(value: Variant<'_>) -> OwnedVariant {
        OwnedVariant::try_from(value).expect("an owned variant")
    }

    #[test]
    fn only_one_asks_for_less_motion() {
        assert!(reduces_motion(1));
    }

    #[test]
    fn zero_and_unknown_values_keep_motion() {
        assert!(!reduces_motion(0));
        assert!(!reduces_motion(2));
        assert!(!reduces_motion(u32::MAX));
    }

    #[test]
    fn a_plain_number_is_read_directly() {
        assert!(reduced_motion_of(&owned(Variant::from(1u32))));
    }

    #[test]
    fn a_number_nested_in_a_variant_is_unwrapped() {
        let nested = Variant::Variant(Box::new(Variant::from(1u32)));

        assert!(reduced_motion_of(&owned(nested)));
    }

    #[test]
    fn a_value_that_is_not_a_number_keeps_motion() {
        assert!(!reduced_motion_of(&owned(Variant::from("yes"))));
    }
}
