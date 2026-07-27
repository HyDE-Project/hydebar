//! Listener task reading newline delimited JSON from an external process.

use std::{process::Stdio, sync::Arc};

use log::{error, info};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader, Lines},
    process::Command
};

use super::{
    Message,
    data::CustomListenData,
    error::{CustomCommandError, CustomListenerError, truncate_snippet}
};
use crate::{ModuleEventSender, modules::ModuleError, services::ServiceEvent};

pub(super) fn send_event(
    sender: &ModuleEventSender<Message>,
    event: ServiceEvent<super::CustomCommandService>
) -> Result<(), ModuleError> {
    sender
        .try_send(Message::Event(event))
        .map_err(ModuleError::from)
}

pub(super) async fn forward_custom_updates<R>(
    reader: &mut Lines<R>,
    module_name: &str,
    sender: &ModuleEventSender<Message>
) -> Result<(), CustomListenerError>
where
    R: AsyncBufRead + Unpin
{
    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|err| CustomListenerError::Command(CustomCommandError::Read(Arc::new(err))))?
    {
        match serde_json::from_str::<CustomListenData>(&line) {
            Ok(event) => {
                send_event(sender, ServiceEvent::Update(event))
                    .map_err(CustomListenerError::Module)?;
            }
            Err(err) => {
                let parse_error =
                    CustomCommandError::Parse(truncate_snippet(&line), Arc::new(err));
                error!(
                    "Custom module '{module_name}' failed to parse JSON output: {parse_error:?}"
                );
                send_event(sender, ServiceEvent::Error(parse_error.clone()))
                    .map_err(CustomListenerError::Module)?;
            }
        }
    }

    Ok(())
}

pub(super) async fn run_custom_listener(
    module_name: Arc<str>,
    command: Arc<str>,
    sender: ModuleEventSender<Message>
) -> Result<(), CustomListenerError> {
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(command.as_ref())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| CustomListenerError::Command(CustomCommandError::Spawn(Arc::new(err))))?;

    let stdout = child.stdout.take().ok_or(CustomListenerError::Command(
        CustomCommandError::MissingStdout
    ))?;

    let mut reader = BufReader::new(stdout).lines();

    forward_custom_updates(&mut reader, module_name.as_ref(), &sender).await?;

    match child.wait().await {
        Ok(status) => {
            info!("Custom module '{module_name}' listener exited with status: {status}");
            if status.success() {
                Ok(())
            } else {
                Err(CustomListenerError::Command(
                    CustomCommandError::NonZeroExit {
                        status: status.code()
                    }
                ))
            }
        }
        Err(err) => Err(CustomListenerError::Command(CustomCommandError::Wait(
            Arc::new(err)
        )))
    }
}
