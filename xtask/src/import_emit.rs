//! Rendering a migration into TOML the bar reads.
//!
//! Emission quotes through one escaper and spells modules through
//! [`ModuleName::as_str`], so the output parses back into the same names
//! the mapping read: no hand-kept strings on either side.

use std::fmt::Write as _;

use super::import::{Converted, CustomEntry, SectionEntry};

/// Renders the migration as a configuration file.
pub fn render(converted: &Converted, source: &str) -> String {
    let mut output = format!(
        "# Migrated from {source} by `cargo xtask import-waybar`.\n# Review module by module: scripts listed on standard error need a look.\n"
    );

    if let Some(position) = &converted.position {
        let _ = writeln!(output, "position = \"{position}\"");
    }
    if let Some(layer) = &converted.layer {
        let _ = writeln!(output, "layer = \"{layer}\"");
    }

    if converted.position.is_some() || converted.layer.is_some() {
        output.push('\n');
    }

    output.push_str("[modules]\n");
    for (section, entries) in &converted.sections {
        let rendered: Vec<String> = entries.iter().map(render_entry).collect();
        let _ = writeln!(output, "{section} = [{}]", rendered.join(", "));
    }

    if let Some(format) = &converted.clock_format {
        let _ = writeln!(output, "\n[clock]\nformat = {}", toml_string(format));
    }

    for custom in &converted.customs {
        output.push_str(&render_custom(custom));
    }

    output
}

/// Renders one section entry: a name or an island of names.
fn render_entry(entry: &SectionEntry) -> String {
    match entry {
        SectionEntry::Single(name) => toml_string(name),
        SectionEntry::Group(members) => {
            let rendered: Vec<String> = members.iter().map(|name| toml_string(name)).collect();
            format!("[{}]", rendered.join(", "))
        }
    }
}

/// Renders one carried-over custom module.
fn render_custom(custom: &CustomEntry) -> String {
    let mut output = format!("\n[[CustomModule]]\nname = {}\n", toml_string(&custom.name));

    if let Some(exec) = &custom.exec {
        let _ = writeln!(output, "exec = {}", toml_string(exec));
    }
    if let Some(interval) = custom.interval {
        let _ = writeln!(output, "interval = {interval}");
    }
    if let Some(signal) = custom.signal {
        let _ = writeln!(output, "signal = {signal}");
    }
    if let Some(command) = &custom.command {
        let _ = writeln!(output, "command = {}", toml_string(command));
    }

    output
}

/// Quotes a string for TOML, escaping what must not stand raw.
fn toml_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');

    for char in value.chars() {
        match char {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            char if u32::from(char) < 0x20 => {
                let _ = write!(output, "\\u{:04X}", u32::from(char));
            }
            char => output.push(char)
        }
    }

    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_escape_for_toml() {
        assert_eq!(toml_string("plain"), "\"plain\"");
        assert_eq!(toml_string("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(toml_string("a\\b"), "\"a\\\\b\"");
        assert_eq!(toml_string("a\nb"), "\"a\\nb\"");
    }

    #[test]
    fn islands_render_as_arrays() {
        let entry = SectionEntry::Group(vec!["Clock".to_owned(), "mine".to_owned()]);

        assert_eq!(render_entry(&entry), "[\"Clock\", \"mine\"]");
    }
}
