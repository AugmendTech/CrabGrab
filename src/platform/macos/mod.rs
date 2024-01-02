mod capture_stream;
mod frame;

pub use capture_stream::MacosCaptureStream as ImplCaptureStream;
pub use frame::MacosAudioFrame as ImplAudioFrame;
pub use frame::MacosVideoFrame as ImplVideoFrame;
