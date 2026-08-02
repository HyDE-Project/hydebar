//! Reading a module name out of the configuration, aliases included.

use std::fmt;

use serde::{Deserialize, Deserializer};

use super::name::ModuleName;

impl<'de> Deserialize<'de> for ModuleName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        struct ModuleNameVisitor;

        impl serde::de::Visitor<'_> for ModuleNameVisitor {
            type Value = ModuleName;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a ModuleName")
            }

            fn visit_str<E>(self, value: &str) -> Result<ModuleName, E>
            where
                E: serde::de::Error
            {
                match value {
                    "hyde-menu" => return Ok(ModuleName::HydeMenu),
                    "cpu" => return Ok(ModuleName::Cpu),
                    "memory" => return Ok(ModuleName::Memory),
                    "cpu-temp" | "temperature" => return Ok(ModuleName::CpuTemp),
                    "gpu-temp" => return Ok(ModuleName::GpuTemp),
                    _ => {}
                }

                Ok(ModuleName::BUILT_IN
                    .iter()
                    .find(|module| module.as_str() == value)
                    .cloned()
                    .unwrap_or_else(|| ModuleName::Custom(value.to_string())))
            }
        }

        deserializer.deserialize_str(ModuleNameVisitor)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde::de::value::{Error as DeError, StrDeserializer};

    use super::*;

    #[test]
    fn module_name_deserializes_idle_inhibitor() {
        let name = ModuleName::deserialize(StrDeserializer::<DeError>::new("IdleInhibitor"))
            .expect("known variant");
        assert_eq!(name, ModuleName::IdleInhibitor);
    }

    #[test]
    fn module_name_deserializes_custom_values() {
        let name = ModuleName::deserialize(StrDeserializer::<DeError>::new("MyCustom"))
            .expect("custom variant");
        assert!(matches!(name, ModuleName::Custom(value) if value == "MyCustom"));
    }

    #[test]
    fn the_theme_module_reads_back_as_it_is_written() {
        let name: ModuleName =
            Deserialize::deserialize(StrDeserializer::<DeError>::new("Themes")).expect("name");

        assert_eq!(name, ModuleName::Themes);
        assert_eq!(name.as_str(), "Themes");
    }

    /// A hand-kept name list once forgot `Wallpaper`, turning the built-in
    /// module into an undefined custom one and discarding the whole user
    /// configuration at the validation step. Every shipped module must read
    /// back as itself.
    #[test]
    fn every_built_in_module_reads_back_as_it_is_written() {
        for module in &ModuleName::BUILT_IN {
            let name: ModuleName =
                Deserialize::deserialize(StrDeserializer::<DeError>::new(module.as_str()))
                    .expect("built-in name");

            assert_eq!(&name, module);
        }
    }

    /// The layouts spell the processor and memory readouts in lower case, so
    /// both spellings have to land on the standalone modules.
    #[test]
    fn the_processor_and_memory_entries_read_in_both_spellings() {
        for spelling in ["cpu", "Cpu"] {
            let name: ModuleName =
                Deserialize::deserialize(StrDeserializer::<DeError>::new(spelling)).expect("name");

            assert_eq!(name, ModuleName::Cpu);
        }

        for spelling in ["memory", "Memory"] {
            let name: ModuleName =
                Deserialize::deserialize(StrDeserializer::<DeError>::new(spelling)).expect("name");

            assert_eq!(name, ModuleName::Memory);
        }
    }

    /// The user's own `[[CustomModule]] name = "theme"` must keep being a
    /// custom module: the built in one answers to `Themes`, and nothing else.
    #[test]
    fn a_lowercase_theme_stays_the_custom_module_of_that_name() {
        let name: ModuleName =
            Deserialize::deserialize(StrDeserializer::<DeError>::new("theme")).expect("name");

        assert_eq!(name, ModuleName::Custom("theme".to_owned()));
    }
}
