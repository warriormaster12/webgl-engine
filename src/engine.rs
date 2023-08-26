pub mod servers;
use servers::renderer;
use servers::renderer::resources::{CommandBuffer, RenderPassBuilder};
use winit::window;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct Engine {
    renderer_server: renderer::RendererServer,
    window: Window,
    event_loop: Option<EventLoop<()>>,
}

impl Engine {
    pub async fn new(name: &str, resolution: (u32, u32)) -> Engine {
        let event_loop = EventLoop::new();
        let window = winit::window::Window::new(&event_loop).unwrap();
        window.set_title(&name);
        window.set_inner_size(PhysicalSize::new(resolution.0, resolution.1));
        let renderer_server = renderer::RendererServer::new(&window).await;
        Engine {
            renderer_server,
            window,
            event_loop: Some(event_loop),
        }
    }

    pub fn get_renderer_server(&mut self) -> &mut renderer::RendererServer {
        &mut self.renderer_server
    }

    pub fn app_loop(mut self, mut update: Box<dyn FnMut(&mut Self)>) {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    // Reconfigure the surface with the new size
                    // On macos the window needs to be redrawn manually after resizing
                    self.resize((size.width, size.height));
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    update(&mut self);
                    self.window.request_redraw(); // with this call inside RedrawRequested event, we can tell the window to basically redraw every frame
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => {}
            }
        });
    }

    fn resize(&mut self, new_size: (u32, u32)) {
        self.renderer_server.update_swapchain(new_size);
    }
}
