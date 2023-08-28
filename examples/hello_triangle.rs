use wgpu_engine::engine::servers::renderer::resources::{
    CommandBuffer, RenderPassBuilder, RenderPipeline,
};
use wgpu_engine::engine::Engine;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        let mut eng = pollster::block_on(Engine::new("hello triangle", (1280, 720)));
        let triangle_pipeline: RenderPipeline;
        {
            let renderer_server = eng.get_renderer_server();
            triangle_pipeline = RenderPipeline::new()
                .new_shader(include_str!("shaders/hello_triangle.wgsl"))
                .new_target(renderer_server.get_swapchain().get_format().into())
                .build("triangle pipeline", &renderer_server.device);
        }
        eng.app_loop(
            Box::new(move |engine| {
                let renderer_server = engine.get_renderer_server();
                let (frame, frame_view, _depth_view) = renderer_server.get_new_frame();
                let mut main_buffer =
                    CommandBuffer::new_command_buffer(&renderer_server.device, "main_buffer");
                {
                    let mut main_pass = RenderPassBuilder::new("main_pass")
                        .color_attachment(&frame_view, [0.1, 0.5, 0.3, 1.0])
                        .build(&mut main_buffer);
                    main_pass.set_pipeline(triangle_pipeline.get_native_pipeline());
                    main_pass.draw(0..3, 0..1);
                }
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
