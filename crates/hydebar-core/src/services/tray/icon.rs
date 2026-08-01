use freedesktop_icons::lookup;
use iced::widget::{image, svg};
use linicon_theme::get_icon_theme;
use log::{debug, trace};

use super::{TrayIcon, dbus::Icon};

pub(crate) fn icon_from_pixmaps(pixmaps: Vec<Icon>) -> Option<TrayIcon> {
    pixmaps
        .into_iter()
        .max_by_key(|icon| {
            trace!("tray icon w {}, h {}", icon.width, icon.height);
            (icon.width, icon.height)
        })
        .map(|mut icon| {
            for pixel in icon.bytes.chunks_exact_mut(4) {
                pixel.rotate_left(1);
            }

            let (width, height, bytes) =
                trim_transparent(icon.width as u32, icon.height as u32, icon.bytes);

            TrayIcon::Image(image::Handle::from_rgba(width, height, bytes))
        })
}

/// Crops the fully transparent border off an RGBA pixmap.
///
/// Every application pads its tray icon to its own taste — one ships 16
/// pixels of ink on a 22 pixel canvas, the next fills the canvas edge to
/// edge — and icons scaled to one box height come out at visibly different
/// sizes. Trimming to the ink first is what makes one height mean one size.
fn trim_transparent(width: u32, height: u32, bytes: Vec<u8>) -> (u32, u32, Vec<u8>) {
    let alpha_at = |x: u32, y: u32| bytes[((y * width + x) * 4 + 3) as usize] > 0;

    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    for y in 0..height {
        for x in 0..width {
            if alpha_at(x, y) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x > max_x || min_y > max_y || (min_x == 0 && min_y == 0 && max_x == width - 1 && max_y == height - 1)
    {
        return (width, height, bytes);
    }

    let new_width = max_x - min_x + 1;
    let new_height = max_y - min_y + 1;
    let mut trimmed = Vec::with_capacity((new_width * new_height * 4) as usize);

    for y in min_y..=max_y {
        let start = ((y * width + min_x) * 4) as usize;
        let end = start + (new_width * 4) as usize;
        trimmed.extend_from_slice(&bytes[start..end]);
    }

    (new_width, new_height, trimmed)
}

pub(crate) fn icon_from_name(icon_name: &str) -> Option<TrayIcon> {
    debug!("resolving icon from name {icon_name}");

    let theme = get_icon_theme();
    if let Some(theme_name) = &theme {
        debug!("icon theme found {theme_name}");
    }

    let icon_path = if let Some(theme_name) = theme.as_deref() {
        // Try with theme first
        lookup(icon_name)
            .with_cache()
            .with_theme(theme_name)
            .find()
            // Fall back to default lookup if theme lookup fails
            .or_else(|| lookup(icon_name).with_cache().find())
    } else {
        // No theme, use default lookup
        lookup(icon_name).with_cache().find()
    }?;

    if icon_path.extension().is_some_and(|ext| ext == "svg") {
        Some(rasterized_svg(&icon_path).unwrap_or_else(|| {
            TrayIcon::Svg(svg::Handle::from_path(icon_path))
        }))
    } else {
        Some(trimmed_raster(&icon_path))
    }
}

/// Side the vector icons are rasterised at, in pixels.
///
/// Comfortably above any bar the theme can ask for, so the renderer only
/// ever scales down.
const SVG_RASTER_SIDE: f32 = 96.0;

/// A vector icon rendered to pixels and trimmed to its ink.
///
/// An SVG carries its padding inside its own view box, where no widget can
/// reach it: two vector icons drawn into equal boxes still come out at
/// whatever sizes their authors left around the ink. Rasterising with the
/// same engine the renderer uses, then trimming like any pixmap, is what
/// puts them in one row with everything else.
fn rasterized_svg(path: &std::path::Path) -> Option<TrayIcon> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;

    let size = tree.size();
    let scale = SVG_RASTER_SIDE / size.width().max(size.height()).max(1.0);
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut()
    );

    let bytes = straight_alpha(pixmap.take());
    let (width, height, bytes) = trim_transparent(pixmap_width(width), height.max(1), bytes);

    Some(TrayIcon::Image(image::Handle::from_rgba(
        width, height, bytes
    )))
}

/// Width as the pixmap actually allocated it.
fn pixmap_width(width: u32) -> u32 {
    width.max(1)
}

