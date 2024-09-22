use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, sync::{atomic::{self, AtomicBool, AtomicU64}, Arc}, time::{Duration, Instant}, fmt::Debug};

use futures::executor::block_on;
use objc2::runtime::AnyObject;
use parking_lot::Mutex;

use crate::{capture_stream::{CaptureConfig, StreamCreateError, StreamError, StreamEvent}, platform::platform_impl::{frame::MacosSCStreamVideoFrame, objc_wrap::NSNumber}, prelude::{AudioCaptureConfig, AudioFrame, Capturable, CaptureConfigError, CapturePixelFormat, Point, StreamStopError, VideoFrame}, util::{Rect, Size}};
use super::{frame::{MacosAudioFrame, MacosCGDisplayStreamVideoFrame, MacosVideoFrame}, objc_wrap::{kCFBooleanFalse, kCFBooleanTrue, kCGDisplayStreamDestinationRect, kCGDisplayStreamMinimumFrameTime, kCGDisplayStreamPreserveAspectRatio, kCGDisplayStreamQueueDepth, kCGDisplayStreamShowCursor, kCGDisplayStreamSourceRect, CFNumber, CGDisplayStream, CGDisplayStreamFrameStatus, CGPoint, CGRect, CGSize, CMSampleBuffer, CMTime, DispatchQueue, IOSurface, NSArray, NSDictionary, NSString, SCCaptureResolutionType, SCContentFilter, SCFrameStatus, SCStream, SCStreamCallbackError, SCStreamColorMatrix, SCStreamConfiguration, SCStreamFrameInfoStatus, SCStreamHandler, SCStreamOutputType, SCStreamPixelFormat, SCStreamSampleRate}};

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
    stream: MacosCaptureStreamInternal,
    stopped_flag: Arc<AtomicBool>,
    shared_callback: Arc<Mutex<Box<dyn FnMut(Result<StreamEvent, StreamError>) + Send + 'static>>>,
    #[cfg(feature = "metal")]
    pub(crate) metal_device: metal::Device,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_device: Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// The "resolution type" of the capture
pub enum MacosCaptureResolutionType {
    /// Automatically select the resolution type
    Automatic,
    /// Select the highest available capture resolution (usually the physical resolution being rendered)
    Best,
    /// One linear screen unit per pixel, IE the "virtual resolution"
    Nominal,
}

pub trait MacosCaptureConfigExt {
    /// Set whether or not to scale content to the output size
    fn with_scale_to_fit(self, scale_to_fit: bool) -> Self;
    /// Set the maximum capture frame-rate
    fn with_maximum_fps(self, maximum_fps: Option<f32>) -> Self;
    #[cfg(feature = "metal")]
    /// Set the metal device to use for texture creation
    fn with_metal_device(self, metal_device: metal::Device) -> Self;
    /// Set the resolution type of the capture. Does nothing on macos before OS 14.0
    fn with_resolution_type(self, resolution_type: MacosCaptureResolutionType) -> Self;
}

#[derive(Clone)]
pub(crate) struct MacosCaptureConfig {
    pub(crate) scale_to_fit: bool,
    pub(crate) maximum_fps: Option<f32>,
    pub(crate) resolution_type: MacosCaptureResolutionType,
    #[cfg(feature = "metal")]
    pub(crate) metal_device: Option<metal::Device>,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_device: Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>,
}

impl Debug for MacosCaptureConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacosCaptureConfig").field("scale_to_fit", &self.scale_to_fit).field("maximum_fps", &self.maximum_fps).finish()
    }
}

impl MacosCaptureConfig {
    pub fn new() -> Self {
        Self {
            scale_to_fit: true,
            maximum_fps: None,
            resolution_type: MacosCaptureResolutionType::Nominal,
            #[cfg(feature = "metal")]
            metal_device: None,
            #[cfg(feature = "wgpu")]
            wgpu_device: None,
        }
    }
}

impl MacosCaptureConfigExt for CaptureConfig {
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

