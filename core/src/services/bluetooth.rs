//! Bluetooth service exposing adapter state and paired devices to the bar.

use std::{any::TypeId, ops::Deref};

use iced::{Subscription, Task, stream::channel};

use super::{ReadOnlyService, Service, ServiceEvent};

mod commands;
mod data;
mod dbus;
mod listener;
mod rfkill;

pub use commands::BluetoothCommand;
pub use data::{BluetoothData, BluetoothDevice, BluetoothState};

/// The conversation with the bluetooth daemon.
#[derive(Debug, Clone)]
pub struct BluetoothService {
    conn: zbus::Connection,
    data: BluetoothData
}

impl Deref for BluetoothService {
    type Target = BluetoothData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for BluetoothService {
    type UpdateEvent = BluetoothData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
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

impl Service for BluetoothService {
    type Command = BluetoothCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let service = self.clone();
        let fallback = self.data.clone();

        Task::perform(
            async move { Self::run_command(service, command).await },
            move |maybe_event| {
                maybe_event.unwrap_or_else(|| ServiceEvent::Update(fallback.clone()))
            }
        )
    }
}
