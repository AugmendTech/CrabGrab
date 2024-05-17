use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;

pub(crate) mod capture_stream;
mod capturable_content;
mod audio_capture_stream;
pub(crate) mod frame;

pub(crate) struct AutoHandle(HANDLE);
impl Drop for AutoHandle {
    fn drop(&mut self) {
        unsafe { let _ = CloseHandle(self.0); }
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
