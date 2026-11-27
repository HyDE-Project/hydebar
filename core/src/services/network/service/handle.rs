//! The network state the bar reads and commands.

use std::ops::Deref;

use super::choice::BackendChoice;
use crate::services::network::data::model::NetworkData;

/// Reactive service responsible for keeping track of the system network state.
///
/// # Examples
/// ```no_run
/// # async fn demo(service: &mut hydebar_core::services::network::NetworkService) {
/// use hydebar_core::services::network::NetworkServiceError;
/// service.apply_error(NetworkServiceError::new("temporary failure"));
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct NetworkService {
    pub(super) data:           NetworkData,
    pub(super) conn:           zbus::Connection,
    pub(super) backend_choice: BackendChoice
}

impl Deref for NetworkService {
    type Target = NetworkData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
