use std::time::Duration;

use crate::platform::platform_impl::objc_wrap::{debug_objc_object, CGMainDisplayID, SCContentSharingPicker, SCContentSharingPickerConfiguration, SCContentSharingPickerEvent, SCContentSharingPickerModeSingleDisplay, SCContentSharingPickerModeSingleWindow, SCContentSharingPickerObserver, SCShareableContentStyle};

use super::{PickedSharableContent, SharableContentPickerError, SharableContentPickerConfig};
use futures::channel::oneshot;

pub async fn pick_sharable_content(config: SharableContentPickerConfig) -> Result<Option<PickedSharableContent>, SharableContentPickerError> {
    unsafe { CGMainDisplayID(); }
    let configuration = SCContentSharingPickerConfiguration::new();
    let allowed_picker_modes = 
        if config.display { SCContentSharingPickerModeSingleDisplay } else { 0 } |
        if config.window { SCContentSharingPickerModeSingleWindow } else { 0 };
    configuration.set_allowed_picker_modes(allowed_picker_modes);

    let picker = SCContentSharingPicker::shared();
    picker.set_configuration_for_stream(configuration, None);
    let (tx, rx) = oneshot::channel();
    let mut tx_opt = Some(tx);
    let observer = SCContentSharingPickerObserver::new(move |event| {
        if let Some(tx_opt) = tx_opt.take() {
            tx_opt.send(event).expect("SCContentSharingPickerObserver callback send");
        }
    });

    picker.add(observer);
    picker.set_active(true);
    picker.present_using_content_style(SCShareableContentStyle::None);

    match rx.await {
        Ok(event) => {
            match event {
                Ok(SCContentSharingPickerEvent::Cancelled) => Ok(None),
                Ok(SCContentSharingPickerEvent::DidUpdate { filter, stream }) => {
                    debug_objc_object(filter.0);
                    todo!()
                }
                Err(e) => Err(SharableContentPickerError::Other(format!("Failed to receive sharable content from picker: {}", e.description())))
            }
        }
        Err(e) => Err(SharableContentPickerError::Other(format!("Failed to receive event from future")))
    }
}