    #[cfg(feature = "metal")]
    fn with_metal_device(self, metal_device: metal::Device) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                metal_device: Some(metal_device),
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_resolution_type(self, resolution_type: MacosCaptureResolutionType) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                resolution_type,
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct MacosCaptureAccessToken();

unsafe impl Send for MacosCaptureAccessToken {}
unsafe impl Sync for MacosCaptureAccessToken {}

impl MacosCaptureAccessToken {
    pub(crate) fn allows_borderless(&self) -> bool {
        true
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

    pub fn check_access(_borderless: bool) -> Option<MacosCaptureAccessToken> {
        if SCStream::preflight_access() {
            Some(MacosCaptureAccessToken())
        } else {
            None
        }
    }

    pub async fn request_access(_borderless: bool) -> Option<MacosCaptureAccessToken> {
        if SCStream::request_access().await {
            Some(MacosCaptureAccessToken())
        } else {
            None
        }
    }

    pub fn new(token: MacosCaptureAccessToken, capture_config: CaptureConfig, mut callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        let _ = token;
        let shared_callback = Arc::new(Mutex::new(callback as Box<dyn FnMut(Result<StreamEvent, StreamError>) + Send + 'static>));
        let stream_shared_callback = shared_callback.clone();
        #[cfg(feature = "metal")]
        let mut metal_device = match capture_config.impl_capture_config.metal_device {
            Some(metal_device) => metal_device,
            None => {
                match metal::Device::system_default() {
                    Some(device) => device,
                    None => return Err(StreamCreateError::Other("Failed to create system default metal device".into()))
                }
            }
        };
        #[cfg(feature = "metal")]
        let callback_metal_device = metal_device.clone();
        #[cfg(feature = "wgpu")]
        let wgpu_device = capture_config.impl_capture_config.wgpu_device.clone();
        #[cfg(feature = "wgpu")]
        let callback_wgpu_device = wgpu_device.clone();
        match capture_config.target {
            Capturable::Window(window) => {
                let mut config = SCStreamConfiguration::new();
                let (pixel_format, set_color_matrix) = match capture_config.pixel_format {
                    CapturePixelFormat::Bgra8888 =>    (SCStreamPixelFormat::BGRA8888, false),
                    CapturePixelFormat::Argb2101010 => (SCStreamPixelFormat::L10R, false),
                    CapturePixelFormat::V420 =>        (SCStreamPixelFormat::V420, true),
                    CapturePixelFormat::F420 =>        (SCStreamPixelFormat::F420, true),
                };
                if set_color_matrix {
                    config.set_color_matrix(SCStreamColorMatrix::ItuR709_2);
                }
                config.set_pixel_format(pixel_format);
                config.set_minimum_time_interval(CMTime::new_with_seconds(capture_config.impl_capture_config.maximum_fps.map(|x| 1.0 / x).unwrap_or(1.0 / 120.0) as f64, 240));
                /*config.set_source_rect(CGRect {
                    origin: CGPoint {
                        x: capture_config.source_rect.origin.x,
                        y: capture_config.source_rect.origin.y,
                    },
                    size: CGSize {
                        x: capture_config.source_rect.size.width,
                        y: capture_config.source_rect.size.height
                    }
                });*/
                let resolution_type = match capture_config.impl_capture_config.resolution_type {
                    MacosCaptureResolutionType::Automatic => SCCaptureResolutionType::SCCaptureResolutionAutomatic,
                    MacosCaptureResolutionType::Best => SCCaptureResolutionType::SCCaptureResolutionBest,
                    MacosCaptureResolutionType::Nominal => SCCaptureResolutionType::SCCaptureResolutionNominal,
                };
                _ = config.set_resolution_type(resolution_type);
                config.set_size(CGSize {
                    x: capture_config.output_size.width,
                    y: capture_config.output_size.height,
                });
                config.set_scales_to_fit(capture_config.impl_capture_config.scale_to_fit);
                config.set_queue_depth(capture_config.buffer_count as isize);
                config.set_show_cursor(capture_config.show_cursor);
                match capture_config.capture_audio {
                    Some(audio_config) => {
                        config.set_capture_audio(true);
                        let channel_count = match audio_config.channel_count {
                            crate::prelude::AudioChannelCount::Mono => 1,
                            crate::prelude::AudioChannelCount::Stereo => 2,
                        };
                        config.set_channel_count(channel_count);
                        config.set_exclude_current_process_audio(audio_config.impl_capture_audio_config.exclude_current_process_audio);
                        let sample_rate = match audio_config.sample_rate {
                            crate::prelude::AudioSampleRate::Hz8000 =>  SCStreamSampleRate::R8000,
                            crate::prelude::AudioSampleRate::Hz16000 => SCStreamSampleRate::R16000,
                            crate::prelude::AudioSampleRate::Hz24000 => SCStreamSampleRate::R24000,
                            crate::prelude::AudioSampleRate::Hz48000 => SCStreamSampleRate::R48000,
                        };
                        config.set_sample_rate(sample_rate);
                    },
                    None => {
                        config.set_capture_audio(false);
                    }
                }

                let filter = SCContentFilter::new_with_desktop_independent_window(&window.impl_capturable_window.window);

                let handler_queue = DispatchQueue::make_concurrent("com.augmend.crabgrab.window_capture".into());

                let mut audio_frame_id_counter = AtomicU64::new(0);
                let mut video_frame_id_counter = AtomicU64::new(0);

                let stopped_flag = Arc::new(AtomicBool::new(false));
                let callback_stopped_flag = stopped_flag.clone();
                
                let handler = SCStreamHandler::new(Box::new(move |stream_result: Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>| {
                    let mut callback = stream_shared_callback.lock();
                    let capture_time = Instant::now();
                    match stream_result {
                        Ok((sample_buffer, output_type)) => {
                            match output_type {
                                SCStreamOutputType::Audio => {
                                    let frame_id = audio_frame_id_counter.fetch_add(1, atomic::Ordering::AcqRel);
                                    // TODO...
                                },
                                SCStreamOutputType::Screen => {
                                    let attachments = sample_buffer.get_sample_attachment_array();
                                    if attachments.len() == 0 {
                                        return;
                                    }
                                    let status_nsnumber_ptr = unsafe { attachments[0].get_value(SCStreamFrameInfoStatus) };
                                    if status_nsnumber_ptr.is_null() {
                                        return;
                                    }
                                    let status_i32 = unsafe { NSNumber::from_id_unretained(status_nsnumber_ptr as *mut AnyObject).as_i32() };
                                    let status_opt = SCFrameStatus::from_i32(status_i32);
                                    if status_opt.is_none() {
                                        return;
                                    }
                                    match status_opt.unwrap() {
                                        SCFrameStatus::Complete => {
                                            if callback_stopped_flag.load(atomic::Ordering::Acquire) {
                                                return;
                                            }
                                            let frame_id = video_frame_id_counter.fetch_add(1, atomic::Ordering::AcqRel);
                                            let video_frame = VideoFrame {
                                                impl_video_frame: MacosVideoFrame::SCStream(MacosSCStreamVideoFrame {
                                                    sample_buffer,
                                                    capture_time,
                                                    dictionary: RefCell::new(None),
                                                    frame_id,
                                                    #[cfg(feature = "metal")]
                                                    metal_device: Some(callback_metal_device.clone()),
                                                    #[cfg(feature = "wgpu")]
                                                    wgpu_device: callback_wgpu_device.clone(),
                                                })
                                            };
                                            (callback)(Ok(StreamEvent::Video(video_frame)));
                                        },
                                        SCFrameStatus::Suspended |
                                        SCFrameStatus::Idle => {
                                            if callback_stopped_flag.load(atomic::Ordering::Acquire) {
                                                return;
                                            }
                                            (callback)(Ok(StreamEvent::Idle));
                                        },
                                        SCFrameStatus::Stopped => {
                                            if callback_stopped_flag.fetch_or(true, atomic::Ordering::AcqRel) {
                                                return;
                                            }
                                            (callback)(Ok(StreamEvent::End));
                                        }
                                        _ => {}
                                    }

                                    
                                },
                            }
                        },
                        Err(err) => {
                            let event = match err {
                                SCStreamCallbackError::StreamStopped => {
                                    if callback_stopped_flag.fetch_or(true, atomic::Ordering::AcqRel) {
                                        return;
                                    }
                                    Ok(StreamEvent::End)
                                },
                                SCStreamCallbackError::SampleBufferCopyFailed => Err(StreamError::Other("Failed to copy sample buffer".into())),
                                SCStreamCallbackError::Other(e) => Err(StreamError::Other(format!("Internal stream failure: [description: {}, reason: {}, code: {}, domain: {}]", e.description(), e.reason(), e.code(), e.domain()))),
                            };
                            (callback)(event);
                        }
                    }
                }));

                let mut sc_stream = SCStream::new(filter, config, handler_queue, handler)
                    .map_err(|error| StreamCreateError::Other(error))?;

                sc_stream.start();

                Ok(MacosCaptureStream {
                    stopped_flag,
                    shared_callback,
                    stream: MacosCaptureStreamInternal::Window(sc_stream),
                    #[cfg(feature = "metal")]
                    metal_device,
                    #[cfg(feature = "wgpu")]
                    wgpu_device
                })
            },
            Capturable::Display(display) => {
                let mut config = SCStreamConfiguration::new();
                let (pixel_format, set_color_matrix) = match capture_config.pixel_format {
                    CapturePixelFormat::Bgra8888 =>    (SCStreamPixelFormat::BGRA8888, false),
                    CapturePixelFormat::Argb2101010 => (SCStreamPixelFormat::L10R, false),
                    CapturePixelFormat::V420 =>        (SCStreamPixelFormat::V420, true),
                    CapturePixelFormat::F420 =>        (SCStreamPixelFormat::F420, true),
                };
                if set_color_matrix {
                    config.set_color_matrix(SCStreamColorMatrix::ItuR709_2);
                }
                config.set_pixel_format(pixel_format);
                config.set_minimum_time_interval(CMTime::new_with_seconds(capture_config.impl_capture_config.maximum_fps.map(|x| 1.0 / x).unwrap_or(1.0 / 120.0) as f64, 240));
                /*config.set_source_rect(CGRect {
                    origin: CGPoint {
                        x: capture_config.source_rect.origin.x,
                        y: capture_config.source_rect.origin.y,
                    },
                    size: CGSize {
                        x: capture_config.source_rect.size.width,
                        y: capture_config.source_rect.size.height
                    }
                });*/
                let resolution_type = match capture_config.impl_capture_config.resolution_type {
                    MacosCaptureResolutionType::Automatic => SCCaptureResolutionType::SCCaptureResolutionAutomatic,
                    MacosCaptureResolutionType::Best => SCCaptureResolutionType::SCCaptureResolutionBest,
                    MacosCaptureResolutionType::Nominal => SCCaptureResolutionType::SCCaptureResolutionNominal,
                };
                _ = config.set_resolution_type(resolution_type);
                config.set_size(CGSize {
                    x: capture_config.output_size.width,
                    y: capture_config.output_size.height,
                });
                config.set_scales_to_fit(capture_config.impl_capture_config.scale_to_fit);
                config.set_queue_depth(capture_config.buffer_count as isize);
                config.set_show_cursor(capture_config.show_cursor);
                match capture_config.capture_audio {
                    Some(audio_config) => {
                        config.set_capture_audio(true);
                        let channel_count = match audio_config.channel_count {
                            crate::prelude::AudioChannelCount::Mono => 1,
                            crate::prelude::AudioChannelCount::Stereo => 2,
                        };
                        config.set_channel_count(channel_count);
                        config.set_exclude_current_process_audio(audio_config.impl_capture_audio_config.exclude_current_process_audio);
                        let sample_rate = match audio_config.sample_rate {
                            crate::prelude::AudioSampleRate::Hz8000 =>  SCStreamSampleRate::R8000,
                            crate::prelude::AudioSampleRate::Hz16000 => SCStreamSampleRate::R16000,
                            crate::prelude::AudioSampleRate::Hz24000 => SCStreamSampleRate::R24000,
                            crate::prelude::AudioSampleRate::Hz48000 => SCStreamSampleRate::R48000,
                        };
                        config.set_sample_rate(sample_rate);
                    },
                    None => {
                        config.set_capture_audio(false);
                    }
                }
                let mut filter;
                let windows = capture_config.excluded_windows;
                let mut sc_windows = Vec::new();
                if let Some(windows) = windows {
                    for w in windows {
                        sc_windows.push(w.impl_capturable_window.window);
                    }
                    filter = SCContentFilter::new_with_display_excluding_window(display.impl_capturable_display.display, Some(sc_windows));
                } else {
                    filter = SCContentFilter::new_with_display_excluding_window(display.impl_capturable_display.display, None);
                }


                let handler_queue = DispatchQueue::make_concurrent("com.augmend.crabgrab.window_capture".into());

                let mut audio_frame_id_counter = AtomicU64::new(0);
                let mut video_frame_id_counter = AtomicU64::new(0);

                let stopped_flag = Arc::new(AtomicBool::new(false));
                let callback_stopped_flag = stopped_flag.clone();
                
                let handler = SCStreamHandler::new(Box::new(move |stream_result: Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>| {
                    let mut callback = stream_shared_callback.lock();
                    let capture_time = Instant::now();
                    match stream_result {
                        Ok((sample_buffer, output_type)) => {
                            match output_type {
                                SCStreamOutputType::Audio => {
                                    let frame_id = audio_frame_id_counter.fetch_add(1, atomic::Ordering::AcqRel);
                                    // TODO...
                                },
                                SCStreamOutputType::Screen => {
                                    let attachments = sample_buffer.get_sample_attachment_array();
                                    if attachments.len() == 0 {
                                        return;
                                    }
                                    let status_nsnumber_ptr = unsafe { attachments[0].get_value(SCStreamFrameInfoStatus) };
                                    if status_nsnumber_ptr.is_null() {
                                        return;
                                    }
                                    let status_i32 = unsafe { NSNumber::from_id_unretained(status_nsnumber_ptr as *mut AnyObject).as_i32() };
                                    let status_opt = SCFrameStatus::from_i32(status_i32);
                                    if status_opt.is_none() {
                                        return;
                                    }
                                    match status_opt.unwrap() {
                                        SCFrameStatus::Complete => {
                                            if callback_stopped_flag.load(atomic::Ordering::Acquire) {
                                                return;
                                            }
                                            let frame_id = video_frame_id_counter.fetch_add(1, atomic::Ordering::AcqRel);
                                            let video_frame = VideoFrame {
                                                impl_video_frame: MacosVideoFrame::SCStream(MacosSCStreamVideoFrame {
                                                    sample_buffer,
                                                    capture_time,
                                                    dictionary: RefCell::new(None),
                                                    frame_id,
                                                    #[cfg(feature = "metal")]
                                                    metal_device: Some(callback_metal_device.clone()),
                                                    #[cfg(feature = "wgpu")]
                                                    wgpu_device: callback_wgpu_device.clone(),
                                                })
                                            };
                                            (callback)(Ok(StreamEvent::Video(video_frame)));
                                        },
                                        SCFrameStatus::Suspended |
                                        SCFrameStatus::Idle => {
                                            if callback_stopped_flag.load(atomic::Ordering::Acquire) {
                                                return;
                                            }
                                            (callback)(Ok(StreamEvent::Idle));
                                        },
                                        SCFrameStatus::Stopped => {
                                            if callback_stopped_flag.fetch_or(true, atomic::Ordering::AcqRel) {
                                                return;
                                            }
                                            (callback)(Ok(StreamEvent::End));
                                        }
                                        _ => {}
                                    }

                                    
                                },
                            }
                        },
                        Err(err) => {
                            let event = match err {
                                SCStreamCallbackError::StreamStopped => {
                                    if callback_stopped_flag.fetch_or(true, atomic::Ordering::AcqRel) {
                                        return;
                                    }
                                    Ok(StreamEvent::End)
                                },
                                SCStreamCallbackError::SampleBufferCopyFailed => Err(StreamError::Other("Failed to copy sample buffer".into())),
                                SCStreamCallbackError::Other(e) => Err(StreamError::Other(format!("Internal stream failure: [description: {}, reason: {}, code: {}, domain: {}]", e.description(), e.reason(), e.code(), e.domain()))),
                            };
                            (callback)(event);
                        }
                    }
                }));

                let mut sc_stream = SCStream::new(filter, config, handler_queue, handler)
                    .map_err(|error| StreamCreateError::Other(error))?;

                sc_stream.start();

                Ok(MacosCaptureStream {
                    stopped_flag,
                    shared_callback,
                    stream: MacosCaptureStreamInternal::Window(sc_stream),
                    #[cfg(feature = "metal")]
                    metal_device,
                    #[cfg(feature = "wgpu")]
                    wgpu_device
                })
            }
        }

    }

    pub(crate) fn stop(&mut self) -> Result<(), StreamStopError> {
        {
            let mut callback = self.shared_callback.lock();
            if !self.stopped_flag.fetch_or(true, atomic::Ordering::AcqRel) {
                (callback)(Ok(StreamEvent::End));
            } else {
                return Ok(());
            }
        }
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
