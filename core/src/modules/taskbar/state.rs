//! Message folding and command dispatch for the taskbar strip.

use std::sync::Arc;

use log::error;

use super::{Message, Taskbar};

impl Taskbar {
    /// Folds one message into the window list.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ClientsChanged(clients) => {
                self.clients = clients;
            }
            Message::Focus(address) => {
                let port = Arc::clone(&self.hyprland);
                self.spawn_dispatch(move || port.focus_window(&address));
            }
        }
    }

    /// Runs a compositor dispatch off the thread the bar draws on.
    ///
    /// The port retries with a timeout when the compositor socket stalls;
    /// waiting that out on the update thread would freeze every module. A
    /// module that was never registered has no runtime and dispatches
    /// inline, which keeps tests synchronous.
    fn spawn_dispatch(
        &self,
        dispatch: impl FnOnce() -> Result<(), hydebar_proto::ports::hyprland::HyprlandError>
        + Send
        + 'static
    ) {
        match &self.runtime {
            Some(runtime) => {
                runtime.spawn_blocking(move || {
                    if let Err(err) = dispatch() {
                        error!("failed to focus a window from the taskbar: {err}");
                    }
                });
            }
            None => {
                if let Err(err) = dispatch() {
                    error!("failed to focus a window from the taskbar: {err}");
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{Message, Taskbar};
    use crate::{modules::taskbar::test_client, test_utils::MockHyprlandPort};

    #[test]
    fn a_fresh_list_replaces_the_old_one() {
        let mut taskbar = Taskbar::new(Arc::new(MockHyprlandPort::default()));

        taskbar.update(Message::ClientsChanged(vec![test_client("0x1", true)]));
        taskbar.update(Message::ClientsChanged(vec![
            test_client("0x2", false),
            test_client("0x3", true),
        ]));

        assert_eq!(taskbar.clients.len(), 2);
    }

    #[test]
    fn a_press_reaches_the_compositor() {
        let port = Arc::new(MockHyprlandPort::default());
        let mut taskbar = Taskbar::new(Arc::clone(&port) as Arc<dyn HyprlandPort>);

        taskbar.update(Message::Focus("0x1".to_owned()));

        assert_eq!(
            port.focus_window_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }
}
