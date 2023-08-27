use bytemuck::{Pod, Zeroable};
use wgpu_engine::engine::servers::renderer::resources::{
    Buffer, CommandBuffer, RenderPassBuilder, RenderPipeline, VertexBufferLayout,
};
use wgpu_engine::engine::Engine;

use std::{f32::consts, mem};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
    }
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

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        let mut eng = pollster::block_on(Engine::new("hello cube", (1280, 720)));
        let mut triangle_pipeline: Option<RenderPipeline>;
        let vertex_buffer: Option<Buffer>;
        let index_buffer: Option<Buffer>;
        let index_count: u32;
        {
            let renderer_server = eng.get_renderer_server();

            let mx_total = generate_matrix(1280 as f32 / 720 as f32);
            let mx_ref: &[f32; 16] = mx_total.as_ref();
            let uniform_buffer = Buffer::new("uniform buffer")
                .new_content(bytemuck::cast_slice(mx_ref))
                .new_usage(wgpu::BufferUsages::UNIFORM)
                .build(&renderer_server.device);

            let mut pipeline = RenderPipeline::new()
                .new_shader(include_str!("shaders/hello_cube.wgsl"))
                .new_vertex_buffer(
                    &VertexBufferLayout::new()
                        .new_array_stride(mem::size_of::<Vertex>() as u64)
                        .new_step_mode(wgpu::VertexStepMode::Vertex)
                        .new_attribute(wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        })
                        .build(),
                )
                .new_target(renderer_server.get_swapchain().get_format().into())
                .build("triangle pipeline", &renderer_server.device);
            pipeline.bind_resource(
                &renderer_server.device,
                0,
                &[uniform_buffer.get_native_buffer().as_entire_binding()],
            );
            triangle_pipeline = Some(pipeline);

            //create vertex & index buffer
            let (vertex_data, index_data) = create_vertices();
            index_count = index_data.len() as u32;
            vertex_buffer = Some(
                Buffer::new("vertex buffer")
                    .new_content(bytemuck::cast_slice(&vertex_data))
                    .new_usage(wgpu::BufferUsages::VERTEX)
                    .build(&renderer_server.device),
            );
            index_buffer = Some(
                Buffer::new("index buffer")
                    .new_content(bytemuck::cast_slice(&index_data))
                    .new_usage(wgpu::BufferUsages::INDEX)
                    .build(&renderer_server.device),
            );
        }
        eng.app_loop(Box::new(move |engine| {
            let renderer_server = engine.get_renderer_server();
            let (frame, frame_view, _depth_view) = renderer_server.get_new_frame();
            let mut main_buffer =
                CommandBuffer::new_command_buffer(&renderer_server.device, "main_buffer");
            {
                let mut main_pass = RenderPassBuilder::new("main_pass")
                    .color_attachment(&frame_view, [0.1, 0.5, 0.3, 1.0])
                    .build(&mut main_buffer);
                if let Some(pipeline) = triangle_pipeline.as_mut() {
                    main_pass.set_pipeline(pipeline.get_native_pipeline());
                    if let (Some(vertex_buffer), Some(index_buffer)) =
                        (vertex_buffer.as_ref(), index_buffer.as_ref())
                    {
                        let bind_groups = pipeline.get_bind_groups();
                        main_pass.set_bind_group(0, bind_groups[0], &[0]);
                        main_pass.set_vertex_buffer(0, vertex_buffer.get_native_buffer().slice(..));
                        main_pass.set_index_buffer(
                            index_buffer.get_native_buffer().slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        main_pass.draw_indexed(0..index_count, 0, 0..1);
                    }
                }
            }
            main_buffer.finish_command_buffer(&renderer_server.queue);
            frame.present();
        }));
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
