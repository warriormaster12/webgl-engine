use std::{mem, f32::consts, borrow::Cow, collections::HashMap};
use naga;

use wgpu::util::DeviceExt;
use winit::window::Window;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub enum BindingGroupType {
    Global = 0,
    PerFrame = 1,
    Resource = 2,
    PerObject = 3,
}

const MAX_MESH_COUNT:u64 = 10000;

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
    meshes: Vec<Mesh>,
    buffers: HashMap<String, wgpu::Buffer>,
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
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .expect("Failed to create device");
        let swapchain = Swapchain::new(&adapter, &device, &surface, (window.inner_size().width, window.inner_size().height));
        Context {instance, adapter, device, queue,surface,swapchain, render_pipelines: HashMap::new(), meshes: Vec::new(), buffers: HashMap::new()}
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

    pub fn create_mesh(&mut self, id: String,material: Material) {
        let mesh = Mesh::new(self, id,material);
        self.meshes.push(mesh);
    }

    pub fn get_mesh(&self, id: String) -> Option<&Mesh> {
        if let Some(mesh) = self.meshes.iter().find(|&m| m.id == id) {
            return Some(mesh);
        }
        return None;
    }

    pub fn bind_meshes_to_pipeline(&mut self) {
        if self.buffers.get("mesh buffer").is_none() {
            let mesh_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mesh Buffer"),
                size: mem::size_of::<GPUMesh>() as u64 * MAX_MESH_COUNT,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.bind_resource_to_pipeline("shader".to_string(), BindingGroupType::Resource, [mesh_buffer.as_entire_binding()].to_vec());
            self.buffers.insert("mesh buffer".to_string(), mesh_buffer);
        }
    }

    pub fn bind_resource_to_pipeline(&mut self, id: String, group: BindingGroupType, resources: Vec<wgpu::BindingResource<'_>>) {
        if let Some(pipeline) = self.render_pipelines.get_mut(&id) {
            pipeline.bind_resource(&self.device, group, resources)
        }
    }

    fn draw(&mut self, view: wgpu::TextureView) {
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {r: 0.1, g: 0.1, b: 0.1, a: 1.0}),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            for pipeline in self.render_pipelines.values() {
                rpass.set_pipeline(&pipeline.pipeline);
                if pipeline.group_layouts.len() == pipeline.bind_groups.len() {
                    if let Some(global_group) = pipeline.bind_groups.get(&BindingGroupType::Global) {
                        rpass.set_bind_group(BindingGroupType::Global as u32, &global_group, &[]);
                    }
                    if let Some(per_frame) = pipeline.bind_groups.get(&BindingGroupType::PerFrame) {
                        rpass.set_bind_group(BindingGroupType::PerFrame as u32, &per_frame, &[]);
                    }
                    for i in 0..self.meshes.len() {
                        if let Some((buffer_id, data)) = self.meshes[i].update_model_mx() {
                            if let Some(buffer) = self.buffers.get(&buffer_id) {
                                self.queue.write_buffer(buffer, 0, bytemuck::bytes_of(&data));
                            }
                        }
                        if let Some(per_resource) = pipeline.bind_groups.get(&BindingGroupType::Resource) {
                            rpass.set_bind_group(BindingGroupType::Resource as u32, &per_resource, &[]);
                        }
                        if let Some(per_object) = pipeline.bind_groups.get(&BindingGroupType::PerObject) {
                            rpass.set_bind_group(BindingGroupType::PerObject as u32, &per_object, &[]);
                        }
                        rpass.set_index_buffer(self.meshes[i].index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        rpass.set_vertex_buffer(0, self.meshes[i].vertex_buffer.slice(..));
                        rpass.draw_indexed(0..self.meshes[i].indicies.len() as u32, 0, 0..1);
                    }
                }

            }
        }
        self.queue.submit(Some(encoder.finish()));
    }

    pub fn present(&mut self) {
        let frame = self.surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        self.draw(frame.texture.create_view(&wgpu::TextureViewDescriptor::default()));
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
            glam::Vec3::new(1.5, -5.0, 3.0),
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
        context.bind_resource_to_pipeline("shader".to_string(), BindingGroupType::Global, [camera_buffer.as_entire_binding()].to_vec());
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
    group_layouts: HashMap<u32,wgpu::BindGroupLayout>,
    bind_groups: HashMap<BindingGroupType,wgpu::BindGroup>
}
impl RenderPipeline {
    pub fn new(context: &Context) -> RenderPipeline {
        //Shader and pipeline
        let module = context.device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: None, 
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("context/shader.wgsl"))) 
        });
        let naga_module = naga::front::wgsl::parse_str(include_str!("context/shader.wgsl")).unwrap();
        let mut group_layouts: HashMap<u32, wgpu::BindGroupLayout> = HashMap::new();
        let mut entries: HashMap<u32, Vec<wgpu::BindGroupLayoutEntry>> = HashMap::new();
        for global_handle in naga_module.global_variables.iter() {
            let handle = &naga_module.global_variables[global_handle.0];
            if let Some(bindings) = &handle.binding {
                let ty = match naga_module.types[handle.ty].inner {
                    naga::TypeInner::Struct { .. } => {
                        wgpu::BindingType::Buffer { 
                            ty: if handle.space == naga::AddressSpace::Uniform {
                                wgpu::BufferBindingType::Uniform
                            } else {
                                wgpu::BufferBindingType::Storage { read_only: true }
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        }
                    },
                    //naga::TypeInner::Image { .. } => quote!(&'a wgpu::TextureView),
                    naga::TypeInner::Image { .. } => {
                        wgpu::BindingType::Texture { 
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        }
                    }
                    //naga::TypeInner::Array { .. } => quote!(wgpu::BufferBinding<'a>),
                    _ => panic!("Unsupported type for binding fields."),
                };
                let entry = wgpu::BindGroupLayoutEntry{
                    binding: bindings.binding,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: ty,
                    count: None
                };
                if let Some(entries) = entries.get_mut(&bindings.group) {
                    entries.push(entry);
                } else {
                    let mut temp_vec: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
                    temp_vec.push(entry);
                    entries.insert(bindings.group,temp_vec);
                }
            }
        }
        for (key, value) in entries {
            let layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { 
                label: None,
                entries: &value,
            });
            group_layouts.insert(key, layout);
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
        let mut layout_ref:Vec<&wgpu::BindGroupLayout> = Vec::new();
        for i in 0..group_layouts.len() as u32 {
            if let Some(layout) = group_layouts.get(&i) {
                layout_ref.push(layout);
            }
        }
        let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &context.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor { 
                        label: None,
                        bind_group_layouts: &layout_ref,
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
    pub fn bind_resource(&mut self, device: &wgpu::Device, group: BindingGroupType, resources: Vec<wgpu::BindingResource>) {
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        for i in 0..resources.len() {
            let entry = wgpu::BindGroupEntry{
                binding: i as u32,
                resource: resources[i].clone(), 
            };
            entries.push(entry);
        }
        let i = group as u32;
        if let Some(layout) = self.group_layouts.get(&i) {
            self.bind_groups.entry(group).or_insert(device.create_bind_group(
                &wgpu::BindGroupDescriptor{
                    label: None,
                    layout: layout,
                    entries: &entries,
                }
            ));
        }
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

impl Material {
    pub fn new(context: &mut Context) -> Material {

        let material_data = GPUMaterialData {color: [1.0,1.0,1.0,1.0]};
        let material_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::bytes_of(&material_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        context.bind_resource_to_pipeline("shader".to_string(), BindingGroupType::PerFrame, [material_buffer.as_entire_binding()].to_vec());
        Material { material_buffer }
    }
    pub fn set_color(&self, context: &Context, color: [f32; 4]) {
        let material_data = GPUMaterialData {color};
        context.queue.write_buffer(&self.material_buffer, 0, bytemuck::bytes_of(&material_data));
    }

}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GPUMesh {
    pub model_mx: [f32; 16]
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

pub struct Transform {
    translation: glam::Vec3,
    rotation: glam::Vec3,
    scale: glam::Vec3,
    values_changed: bool
}

impl Transform {
    pub fn new() -> Transform {
        Transform { translation: glam::Vec3::ZERO, rotation: glam::Vec3::ZERO, scale: glam::Vec3::ONE, values_changed: true }
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
    pub fn get_values_changed(&self) -> bool {
        return self.values_changed;
    }
    pub fn set_values_changed(&mut self, input: bool) {
        self.values_changed = input;
    }
    pub fn generate_transform_matrix(&self) -> glam::Mat4 {
        let mut translation_mx = glam::Mat4::IDENTITY;
        translation_mx.w_axis[0] =  self.translation.x;
        translation_mx.w_axis[1] =  self.translation.y;
        translation_mx.w_axis[2] =  self.translation.z;
        let mut scale_mx = glam::Mat4::IDENTITY;
        scale_mx.x_axis.x = self.scale.x;
        scale_mx.y_axis.y = self.scale.y;
        scale_mx.z_axis.z = self.scale.z;
        return translation_mx * scale_mx;
    }
}

pub struct Mesh {
    id: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indicies: Vec<u16>,
    pub verticies: Vec<Vertex>,
    pub material: Material,
    pub transform: Transform,
}

impl Mesh {
    pub fn new(context: &mut Context, id:String, material: Material) -> Mesh{
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
        let transform = Transform::new();
        // TODO: this shouldn't be here. There should be one large buffer that handles all of the meshes
        // Move vertex, index and mesh buffer for Context to handle
        

        Mesh { id, vertex_buffer, index_buffer, indicies, verticies, material, transform }
    }
    pub fn update_model_mx(&self) -> Option<(String, GPUMesh)>{
        if self.transform.get_values_changed() {
            let model_ref = self.transform.generate_transform_matrix();
            let mesh_data = GPUMesh{model_mx: model_ref.as_ref().clone()};
            return Some(("mesh buffer".to_string(), mesh_data));
        }
        return None;

    }
}

