//! Cropping transparent borders off icon pixmaps.

/// Crops the fully transparent border off an RGBA pixmap.
///
/// Every application pads its tray icon to its own taste — one ships 16
/// pixels of ink on a 22 pixel canvas, the next fills the canvas edge to
/// edge — and icons scaled to one box height come out at visibly different
/// sizes. Trimming to the ink first is what makes one height mean one size.
pub(super) fn trim_transparent(width: u32, height: u32, bytes: Vec<u8>) -> (u32, u32, Vec<u8>) {
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

    if min_x > max_x
        || min_y > max_y
        || (min_x == 0 && min_y == 0 && max_x == width - 1 && max_y == height - 1)
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
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
