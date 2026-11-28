#![warn(clippy::pedantic)]

mod application;
mod conversion;
mod error;
pub(crate) mod event_loop;
pub(crate) mod handlers;
pub(crate) mod output_subscription;
mod settings;
mod state;
pub(crate) mod surface_manager;
pub(crate) mod ui_builder;
mod wayland_clipboard;
mod window_handle;
// Internal task module (re-exported publicly via the `task` module below)
#[path = "task.rs"]
mod task_impl;

// === Layer shell API ===
pub use application::application;
pub use error::Error;
pub use output_subscription::output_events;
pub use settings::{
    Anchor, KeyboardInteractivity, Layer, LayerShellSettings, OutputEvent, OutputId, OutputInfo,
    SurfaceId,
};
pub use task_impl::{
    InputRegionRect, destroy_layer_surface, new_layer_surface, set_anchor, set_exclusive_zone,
    set_input_region, set_keyboard_interactivity, set_layer, set_margin, set_size,
};

// === Full iced public API re-export ===

// Core types
pub use Alignment::Center;
pub use Length::{Fill, FillPortion, Shrink};
pub use iced_core::{
    Alignment, Animation, Background, Border, Color, ContentFit, Degrees, Font, Gradient, Length,
    Padding, Pixels, Point, Radians, Rectangle, Rotation, Shadow, Size, Theme, Transformation,
    Vector,
};

// Futures / streams
pub use iced_futures::Subscription;
pub use iced_futures::futures;
pub use iced_futures::stream;

// Renderer
pub use iced_renderer::Renderer;

// All widgets
pub use iced_widget::*;

// Our Task wrapper (re-exported from task module)
pub use task::Task;

// Core crate for advanced usage
pub use iced_core as core;

// --- Modules matching iced's public API ---

pub mod alignment {
    pub use iced_core::alignment::*;
}

pub mod animation {
    pub use iced_core::animation::*;
}

pub mod border {
    pub use iced_core::border::*;
}

pub mod gradient {
    pub use iced_core::gradient::*;
}

pub mod padding {
    pub use iced_core::padding::*;
}

pub mod theme {
    pub use iced_core::theme::*;
}

pub mod font {
    pub use iced_core::font::*;
    pub use iced_runtime::font::*;
}

pub mod event {
    pub use iced_core::event::{Event, Status};
    pub use iced_futures::event::{listen, listen_raw, listen_with};
}

pub mod keyboard {
    pub use iced_core::keyboard::key;
    pub use iced_core::keyboard::{Event, Key, Location, Modifiers};
    pub use iced_futures::keyboard::listen;
}

pub mod mouse {
    pub use iced_core::mouse::{Button, Cursor, Event, Interaction, ScrollDelta};
}

pub mod touch {
    pub use iced_core::touch::{Event, Finger};
}

pub mod window {
    pub use iced_core::window::*;
}

pub mod overlay {
    pub type Element<'a, Message, Theme = iced_core::Theme, Renderer = iced_renderer::Renderer> =
        iced_core::overlay::Element<'a, Message, Theme, Renderer>;
}

pub mod widget {
    pub use iced_widget::*;
}

pub mod task {
    pub use crate::task_impl::{
        InputRegionRect, Task, destroy_layer_surface, new_layer_surface, set_anchor,
        set_exclusive_zone, set_input_region, set_keyboard_interactivity, set_layer, set_margin,
        set_size,
    };
    pub use iced_runtime::task::Handle;
}

pub mod id {
    pub use iced_widget::Id;
}

#[allow(unused_imports)]
pub mod time {
    pub use iced_futures::backend::default::time::*;
}

pub mod clipboard {
    pub use iced_runtime::clipboard::{read, read_primary, write, write_primary};
}

pub mod executor {
    pub use iced_futures::Executor;
    pub use iced_futures::backend::default::Executor as Default;
}

pub mod system {
    pub use iced_runtime::system::{theme, theme_changes};
}

#[cfg(feature = "advanced")]
pub mod advanced {
    pub use iced_core::*;
}

#[cfg(feature = "highlighter")]
pub use iced_highlighter as highlighter;

/// A generic iced UI element.
pub type Element<'a, Message> = iced_core::Element<'a, Message, Theme, Renderer>;

pub type Result = std::result::Result<(), Error>;
