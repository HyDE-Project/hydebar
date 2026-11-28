//! Which drawing backend the bar asks its renderer for.
//!
//! The renderer tries the chosen backend at startup and, in `auto`, falls
//! back to the other when the first cannot be brought up. The choice is a
//! hardware policy, not a look: it is read once, before the first frame,
//! and a hot reload does not re-pick it.

use serde::Deserialize;

/// Which backend the bar draws with.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RendererChoice {
    /// Draw on the GPU when one answers, on the CPU when none does.
    #[default]
    Auto,
    /// Draw on the GPU only; fail to start when no GPU can be brought up.
    Gpu,
    /// Draw on the CPU only; never touch the GPU.
    Cpu
}

impl RendererChoice {
    /// The renderer's own name for this backend, or [`None`] to let the
    /// renderer choose and fall back on its own.
    #[must_use]
    pub const fn backend_name(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Gpu => Some("wgpu"),
            Self::Cpu => Some("tiny-skia")
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::RendererChoice;

    #[test]
    fn auto_is_the_default_and_names_no_backend() {
        assert_eq!(RendererChoice::default(), RendererChoice::Auto);
        assert_eq!(RendererChoice::Auto.backend_name(), None);
    }

    #[test]
    fn the_explicit_choices_name_the_backend_exactly() {
        assert_eq!(RendererChoice::Gpu.backend_name(), Some("wgpu"));
        assert_eq!(RendererChoice::Cpu.backend_name(), Some("tiny-skia"));
    }

    #[derive(serde::Deserialize)]
    struct Holder {
        v: RendererChoice
    }

    #[test]
    fn the_configuration_words_deserialize() {
        for (word, expected) in [
            ("auto", RendererChoice::Auto),
            ("gpu", RendererChoice::Gpu),
            ("cpu", RendererChoice::Cpu)
        ] {
            let parsed: Holder = toml::from_str(&format!(r#"v = "{word}""#))
                .unwrap_or_else(|err| panic!("{word}: {err}"));

            assert_eq!(parsed.v, expected);
        }
    }

    #[test]
    fn an_unknown_backend_word_is_rejected() {
        assert!(
            toml::from_str::<Holder>(r#"v = "discrete""#).is_err(),
            "only the three known words may configure the backend"
        );
    }
}
