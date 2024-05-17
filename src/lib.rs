//! A cross-platform screen/window/audio capture library
//! 
//! ## MacOS Docs
//! 
//! Since we depend on the metal crate, our docs won't build for macos under docs.rs's linux containers. As a workaround, you can see our build of the docs for MacOS here:
//! [MacOS Documentation](https://augmendtech.github.io/CrabGrab/macos_docs/crabgrab/index.html)
//! 
//! ## Feature flags
//! 
//! ### GPU Inter-op
//! 
//! - **`dx11`** - enables retrieving the surface of a video frame and getting the DX11 device instance for the stream (Windows only)
//! - **`dxgi`** - enables retrieving the surface of a video frame and getting the DXGI device instance for the stream (Windows only)
//! - **`metal`** - enables retrieving the Metal textures for a video frame and getting the Metal device instance for the stream (MacOS only)
//! - **`iosurface`** - enables retrieving the IOSurface for a video frame (MacOS only)
//! - **`wgpu`** - enables retrieving a Wgpu texture from a video frame and getting the Wgpu device instance wrapper for the stream
//! 
//! ### Bitmap output
//! 
//! - **`bitmap`** - enables creating raw bitmap copies of frames in system memory
//! 
//! ### Screenshots
//! 
//! - **`screenshot`** - provides an easy-to-use function wrapping `CaptureStream` for single-frame capture
//! 
//! ## Example
//! 
//! ```
//! use std::time::Duration;
//! use crabgrab::prelude::*;
//! 
//! // spin up the async runtime
//! let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
//! // run our capture code in an async context
//! let future = runtime.spawn(async {
//!     // ensure we have priveleges to capture content
//!     let token = match CaptureStream::test_access(false) {
//!         Some(token) => token,
//!         None => CaptureStream::request_access(false).await.expect("Expected capture access")
//!     };
//!     // filter to normal windows
//!     let filter = CapturableContentFilter::NORMAL_WINDOWS;
//!     // get capturable content matching the filter
//!     let content = CapturableContent::new(filter).await.unwrap();
//!     // find the window we want to capture
//!     let window = content.windows().filter(|window| {
//!         let app_identifier = window.application().identifier();
//!         app_identifier.to_lowercase().contains("finder") || app_identifier.to_lowercase().contains("explorer")
//!     }).next();
//!     match window {
//!         Some(window) => {
//!             println!("capturing window: {}", window.title()); 
//!             // create a captuere config using the first supported pixel format
//!             let config = CaptureConfig::with_window(window, CaptureStream::supported_pixel_formats()[0]).unwrap();
//!             // create a capture stream with an event handler callback
//!             let mut stream = CaptureStream::new(token, config, |stream_event| {
//!                 match stream_event {
//!                     Ok(event) => {
//!                         match event {
//!                             StreamEvent::Video(frame) => {
//!                                 println!("Got frame: {}", frame.frame_id());
//!                             },
//!                             _ => {}
//!                         }
//!                     },
//!                     Err(error) => {
//!                         println!("Stream error: {:?}", error);
//!                     }
//!                 }
//!             }).unwrap();
//!             // wait for a while to capture some frames
//!             tokio::task::block_in_place(|| std::thread::sleep(Duration::from_millis(4000)));
//!             stream.stop().unwrap();
//!         },
//!         None => { println!("Failed to find window"); }
//!     }
//! });
//! // wait for the future to complete
//! runtime.block_on(future).unwrap();
//! // shutdown the async runtime
//! runtime.shutdown_timeout(Duration::from_millis(10000));
//! ````
//! 

/// Platform-specific extensions
pub mod platform;
/// Extension features
pub mod feature;

/// Geometry types
pub mod util;
/// Audio and video frames
pub mod frame;
/// The actual capture stream and related constructs
pub mod capture_stream;
/// Enumeration of capturable items
pub mod capturable_content;

/// Everything
pub mod prelude;