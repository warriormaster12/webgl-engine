use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window, dpi::PhysicalSize,
};

use std::mem;

mod renderer;
use renderer::context;

const MAX_MESH_COUNT:u64 = 10000;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut context = context::Context::new(&window).await;
    // Create other resources
    context.add_render_pipeline("shader");
    context.create_buffer("mesh_buffer", context.get_storage_aligned_buffer_size(mem::size_of::<context::GPUMesh>() as u64 * MAX_MESH_COUNT), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, None);
    context.bind_resource_to_pipeline(
        "shader", 
        context::BindingGroupType::Resource, 
        [context::BindingResource {
            id: "mesh_buffer".to_string(), 
            resource_type: context::BindingResourceType::Buffer, 
            entire_binding: false, 
            offset: 0, 
            size: context::GPUMesh::get_size() }
        ].to_vec()
    );
    let material = context::Material::new(&mut context, "testt");
    let camera = context::Camera::new(&mut context, 90.0);
    context.create_mesh("cube", material);
    let material2 = context::Material::new(&mut context, "test2");
    context.create_mesh("cube2", material2);
    if let Some(mesh) = context.get_mesh("cube") {
        mesh.material.set_color(&context, [1.0, 1.0, 0.0, 1.0]);
    }
    if let Some(mesh) = context.get_mesh_mut("cube2") {
        //mesh.material.set_color(&context,  [1.0, 1.0, 1.0, 1.0]);
        mesh.transform.set_translation(glam::Vec3 {x: 1.0, y: 0.0, z: 1.0});
    }
    let mut rotation = 0.0;
    event_loop.run(move |event, _, control_flow| {

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                context.update_swapchain((size.width, size.height));
                camera.update(&context);
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                rotation += 90.0 * 0.01;
                if let Some(mesh) = context.get_mesh_mut("cube") {
                    mesh.transform.set_rotation(glam::Vec3 { x: rotation, y: rotation, z: rotation })
                }
                if let Some(mesh) = context.get_mesh_mut("cube2") {
                    mesh.transform.set_rotation(glam::Vec3 { x: 0.0, y: rotation, z: 0.0 })
                }
                context.present();
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
