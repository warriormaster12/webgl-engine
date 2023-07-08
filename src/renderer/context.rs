use std::{borrow::Cow, mem};

use wgpu::util::DeviceExt;
use winit::window::Window;

mod shader;

#[allow(dead_code)]
pub struct Context {
    instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    //swapchain
    swapchain: Swapchain,
    pub surface: wgpu::Surface,
}

use bytemuck::{Pod, Zeroable};

impl Context {
    pub async fn new(window: &Window) -> Context {
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
        let swapchain = Swapchain::new(&adapter, &device, &surface, (window.inner_size().width, window.inner_size().height));
        Context {instance, adapter, device, queue,surface,swapchain}
    } 

    pub fn get_swapchain(&self)-> &Swapchain {
        return &self.swapchain;
    }

    pub fn update_swapchain(&mut self, resolution: (u32, u32)) {
        self.swapchain.config.width = resolution.0;
        self.swapchain.config.height = resolution.1;
        self.surface.configure(&self.device, &self.swapchain.config);
    }

    fn draw(&self, encoder:&mut wgpu::CommandEncoder, frame: &wgpu::SurfaceTexture, meshes: &Vec<Mesh>) {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        for i in 0..meshes.len() {
            rpass.set_pipeline(&meshes[i].material.pipeline);
            shader::bind_groups::set_bind_groups(
                &mut rpass,
                shader::bind_groups::BindGroups {bind_group0: &meshes[i].material.bind_group},
            );
            rpass.set_index_buffer(meshes[i].index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, meshes[i].vertex_buffer.slice(..));
            rpass.draw_indexed(0..meshes[i].indicies.len() as u32, 0, 0..1);
        }
    }

    pub fn present(&self, meshes: &Vec<Mesh>) {
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let frame = self.surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        self.draw(&mut encoder, &frame,meshes);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
    
}
pub struct Swapchain {
    config: wgpu::SurfaceConfiguration
}

impl Swapchain{
    fn new(adapter: &wgpu::Adapter, device: &wgpu::Device, surface: &wgpu::Surface, resolution: (u32, u32)) -> Swapchain {
        //Swapchain
        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: resolution.0,
            height: resolution.1,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Swapchain {config: config}
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn get_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}

pub struct Material {
  pub pipeline: wgpu::RenderPipeline,
  pub bind_group: shader::bind_groups::BindGroup0,
}

impl Material {
    pub fn new(context: &Context, buffers: [&wgpu::Buffer;2]) -> Material {
        //Shader and pipeline

        let module = shader::create_shader_module(&context.device);
        
        let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&shader::create_pipeline_layout(&context.device)),
            vertex: shader::vertex_state(&module, &shader::vs_main_entry(wgpu::VertexStepMode::Vertex)),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: shader::ENTRY_FS_MAIN,
                targets: &[Some(context.get_swapchain().get_format().into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Vulkan equivalent to descriptor sets
        let bind_group = shader::bind_groups::BindGroup0::from_bindings(
            &context.device,
            shader::bind_groups::BindGroupLayout0 {
                camera: buffers[0].as_entire_buffer_binding(),
                triangle_data: buffers[1].as_entire_buffer_binding(),
            }
        );

        Material { pipeline: render_pipeline, bind_group }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pos: [f32; 4],
    tex_coord: [f32; 2]
}

fn vertex(pos: [i8; 3], tex_coords: [i8; 2]) -> Vertex {
    Vertex { pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0], tex_coord: [tex_coords[0] as f32, tex_coords[1] as f32] }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        // top (0, 0, 1)
        vertex([-1, -1, 1], [0, 0]),
        vertex([1, -1, 1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([-1, 1, 1], [0, 1]),
        // bottom (0, 0, -1)
        vertex([-1, 1, -1], [1, 0]),
        vertex([1, 1, -1], [0, 0]),
        vertex([1, -1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // right (1, 0, 0)
        vertex([1, -1, -1], [0, 0]),
        vertex([1, 1, -1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([1, -1, 1], [0, 1]),
        // left (-1, 0, 0)
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, 1, 1], [0, 0]),
        vertex([-1, 1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // front (0, 1, 0)
        vertex([1, 1, -1], [1, 0]),
        vertex([-1, 1, -1], [0, 0]),
        vertex([-1, 1, 1], [0, 1]),
        vertex([1, 1, 1], [1, 1]),
        // back (0, -1, 0)
        vertex([1, -1, 1], [0, 0]),
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, -1, -1], [1, 1]),
        vertex([1, -1, -1], [0, 1]),
    ];

    let index_data: &[u16] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    (vertex_data.to_vec(), index_data.to_vec())
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indicies: Vec<u16>,
    pub verticies: Vec<Vertex>,
    pub material: Material
}

impl Mesh {
    pub fn new(context: &Context, material: Material) -> Mesh{
        let (verticies, indicies) = create_vertices();

        let vertex_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&verticies),
            usage: wgpu::BufferUsages::VERTEX,
        });
    
        let index_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indicies),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh { vertex_buffer: vertex_buffer, index_buffer: index_buffer, indicies: indicies, verticies: verticies, material: material }
    }
}

