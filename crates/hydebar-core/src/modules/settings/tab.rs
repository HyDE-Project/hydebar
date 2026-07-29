//! Sections the settings window is split into.

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
    pub const ALL: [Tab; 2] = [Tab::Appearance, Tab::Modules];

    /// Name shown on the tab.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Tab::Appearance => "Appearance",
            Tab::Modules => "Modules"
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
}
