use futures::executor::block_on;

fn main() {
    block_on(async {
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            flags: wgpu::InstanceFlags::default(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });
        let wgpu_adapter = wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            force_fallback_adapter: false,
            compatible_surface: None,
        }).await.expect("Expected wgpu adapter");
        let (wgpu_device, wgpu_queue) = wgpu_adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("wgpu adapter"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
        }, None).await.expect("Expected wgpu device");
        
    });
}