/// Converts premultiplied RGBA to the straight alpha the renderer expects.
fn straight_alpha(mut bytes: Vec<u8>) -> Vec<u8> {
    for pixel in bytes.chunks_exact_mut(4) {
        let alpha = pixel[3] as u16;

        if alpha > 0 && alpha < 255 {
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u16 * 255) / alpha).min(255) as u8;
            }
        }
    }

    bytes
}

/// A raster icon file, trimmed to its ink like a pixmap would be.
///
/// Theme files pad their icons exactly the way pixmaps do, so an untrimmed
/// file next to a trimmed pixmap would put the size mismatch right back on
/// the bar. A file that cannot be decoded is handed to the renderer whole —
/// it decodes more formats than the trimmer reads.
fn trimmed_raster(path: &std::path::Path) -> TrayIcon {
    let Ok(decoded) = ::image::open(path) else {
        return TrayIcon::Image(image::Handle::from_path(path));
    };

    let rgba = decoded.into_rgba8();
    let (width, height) = rgba.dimensions();
    let (width, height, bytes) = trim_transparent(width, height, rgba.into_raw());

    TrayIcon::Image(image::Handle::from_rgba(width, height, bytes))
}

#[cfg(test)]
fn icon_path_with_theme_fallback<F, G>(
    theme: Option<String>,
    mut themed_lookup: F,
    mut default_lookup: G
) -> Option<std::path::PathBuf>
where
    F: FnMut(&str) -> Option<std::path::PathBuf>,
    G: FnMut() -> Option<std::path::PathBuf>
{
    if let Some(theme_name) = theme.as_deref()
        && let Some(path) = themed_lookup(theme_name)
    {
        return Some(path);
    }

    default_lookup()
}

#[cfg(test)]
mod trim_tests {
    use super::trim_transparent;

    /// A canvas of `side`×`side` clear pixels with an opaque square between
    /// `from` and `to` inclusive.
    fn canvas(side: u32, from: u32, to: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; (side * side * 4) as usize];

        for y in from..=to {
            for x in from..=to {
                bytes[((y * side + x) * 4 + 3) as usize] = 255;
            }
        }

        bytes
    }

    #[test]
    fn the_transparent_border_is_cut_to_the_ink() {
        let (width, height, bytes) = trim_transparent(22, 22, canvas(22, 3, 18));

        assert_eq!((width, height), (16, 16));
        assert_eq!(bytes.len(), 16 * 16 * 4);
        assert!(bytes.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn an_icon_filling_its_canvas_is_left_alone() {
        let full = canvas(16, 0, 15);
        let (width, height, bytes) = trim_transparent(16, 16, full.clone());

        assert_eq!((width, height), (16, 16));
        assert_eq!(bytes, full);
    }

    #[test]
    fn a_fully_transparent_icon_is_left_alone() {
        let clear = vec![0u8; 8 * 8 * 4];
        let (width, height, bytes) = trim_transparent(8, 8, clear.clone());

        assert_eq!((width, height), (8, 8));
        assert_eq!(bytes, clear);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering}
    };

    use super::icon_path_with_theme_fallback;

    #[test]
    fn uses_theme_when_available() {
        let theme_calls = AtomicUsize::new(0);
        let default_calls = AtomicUsize::new(0);

        let expected = PathBuf::from("/tmp/themed.svg");

        let result = icon_path_with_theme_fallback(
            Some(String::from("test")),
            |_| {
                theme_calls.fetch_add(1, Ordering::Relaxed);
                Some(expected.clone())
            },
            || {
                default_calls.fetch_add(1, Ordering::Relaxed);
                Some(PathBuf::from("/tmp/default.svg"))
            }
        );

        assert_eq!(theme_calls.load(Ordering::Relaxed), 1);
        assert_eq!(default_calls.load(Ordering::Relaxed), 0);
        assert_eq!(result.as_deref(), Some(expected.as_path()));
    }

    #[test]
    fn falls_back_to_default_when_theme_missing() {
        let theme_calls = AtomicUsize::new(0);
        let default_calls = AtomicUsize::new(0);

        let expected = PathBuf::from("/tmp/default.svg");

        let result = icon_path_with_theme_fallback(
            Some(String::from("test")),
            |_| {
                theme_calls.fetch_add(1, Ordering::Relaxed);
                None
            },
            || {
                default_calls.fetch_add(1, Ordering::Relaxed);
                Some(expected.clone())
            }
        );

        assert_eq!(theme_calls.load(Ordering::Relaxed), 1);
        assert_eq!(default_calls.load(Ordering::Relaxed), 1);
        assert_eq!(result.as_deref(), Some(expected.as_path()));
    }
}
