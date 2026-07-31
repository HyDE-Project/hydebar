//! Connection to the PulseAudio server.

use std::any::TypeId;

use libpulse_binding::{
    context::{self, Context, FlagSet, introspect::Introspector},
    mainloop::standard::{IterateResult, Mainloop},
    proplist::{Proplist, properties::APPLICATION_NAME}
};
use masterror::{AppError, AppResult};

use super::BackendHandle;

/// Connection to the PulseAudio daemon owned by a single backend thread.
///
/// Field order is load bearing: the introspector borrows the context and the
/// context registers io events on the mainloop, so both must be torn down
/// before the mainloop. Dropping the mainloop first makes libpulse abort the
/// process.
pub(super) struct PulseAudioServer {
    pub(super) introspector: Introspector,
    pub(super) context:      Context,
    pub(super) mainloop:     Mainloop
}

impl Drop for PulseAudioServer {
    fn drop(&mut self) {
        self.context.disconnect();
    }
}

impl PulseAudioServer {
    pub(super) fn new() -> AppResult<Self> {
        let name = format!("{:?}", TypeId::of::<Self>());
        let mut proplist =
            Proplist::new().ok_or_else(|| AppError::internal("create PulseAudio properties"))?;
        proplist
            .set_str(APPLICATION_NAME, name.as_str())
            .map_err(|_| AppError::internal("failed to set application name"))?;

        let mut mainloop =
            Mainloop::new().ok_or_else(|| AppError::internal("create PulseAudio mainloop"))?;

        let mut context = Context::new_with_proplist(&mainloop, name.as_str(), &proplist)
            .ok_or_else(|| AppError::internal("create PulseAudio context"))?;

        context
            .connect(None, FlagSet::NOFLAGS, None)
            .map_err(|e| AppError::internal(format!("connect PulseAudio context: {}", e)))?;

        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    return Err(AppError::internal("PulseAudio mainloop failed during init"));
                }
                IterateResult::Success(_) => {
                    if context.get_state() == context::State::Ready {
                        break;
                    }
                }
            }
        }

        let introspector = context.introspect();

        Ok(Self {
            introspector,
            context,
            mainloop
        })
    }

    pub(super) async fn start() -> AppResult<BackendHandle> {
        let (from_server_tx, from_server_rx) = tokio::sync::mpsc::unbounded_channel();
        let (to_server_tx, to_server_rx) = tokio::sync::mpsc::unbounded_channel();

        let listener = Self::start_listener(from_server_tx.clone()).await?;
        let commander = Self::start_commander(from_server_tx.clone(), to_server_rx).await?;

        Ok(BackendHandle::new(
            from_server_rx,
            to_server_tx,
            listener,
            commander
        ))
    }
}
