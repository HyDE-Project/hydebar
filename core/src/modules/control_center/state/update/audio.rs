//! Handling of audio messages: service events, mute, volume and device
//! switches.

use super::super::super::{
    ControlCenter, Message, SubMenu, audio::AudioMessage, commands::ControlCenterCommandExt
};
use crate::{
    config::ControlCenterModuleConfig,
    outputs::Outputs,
    services::{ReadOnlyService, ServiceEvent, audio::AudioCommand}
};

/// Volume moved by one wheel notch over the bar entry, in percent.
const WHEEL_VOLUME_STEP: i32 = 5;

impl ControlCenter {
    pub(super) fn handle_audio(
        &mut self,
        msg: AudioMessage,
        config: &ControlCenterModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) {
        match msg {
            AudioMessage::Event(event) => match *event {
                ServiceEvent::Init(service) => {
                    self.audio = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(audio) = self.audio.as_mut() {
                        audio.update(data);

                        if self.sub_menu == Some(SubMenu::Sinks) && audio.sinks.len() < 2 {
                            self.sub_menu = None;
                        }

                        if self.sub_menu == Some(SubMenu::Sources) && audio.sources.len() < 2 {
                            self.sub_menu = None;
                        }
                    }
                }
                ServiceEvent::Error(err) => {
                    log::error!("Audio service error: {err:?}");
                }
            },
            AudioMessage::ToggleSinkMute => {
                let _spawned = self.spawn_audio_command(AudioCommand::ToggleSinkMute);
            }
            AudioMessage::SinkVolumeChanged(value) => {
                let _spawned = self.spawn_audio_command(AudioCommand::SinkVolume(value));
            }
            AudioMessage::SinkVolumeWheel(direction) => {
                if let Some(audio) = self.audio.as_ref() {
                    let value =
                        (audio.cur_sink_volume + direction * WHEEL_VOLUME_STEP).clamp(0, 100);

                    let _spawned = self.spawn_audio_command(AudioCommand::SinkVolume(value));
                }
            }
            AudioMessage::DefaultSinkChanged(name, port) => {
                let _spawned = self.spawn_audio_command(AudioCommand::DefaultSink(name, port));
            }
            AudioMessage::ToggleSourceMute => {
                let _spawned = self.spawn_audio_command(AudioCommand::ToggleSourceMute);
            }
            AudioMessage::SourceVolumeChanged(value) => {
                let _spawned = self.spawn_audio_command(AudioCommand::SourceVolume(value));
            }
            AudioMessage::DefaultSourceChanged(name, port) => {
                let _spawned = self.spawn_audio_command(AudioCommand::DefaultSource(name, port));
            }
            AudioMessage::SinksMore(id) => {
                if let Some(cmd) = &config.audio_sinks_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
            AudioMessage::SourcesMore(id) => {
                if let Some(cmd) = &config.audio_sources_more_cmd {
                    crate::utils::launcher::execute_command(cmd.clone());
                    let _ = outputs.close_menu::<Message>(id, main_config);
                }
            }
        }
    }
}
