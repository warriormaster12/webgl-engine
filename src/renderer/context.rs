use std::{mem, f32::consts, borrow::Cow, collections::HashMap, ops::Index};
use naga;

use wgpu::util::DeviceExt;
use winit::window::Window;



#[allow(dead_code)]
pub struct Context {
    instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    //swapchain
    swapchain: Swapchain,
    pub surface: wgpu::Surface,
    render_pipelines: HashMap<String, RenderPipeline>,
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
        Context {instance, adapter, device, queue,surface,swapchain, render_pipelines: HashMap::new()}
    } 

    pub fn get_swapchain(&self)-> &Swapchain {
        return &self.swapchain;
    }

    pub fn update_swapchain(&mut self, resolution: (u32, u32)) {
        self.swapchain.config.width = resolution.0;
        self.swapchain.config.height = resolution.1;
        self.surface.configure(&self.device, &self.swapchain.config);
    }

    pub fn add_render_pipeline(&mut self, id: String) {
        if self.render_pipelines.get(&id).is_none() {
            let pipeline = RenderPipeline::new(self);
            self.render_pipelines.insert(id, pipeline);
        }
    }

    pub fn get_pipeline(&mut self, id: String) -> Option<&mut RenderPipeline> {
        if self.render_pipelines.get(&id).is_some() {
            return self.render_pipelines.get_mut(&id);
        }
        return None;
    }

    pub fn bind_resource_to_pipeline(&mut self, id: String, group: usize, resources: Vec<wgpu::BindingResource<'_>>) {
        if let Some(pipeline) = self.render_pipelines.get_mut(&id) {
            pipeline.bind_resource(&self.device, group, resources)
        }
    }

    fn draw(&self, view: wgpu::TextureView, meshes: &Vec<Mesh>) {
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {r: 0.3, g: 0.5, b: 1.0, a: 1.0}),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            for pipeline in self.render_pipelines.values() {
                rpass.set_pipeline(&pipeline.pipeline);
                for i in 0..meshes.len() {
                    for (index, group) in &pipeline.bind_groups {
                        rpass.set_bind_group(index.clone() as u32, &group, &[]);
                    }
                    if pipeline.group_layouts.len() == pipeline.bind_groups.len() {
                        rpass.set_index_buffer(meshes[i].index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        rpass.set_vertex_buffer(0, meshes[i].vertex_buffer.slice(..));
                        rpass.draw_indexed(0..meshes[i].indicies.len() as u32, 0, 0..1);
                    }
                }

            }
        }
        self.queue.submit(Some(encoder.finish()));
    }

    pub fn present(&self, meshes: &Vec<Mesh>) {
        let frame = self.surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        self.draw(frame.texture.create_view(&wgpu::TextureViewDescriptor::default()),meshes);
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

    pub fn get_aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    pub fn get_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}

pub struct Camera {
    pub fov: f32,
    pub is_active: bool,
    camera_buffer: wgpu::Buffer,
}
impl Camera {
    fn generate_matrix(aspect_ratio: f32, fov:f32, z_near:f32, z_far:f32) -> glam::Mat4 {
        let projection = glam::Mat4::perspective_rh(fov * consts::PI / 180., aspect_ratio, z_near, z_far);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(1.5f32, -5.0, 3.0),
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        );
        projection * view
    }
    pub fn new(context: &mut Context, fov: f32) -> Camera {
        let mx_total = Camera::generate_matrix(context.get_swapchain().get_aspect_ratio(), fov, 1.0, 100.0);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        let camera_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        context.bind_resource_to_pipeline("shader".to_string(), 0, [camera_buffer.as_entire_binding()].to_vec());
        Camera { fov: fov, is_active: true, camera_buffer: camera_buffer }
    }

    pub fn update(&self, context: &Context) {
        let mx_total = Camera::generate_matrix(context.get_swapchain().get_aspect_ratio(), self.fov, 1.0, 100.0);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        context.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(mx_ref));
    }
}

pub struct RenderPipeline {
    pipeline: wgpu::RenderPipeline,
    group_layouts: Vec<wgpu::BindGroupLayout>,
    bind_groups: HashMap<usize,wgpu::BindGroup>
}
impl RenderPipeline {
    pub fn new(context: &Context) -> RenderPipeline {
        //Shader and pipeline
        let module = context.device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: None, 
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("context/shader.wgsl"))) 
        });
        let naga_module = naga::front::wgsl::parse_str(include_str!("context/shader.wgsl")).unwrap();
        let mut group_layouts: Vec<wgpu::BindGroupLayout> = Vec::new();
        for global_handle in naga_module.global_variables.iter() {
            let handle = &naga_module.global_variables[global_handle.0];
            let mut entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
            if let Some(bindings) = &handle.binding {
                let ty = match naga_module.types[handle.ty].inner {
                    naga::TypeInner::Struct { .. } => {
                        wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None 
                        }
                    },
                    //naga::TypeInner::Image { .. } => quote!(&'a wgpu::TextureView),
                    //naga::TypeInner::Sampler { .. } => quote!(&'a wgpu::Sampler),
                    //naga::TypeInner::Array { .. } => quote!(wgpu::BufferBinding<'a>),
                    _ => panic!("Unsupported type for binding fields."),
                };
                let entry = wgpu::BindGroupLayoutEntry{
                    binding: bindings.binding,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: ty,
                    count: None
                };
                entries.push(entry);
            }
            if entries.len() > 0 {
                let layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { 
                    label: None,
                    entries: &entries,
                });
                group_layouts.push(layout);
            }
        }
        
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        }];
        let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &context.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor { 
                        label: None,
                        bind_group_layouts: &[&group_layouts[0], &group_layouts[1]],
                        push_constant_ranges: &[], 
                    }
                )
            ),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: &naga_module.entry_points[0].name,
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: &naga_module.entry_points[1].name,
                targets: &[Some(context.get_swapchain().get_format().into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        RenderPipeline { pipeline: render_pipeline, group_layouts: group_layouts, bind_groups: HashMap::new() }
    }
    pub fn bind_resource(&mut self, device: &wgpu::Device, group: usize, resources: Vec<wgpu::BindingResource>) {
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        for i in 0..resources.len() {
            let entry = wgpu::BindGroupEntry{
                binding: i as u32,
                resource: resources[i].clone(), 
            };
            entries.push(entry);
        }
        self.bind_groups.entry(group).or_insert(device.create_bind_group(
            &wgpu::BindGroupDescriptor{
                label: None,
                layout: &self.group_layouts[group],
                entries: &entries,
            }
        ));
    }
}
pub struct Material {
  material_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GPUMaterialData {
    pub color: [f32; 4]
}

struct GPUMesh {
    pub model_mx: [f32; 16]
}

impl Material {
    pub fn new(context: &mut Context) -> Material {

        let material_data = GPUMaterialData {color: [1.0,1.0,1.0,1.0]};
        let material_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::bytes_of(&material_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        context.bind_resource_to_pipeline("shader".to_string(), 1, [material_buffer.as_entire_binding()].to_vec());
        Material { material_buffer }
    }
    pub fn set_color(&self, context: &Context, color: [f32; 4]) {
        let material_data = GPUMaterialData {color};
        context.queue.write_buffer(&self.material_buffer, 0, bytemuck::bytes_of(&material_data));
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
    pub fn new(device: &wgpu::Device, material: Material) -> Mesh{
        let (verticies, indicies) = create_vertices();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&verticies),
            usage: wgpu::BufferUsages::VERTEX,
        });
    
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indicies),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh { vertex_buffer: vertex_buffer, index_buffer: index_buffer, indicies: indicies, verticies: verticies, material: material }
    }
}

