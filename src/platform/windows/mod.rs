use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;

mod capture_stream;
mod capturable_content;
mod run_loop;
mod frame;

pub(crate) struct AutoHandle(HANDLE);
impl Drop for AutoHandle {
    fn drop(&mut self) {
        unsafe { let _ = CloseHandle(self.0); }
    }
}


pub use capturable_content::WindowsCapturableApplication as ImplCapturableApplication;
pub use capturable_content::WindowsCapturableDisplay as ImplCapturableDisplay;
pub use capturable_content::WindowsCapturableWindow as ImplCapturableWindow;
pub use capturable_content::WindowsCapturableContent as ImplCapturableContent;

pub use capture_stream::WindowsCaptureStream as ImplCaptureStream;
pub use capture_stream::WindowsCaptureConfig as ImplCaptureConfig;
pub use capture_stream::WindowsAudioCaptureConfig as ImplAudioCaptureConfig;
pub use capture_stream::WindowsPixelFormat as ImplPixelFormat;

pub use frame::WindowsVideoFrame as ImplVideoFrame;
pub use frame::WindowsAudioFrame as ImplAudioFrame;

pub use run_loop::WindowsRunLoop as ImplRunloop;
