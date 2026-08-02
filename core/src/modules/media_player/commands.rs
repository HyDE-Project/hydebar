//! Dispatch of player commands to the service.

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
    pub(super) fn handle_command(&self, service_name: String, command: PlayerCommand) {
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

                sender.send(Message::Event(Box::new(event)));
            });
        }
    }

    pub(super) fn get_title(d: &MprisPlayerData, config: &MediaPlayerModuleConfig) -> String {
        d.metadata.as_ref().map_or_else(
            || "No Title".to_string(),
            |m| truncate_text(&m.to_string(), config.max_title_length)
        )
    }
}
