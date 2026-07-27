//! Dispatch of player commands to the service.

use log::warn;

use super::{MediaPlayer, Message};
use crate::{
    config::MediaPlayerModuleConfig,
    services::{
        ServiceEvent,
        mpris::{
            MprisPlayerCommand, MprisPlayerData, MprisPlayerEvent, MprisPlayerService,
            PlayerCommand
        }
    },
    utils::truncate_text
};

impl MediaPlayer {
    pub(super) fn handle_command(&mut self, service_name: String, command: PlayerCommand) {
        let runtime = self.runtime.clone();
        let sender = self.sender.clone();
        let service = self.service.clone();

        if let (Some(runtime), Some(sender)) = (runtime, sender) {
            runtime.spawn(async move {
                let result = MprisPlayerService::execute_command(
                    service,
                    MprisPlayerCommand {
                        service_name,
                        command
                    }
                )
                .await;

                let event = match result {
                    Ok(data) => ServiceEvent::Update(MprisPlayerEvent::Refresh(data)),
                    Err(error) => ServiceEvent::Error(error)
                };

                if let Err(err) = sender.try_send(Message::Event(event)) {
                    warn!("failed to publish media player command result: {err}");
                }
            });
        }
    }

    pub(super) fn get_title(d: &MprisPlayerData, config: &MediaPlayerModuleConfig) -> String {
        match &d.metadata {
            Some(m) => truncate_text(&m.to_string(), config.max_title_length),
            None => "No Title".to_string()
        }
    }
}
