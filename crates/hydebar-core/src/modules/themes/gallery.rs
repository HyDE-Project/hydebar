//! The upstream theme catalogue, and the reader that fetches it.

mod fetch;
mod index;
mod local;

pub use fetch::load;
pub(super) use local::{local_author, local_screenshots};

/// One theme the gallery offers.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryTheme {
    pub name:        String,
    pub link:        String,
    pub owner:       String,
    pub description: String,
    /// The two colours the index announces the theme with.
    pub colors:      [iced::Color; 2]
}

/// The command the install runs, quoted for the shell.
pub fn import_command(name: &str, link: &str) -> String {
    format!(
        "hydectl theme import --name '{}' --url '{}'",
        name.replace('\'', "'\\''"),
        link.replace('\'', "'\\''")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_import_command_survives_a_quoted_name() {
        let command = import_command("O'Dark", "https://x/y");

        assert!(command.starts_with("hydectl theme import --name "));
        assert!(command.contains("https://x/y"));
    }
}
