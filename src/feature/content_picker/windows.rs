use std::time::Duration;

use windows::{ApplicationModel::Core::CoreApplication, Graphics::Capture::GraphicsCapturePicker, Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED}, UI::Core::{CoreDispatcher, CoreDispatcherPriority, DispatchedHandler}};

use super::{PickedSharableContent, SharableContentPickerError, SharableContentPickerConfig};

pub async fn pick_sharable_content(config: SharableContentPickerConfig) -> Result<Option<PickedSharableContent>, SharableContentPickerError> {
    let close_clr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    if !config.display || !config.window || config.excluded_apps.len() != 0 {
        return Err(SharableContentPickerError::ConfigFilteringUnsupported);
    }
    let picker = GraphicsCapturePicker::new()
        .map_err(|error| SharableContentPickerError::Other(format!("Faild to create picker instance: {}", error.to_string())))?;
    let item = picker.PickSingleItemAsync()
        .map_err(|error| SharableContentPickerError::Other(format!("Failed to start pick dialogue: {}", error.to_string())))?.await;
    //println!("item: {:?}", item);
    std::thread::sleep(Duration::from_secs(10));
    todo!()
}
