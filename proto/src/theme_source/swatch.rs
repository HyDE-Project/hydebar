//! The colours a theme would paint the desktop in, read for previewing it.
//!
//! A list of theme names answers "which themes are there", not "what do they
//! look like". The look is on disk already, in the theme's own files: every
//! `HyDE` theme ships its terminal palette, and that palette *is* the theme's
//! identity — its background, its text, and the signature hues its author
//! chose. Colours extracted from the theme's wallpaper answer only when a
//! theme ships no palette of its own: they are clusters squeezed out of a
//! picture, honest but muddy, and no substitute for the author's word.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, PoisonError},
    time::SystemTime
};

use super::{color::parse_color, dcol::DcolPalette};
use crate::{hyde_dirs::HydeDirs, hyde_files, theme_source::Rgba};

/// Digest of a wallpaper file, remembered by the file's identity on disk.
#[derive(Debug, Clone)]
struct CachedDigest {
    len:      u64,
    modified: Option<SystemTime>,
    digest:   String
}

/// Wallpaper digests already computed this session.
///
/// Hashing a wallpaper reads the whole multi-megabyte file, the menu asks for
/// every theme's palette each time it opens, and wallpapers change rarely. An
/// entry is revalidated by size and mtime, so an untouched file costs a stat
/// and an edited one is read again.
static DIGESTS: LazyLock<Mutex<HashMap<PathBuf, CachedDigest>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Extension of the per-application style files a theme ships.
///
/// Which applications those are is the theme's business: the reader scans
/// whatever is there and takes the first file that states a palette, so a
/// theme shipping styles for any terminal — or none — is read the same way.
const THEME_FILE_EXTENSION: &str = "theme";

/// The colours a theme announces itself with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeSwatch {
    /// Surface the theme paints things on.
    pub background: Rgba,
    /// Text the theme sets on that surface.
    pub text:       Rgba,
    /// Accent the theme highlights with.
    pub accent:     Rgba,
    /// The palette itself, for drawing the theme rather than naming it.
    ///
    /// The three remaining primaries and the brightest accent: enough to tell
    /// two dark themes apart at a glance, which one flat surface colour never
    /// could.
    pub palette:    [Rgba; 4]
}

/// Reads the swatch of `theme`, if anything on disk answers for its colours.
///
/// The theme's own terminal palette wins — it is the author's statement of
/// what the theme looks like. The palettes `HyDE` keeps for it — one the theme
/// pinned, else one extracted from its current wallpaper — answer otherwise.
/// A theme with none of it has no swatch, and the caller paints its entry the
/// way it paints everything.
#[must_use]
pub fn theme_swatch(dirs: &HydeDirs, theme: &str) -> Option<ThemeSwatch> {
    shipped_swatch(dirs, theme).or_else(|| extracted_swatch(dirs, theme))
}

/// The swatch as the theme's author stated it, from the style files it ships.
///
/// Every style file of a theme is a deployment header followed by `key value`
/// lines, and any of them stating a full colour scheme — a background, a
/// foreground and the numbered scheme colours — states the theme's identity.
/// The files are tried in name order so the answer is stable, and the first
/// one that yields a whole swatch wins.
fn shipped_swatch(dirs: &HydeDirs, theme: &str) -> Option<ThemeSwatch> {
    let mut files: Vec<_> = hyde_files::entries(&dirs.theme_dir(theme))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == THEME_FILE_EXTENSION)
        })
        .collect();
    files.sort();

    files
        .into_iter()
        .filter_map(|path| hyde_files::text(&path))
        .find_map(|source| swatch_of(&source))
}

/// Reads one style file as a swatch, if it states a whole colour scheme.
///
/// The four dots are the scheme's first accent hues — its red, green, yellow
/// and blue — which is where one colour scheme differs from another to a
/// reader's eye.
fn swatch_of(source: &str) -> Option<ThemeSwatch> {
    let mut background = None;
    let mut foreground = None;
    let mut border = None;
    let mut hues: [Option<Rgba>; 4] = [None; 4];

    for line in source.lines().skip(1) {
        let mut parts = line.split([' ', '\t', '=']).filter(|part| !part.is_empty());
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };

        let slot = match key {
            "background" => &mut background,
            "foreground" => &mut foreground,
            "active_border_color" => &mut border,
            "color1" => &mut hues[0],
            "color2" => &mut hues[1],
            "color3" => &mut hues[2],
            "color4" => &mut hues[3],
            _ => continue
        };

        if slot.is_none() {
            *slot = parse_color(value.trim_matches(['"', '\'']));
        }
    }

    let palette = [hues[0]?, hues[1]?, hues[2]?, hues[3]?];

    Some(ThemeSwatch {
        background: background?,
        text: foreground?,
        accent: border.or(hues[3])?,
        palette
    })
}

/// The swatch from the palettes `HyDE` keeps for the theme.
fn extracted_swatch(dirs: &HydeDirs, theme: &str) -> Option<ThemeSwatch> {
    let palette = pinned_palette(dirs, theme).or_else(|| wallpaper_palette(dirs, theme))?;

    Some(ThemeSwatch {
        background: palette.primary[0],
        text:       palette.text[0],
        accent:     palette.primary[3],
        palette:    [
            palette.primary[1],
            palette.primary[2],
            palette.primary[3],
            palette.accents[3][8]
        ]
    })
}

/// The palette the theme ships under its own name.
fn pinned_palette(dirs: &HydeDirs, theme: &str) -> Option<DcolPalette> {
    DcolPalette::parse(&hyde_files::text(&dirs.theme_dcol(theme))?)
}

