use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, sync::Arc, time::{Duration, Instant}};

use futures::executor::block_on;
use objc::runtime::Object;
use parking_lot::Mutex;

use crate::{capture_stream::{CaptureConfig, StreamCreateError, StreamError, StreamEvent}, platform::platform_impl::{frame::MacosSCStreamVideoFrame, objc_wrap::NSNumber}, prelude::{AudioCaptureConfig, AudioFrame, Capturable, CaptureConfigError, CapturePixelFormat, StreamStopError, VideoFrame}, util::{Rect, Size}};
use super::{frame::{MacosAudioFrame, MacosCGDisplayStreamVideoFrame, MacosVideoFrame}, objc_wrap::{kCFBooleanFalse, kCFBooleanTrue, kCGDisplayStreamDestinationRect, kCGDisplayStreamMinimumFrameTime, kCGDisplayStreamPreserveAspectRatio, kCGDisplayStreamQueueDepth, kCGDisplayStreamShowCursor, kCGDisplayStreamSourceRect, CFNumber, CGDisplayStream, CGDisplayStreamFrameStatus, CGPoint, CGRect, CGSize, CMTime, DispatchQueue, NSArray, NSDictionary, NSString, SCContentFilter, SCStream, SCStreamCallbackError, SCStreamColorMatrix, SCStreamConfiguration, SCStreamHandler, SCStreamOutputType, SCStreamPixelFormat}};

pub type MacosPixelFormat = SCStreamPixelFormat;

impl TryFrom<CapturePixelFormat> for SCStreamPixelFormat {
    type Error = StreamCreateError;

    fn try_from(value: CapturePixelFormat) -> Result<Self, Self::Error> {
        match value {
            CapturePixelFormat::Bgra8888 => Ok(SCStreamPixelFormat::BGRA8888),
            CapturePixelFormat::Argb2101010 => Ok(SCStreamPixelFormat::L10R),
            CapturePixelFormat::F420 => Ok(SCStreamPixelFormat::F420),
            CapturePixelFormat::V420 => Ok(SCStreamPixelFormat::V420),
            _ => Err(StreamCreateError::UnsupportedPixelFormat)
        }
    }
}

enum MacosCaptureStreamInternal {
    Window(SCStream),
    Display(CGDisplayStream),
}

pub(crate) struct MacosCaptureStream {
    stream: MacosCaptureStreamInternal
}

pub trait MacosCaptureConfigExt {
    fn with_output_size(self, size: Size) -> Self;
    fn with_scale_to_fit(self, scale_to_fit: bool) -> Self;
    fn with_maximum_fps(self, maximum_fps: Option<f32>) -> Self;
    fn with_queue_depth(self, queue_depth: usize) -> Self;
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct MacosCaptureConfig {
    output_size: Size,
    scale_to_fit: bool,
    maximum_fps: Option<f32>,
    queue_depth: usize,
}

impl MacosCaptureConfig {
    pub fn new(source_rect: Rect) -> Self {
        Self {
            output_size: source_rect.size,
            scale_to_fit: true,
            maximum_fps: None,
            queue_depth: 3,
        }
    }
}

impl MacosCaptureConfigExt for CaptureConfig {
    fn with_output_size(self, output_size: Size) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                output_size,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_scale_to_fit(self, scale_to_fit: bool) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                scale_to_fit,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_maximum_fps(self, maximum_fps: Option<f32>) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                maximum_fps,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_queue_depth(self, queue_depth: usize) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                queue_depth,
                ..self.impl_capture_config
            },
            ..self
        }
    }
}

pub trait MacosAudioCaptureConfigExt {
    fn set_exclude_current_process_audio(self, exclude_current_process_audio: bool) -> Self;
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct MacosAudioCaptureConfig {
    exclude_current_process_audio: bool,
}

impl MacosAudioCaptureConfig {
    pub fn new() -> Self {
        Self {
            exclude_current_process_audio: false,
        }
    }
}

impl MacosAudioCaptureConfigExt for AudioCaptureConfig {
    fn set_exclude_current_process_audio(self, exclude_current_process_audio: bool) -> Self {
        Self {
            impl_capture_audio_config: MacosAudioCaptureConfig {
                exclude_current_process_audio,
                ..self.impl_capture_audio_config
            },
            ..self
        }
    }
}

impl MacosCaptureStream {
    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        &[
            CapturePixelFormat::V420,
            CapturePixelFormat::F420,
            CapturePixelFormat::Bgra8888,
            CapturePixelFormat::Argb2101010,
        ]
    }

    pub fn check_access() -> bool {
        return SCStream::preflight_access()
    }

    pub async fn request_access() -> bool {
        SCStream::request_access().await
    }

    pub fn new(config: CaptureConfig, mut callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        match config.target {
            Capturable::Window(window) => {
                
                let mut config = SCStreamConfiguration::new();
                config.set_size(CGSize {
                    x: window.rect().size.width * 2.0,
                    y: window.rect().size.height * 2.0,
                });
                config.set_scales_to_fit(false);
                config.set_pixel_format(SCStreamPixelFormat::V420);
                config.set_color_matrix(SCStreamColorMatrix::ItuR709_2);
                config.set_capture_audio(false);
                config.set_scales_to_fit(false);
                config.set_source_rect(CGRect {
                    origin: CGPoint {
                        x: 0.0, y: 0.0
                    },
                    size: CGSize {
                        x: window.rect().size.width,
                        y: window.rect().size.height
                    }
                });

                let filter = SCContentFilter::new_with_desktop_independent_window(window.impl_capturable_window.window);

                let handler_queue = DispatchQueue::make_serial("com.augmend.crabgrab.window_capture".into());
                
                let handler = SCStreamHandler::new(Box::new(move |stream_result| {
                    println!("callback!");
                }));

                let mut sc_stream = SCStream::new(filter, config, handler_queue, handler)
                    .map_err(|error| StreamCreateError::Other(error))?;

                sc_stream.start();

                Ok(MacosCaptureStream {
                    stream: MacosCaptureStreamInternal::Window(sc_stream)
                })
            },
            Capturable::Display(_) => Err(StreamCreateError::Other("Macos display capture unimplemented".into()))
        }

    }

    pub(crate) fn stop(&mut self) -> Result<(), StreamStopError> {
        match &mut self.stream {
            MacosCaptureStreamInternal::Window(stream) => { stream.stop(); Ok(()) },
            MacosCaptureStreamInternal::Display(stream) => stream.stop().map_err(|_| StreamStopError::Other("Unkown".into())),
        }
    }
}

impl Drop for MacosCaptureStream {
    fn drop(&mut self) {
        self.stop();
    }
}
