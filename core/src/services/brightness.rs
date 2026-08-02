//! Brightness service exposing the backlight level to the bar.

use std::{any::TypeId, ops::Deref, path::PathBuf};

use iced::{Subscription, Task, stream::channel};

use super::{ReadOnlyService, Service, ServiceEvent};

mod backlight;
mod dbus;
mod error;
mod listener;

pub use error::BrightnessError;

#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    pub current: u32,
    pub max:     u32
}

#[derive(Debug, Clone)]
pub struct BrightnessService {
    data:        BrightnessData,
    device_path: PathBuf,
    conn:        zbus::Connection
}

impl Deref for BrightnessService {
    type Target = BrightnessData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub struct BrightnessEvent(u32);

#[derive(Debug, Clone)]
pub enum BrightnessCommand {
    Set(u32),
    Refresh
}

impl BrightnessService {
    pub async fn run_command(self, command: BrightnessCommand) -> ServiceEvent<Self> {
        match command {
            BrightnessCommand::Set(value) => {
                match Self::set_brightness(&self.conn, &self.device_path, value).await {
                    Ok(()) => ServiceEvent::Update(BrightnessEvent(value)),
                    Err(err) => ServiceEvent::Error(err)
                }
            }
            BrightnessCommand::Refresh => match Self::get_actual_brightness(&self.device_path) {
                Ok(value) => ServiceEvent::Update(BrightnessEvent(value)),
                Err(err) => ServiceEvent::Error(err)
            }
        }
    }
}

impl ReadOnlyService for BrightnessService {
    type UpdateEvent = BrightnessEvent;
    type Error = BrightnessError;

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data.current = event.0;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with(id, |&_id| {
            channel(100, async |mut output| {
                Self::listen(&mut output).await;
            })
        })
    }
}

impl Service for BrightnessService {
    type Command = BrightnessCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let service = self.clone();

        Task::perform(
            async move { Self::run_command(service, command).await },
            |event| event
        )
    }
}
