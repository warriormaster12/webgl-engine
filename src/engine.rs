mod renderer;
use renderer::context;
use std::mem;
use winit::window::Window;

const MAX_MESH_COUNT: u64 = 10000;
pub struct Engine {
    context: context::Context,
    meshes: Vec<context::Mesh>,
    resolution: (u32, u32),
}

impl Engine {
    pub async fn new(window: &Window, init: fn()) -> Engine {
        let mut context = context::Context::new(window).await;
        let resolution = (window.inner_size().width, window.inner_size().height);
        // Global buffers
        {
            context.create_buffer(
                "mesh_buffer",
                context.get_storage_aligned_buffer_size(
                    mem::size_of::<context::GPUMesh>() as u64 * MAX_MESH_COUNT,
                ),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                None,
            );
            context.create_buffer(
                "camera_buffer",
                mem::size_of::<[f32; 16]>() as u64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                None,
            );
            context.create_buffer(
                "material_buffer",
                mem::size_of::<context::GPUMaterialData>() as u64 * MAX_MESH_COUNT,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                None,
            );
        }
        init(); // might initialize game items;
        Engine {
            context: context,
            meshes: Vec::new(),
            resolution: resolution,
        }
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        self.resolution = new_size;
        self.context.update_swapchain(self.resolution);
    }

    pub fn update(&mut self, update: fn()) {
        update(); // might become the game loop itself
        self.context.present();
    }
}
