use naga;
use std::{borrow::Cow, collections::HashMap, mem};

use winit::window::Window;

pub mod resources;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub enum BindingGroupType {
    Global = 0,
    PerFrame = 1,
    Resource = 2,
    PerObject = 3,
}

#[derive(Clone)]
pub enum BindingResourceType {
    Buffer = 0,
    BufferArray = 1,
    Sampler = 2,
    SamplerArray = 3,
    TextureView = 4,
    TextureViewArray = 5,
}
#[derive(Clone)]
pub struct BindingResource {
    pub id: &'static str,
    pub resource_type: BindingResourceType,
    pub entire_binding: bool, // if true, offset and size are ignored
    pub offset: wgpu::BufferAddress,
    pub size: u64,
}

impl Default for BindingResource {
    fn default() -> Self {
        Self {
            id: "None",
            resource_type: BindingResourceType::Buffer,
            entire_binding: true,
            offset: 0,
            size: 0,
        }
    }
}

#[allow(dead_code)]
pub struct RendererServer {
    instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    //swapchain
    swapchain: Swapchain,
    pub surface: wgpu::Surface,
}

use bytemuck::{Pod, Zeroable};
impl RendererServer {
    pub async fn new(window: &Window) -> RendererServer {
        //Instance and device init
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        let swapchain = Swapchain::new()
            .new_resolution((window.inner_size().width, window.inner_size().height))
            .build(&adapter, &device, &surface);
        RendererServer {
            instance,
            adapter,
            device,
            queue,
            surface,
            swapchain,
        }
    }

    pub fn get_swapchain(&self) -> &Swapchain {
        return &self.swapchain;
    }

    pub fn update_swapchain(&mut self, resolution: (u32, u32)) {
        Swapchain::new().new_resolution(resolution);
    }

    pub fn get_new_frame(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView, &wgpu::TextureView) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (frame, frame_view, &self.get_swapchain().depth_view)
    }

    pub fn get_uniform_aligned_buffer_size(&self, value: wgpu::BufferAddress) -> u64 {
        let uniform_alignment = {
            let alignment =
                self.device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            wgpu::util::align_to(value, alignment)
        };
        uniform_alignment
    }

    pub fn get_storage_aligned_buffer_size(&self, value: wgpu::BufferAddress) -> u64 {
        let storage_alignment = {
            let alignment =
                self.device.limits().min_storage_buffer_offset_alignment as wgpu::BufferAddress;
            wgpu::util::align_to(value, alignment)
        };
        storage_alignment
    }
}

struct SwapchainBuilder {
    resolution: (u32, u32),
    present_mode: wgpu::PresentMode,
}

impl SwapchainBuilder {
    fn new_resolution(&mut self, resolution: (u32, u32)) -> &mut Self {
        self.resolution = resolution;
        self
    }
    fn new_present_mode(&mut self, mode: wgpu::PresentMode) -> &mut Self {
        self.present_mode = mode;
        self
    }
    fn build(
        &mut self,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        surface: &wgpu::Surface,
    ) -> Swapchain {
        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: self.resolution.0,
            height: self.resolution.1,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        let depth_format = wgpu::TextureFormat::Depth32Float;
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("Swapchain depth texture"),
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        surface.configure(&device, &config);
        Swapchain {
            config: config,
            depth_view: depth_view,
            depth_format: depth_format,
        }
    }
}
pub struct Swapchain {
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    depth_format: wgpu::TextureFormat,
}

impl Swapchain {
    fn new() -> SwapchainBuilder {
        SwapchainBuilder {
            resolution: (0, 0),
            present_mode: wgpu::PresentMode::Fifo,
        }
    }
    pub fn get_resolution(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    pub fn get_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn get_depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct RenderPipelineSettings {
    pub shader: &'static str,
    pub cull_mode: wgpu::Face,
    pub depth_testing: bool,
    pub depth_write_enabled: bool,
    pub depth_compare: wgpu::CompareFunction,
}

impl Default for RenderPipelineSettings {
    fn default() -> Self {
        Self {
            shader: "",
            cull_mode: wgpu::Face::Back,
            depth_testing: false,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GPUMesh {
    pub model_mx: [f32; 16],
}

impl GPUMesh {
    pub fn new() -> GPUMesh {
        let model_mx = glam::Mat4::IDENTITY;
        GPUMesh {
            model_mx: model_mx.as_ref().clone(),
        }
    }
    pub fn get_size() -> u64 {
        return mem::size_of::<Self>() as u64;
    }
}

pub struct Transform {
    translation: glam::Vec3,
    rotation: glam::Vec3,
    scale: glam::Vec3,
    values_changed: bool,
}

impl Transform {
    pub fn new() -> Transform {
        Transform {
            translation: glam::Vec3::ZERO,
            rotation: glam::Vec3::ZERO,
            scale: glam::Vec3::ONE,
            values_changed: true,
        }
    }
    pub fn get_translation(&self) -> &glam::Vec3 {
        return &self.translation;
    }
    pub fn set_translation(&mut self, input: glam::Vec3) {
        self.translation = input;
        self.values_changed = true;
    }
    pub fn get_scale(&self) -> &glam::Vec3 {
        return &self.scale;
    }
    pub fn set_scale(&mut self, input: glam::Vec3) {
        self.scale = input;
        self.values_changed = true;
    }
    pub fn get_rotation(&self) -> &glam::Vec3 {
        return &self.rotation;
    }
    pub fn set_rotation(&mut self, input: glam::Vec3) {
        self.rotation = input;
        self.values_changed = true;
    }
    pub fn get_values_changed(&self) -> bool {
        return self.values_changed;
    }
    pub fn set_values_changed(&mut self, input: bool) {
        self.values_changed = input;
    }
    pub fn generate_transform_matrix(&self) -> glam::Mat4 {
        let rot_quat = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x.to_radians(),
            self.rotation.y.to_radians(),
            self.rotation.z.to_radians(),
        );
        return glam::Mat4::from_scale_rotation_translation(self.scale, rot_quat, self.translation);
    }
}
