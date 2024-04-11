#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::pick_sharable_content;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::pick_sharable_content;

use crate::prelude::{CapturableApplication, CapturableWindow, CapturableDisplay};

pub struct SharableContentPickerConfig {
    pub display: bool,
    pub window: bool,
    pub excluded_apps: Vec<CapturableApplication>,
}

pub enum SharableContentPickerError {
    EmptyConfig,
    Other(String),
}

pub enum PickedSharableContent {
    Window(CapturableWindow),
    Display(CapturableDisplay),
}
