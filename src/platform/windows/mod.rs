use windows::Win32::{System::Com::{CoInitializeEx, CoUninitialize, COINIT}, Foundation::{CloseHandle, HANDLE}};

pub(crate) mod capture_stream;
mod capturable_content;
mod audio_capture_stream;
pub(crate) mod frame;

pub(crate) struct AutoHandle(pub HANDLE);
impl Drop for AutoHandle {
    fn drop(&mut self) {
        unsafe { let _ = CloseHandle(self.0); }
    }
}

pub(crate) struct AutoCom(Option<COINIT>);

impl AutoCom {
    fn new(coinit: COINIT) -> Self {
        let inner = unsafe {
            if CoInitializeEx(None, coinit).is_ok() {
                Some(coinit)
            } else {
                None
            }
        };
        Self(inner)
    }

    fn no_init() -> Self {
        Self(None)
    }
}

impl Drop for AutoCom {
    fn drop(&mut self) {
        if let Some(_coinit) = self.0.take() {
            unsafe { CoUninitialize() };
        }
    }
}


pub(crate) use capturable_content::WindowsCapturableApplication as ImplCapturableApplication;
pub(crate) use capturable_content::WindowsCapturableDisplay as ImplCapturableDisplay;
pub(crate) use capturable_content::WindowsCapturableWindow as ImplCapturableWindow;
pub(crate) use capturable_content::WindowsCapturableContent as ImplCapturableContent;
pub(crate) use capturable_content::WindowsCapturableContentFilter as ImplCapturableContentFilter;

pub(crate) use capture_stream::WindowsCaptureStream as ImplCaptureStream;
pub(crate) use capture_stream::WindowsCaptureConfig as ImplCaptureConfig;
pub(crate) use capture_stream::WindowsAudioCaptureConfig as ImplAudioCaptureConfig;
pub(crate) use capture_stream::WindowsCaptureAccessToken as ImplCaptureAccessToken;

pub(crate) use frame::WindowsVideoFrame as ImplVideoFrame;
pub(crate) use frame::WindowsAudioFrame as ImplAudioFrame;

pub use capture_stream::WindowsCaptureConfigExt;

/// Windows-specific extensions to capturable windows
pub use capturable_content::WindowsCapturableWindowExt;
/// Windows-specific extensions to capturable content filters
pub use capturable_content::WindowsCapturableContentFilterExt;
/// Re-exported from the `windows` crate
pub use capturable_content::HWND;
