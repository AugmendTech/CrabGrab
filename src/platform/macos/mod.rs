mod capture_stream;
mod frame;
mod capturable_content;
mod objc_wrap;

pub(crate) use capture_stream::MacosCaptureStream as ImplCaptureStream;
pub(crate) use capture_stream::MacosAudioCaptureConfig as ImplAudioCaptureConfig;
pub(crate) use capture_stream::MacosCaptureConfig as ImplCaptureConfig;
pub(crate) use frame::MacosAudioFrame as ImplAudioFrame;
pub(crate) use frame::MacosVideoFrame as ImplVideoFrame;
pub(crate) use capturable_content::MacosCapturableContent as ImplCapturableContent;
pub(crate) use capturable_content::MacosCapturableWindow as ImplCapturableWindow;
pub(crate) use capturable_content::MacosCapturableDisplay as ImplCapturableDisplay;

pub use capture_stream::MacosAudioCaptureConfigExt;
pub use capture_stream::MacosCaptureConfigExt;
