pub mod scene;
pub mod servers;
use servers::renderer;
use servers::renderer::resources::Buffer;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use scene::nodes::BaseNode;

use std::{collections::VecDeque, mem};

use self::{
    scene::nodes::mesh_instance::{mesh::Mesh, MeshInstance},
    servers::renderer::resources::{CommandBuffer, RenderPassBuilder, RenderPipeline},
};

pub struct Engine {
    renderer_server: renderer::RendererServer,
    window: Window,
    event_loop: Option<EventLoop<()>>,
    root_node: Option<Box<dyn BaseNode>>,
    buffers: Vec<Buffer>,
    render_pipelines: Vec<RenderPipeline>,
}

impl Engine {
    pub async fn new(name: &str, resolution: (u32, u32)) -> Engine {
        let event_loop = EventLoop::new();
        let window = Window::new(&event_loop).unwrap();
        window.set_title(&name);
        window.set_inner_size(PhysicalSize::new(resolution.0, resolution.1));
        let renderer_server = renderer::RendererServer::new(&window).await;

        // global buffers
        let mut buffers: Vec<Buffer> = Vec::new();
        let mesh_buffer = Buffer::new("mesh_buffer")
            .new_size(mem::size_of::<glam::Mat4>() as u64 * 10000)
            .new_usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
            .build(&renderer_server.device);
        buffers.push(mesh_buffer);

        Engine {
            renderer_server,
            window,
            event_loop: Some(event_loop),
            root_node: None,
            buffers: buffers,
            render_pipelines: Vec::new(),
        }
    }

    pub fn get_renderer_server(&self) -> &renderer::RendererServer {
        &self.renderer_server
    }

    pub fn set_vertex_buffer(&mut self, mesh_id: &str, data: &[u8]) {
        if let Some(idx) = self
            .buffers
            .iter()
            .position(|buf| buf.get_buffer_id() == format!("{} vertex_buffer", mesh_id).as_str())
        {
            self.buffers[idx].write(&self.renderer_server.queue, 0, data);
        } else {
            self.buffers.push(
                Buffer::new(format!("{} vertex_buffer", mesh_id).as_str())
                    .new_content(data)
                    .new_usage(wgpu::BufferUsages::VERTEX)
                    .build(&self.renderer_server.device),
            );
        }
    }

    pub fn set_index_buffer(&mut self, mesh_id: &str, data: &[u8]) {
        if let Some(idx) = self
            .buffers
            .iter()
            .position(|buf| buf.get_buffer_id() == format!("{} index_buffer", mesh_id).as_str())
        {
            self.buffers[idx].write(&self.renderer_server.queue, 0, data);
        } else {
            self.buffers.push(
                Buffer::new(format!("{} index_buffer", mesh_id).as_str())
                    .new_content(data)
                    .new_usage(wgpu::BufferUsages::INDEX)
                    .build(&self.renderer_server.device),
            );
        }
    }

    pub fn set_buffer(&mut self, buffer: Buffer) {
        if let Some(index) = self
            .buffers
            .iter()
            .position(|b| b.get_buffer_id() == buffer.get_buffer_id())
        {
            self.buffers.insert(index, buffer);
        } else {
            self.buffers.push(buffer);
        }
    }

    pub fn write_to_buffer(&mut self, id: &str, offset: u64, data: &[u8]) {
        if let Some(buffer) = self.buffers.iter_mut().find(|b| b.get_buffer_id() == id) {
            buffer.write(&self.renderer_server.queue, offset, data);
        } else {
            println!("Couldn't find a buffer with id: {}", id);
        }
    }

    pub fn set_render_pipeline(&mut self, pipeline_id: &str, pipeline: RenderPipeline) {
        if let Some(i) = self
            .render_pipelines
            .iter()
            .position(|p| p.get_id() == pipeline_id)
        {
            self.render_pipelines[i] = pipeline;
        } else {
            self.render_pipelines.push(pipeline);
        }
    }

    pub fn bind_resources_to_pipeline(
        &mut self,
        pipeline_id: &str,
        group: u8,
        resource_ids: &[&str],
    ) {
        if let Some(pipeline) = self
            .render_pipelines
            .iter_mut()
            .find(|p| p.get_id() == pipeline_id.to_string())
        {
            let mut binding_resources: Vec<wgpu::BindingResource> = Vec::new();
            for i in 0..resource_ids.len() {
                if let Some(buffer) = self
                    .buffers
                    .iter()
                    .find(|buf| buf.get_buffer_id() == resource_ids[i])
                {
                    binding_resources.push(buffer.get_native_buffer().as_entire_binding());
                }
            }
            pipeline.bind_resource(&self.renderer_server.device, group, &binding_resources)
        }
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        (
            self.window.inner_size().width,
            self.window.inner_size().height,
        )
    }

