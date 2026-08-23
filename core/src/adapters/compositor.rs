//! The bar's own client for the compositor it runs under.
//!
//! Everything here speaks the socket the compositor already listens on, the
//! one [`hydebar_proto::compositor_ipc`] opens: questions in JSON, commands as
//! text, answers read as only the fields the bar draws. Owning the client is
//! what lets a record carry a field the bar needs rather than the fields a
//! general purpose crate chose to model, and it keeps the bar honest about how
//! little of the compositor it actually asks for.

pub mod command;
pub mod events;
pub mod query;
pub mod records;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::query;

    /// Reads every answer the bar asks for off the running compositor.
    ///
    /// Skipped where there is no session, which is every continuous
    /// integration runner; on a developer's own machine it is the one test
    /// that proves the records match what the compositor actually writes.
    #[test]
    fn every_answer_the_bar_asks_for_reads_on_a_live_session() {
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
            return;
        }

        assert!(query::monitors("smoke").is_ok(), "monitors");
        assert!(query::workspaces("smoke").is_ok(), "workspaces");
        assert!(query::active_workspace("smoke").is_ok(), "active workspace");
        assert!(query::clients("smoke").is_ok(), "clients");
        assert!(query::active_window("smoke").is_ok(), "active window");
        assert!(query::devices("smoke").is_ok(), "devices");
        assert!(
            query::option_text("smoke", "input:kb_layout").is_ok(),
            "keyboard layouts"
        );
    }
}
