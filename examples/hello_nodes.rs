use glam::Vec3;
use wgpu_engine::engine::scene::nodes::mesh_instance::mesh::{Vertex, VertexDataBuilder};
use wgpu_engine::engine::scene::nodes::mesh_instance::MeshInstance;
use wgpu_engine::engine::scene::nodes::BaseNode;
use wgpu_engine::engine::servers::renderer::resources::{
    Buffer, RenderPipeline, VertexBufferLayout,
};

use wgpu_engine::engine::scene::nodes::node::Node;
use wgpu_engine::engine::Engine;

use std::{f32::consts, mem};

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
        let mesh = VertexDataBuilder::new()
            .set_vertex_positions(&[
                // top
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
                // bottom
                [-1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, -1.0, -1.0],
                [-1.0, -1.0, -1.0],
                // right
                [1.0, -1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, 1.0, 1.0],
                [1.0, -1.0, 1.0],
                // left
                [-1.0, -1.0, 1.0],
                [-1.0, 1.0, 1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, -1.0, -1.0],
                //front
                [1.0, 1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                //back
                [1.0, -1.0, 1.0],
                [-1.0, -1.0, 1.0],
                [-1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0],
            ])
            .set_indicies(&[
                0, 1, 2, 2, 3, 0, // top
                4, 5, 6, 6, 7, 4, // bottom
                8, 9, 10, 10, 11, 8, // right
                12, 13, 14, 14, 15, 12, // left
                16, 17, 18, 18, 19, 16, // front
                20, 21, 22, 22, 23, 20, // back
            ])
            .build("cube", &mut eng);
        eng.add_root_node(Node::new("scene root"));
        let root = eng.get_root_node_mut::<Node>().unwrap();
        root.add_node(Box::new(Node::new("my_node")));
        root.add_node(Box::new(Node::new("another_node")));
        root.as_any_mut()
            .downcast_mut::<Node>()
            .unwrap()
            .transform
            .set_translation(Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        if let Some(another_node) = root.get_node_mut("another_node") {
            another_node.as_any_mut().downcast_mut::<Node>().unwrap();
            another_node.add_node(Box::new(MeshInstance::new("my_cube_instance")));
            if let Some(my_cube) = another_node.get_node_mut("my_cube_instance") {
                let instance = my_cube.as_any_mut().downcast_mut::<MeshInstance>().unwrap();
                instance.mesh = Some(mesh);
                instance.pipeline_id = "triangle pipeline".to_string();
                instance.transform.set_translation(Vec3 {
                    x: 0.0,
                    y: 2.0,
                    z: 0.0,
                });
            }
        }
        {
            let res: (u32, u32);
            {
                res = eng.get_resolution();
            }
            let mx_total = generate_matrix(res.0 as f32 / res.1 as f32);
            let mx_ref: &[f32; 16] = mx_total.as_ref();
            let uniform_buffer: Buffer;
            {
                let renderer_server = eng.get_renderer_server();
                uniform_buffer = Buffer::new("uniform buffer")
                    .new_content(bytemuck::cast_slice(mx_ref))
                    .new_usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
                    .build(&renderer_server.device);
            }
            eng.set_buffer(uniform_buffer);
            {
                let renderer_server = eng.get_renderer_server();
                eng.set_render_pipeline(
                    "triangle pipeline",
                    RenderPipeline::new()
                        .new_shader(include_str!("shaders/hello_nodes.wgsl"))
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
                        .build("triangle pipeline", &renderer_server.device),
                );
            }

            eng.bind_resources_to_pipeline(
                "triangle pipeline",
                0,
                &["uniform buffer", "mesh_buffer"],
            );
        }
        let mut time: f32 = 0.0;
        eng.app_loop(
            Box::new(move |engine| {
                if let Some(node) = engine.get_root_node_mut::<Node>() {
                    time += 45.0 * 0.01;
                    node.as_any_mut()
                        .downcast_mut::<Node>()
                        .unwrap()
                        .transform
                        .set_rotation(Vec3 {
                            x: 0.0,
                            y: 0.0,
                            z: time,
                        });
                }
            }),
            Box::new(move |engine, resolution| {
                let mx_total = generate_matrix(resolution.0 as f32 / resolution.1 as f32);
                let mx_ref: &[f32; 16] = mx_total.as_ref();
                engine.write_to_buffer("uniform buffer", 0, bytemuck::cast_slice(mx_ref));
            }),
        );
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
