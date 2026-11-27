//! The outcome of one attempt at serving the notification bus.

/// Why one attempt at serving the notification bus came to an end.
pub(super) enum SessionEnd {
    /// The bar dropped its receiver; there is nobody left to serve.
    UiClosed,
    /// The bus or the takeover refused; worth another knock later.
    Failed
}
