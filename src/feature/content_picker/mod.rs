#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::pick_sharable_content;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::pick_sharable_content;

use crate::prelude::{CapturableApplication, CapturableWindow, CapturableDisplay};

/// Configuration for the content picker
/// 
/// Note: not all platforms support filtering or picking displays with their native content picker
pub struct SharableContentPickerConfig {
    /// Allow picking displays
    pub display: bool,
    /// Allow picking windows
    pub window: bool,
    /// Applications to exclude
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
    /// The picker config doesn't allow for any selectable content
    EmptyConfig,
    /// The backend doesn't support filtering content as specified by the config
    ConfigFilteringUnsupported,
    Other(String),
}

/// Content picked by the picker
pub enum PickedSharableContent {
    Window(CapturableWindow),
    Display(CapturableDisplay),
}
