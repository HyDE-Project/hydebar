//! Message handling for the media player module.

use log::error;

use super::{MediaPlayer, Message};
use crate::services::{ReadOnlyService, ServiceEvent, mpris::PlayerCommand};

impl MediaPlayer {
    pub fn update(
        &mut self,
        message: Message,
        config: &hydebar_proto::config::MediaPlayerModuleConfig
    ) {
        match message {
            Message::Prev(s) => self.handle_command(s, PlayerCommand::Prev),
            Message::PlayPause(s) => self.handle_command(s, PlayerCommand::PlayPause),
            Message::Next(s) => self.handle_command(s, PlayerCommand::Next),
            Message::SetVolume(s, v) => self.handle_command(s, PlayerCommand::Volume(v)),
            Message::Event(event) => {
                match *event {
                    ServiceEvent::Init(s) => {
                        self.service = Some(s);
                    }
                    ServiceEvent::Update(d) => {
                        if let Some(service) = self.service.as_mut() {
                            service.update(d);
                        }
                    }
                    ServiceEvent::Error(error) => {
                        error!("media player service error: {error}");
                    }
                }

                self.bar_title = self
                    .service
                    .as_ref()
                    .and_then(|s| s.first())
                    .map(|d| Self::get_title(d, config));
            }
        }
    }
}
