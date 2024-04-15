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

impl Default for SharableContentPickerConfig {
    fn default() -> Self {
        Self {
            display: true,
            window: true,
            excluded_apps: vec![]
        }
    }
}

#[derive(Debug)]
pub enum SharableContentPickerError {
    EmptyConfig,
    ConfigFilteringUnsupported,
    Other(String),
}

pub enum PickedSharableContent {
    Window(CapturableWindow),
    Display(CapturableDisplay),
}
