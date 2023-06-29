use std::{borrow::Cow, mem, f32::consts};
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window}, dpi::PhysicalSize,
};

use bytemuck::{Pod, Zeroable};


#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TriangleUniform {
    color: [f32; 4]
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
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

fn generate_matrix(aspect_ratio: f32) -> glam::Mat4 {
    let projection = glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 1.0, 10.0);
    let view = glam::Mat4::look_at_rh(
        glam::Vec3::new(1.5f32, -5.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Z,
    );
    projection * view
}


async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
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

    //Swapchain
    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    // Create the vertex and index buffers
    let vertex_size = mem::size_of::<Vertex>();
    let (vertex_data, index_data) = create_vertices();

    let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsages::INDEX,
    });

     // Create other resources
     let mx_total = generate_matrix(config.width as f32 / config.height as f32);
     let mx_ref: &[f32; 16] = mx_total.as_ref();
     let matrix_u_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
         label: Some("Uniform Buffer"),
         contents: bytemuck::cast_slice(mx_ref),
         usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
     });

    //Color uniform buffer
    let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Uniform Buffer Color"),
        size: mem::size_of::<TriangleUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    //Shader and pipeline
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    //Vulkan equivalent to descriptor set layouts
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { 
                    ty: wgpu::BufferBindingType::Uniform, 
                    has_dynamic_offset: false, 
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: vertex_size as wgpu::BufferAddress,
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

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // Vulkan equivalent to descriptor sets
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: matrix_u_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform_buf.as_entire_binding(),
            },
        ],
        label: None,
    });

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &shader, &pipeline_layout);

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                config.width = size.width;
                config.height = size.height;
                surface.configure(&device, &config);
                let mx_total = generate_matrix(config.width as f32 / config.height as f32);
                let mx_ref: &[f32; 16] = mx_total.as_ref();
                queue.write_buffer(&matrix_u_buffer, 0, bytemuck::cast_slice(mx_ref));
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let triangle_data = TriangleUniform {
                    color: [1.0, 0.5, 0.7, 1.0],
                };
                queue.write_buffer(&uniform_buf, 0, bytemuck::bytes_of(&triangle_data));
                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
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
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_bind_group(1, &bind_group, &[]);
                    rpass.set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.set_vertex_buffer(0, vertex_buf.slice(..));
                    rpass.draw_indexed(0..index_data.len() as u32, 0, 0..1);
                }
                queue.submit(Some(encoder.finish()));
                frame.present();
                window.request_redraw(); // with this call inside RedrawRequested event, we can tell the window to basically redraw every frame
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}


fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    window.set_title("wgpu-engine");
    window.set_inner_size(PhysicalSize::new(1280, 720));
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
