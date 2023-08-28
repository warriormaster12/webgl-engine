use wgpu_engine::engine::servers::renderer::resources::{CommandBuffer, RenderPassBuilder};
use wgpu_engine::engine::Engine;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        let eng = pollster::block_on(Engine::new("hello window", (1280, 720)));
        eng.app_loop(
            Box::new(move |engine| {
                let renderer_server = engine.get_renderer_server();
                let (frame, frame_view, depth_view) = renderer_server.get_new_frame();
                let mut main_buffer =
                    CommandBuffer::new_command_buffer(&renderer_server.device, "main_buffer");
                RenderPassBuilder::new("main_pass")
                    .color_attachment(&frame_view, [0.1, 0.1, 0.3, 1.0])
                    .depth_stencil_attachment(depth_view)
                    .depth_ops(1.0)
                    .build(&mut main_buffer);
                main_buffer.finish_command_buffer(&renderer_server.queue);
                frame.present();
            }),
            Box::new(move |_engine, _resolution| {}),
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
