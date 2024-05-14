#![allow(unused)]

pub(crate) mod capture_stream;
pub(crate) mod frame;
pub(crate) mod capturable_content;
pub(crate) mod objc_wrap;

pub(crate) use capture_stream::MacosCaptureStream as ImplCaptureStream;
pub(crate) use capture_stream::MacosAudioCaptureConfig as ImplAudioCaptureConfig;
pub(crate) use capture_stream::MacosCaptureConfig as ImplCaptureConfig;
pub(crate) use frame::MacosAudioFrame as ImplAudioFrame;
pub(crate) use frame::MacosVideoFrame as ImplVideoFrame;
pub(crate) use capturable_content::MacosCapturableContent as ImplCapturableContent;
pub(crate) use capturable_content::MacosCapturableWindow as ImplCapturableWindow;
pub(crate) use capturable_content::MacosCapturableDisplay as ImplCapturableDisplay;
pub(crate) use capture_stream::MacosPixelFormat as ImplPixelFormat;
pub(crate) use capturable_content::MacosCapturableApplication as ImplCapturableApplication;
pub(crate) use capture_stream::MacosCaptureAccessToken as ImplCaptureAccessToken;

/// Macos-specific extensions for audio capture configs
pub use capture_stream::MacosAudioCaptureConfigExt;
/// Macos-specific extensions for capture configs
pub use capture_stream::MacosCaptureConfigExt;

pub use capturable_content::MacosCapturableWindowNativeWindowId;