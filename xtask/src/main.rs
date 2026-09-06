//! Developer tooling for hydebar.
//!
//! Today it scaffolds new bar modules: `cargo xtask new-module demo --kind
//! polling` writes the configuration, state, view and trait files, then
//! prints the wiring checklist that turns the skeleton into a module.

use std::{env, path::PathBuf, process::ExitCode};

mod checklist;
mod generate;
mod import;
mod import_emit;
mod kinds;
mod names;

use kinds::ModuleKind;

/// Usage lines printed for any invocation the parser refuses.
const USAGE: &str = "usage:\n  cargo xtask new-module <name> [--kind simple|polling|listener|menu] [--label \"...\"] [--doc \"...\"]\n  cargo xtask import-waybar <config.jsonc> [--out <config.toml>]";

/// Command-line flags of `new-module`, in the order they are read.
struct Args {
    /// Module name in `snake_case`.
    name:  String,
    /// Background work the module owns.
    kind:  Option<String>,
    /// Human label, defaulting to the type name.
    label: Option<String>,
    /// Variant documentation.
    doc:   Option<String>
}

/// Reads the `new-module` flags after the subcommand name.
fn parse_args(arguments: &[String]) -> Result<Args, String> {
    let mut args = Args {
        name:  String::new(),
        kind:  None,
        label: None,
        doc:   None
    };
    let mut positionals = 0;

    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--kind" => {
                args.kind = Some(flag_value(&mut rest, "--kind")?.to_owned());
            }
            "--label" => {
                args.label = Some(flag_value(&mut rest, "--label")?.to_owned());
            }
            "--doc" => {
                args.doc = Some(flag_value(&mut rest, "--doc")?.to_owned());
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag '{flag}'\n{USAGE}"));
            }
            positional => {
                positionals += 1;
                if positionals > 1 {
                    return Err(format!("one module name, got more\n{USAGE}"));
                }
                positional.clone_into(&mut args.name);
            }
        }
    }

    if args.name.is_empty() {
        return Err(format!("a module name is required\n{USAGE}"));
    }

    Ok(args)
}

/// Reads the value following a `--flag`, naming the flag when it is missing.
fn flag_value<'a>(rest: &mut std::slice::Iter<'a, String>, flag: &str) -> Result<&'a str, String> {
    rest.next()
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))
}

/// Walks up from the working directory to the workspace root.
fn workspace_root() -> Result<PathBuf, String> {
    let mut root = env::current_dir()
        .map_err(|error| format!("cannot read the working directory: {error}"))?;

    loop {
        if root.join("Cargo.toml").is_file() && fs_contains_workspace(&root.join("Cargo.toml")) {
            return Ok(root);
        }

        if !root.pop() {
            return Err("run from inside the hydebar workspace".to_owned());
        }
    }
}

/// Whether the manifest declares a workspace.
fn fs_contains_workspace(manifest: &std::path::Path) -> bool {
    std::fs::read_to_string(manifest).is_ok_and(|content| content.contains("[workspace]"))
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().skip(1).collect();

    if arguments.first().map(String::as_str) == Some("import-waybar") {
        return import::run_command(&arguments[1..]);
    }

    if arguments.first().map(String::as_str) != Some("new-module") {
        return Err(USAGE.to_owned());
    }

    let args = parse_args(&arguments[1..])?;
    let kind = ModuleKind::from_flag(args.kind.as_deref()).map_err(|error| error.to_string())?;
    let names = names::derive(&args.name, args.label.as_deref(), args.doc.as_deref())
        .map_err(|error| error.to_string())?;

    let root = workspace_root()?;
    let files = generate::render(&names, kind);
    generate::write_files(&root, &files)
        .map_err(|error| format!("cannot write the scaffold: {error}"))?;

    for file in &files {
        println!("wrote {}", file.path.display());
    }
    println!();

    checklist::print(&names, kind);

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_parses() {
        let args = parse_args(&["demo".to_owned()]).expect("a name parses");

        assert_eq!(args.name, "demo");
        assert_eq!(args.kind, None);
    }

    #[test]
    fn every_flag_parses() {
        let args = parse_args(&[
            "demo".to_owned(),
            "--kind".to_owned(),
            "menu".to_owned(),
            "--label".to_owned(),
            "Demo".to_owned(),
            "--doc".to_owned(),
            "Does demos.".to_owned()
        ])
        .expect("every flag parses");

        assert_eq!(args.kind, Some("menu".to_owned()));
        assert_eq!(args.label, Some("Demo".to_owned()));
        assert_eq!(args.doc, Some("Does demos.".to_owned()));
    }

    #[test]
    fn missing_names_and_values_are_refused() {
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&["a".to_owned(), "b".to_owned()]).is_err());
        assert!(parse_args(&["--kind".to_owned()]).is_err());
        assert!(parse_args(&["--nope".to_owned()]).is_err());
    }
}
