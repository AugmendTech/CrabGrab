use std::cell::RefCell;
use std::time::Instant;

use crate::feature::screenshot::ScreenshotError;
use crate::frame::VideoFrame;
use crate::platform::macos::frame::{MacosSCStreamVideoFrame, MacosVideoFrame};
use crate::platform::macos::objc_wrap::{CGPoint, CGRect, CGSize, CMTime, NSArray, SCContentFilter, SCScreenshotManager, SCStreamColorMatrix, SCStreamConfiguration, SCStreamPixelFormat};
use crate::platform::platform_impl::objc_wrap::CGMainDisplayID;
use crate::prelude::{Capturable, CaptureConfig, CapturePixelFormat};

pub async fn take_screenshot(config: CaptureConfig) -> Result<VideoFrame, ScreenshotError> {
    // Force core graphics initialization
    println!("A");
    unsafe { CGMainDisplayID() };
    println!("B");
    let mut stream_config = SCStreamConfiguration::new();
    let filter = match &config.target {
        Capturable::Window(window) => SCContentFilter::new_with_desktop_independent_window(&window.impl_capturable_window.window),
        Capturable::Display(display) => SCContentFilter::new_with_display_excluding_apps_excepting_windows(display.impl_capturable_display.display.clone(), NSArray::new(), NSArray::new())
    };
    println!("C");
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
    stream_config.set_pixel_format(pixel_format);
    stream_config.set_source_rect(CGRect {
        origin: CGPoint {
            x: config.source_rect.origin.x,
            y: config.source_rect.origin.y,
        },
        size: CGSize {
            x: config.source_rect.size.width,
            y: config.source_rect.size.height
        }
    });
    stream_config.set_size(CGSize {
        x: config.output_size.width,
        y: config.output_size.height,
    });
    stream_config.set_show_cursor(config.show_cursor);
    stream_config.set_capture_audio(false);
    println!("D");
    let (tx, rx) = futures::channel::oneshot::channel();
    let mut tx = Some(tx);
    #[cfg(feature = "metal")]
    let callback_metal_device = config.impl_capture_config.metal_device.clone();
    println!("F");
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
                            metal_device: callback_metal_device.clone()
                        }
                    )
                })
            },
            Err(error) => Err(ScreenshotError::Other(format!("Failed to capture screenshot: {}", error)))
        };
        tx.take().unwrap().send(screenshot_result).unwrap();
    });
    println!("G");
    rx.await
        .map_err(|_| ScreenshotError::Other("Failed to await callback future".into()))?
}
