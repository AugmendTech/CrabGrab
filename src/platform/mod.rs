#[cfg(target_os = "macos")]
/// Macos-specific extensions
pub mod macos;

#[cfg(target_os = "macos")]
pub(crate) use macos as platform_impl;

#[cfg(target_os = "windows")]
/// Windows-specific extensions
pub mod windows;

#[cfg(target_os = "windows")]
pub(crate)  use windows as platform_impl;


