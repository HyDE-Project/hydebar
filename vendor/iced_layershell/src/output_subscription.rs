use crate::settings::OutputEvent;
use futures::channel::mpsc;
use iced_futures::Subscription;
use std::sync::OnceLock;

static SENDER: OnceLock<mpsc::UnboundedSender<OutputEvent>> = OnceLock::new();
static RECEIVER: OnceLock<std::sync::Mutex<Option<mpsc::UnboundedReceiver<OutputEvent>>>> =
    OnceLock::new();

/// Initialize the output event channel. Called once at startup.
pub(crate) fn init() {
    let (sender, receiver) = mpsc::unbounded();
    SENDER.get_or_init(|| sender);
    RECEIVER.get_or_init(|| std::sync::Mutex::new(Some(receiver)));
}

/// Push output events from the SCTK `OutputHandler`.
pub(crate) fn push_events(events: Vec<OutputEvent>) {
    if let Some(sender) = SENDER.get() {
        for event in events {
            sender.unbounded_send(event).ok();
        }
    }
}

fn create_output_stream() -> impl futures::Stream<Item = OutputEvent> {
    let receiver = RECEIVER
        .get()
        .and_then(|r| r.lock().ok())
        .and_then(|mut guard| guard.take());

    match receiver {
        Some(rx) => futures::stream::StreamExt::left_stream(rx),
        None => futures::stream::StreamExt::right_stream(futures::stream::pending()),
    }
}

/// Subscribe to output (monitor) connect/disconnect/change events.
pub fn output_events() -> Subscription<OutputEvent> {
    Subscription::run(create_output_stream)
}
