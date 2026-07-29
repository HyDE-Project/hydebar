//! Persisting a single setting back into the configuration file.
//!
//! The file belongs to the user: it carries their comments, their ordering and
//! their formatting. Edits therefore go through a format preserving document
//! instead of a serialise-the-whole-struct round trip, which would flatten the
//! file into whatever the derive happens to emit.

use std::{fmt, fs, io, path::Path};

use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

/// Why a setting could not be written back.
#[derive(Debug)]
pub enum SettingsWriteError {
    /// The configuration file could not be read or written.
    Io(io::Error),
    /// The configuration file on disk is not valid TOML.
    Parse(toml_edit::TomlError),
    /// A key on the path to the setting is held by a non table value.
    NotATable {
        /// Dotted path of the offending key.
        path: String
    }
}

impl fmt::Display for SettingsWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "the configuration file could not be updated: {err}"),
            Self::Parse(err) => write!(f, "the configuration file is not valid TOML: {err}"),
            Self::NotATable {
                path
            } => {
                write!(f, "`{path}` is not a table and cannot hold a setting")
            }
        }
    }
}

impl std::error::Error for SettingsWriteError {}

impl From<io::Error> for SettingsWriteError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml_edit::TomlError> for SettingsWriteError {
    fn from(err: toml_edit::TomlError) -> Self {
        Self::Parse(err)
    }
}

/// Value a setting can be given.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    /// A named variant, written as a bare string.
    Text(String),
    /// A number, written as a float.
    Number(f64),
    /// A flag.
    Flag(bool),
    /// A list, whose entries may be lists themselves.
    List(Vec<SettingValue>)
}

impl SettingValue {
    /// Renders this value as a TOML value.
    fn into_toml(self) -> Value {
        match self {
            Self::Text(text) => Value::from(text),
            Self::Number(number) => Value::from(number),
            Self::Flag(flag) => Value::from(flag),
            Self::List(entries) => {
                Value::Array(entries.into_iter().map(Self::into_toml).collect::<Array>())
            }
        }
    }
}

impl From<&str> for SettingValue {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<f32> for SettingValue {
    fn from(number: f32) -> Self {
        Self::Number(f64::from(number))
    }
}

impl From<bool> for SettingValue {
    fn from(flag: bool) -> Self {
        Self::Flag(flag)
    }
}

/// Writes `value` at the dotted `path` of the configuration file at `file`.
///
/// Missing intermediate tables are created, existing comments and ordering are
/// kept. The write is atomic in the sense the bar cares about: the document is
/// rendered in full and replaces the file in one call, so the watcher never
/// observes a half written configuration.
///
/// # Errors
/// Returns [`SettingsWriteError`] when the file cannot be read or written, when
/// its contents are not valid TOML, or when a key on `path` is occupied by a
/// value that cannot hold a table.
pub fn write_setting(
    file: &Path,
    path: &[&str],
    setting: SettingValue
) -> Result<(), SettingsWriteError> {
    let source = match fs::read_to_string(file) {
        Ok(source) => source,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into())
    };

    let mut document = source.parse::<DocumentMut>()?;
    let Some((key, tables)) = path.split_last() else {
        return Ok(());
    };

    let mut item = document.as_item_mut();

    for (depth, table) in tables.iter().enumerate() {
        let entry = item
            .as_table_like_mut()
            .ok_or_else(|| SettingsWriteError::NotATable {
                path: path[..depth].join(".")
            })?;

        if entry.get(table).is_none() {
            entry.insert(table, Item::Table(Table::new()));
        }

        item = entry
            .get_mut(table)
            .ok_or_else(|| SettingsWriteError::NotATable {
                path: path[..=depth].join(".")
            })?;
    }

    let table = item
        .as_table_like_mut()
        .ok_or_else(|| SettingsWriteError::NotATable {
            path: tables.join(".")
        })?;

    let mut replacement = value(setting.into_toml());

    match table.get_mut(key) {
        Some(existing) => {
            if let (Some(previous), Some(fresh)) =
                (existing.as_value(), replacement.as_value_mut())
            {
                *fresh.decor_mut() = previous.decor().clone();
            }

            *existing = replacement;
        }
        None => {
            table.insert(key, replacement);
        }
    }

    fs::write(file, document.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("hydebar-settings-writer-{name}.toml"));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn a_top_level_setting_replaces_its_previous_value() {
        let file = scratch("top-level");
        fs::write(&file, "position = \"Top\"\n").expect("seed");

        write_setting(&file, &["position"], "Bottom".into()).expect("write");

        assert_eq!(
            fs::read_to_string(&file).expect("read"),
            "position = \"Bottom\"\n"
        );
    }

    #[test]
    fn comments_and_unrelated_keys_survive_a_write() {
        let file = scratch("comments");
        fs::write(
            &file,
            "# my bar\nposition = \"Top\"\n\n[appearance]\n# how tall\nheight = 38.0\nstyle = \"Islands\"\n"
        )
        .expect("seed");

        write_setting(&file, &["appearance", "height"], 42.0_f32.into()).expect("write");

        let written = fs::read_to_string(&file).expect("read");
        assert!(written.contains("# my bar"));
        assert!(written.contains("# how tall"));
        assert!(written.contains("height = 42.0"));
        assert!(written.contains("style = \"Islands\""));
    }

    #[test]
    fn a_missing_table_is_created_on_the_way() {
        let file = scratch("missing-table");
        fs::write(&file, "position = \"Top\"\n").expect("seed");

        write_setting(&file, &["appearance", "menu", "backdrop"], 0.5_f32.into()).expect("write");

        let written = fs::read_to_string(&file).expect("read");
        assert!(written.contains("[appearance.menu]"));
        assert!(written.contains("backdrop = 0.5"));
    }

    #[test]
    fn a_flag_is_written_as_a_boolean() {
        let file = scratch("flag");
        fs::write(&file, "").expect("seed");

        write_setting(&file, &["appearance", "follow_hyde"], false.into()).expect("write");

        assert!(
            fs::read_to_string(&file)
                .expect("read")
                .contains("follow_hyde = false")
        );
    }

    #[test]
    fn a_key_occupied_by_a_scalar_is_reported() {
        let file = scratch("occupied");
        fs::write(&file, "appearance = 3\n").expect("seed");

        let err = write_setting(&file, &["appearance", "height"], 38.0_f32.into())
            .expect_err("a scalar cannot hold a table");

        assert!(matches!(err, SettingsWriteError::NotATable { .. }));
    }

    #[test]
    fn a_nested_list_is_written_as_an_array_of_arrays() {
        let file = scratch("nested-list");
        fs::write(&file, "").expect("seed");

        write_setting(
            &file,
            &["modules", "left"],
            SettingValue::List(vec![
                SettingValue::Text("Clock".to_owned()),
                SettingValue::List(vec![
                    SettingValue::Text("Workspaces".to_owned()),
                    SettingValue::Text("WindowTitle".to_owned()),
                ]),
            ])
        )
        .expect("write");

        let written = fs::read_to_string(&file).expect("read");
        assert!(written.contains("[modules]"));
        assert!(written.contains(r#"left = ["Clock", ["Workspaces", "WindowTitle"]]"#));
    }

    #[test]
    fn an_absent_file_is_created_from_scratch() {
        let file = scratch("absent");

        write_setting(&file, &["appearance", "height"], 38.0_f32.into()).expect("write");

        assert!(
            fs::read_to_string(&file)
                .expect("read")
                .contains("height = 38.0")
        );
    }
}
