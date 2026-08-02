//! Turning pixmaps and icon files into renderer-ready images.

use iced::widget::image;
use log::trace;

use super::{
    super::{TrayIcon, dbus::Icon},
    trim::trim_transparent
};

#[expect(
    clippy::cast_sign_loss,
    reason = "a tray pixmap reports positive dimensions over the bus"
)]
pub fn icon_from_pixmaps(pixmaps: Vec<Icon>) -> Option<TrayIcon> {
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
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the scaled sides are positive and bounded by the raster side constant"
)]
pub(super) fn rasterized_svg(path: &std::path::Path) -> Option<TrayIcon> {
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
        let alpha = u16::from(pixel[3]);

        if alpha > 0 && alpha < 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * 255) / alpha).min(255) as u8;
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
pub(super) fn trimmed_raster(path: &std::path::Path) -> TrayIcon {
    let Ok(decoded) = ::image::open(path) else {
        return TrayIcon::Image(image::Handle::from_path(path));
    };

    let rgba = decoded.into_rgba8();
    let (width, height) = rgba.dimensions();
    let (width, height, bytes) = trim_transparent(width, height, rgba.into_raw());

    TrayIcon::Image(image::Handle::from_rgba(width, height, bytes))
}
