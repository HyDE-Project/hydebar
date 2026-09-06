//! Rendering templates into new module files.
//!
//! Placeholders are dumb text (`{{Pascal}}`); every decision lives in
//! [`names`] and [`kinds`], so rendering itself is a substitution table the
//! tests pin down.

use std::{
    fs, io,
    path::{Path, PathBuf}
};

use super::{kinds::ModuleKind, names::ModuleNames};

/// One file to write, relative to the workspace root.
pub struct RenderedFile {
    /// Destination relative to the workspace root.
    pub path:    PathBuf,
    /// Content with every placeholder filled.
    pub content: String
}

/// Fills every placeholder for the module shape and lists the files.
///
/// # Panics
///
/// Never panics at runtime: templates ship with the binary and their
/// placeholders are fixed beside the values filling them.
pub fn render(names: &ModuleNames, kind: ModuleKind) -> Vec<RenderedFile> {
    let (struct_attr, debug_impl) = kind.debug_pieces();
    // Structural placeholders first: their values may themselves name
    // scalar ones (`{{Pascal}}` inside `{{root_init}}`), which the second
    // group then fills. Scalar values never nest, so one pass suffices.
    let values = [
        ("{{root_imports}}", kind.root_imports().to_owned()),
        ("{{root_modules}}", kind.root_modules().to_owned()),
        ("{{module_line}}", kind.module_line().to_owned()),
        ("{{struct_attr}}", struct_attr.to_owned()),
        ("{{root_fields}}", kind.root_fields().to_owned()),
        ("{{root_init}}", kind.root_init().to_owned()),
        ("{{debug_impl}}", debug_impl.to_owned()),
        ("{{press_action}}", kind.press_action().to_owned()),
        ("{{view_imports}}", kind.view_imports().to_owned()),
        ("{{Snake}}", names.snake.clone()),
        ("{{Pascal}}", names.pascal.clone()),
        ("{{label}}", names.label.clone()),
        ("{{doc}}", names.doc.clone())
    ];

    let fill = |template: &str| -> String {
        let mut content = template.to_owned();
        for (placeholder, value) in &values {
            content = content.replace(placeholder, value);
        }
        content
    };

    let mut files = vec![
        RenderedFile {
            path:    PathBuf::from(format!("proto/src/config/{}.rs", names.snake)),
            content: fill(include_str!("../templates/proto_config.txt"))
        },
        RenderedFile {
            path:    PathBuf::from(format!("core/src/modules/{}.rs", names.snake)),
            content: fill(include_str!("../templates/core_root.txt"))
        },
        RenderedFile {
            path:    PathBuf::from(format!("core/src/modules/{}/state.rs", names.snake)),
            content: fill(include_str!("../templates/core_state.txt"))
        },
        RenderedFile {
            path:    PathBuf::from(format!("core/src/modules/{}/view.rs", names.snake)),
            content: fill(include_str!("../templates/core_view.txt"))
        },
    ];

    if kind.needs_module_file() {
        let template = match kind {
            ModuleKind::Polling => Some(include_str!("../templates/core_module_polling.txt")),
            ModuleKind::Listener | ModuleKind::Menu => {
                Some(include_str!("../templates/core_module_listener.txt"))
            }
            ModuleKind::Simple => None
        };

        if let Some(template) = template {
            files.push(RenderedFile {
                path:    PathBuf::from(format!("core/src/modules/{}/module.rs", names.snake)),
                content: fill(template)
            });
        }
    }

    files
}

/// Writes rendered files under the workspace root, refusing overwrites.
///
/// # Errors
///
/// Returns the I/O error when a directory cannot be made or a file cannot
/// be written, including the refusal to overwrite an existing file.
pub fn write_files(root: &Path, files: &[RenderedFile]) -> io::Result<()> {
    for file in files {
        let path = root.join(&file.path);

        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("refusing to overwrite {}", path.display())
            ));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, &file.content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::derive;

    fn rendered(kind: ModuleKind) -> Vec<RenderedFile> {
        let names = derive("demo", None, None).expect("the fixture derives");
        render(&names, kind)
    }

    #[test]
    fn no_placeholder_survives_rendering() {
        for kind in [
            ModuleKind::Simple,
            ModuleKind::Polling,
            ModuleKind::Listener,
            ModuleKind::Menu
        ] {
            for file in rendered(kind) {
                assert!(
                    !file.content.contains("{{"),
                    "{} keeps a placeholder",
                    file.path.display()
                );
            }
        }
    }

    #[test]
    fn a_static_entry_needs_no_trait_file() {
        let files = rendered(ModuleKind::Simple);

        assert_eq!(files.len(), 4);
        assert!(files.iter().all(|file| !file.path.ends_with("module.rs")));
    }

    #[test]
    fn other_shapes_ship_their_trait_implementation() {
        for kind in [ModuleKind::Polling, ModuleKind::Listener, ModuleKind::Menu] {
            let files = rendered(kind);

            assert_eq!(files.len(), 5, "{kind:?}");
            assert!(
                files.iter().any(|file| file.path.ends_with("module.rs")),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn only_a_menu_entry_toggles_a_menu() {
        let plain = rendered(ModuleKind::Listener);
        let menu = rendered(ModuleKind::Menu);

        let view = |files: &[RenderedFile]| {
            files
                .iter()
                .find(|file| file.path.ends_with("view.rs"))
                .expect("a view renders")
                .content
                .clone()
        };

        assert!(view(&plain).contains("None"));
        assert!(!view(&plain).contains("ToggleMenu"));
        assert!(view(&menu).contains("ToggleMenu(MenuType::Demo)"));
        assert!(view(&menu).contains("use crate::menu::MenuType;"));
    }
}
