use std::cell::RefCell;
use std::time::Instant;

use crate::feature::screenshot::ScreenshotError;
use crate::frame::VideoFrame;
use crate::platform::macos::frame::{MacosSCStreamVideoFrame, MacosVideoFrame};
use crate::platform::macos::objc_wrap::{CGSize, NSArray, SCContentFilter, SCScreenshotManager, SCStreamColorMatrix, SCStreamColorSpaceName, SCStreamConfiguration, SCStreamPixelFormat};
use crate::platform::platform_impl::objc_wrap::{CGMainDisplayID, CMTime, DispatchQueue, SCStream, SCStreamCallbackError, SCStreamHandler, SCStreamOutputType};
use crate::prelude::{Capturable, CaptureAccessToken, CaptureConfig, CapturePixelFormat};

/// Take a screenshot of the capturable content given a configuration
pub async fn take_screenshot(token: CaptureAccessToken, config: CaptureConfig) -> Result<VideoFrame, ScreenshotError> {
    let _ = token;
    // Force core graphics initialization
    unsafe { CGMainDisplayID() };
    let mut stream_config = SCStreamConfiguration::new();
    let filter = match &config.target {
        Capturable::Window(window) => SCContentFilter::new_with_desktop_independent_window(&window.impl_capturable_window.window),
        Capturable::Display(display) => SCContentFilter::new_with_display_excluding_apps_excepting_windows(display.impl_capturable_display.display.clone(), NSArray::new(), NSArray::new())
    };
    stream_config.set_scales_to_fit(false);
    let (pixel_format, set_color_matrix) = match config.pixel_format {
        CapturePixelFormat::Bgra8888 =>    (SCStreamPixelFormat::BGRA8888, false),
        CapturePixelFormat::Argb2101010 => (SCStreamPixelFormat::L10R, false),
        CapturePixelFormat::V420 =>        (SCStreamPixelFormat::V420, true),
        CapturePixelFormat::F420 =>        (SCStreamPixelFormat::F420, true),
    };
    if set_color_matrix {
        stream_config.set_color_matrix(SCStreamColorMatrix::ItuR709_2);
    }
    if let Some(color_space) = config.color_space {
        stream_config.set_color_space(SCStreamColorSpaceName::from_string(color_space));
    }
    stream_config.set_pixel_format(pixel_format);
    stream_config.set_size(CGSize {
        x: config.output_size.width,
        y: config.output_size.height,
    });
    stream_config.set_show_cursor(config.show_cursor);
    stream_config.set_capture_audio(false);
    stream_config.set_minimum_time_interval(CMTime::new_with_seconds(0.0, 100));
    let (tx, rx) = futures::channel::oneshot::channel();
    let mut tx = Some(tx);
    #[cfg(feature = "metal")]
    let callback_metal_device = config.impl_capture_config.metal_device.clone();
    #[cfg(feature = "wgpu")]
    let callback_wgpu_device = config.impl_capture_config.wgpu_device.clone();
    let mut persist_scstream = None;
    if SCScreenshotManager::class_exists() {
        SCScreenshotManager::capture_samplebuffer_with_filter_and_configuration(filter, stream_config, move |result| {
            let screenshot_result = match result {
                Ok(sample_buffer) => {
                    let capture_time = Instant::now();
                    Ok(VideoFrame {
                        impl_video_frame: MacosVideoFrame::SCStream(
                            MacosSCStreamVideoFrame {
                                sample_buffer,
                                capture_time,
                                dictionary: RefCell::new(None),
                                frame_id: 0,
                                #[cfg(feature = "metal")]
                                metal_device: callback_metal_device.clone(),
                                #[cfg(feature = "wgpu")]
                                wgpu_device: callback_wgpu_device.clone(),
                            }
                        )
                    })
                },
                Err(error) => Err(ScreenshotError::Other(format!("Failed to capture screenshot: {}", error)))
            };
            tx.take().unwrap().send(screenshot_result).unwrap();
        });
    } else {
        let handler = SCStreamHandler::new(move |stream_result| {
            let screenshot_result = match stream_result {
                Ok((sample_buffer, SCStreamOutputType::Screen)) => {
                    let capture_time = Instant::now();
                    Some(Ok(VideoFrame {
                        impl_video_frame: MacosVideoFrame::SCStream(
                            MacosSCStreamVideoFrame {
                                sample_buffer,
                                capture_time,
                                dictionary: RefCell::new(None),
                                frame_id: 0,
                                #[cfg(feature = "metal")]
                                metal_device: callback_metal_device.clone(),
                                #[cfg(feature = "wgpu")]
                                wgpu_device: callback_wgpu_device.clone(),
                            }
                        )
                    }))
                },
                Err(error) => {
                    let description = match error {
                        SCStreamCallbackError::Other(error) => error.description(),
                        SCStreamCallbackError::SampleBufferCopyFailed => "Failed to copy sample buffer".to_string(),
                        SCStreamCallbackError::StreamStopped => "Stream stopped early".to_string(),
                    };
                    Some(Err(ScreenshotError::Other(format!("Failed to capture screenshot: {}", description))))
                },
                _ => None
            };
            if let (Some(screenshot_result), Some(tx)) = (screenshot_result, tx.take()) {
                tx.send(screenshot_result).unwrap();
            }
        });
        let mut stream = match SCStream::new(
            filter,
            stream_config,
            DispatchQueue::make_serial("crabgrab.screenshot".into()),
            handler
        ) {
            Ok(stream) => stream,
            Err(error) => Err(ScreenshotError::Other(format!("Failed to build SCStream: {}", error)))?,
        };
        stream.start();
        persist_scstream = Some(stream);
    }
    let result = rx.await
        .map_err(|_| ScreenshotError::Other("Failed to await callback future".into()))?;
    if let Some(sc_stream) = persist_scstream {
        drop(sc_stream);
    }
    result
}