    pub fn add_root_node<T: BaseNode + 'static>(&mut self, node: T) {
        self.root_node = Some(Box::new(node));
    }

    pub fn get_root_node_mut<T: BaseNode + 'static>(&mut self) -> Option<&mut T> {
        if let Some(root) = self.root_node.as_mut() {
            let r = root.as_any_mut().downcast_mut::<T>();
            return r;
        }
        None
    }

    pub fn remove_root_node(&mut self) {
        if self.root_node.is_some() {
            self.root_node = None;
        }
    }

    fn draw_a_mesh() {}

    pub fn app_loop(
        mut self,
        mut update: Box<dyn FnMut(&mut Self)>,
        mut resize: Box<dyn FnMut(&mut Self, (u32, u32))>,
    ) {
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
                    resize(&mut self, (size.width, size.height));
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    if let Some(root) = self.root_node.as_mut() {
                        root.update(0.0);
                    }
                    update(&mut self);
                    let (frame, frame_view, _depth_view) = self.renderer_server.get_new_frame();
                    let mut main_buffer = CommandBuffer::new_command_buffer(
                        &self.renderer_server.device,
                        "main_buffer",
                    );
                    if let Some(root) = self.root_node.as_mut() {
                        if let Some(mesh_buffer) = self
                            .buffers
                            .iter_mut()
                            .find(|buf| buf.get_buffer_id() == "mesh_buffer")
                        {
                            let trans_mx = root.get_transformation_matrix().as_ref();
                            mesh_buffer.write(
                                &self.renderer_server.queue,
                                0,
                                bytemuck::cast_slice(trans_mx),
                            );
                        }
                        let mut main_pass = RenderPassBuilder::new("main_pass")
                            .color_attachment(&frame_view, [0.1, 0.5, 0.3, 1.0])
                            .build(&mut main_buffer);
                        let mut stack = VecDeque::new();
                        stack.push_back(root);

                        while let Some(node) = stack.pop_back() {
                            if let Some(mesh_instance) =
                                node.as_any().downcast_ref::<MeshInstance>()
                            {
                                if let Some(mesh) = mesh_instance.mesh.as_ref() {
                                    {
                                        if let Some(pipeline) = self
                                            .render_pipelines
                                            .iter()
                                            .find(|p| p.get_id() == &mesh_instance.pipeline_id)
                                        {
                                            main_pass.set_pipeline(pipeline.get_native_pipeline());
                                            for i in 0..pipeline.get_bind_groups().len() {
                                                main_pass.set_bind_group(
                                                    i as u32,
                                                    pipeline.get_bind_groups()[i],
                                                    &[0, 0],
                                                )
                                            }
                                        } else {
                                            continue;
                                        }
                                        if let Some(vertex_buffer) =
                                            self.buffers.iter().find(|buff| {
                                                buff.get_buffer_id()
                                                    == format!(
                                                        "{} vertex_buffer",
                                                        mesh.get_mesh_id()
                                                    )
                                                    .as_str()
                                            })
                                        {
                                            main_pass.set_vertex_buffer(
                                                0,
                                                vertex_buffer.get_native_buffer().slice(..),
                                            );
                                        } else {
                                            continue;
                                        }
                                        if let Some(index_buffer) =
                                            self.buffers.iter().find(|buff| {
                                                buff.get_buffer_id()
                                                    == format!(
                                                        "{} index_buffer",
                                                        mesh.get_mesh_id()
                                                    )
                                                    .as_str()
                                            })
                                        {
                                            main_pass.set_index_buffer(
                                                index_buffer.get_native_buffer().slice(..),
                                                wgpu::IndexFormat::Uint32,
                                            );
                                        } else {
                                            continue;
                                        }
                                        main_pass.draw_indexed(0..mesh.get_index_count(), 0, 0..1);
                                    }
                                }
                            }
                            for child in node.get_children_mut() {
                                stack.push_back(child);
                            }
                        }
                    }
                    main_buffer.finish_command_buffer(&self.renderer_server.queue);
                    frame.present();
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
