use winit::window::Window;

#[allow(dead_code)]
pub struct ContextInfo {
    instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
}

impl ContextInfo {
    pub async fn init(window: &Window) -> ContextInfo {
        //Instance and device init
        let instance = wgpu::Instance::default();
        let surface = unsafe {
            instance.create_surface(&window)
        }.unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions { 
            power_preference: wgpu::PowerPreference::default(), 
            force_fallback_adapter: false, 
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");
        
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");
        ContextInfo {instance, adapter, device, queue, surface}
    } 
}
