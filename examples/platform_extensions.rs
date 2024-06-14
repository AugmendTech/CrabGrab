use std::time::Duration;

#[cfg(target_os = "macos")]
use crabgrab::platform::macos::MacosCapturableWindowExt as _;

#[cfg(target_os = "windows")]
use crabgrab::platform::windows::WindowsCapturableWindowExt as _;

use crabgrab::prelude::*;

fn main() { 
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .build().unwrap();
    let future = runtime.spawn(async {
        let filter = CapturableContentFilter::NORMAL_WINDOWS;
        let content = CapturableContent::new(filter).await.unwrap();
        for window in content.windows() {
            #[cfg(target_os = "macos")]
            println!("window: {}, app: {}, window layer: {:?}, window level: {:?}", window.title(), window.application().identifier(), window.get_window_layer().ok(), window.get_window_level().ok());
            #[cfg(target_os = "windows")]
            println!("window: {}, window handle: {:?}", window.title(), window.get_window_handle());
        }
    });
    runtime.block_on(future).unwrap();
    runtime.shutdown_timeout(Duration::from_millis(10000));
}
