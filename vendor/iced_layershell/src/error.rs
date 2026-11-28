#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Wayland connection failed: {0}")]
    Connection(#[from] wayland_client::ConnectError),
    #[error("Wayland global error: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),
    #[error("Graphics error: {0}")]
    Graphics(iced_graphics::Error),
    #[error("Layer shell not supported by compositor")]
    LayerShellNotSupported,
    #[error("No initial layer shell settings provided")]
    NoSettings,
    #[error("Event loop error: {0}")]
    EventLoop(String),
}
