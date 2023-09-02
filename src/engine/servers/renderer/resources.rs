use std::{borrow::Cow, collections::HashMap};

use wgpu::util::DeviceExt;

pub struct CommandBuffer {
    encoder: wgpu::CommandEncoder,
}

impl CommandBuffer {
    pub fn new_command_buffer(device: &wgpu::Device, id: &str) -> CommandBuffer {
        let encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(id) });
        CommandBuffer { encoder }
    }

    pub fn finish_command_buffer(self, queue: &wgpu::Queue) {
        let buffers = self.encoder.finish();
        queue.submit(Some(buffers));
    }
}

pub struct BufferBuilder {
    id: String,
    size: u64,
    contents: Option<Vec<u8>>,
    usage: wgpu::BufferUsages,
    mapped_at_creation: bool,
}

impl BufferBuilder {
    pub fn new_size(&mut self, size: u64) -> &mut Self {
        self.size = size;
        self
    }
    pub fn new_content(&mut self, contents: &[u8]) -> &mut Self {
        self.contents = Some(contents.to_vec());
        self
    }
    pub fn new_usage(&mut self, usage: wgpu::BufferUsages) -> &mut Self {
        self.usage = usage;
        self
    }
    //this parameter is ignored if we add content at buffer creation
    pub fn new_mapped_at_creation(&mut self, mapped_at_creation: bool) -> &mut Self {
        self.mapped_at_creation = mapped_at_creation;
        self
    }
    pub fn build(&mut self, device: &wgpu::Device) -> Buffer {
        if let Some(contents) = self.contents.clone() {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&self.id),
                contents: contents.as_slice(),
                usage: self.usage,
            });
            Buffer {
                id: self.id.clone(),
                buffer: buffer,
            }
        } else {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&self.id),
                size: self.size,
                usage: self.usage,
                mapped_at_creation: self.mapped_at_creation,
            });
            Buffer {
                id: self.id.clone(),
                buffer: buffer,
            }
        }
    }
}

pub struct Buffer {
    id: String,
    buffer: wgpu::Buffer,
}

impl Buffer {
    pub fn new(id: &str) -> BufferBuilder {
        BufferBuilder {
            id: id.to_string(),
            size: 0,
            usage: wgpu::BufferUsages::empty(),
            contents: None,
            mapped_at_creation: false,
        }
    }

    pub fn get_buffer_id(&self) -> &str {
        self.id.as_str()
    }

    pub fn write(&mut self, queue: &wgpu::Queue, offset: wgpu::BufferAddress, data: &[u8]) {
        queue.write_buffer(&self.buffer, offset, data);
    }

    pub fn get_native_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

pub struct RenderPassBuilder<'a> {
    id: String,
    color_attachment: Option<wgpu::RenderPassColorAttachment<'a>>,
    depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
}

impl<'a> RenderPassBuilder<'a> {
    pub fn new(id: &str) -> RenderPassBuilder {
        RenderPassBuilder {
            id: id.to_string(),
            color_attachment: None,
            depth_stencil_attachment: None,
        }
    }
    pub fn color_attachment(
        &mut self,
        texture_view: &'a wgpu::TextureView,
        color: [f64; 4],
    ) -> &mut Self {
        self.color_attachment = Some(wgpu::RenderPassColorAttachment {
            view: &texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                }),
                store: true,
            },
        });
        self
    }
    pub fn depth_stencil_attachment(&mut self, view: &'a wgpu::TextureView) -> &mut Self {
        self.depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
            view: &view,
            depth_ops: None,
            stencil_ops: None,
        });
        self
    }
    pub fn depth_ops(&mut self, value: f32) -> &mut Self {
        if let Some(attachment) = self.depth_stencil_attachment.as_mut() {
            attachment.depth_ops = Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(value),
                store: false,
            });
        } else {
            println!("depth_stencil_attachment() has not been called before depth_ops() for RenderPass: {}", self.id);
        }
        self
    }
    pub fn build(&mut self, command_buffer: &'a mut CommandBuffer) -> wgpu::RenderPass<'a> {
        let render_pass: wgpu::RenderPass<'a> =
            command_buffer
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(self.id.as_str()),
                    color_attachments: &[self.color_attachment.clone()],
                    depth_stencil_attachment: self.depth_stencil_attachment.clone(),
                });
        render_pass
    }
}
pub struct VertexBufferLayout {
    array_stride: u64,
    step_mode: wgpu::VertexStepMode,
    attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayout {
    pub fn new() -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Vec::new(),
        }
    }
    pub fn new_array_stride(&mut self, array_stride: u64) -> &mut Self {
        self.array_stride = array_stride;
        self
    }

    pub fn new_step_mode(&mut self, step_mode: wgpu::VertexStepMode) -> &mut Self {
        self.step_mode = step_mode;
        self
    }

    pub fn new_attribute(&mut self, attribute: wgpu::VertexAttribute) -> &mut Self {
        self.attributes.push(attribute);
        self
    }
    pub fn build(&self) -> &Self {
        self
    }
}

