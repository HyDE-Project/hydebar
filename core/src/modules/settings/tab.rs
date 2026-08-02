//! Sections the settings window is split into.
//!
//! The window is about the bar. The desktop it sits on — its theme,
//! wallpaper and colours — is driven from the theme module on the bar
//! instead, so no page here reports or changes any of it.

/// Page of the settings window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    /// Height, style, colours and the rest of the look.
    #[default]
    Appearance,
    /// Which modules the bar shows, in which order and grouping.
    Modules
}

impl Tab {
    /// Every tab, in the order the window lists them.
    pub const ALL: [Self; 2] = [Self::Appearance, Self::Modules];

    /// Name shown on the tab.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Modules => "Modules"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_opens_on_the_appearance_page() {
        assert_eq!(Tab::default(), Tab::Appearance);
    }

    #[test]
    fn every_tab_is_listed_once_and_named() {
        assert_eq!(Tab::ALL.len(), 2);

        for tab in Tab::ALL {
            assert!(!tab.label().is_empty());
        }
    }

    /// The desktop moved to the theme module wholesale, so the window is
    /// left with the two pages that are about the bar itself.
    #[test]
    fn the_window_is_about_the_bar_and_nothing_else() {
        assert_eq!(Tab::ALL, [Tab::Appearance, Tab::Modules]);
    }
}
