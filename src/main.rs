use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window, dpi::PhysicalSize,
};

mod renderer;
use renderer::context;


async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut context = context::Context::new(&window).await;
    // Create other resources
    context.add_render_pipeline("shader".to_string());
    let material = context::Material::new(&mut context);
    let camera = context::Camera::new(&mut context, 90.0);
    context.create_mesh("cube".to_string(), material);
    let material2 = context::Material::new(&mut context);
    context.create_mesh("cube2".to_string(), material2);
    if let Some(mesh) = context.get_mesh("cube".to_string()) {
        mesh.material.set_color(&context, [1.0, 1.0, 0.0, 1.0]);
    }
    if let Some(mesh) = context.get_mesh_mut("cube2".to_string()) {
        //mesh.material.set_color(&context,  [1.0, 1.0, 1.0, 1.0]);
        mesh.transform.set_translation(glam::Vec3 {x: 1.0, y: 0.0, z: 1.0});
    }
    context.bind_meshes_to_pipeline();
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
