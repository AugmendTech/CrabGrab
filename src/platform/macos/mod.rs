#![allow(unused)]

pub(crate) mod capture_stream;
pub(crate) mod frame;
pub(crate) mod capturable_content;
pub(crate) mod objc_wrap;

pub(crate) use capture_stream::MacosCaptureStream as ImplCaptureStream;
pub(crate) use capture_stream::MacosAudioCaptureConfig as ImplAudioCaptureConfig;
pub(crate) use capture_stream::MacosCaptureConfig as ImplCaptureConfig;
pub(crate) use capture_stream::MacosPixelFormat as ImplPixelFormat;
pub(crate) use capture_stream::MacosCaptureAccessToken as ImplCaptureAccessToken;

pub(crate) use frame::MacosAudioFrame as ImplAudioFrame;
pub(crate) use frame::MacosVideoFrame as ImplVideoFrame;

pub(crate) use capturable_content::MacosCapturableContent as ImplCapturableContent;
pub(crate) use capturable_content::MacosCapturableWindow as ImplCapturableWindow;
pub(crate) use capturable_content::MacosCapturableDisplay as ImplCapturableDisplay;
pub(crate) use capturable_content::MacosCapturableContentFilter as ImplCapturableContentFilter;
pub(crate) use capturable_content::MacosCapturableApplication as ImplCapturableApplication;

/// Mac OS specific extensions for audio capture configs
pub use capture_stream::MacosAudioCaptureConfigExt;
/// Mac OS specific extensions for capture configs
pub use capture_stream::MacosCaptureConfigExt;

/// Mac OS specific extensions for capturable windows
pub use capturable_content::MacosCapturableWindowExt;
/// Mac OS specific extensions for capture content filters
pub use capturable_content::MacosCapturableContentFilterExt;
/// Mac OS "window level"
pub use capturable_content::MacosWindowLevel;