pub struct RenderPipelineBuilder<'a> {
    shader_source: String,
    group_layout_overwrite: HashMap<u8, wgpu::BindGroupLayout>,
    vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    cull_mode: Option<wgpu::Face>,
    targets: Vec<Option<wgpu::ColorTargetState>>,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn new_shader(&mut self, source: &str) -> &mut Self {
        self.shader_source = source.to_string();
        self
    }

    pub fn new_vertex_buffer(&mut self, buffer: &'a VertexBufferLayout) -> &mut Self {
        self.vertex_buffers.push(wgpu::VertexBufferLayout {
            array_stride: buffer.array_stride,
            step_mode: buffer.step_mode,
            attributes: &buffer.attributes,
        });
        self
    }

    pub fn new_cull_mode(&mut self, cull_mode: wgpu::Face) -> &mut Self {
        self.cull_mode = Some(cull_mode);
        self
    }

    pub fn new_target(&mut self, target: wgpu::ColorTargetState) -> &mut Self {
        self.targets.push(Some(target));
        self
    }

    //overwrites automatically generated group layout by naga
    // pub fn group_layout_overwrite() -> &mut Self {

    // }

    pub fn build(&mut self, id: &str, device: &wgpu::Device) -> RenderPipeline {
        //Shader and pipeline
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&self.shader_source)),
        });
        let naga_module = naga::front::wgsl::parse_str(&self.shader_source).unwrap();
        let mut group_layouts: HashMap<u8, wgpu::BindGroupLayout> = HashMap::new();
        let mut entries: HashMap<u32, Vec<wgpu::BindGroupLayoutEntry>> = HashMap::new();
        for global_handle in naga_module.global_variables.iter() {
            let handle = &naga_module.global_variables[global_handle.0];
            if let Some(bindings) = &handle.binding {
                let ty = match naga_module.types[handle.ty].inner {
                    naga::TypeInner::Struct { .. } => wgpu::BindingType::Buffer {
                        ty: if handle.space == naga::AddressSpace::Uniform {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage { read_only: true }
                        },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    //naga::TypeInner::Image { .. } => quote!(&'a wgpu::TextureView),
                    naga::TypeInner::Image { .. } => wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    //naga::TypeInner::Array { .. } => quote!(wgpu::BufferBinding<'a>),
                    _ => panic!("Unsupported type for binding fields."),
                };
                let entry = wgpu::BindGroupLayoutEntry {
                    binding: bindings.binding,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: ty,
                    count: None,
                };
                if let Some(entries) = entries.get_mut(&bindings.group) {
                    entries.push(entry);
                } else {
                    let mut temp_vec: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
                    temp_vec.push(entry);
                    entries.insert(bindings.group, temp_vec);
                }
            }
        }
        for (key, value) in entries {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &value,
            });
            group_layouts.insert(key as u8, layout);
        }
        let mut layout_ref: Vec<&wgpu::BindGroupLayout> = Vec::new();
        for i in 0..group_layouts.len() as u8 {
            if let Some(layout) = group_layouts.get(&i) {
                layout_ref.push(layout);
            }
        }
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &layout_ref,
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: &naga_module.entry_points[0].name,
                buffers: &self.vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: &naga_module.entry_points[1].name,
                targets: &self.targets,
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: self.cull_mode,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        RenderPipeline {
            id: id.to_string(),
            pipeline: render_pipeline,
            group_layouts: group_layouts,
            bind_groups: HashMap::new(),
        }
    }
}

pub struct RenderPipeline {
    id: String,
    pipeline: wgpu::RenderPipeline,
    group_layouts: HashMap<u8, wgpu::BindGroupLayout>,
    bind_groups: HashMap<u8, wgpu::BindGroup>,
}

impl RenderPipeline {
    pub fn new<'a>() -> RenderPipelineBuilder<'a> {
        RenderPipelineBuilder {
            shader_source: "".to_string(),
            group_layout_overwrite: HashMap::new(),
            vertex_buffers: Vec::new(),
            cull_mode: None,
            targets: Vec::new(),
        }
    }
    pub fn bind_resource(
        &mut self,
        device: &wgpu::Device,
        group: u8,
        resources: &[wgpu::BindingResource],
    ) {
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        for i in 0..resources.len() {
            let entry = wgpu::BindGroupEntry {
                binding: i as u32,
                resource: resources[i].clone(),
            };
            entries.push(entry);
        }
        let i = group;
        if let Some(layout) = self.group_layouts.get(&i) {
            self.bind_groups
                .entry(group)
                .or_insert(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: layout,
                    entries: &entries,
                }));
        }
    }
    pub fn get_id(&self) -> &str {
        self.id.as_str()
    }
    pub fn get_native_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
    pub fn get_bind_groups(&self) -> Vec<&wgpu::BindGroup> {
        let mut bind_groups: Vec<&wgpu::BindGroup> = Vec::new();
        for i in 0..self.bind_groups.len() {
            let idx = i as u8;
            let mut bind_group = self.bind_groups.get(&idx);
            if let Some(group) = bind_group.as_mut() {
                bind_groups.push(group);
            }
        }
        bind_groups
    }
}
