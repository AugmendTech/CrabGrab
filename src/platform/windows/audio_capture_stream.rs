use windows::{core::IUnknown, Win32::{Media::Audio::{eConsole, eMultimedia, eRender, EDataFlow, IMMDeviceEnumerator, MMDeviceEnumerator}, System::Com::CoCreateInstance}};

pub struct WindowsAudioCaptureStream {
}

pub enum WindowsAudioCaptureStreamCreateError {
    Other(String),
    EnpointEnumerationFailed,
}

impl WindowsAudioCaptureStream {
    pub fn new() -> Result<Self, WindowsAudioCaptureStreamCreateError> {
        unsafe {
            let mut mm_device_enumerator: IMMDeviceEnumerator = Default::default();
            CoCreateInstance(MMDeviceEnumerator, Some(&mut mm_device_enumerator as *mut _), CLSCTX_ALL)
                .map_err(|e| WindowsAudioCaptureStreamCreateError::Other(format!("Failed to create MMDeviceEnumerator: {}", e.to_string())))?;
            let device = mm_device_enumerator.GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|_| WindowsAudioCaptureStreamCreateError::EnpointEnumerationFailed)?;
            Err(WindowsAudioCaptureStreamCreateError::Other("WindowsAudioCaptureStream unimplemented!".into()))
        }
    }
}
