use std::sync::Arc;
use std::time::Duration;

use futures::executor::block_on;
use crabgrab::prelude::*;
use crabgrab::feature::wgpu::WgpuCaptureConfigExt as _;
use crabgrab::feature::wgpu::WgpuVideoFrameExt as _;

#[allow(unused)]
struct Gfx {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl AsRef<wgpu::Device> for Gfx {
    fn as_ref(&self) -> &wgpu::Device {
        &self.device
    }
}

fn main() {
    block_on(async {
        let token = match CaptureStream::test_access(false) {
            Some(token) => token,
            None => CaptureStream::request_access(false).await.expect("Expected capture access")
        };
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(target_os = "windows")]
            backends: wgpu::Backends::DX12,
            #[cfg(target_os = "macos")]
            backends: wgpu::Backends::METAL,
            flags: wgpu::InstanceFlags::VALIDATION | wgpu::InstanceFlags::GPU_BASED_VALIDATION,
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });
        let wgpu_adapter = wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }).await.expect("Expected wgpu adapter");
        let (wgpu_device, wgpu_queue) = wgpu_adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("wgpu adapter"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
        }, None).await.expect("Expected wgpu device");
        let gfx = Arc::new(Gfx {
            device: wgpu_device,
            queue: wgpu_queue,
        });
        let filter = CapturableContentFilter::DISPLAYS;
        let content = CapturableContent::new(filter).await
            .expect("Expected to get capturable displays");
        let display = content.displays().next()
            .expect("Expected at least one capturable display");
        let config = CaptureConfig::with_display(display, CapturePixelFormat::Bgra8888)
            .with_wgpu_device(gfx.clone())
            .expect("Expected config with wgpu device");
        let (tx_result, rx_result) = futures::channel::oneshot::channel::<Result<Option<VideoFrame>, StreamError>>();
        let mut tx_result = Some(tx_result);
        let _stream = CaptureStream::new(token, config, move |event_result| {
            match event_result {
                Ok(event) => {
                    match event {
                        StreamEvent::Video(frame) => {
                            if let Some(tx_result) = tx_result.take() {
                                println!("Sending frame...");
                                tx_result.send(Ok(Some(frame)))
                                    .expect("Expected to send result");
                            }
                        },
                        StreamEvent::End => {
                            if let Some(tx_result) = tx_result.take() {
                                tx_result.send(Ok(None))
                                    .expect("Expected to send result");
                            }
                        },
                        _ => {}
                    }
                },
                Err(error) => {
                    if let Some(tx_result) = tx_result.take() {
                        tx_result.send(Err(error))
                            .expect("Expected to send result");
                    }
                }
            }
        }).expect("Expected capture stream");
        println!("Stream started. Awaiting message...");
        let message = rx_result.await;
        println!("Message received! Inspecting...");
        let frame_opt = message
            .expect("Expected to receive result")
            .expect("Expected to receive frame option");
        println!("Got frame opt. unwrapping...");
        match frame_opt {
            Some(frame) => {
                println!("Got frame! getting wgpu texture...");
                let wgpu_texture = frame.get_wgpu_texture(crabgrab::feature::wgpu::WgpuVideoFramePlaneTexture::Rgba, Some("wgpu video frame"))
                    .expect("Expected wgpu texture from video frame");
                println!("Got wgpu texture! Size: {:?}, Format: {:?}", wgpu_texture.size(), wgpu_texture.format());
            },
            None => {
                println!("Got None! Oh no!");
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
    });
}