/// The palette `HyDE` extracted from the theme's current wallpaper.
///
/// The cache is keyed by the digest of the image file, exactly as the scripts
/// key it, so the bar finds the palette wherever the wallpaper file lives.
fn wallpaper_palette(dirs: &HydeDirs, theme: &str) -> Option<DcolPalette> {
    let image = fs::canonicalize(dirs.theme_dir(theme).join("wall.set")).ok()?;
    let digest = wallpaper_digest(&image)?;
    let cached = dirs
        .hyde_cache_dir()
        .join("dcols")
        .join(format!("{digest}.dcol"));

    DcolPalette::parse(&hyde_files::text(&cached)?)
}

/// Digest of the image at `image`, hashed once per file version.
///
/// The read and the hash happen outside the lock: two themes pointing at two
/// wallpapers hash in parallel, and the lock only guards the map itself.
pub(super) fn wallpaper_digest(image: &Path) -> Option<String> {
    let metadata = fs::metadata(image).ok()?;
    let len = metadata.len();
    let modified = metadata.modified().ok();

    {
        let cache = DIGESTS.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(hit) = cache.get(image)
            && hit.len == len
            && hit.modified == modified
        {
            return Some(hit.digest.clone());
        }
    }

    let bytes = fs::read(image).ok()?;
    let digest = sha1_smol::Sha1::from(&bytes).digest().to_string();

    DIGESTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(
            image.to_owned(),
            CachedDigest {
                len,
                modified,
                digest: digest.clone()
            }
        );

    Some(digest)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use tempfile::TempDir;

    use super::{super::dcol::fixtures::WALL_DCOL, *};

    fn install() -> (TempDir, HydeDirs) {
        let root = TempDir::new().expect("tempdir");
        let dirs = HydeDirs::new(
            root.path().join("config"),
            root.path().join("state"),
            root.path().join("cache"),
            root.path().join("data")
        );

        (root, dirs)
    }

    /// Whatever application the style file was written for, a full colour
    /// scheme in it is the theme's own statement of its colours.
    #[test]
    fn a_shipped_style_file_states_the_swatch() {
        let (_root, dirs) = install();
        let theme_dir = dirs.theme_dir("Nord");
        fs::create_dir_all(&theme_dir).expect("theme dir");
        fs::write(
            theme_dir.join("anyterm.theme"),
            "$HOME/.config/anyterm/theme.conf|reload\n\
             background #1E1E2E\n\
             foreground = \"#CDD6F4\"\n\
             active_border_color #CBA6F7\n\
             color1 #F38BA8\n\
             color2 #A6E3A1\n\
             color3 #F9E2AF\n\
             color4 #89B4FA\n"
        )
        .expect("style file");

        let swatch = theme_swatch(&dirs, "Nord").expect("swatch");

        assert_eq!(swatch.background, Rgba::rgb(0x1E, 0x1E, 0x2E));
        assert_eq!(swatch.text, Rgba::rgb(0xCD, 0xD6, 0xF4));
        assert_eq!(swatch.accent, Rgba::rgb(0xCB, 0xA6, 0xF7));
        assert_eq!(swatch.palette[3], Rgba::rgb(0x89, 0xB4, 0xFA));
    }

    /// A style file with no palette — a wallpaper list, a lock screen — is
    /// passed over for one that states the scheme.
    #[test]
    fn a_file_without_a_scheme_is_passed_over() {
        let (_root, dirs) = install();
        let theme_dir = dirs.theme_dir("Nord");
        fs::create_dir_all(&theme_dir).expect("theme dir");
        fs::write(
            theme_dir.join("aaa.theme"),
            "$HOME/.config/aaa/x|reload\nnothing here\n"
        )
        .expect("empty style");
        fs::write(
            theme_dir.join("bbb.theme"),
            "$HOME/.config/bbb/x|reload\n\
             background #272727\nforeground #EBDBB2\n\
             color1 #EA6962\ncolor2 #A9B665\ncolor3 #D8A657\ncolor4 #7DAEA3\n"
        )
        .expect("scheme style");

        let swatch = theme_swatch(&dirs, "Nord").expect("swatch");

        assert_eq!(swatch.background, Rgba::rgb(0x27, 0x27, 0x27));
    }

    #[test]
    fn a_pinned_palette_answers_for_the_theme() {
        let (_root, dirs) = install();
        let theme_dir = dirs.theme_dir("Nord");
        fs::create_dir_all(&theme_dir).expect("theme dir");
        fs::write(dirs.theme_dcol("Nord"), WALL_DCOL).expect("palette");

        let swatch = theme_swatch(&dirs, "Nord").expect("swatch");

        assert_eq!(swatch.background, Rgba::rgb(0x48, 0x38, 0x28));
    }

    #[test]
    fn a_cached_wallpaper_palette_answers_when_nothing_is_pinned() {
        let (root, dirs) = install();
        let theme_dir = dirs.theme_dir("Nord");
        fs::create_dir_all(&theme_dir).expect("theme dir");

        let image = root.path().join("wall.png");
        fs::write(&image, b"not really a picture").expect("image");
        std::os::unix::fs::symlink(&image, theme_dir.join("wall.set")).expect("link");

        let digest = sha1_smol::Sha1::from(b"not really a picture")
            .digest()
            .to_string();
        let dcols = dirs.hyde_cache_dir().join("dcols");
        fs::create_dir_all(&dcols).expect("dcols dir");
        fs::write(dcols.join(format!("{digest}.dcol")), WALL_DCOL).expect("cache");

        assert!(theme_swatch(&dirs, "Nord").is_some());
    }

    #[test]
    fn a_theme_with_no_palette_anywhere_has_no_swatch() {
        let (_root, dirs) = install();

        assert_eq!(theme_swatch(&dirs, "Ghost"), None);
    }
}
