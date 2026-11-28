//! The wiring checklist printed after scaffolding.
//!
//! Generated files are only half of a module; the rest is registry entries
//! the compiler points at one by one. The checklist orders them so following
//! it top to bottom keeps the tree green at every step, and names the exact
//! lines to add.

use super::{kinds::ModuleKind, names::ModuleNames};

/// Prints what remains after the files land.
pub fn print(names: &ModuleNames, kind: ModuleKind) {
    let snake = &names.snake;
    let pascal = &names.pascal;

    if matches!(kind, ModuleKind::Listener | ModuleKind::Menu) {
        println!("0. core/src/event_bus/queue.rs — the bus event first, or nothing compiles:");
        println!("     {pascal}(modules::{snake}::Message),");
        println!(
            "   gui bus map: update/mapping.rs `ModuleEvent::{pascal}(message) => Message::{pascal}(message),`"
        );
        println!();
    }

    println!("1. proto/src/config/modules/name.rs — one roster row:");
    println!("     /// {}", names.doc);
    println!("     {pascal} => \"{}\",", names.label);
    println!();
    println!("2. proto/src/config.rs — module, export, field, default:");
    println!("     mod {snake};");
    println!("     pub use {snake}::{pascal}ModuleConfig;");
    println!("     #[serde(default)]");
    println!("     pub {snake}: {pascal}ModuleConfig,");
    println!("     {snake}: {pascal}ModuleConfig::default(),");
    println!();
    println!("3. core/src/modules.rs — `pub mod {snake};`.");
    println!();

    if matches!(kind, ModuleKind::Menu) {
        println!("4. core/src/menu/kind.rs — menu variant plus its owner:");
        println!("     {pascal},");
        println!("     Self::{pascal} => ModuleName::{pascal},");
        println!();
    }

    println!(
        "{}. gui wiring, mirroring the clock module:",
        if matches!(kind, ModuleKind::Menu) {
            5
        } else {
            4
        }
    );
    println!("     app/state/app.rs: `pub {snake}: {pascal},` (+ doc line)");
    println!("     app/state/init.rs: `{snake}: {pascal}::default(),`");
    println!("     app/state/message.rs: `{pascal}(modules::{snake}::Message),` + From impl");
    println!(
        "     update/modules/forward.rs: `Message::{pascal}(m) => {{ self.{snake}.update(m); Task::none() }}`"
    );
    println!("     modules/dispatch/view.rs: `ModuleName::{pascal} => self.{snake}.bar_view(),`");
    println!(
        "     modules/dispatch/owner.rs: `ModuleName::{pascal} => &self.{snake},` (and `&mut`)"
    );
    println!("     modules/element/hint.rs: optional, an arm falls back to the label");
    println!(
        "     update/registration/roster/readings.rs: `gate(\"{snake}\", hosts(ModuleName::{pascal}), &mut self.{snake}, ctx, ());`"
    );
    println!();

    if matches!(kind, ModuleKind::Menu) {
        println!(
            "6. gui menu page + toggle: view/pages.rs arm (`measured_page` unless the menu has its own content), update/menus/toggle.rs only if the menu needs preparation (`_ => {{}}` covers the rest)."
        );
        println!();
    }

    println!("7. Optional waybar name: proto/src/bar_layout/mapping.rs.");
    println!(
        "8. Run `cargo fmt`, then `cargo check --workspace`: the compiler names any arm left behind."
    );
}
