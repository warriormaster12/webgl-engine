use bytemuck::{Pod, Zeroable};

use crate::engine::Engine;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
}

pub struct VertexDataBuilder {
    pos: Vec<[f32; 3]>,
    indicies: Vec<u32>,
}

impl VertexDataBuilder {
    pub fn new() -> VertexDataBuilder {
        VertexDataBuilder {
            pos: Vec::new(),
            indicies: Vec::new(),
        }
    }
    pub fn set_vertex_positions(&mut self, pos: &[[f32; 3]]) -> &mut Self {
        self.pos = pos.to_vec();
        self
    }

    pub fn set_indicies(&mut self, indicies: &[u32]) -> &mut Self {
        self.indicies = indicies.to_vec();
        self
    }

    pub fn build(&self, mesh_id: &str, engine: &mut Engine) -> Mesh {
        let mut vertex_data: Vec<Vertex> = Vec::new();
        let mut indicies: Vec<u32> = Vec::new();
        for i in 0..self.pos.len() {
            // vertex position is always used so we will use it as the vertex_count
            vertex_data.push(Vertex { pos: self.pos[i] });
        }
        for i in 0..self.indicies.len() {
            indicies.push(self.indicies[i]);
        }
        if vertex_data.len() > 0 {
            engine.set_vertex_buffer(mesh_id, bytemuck::cast_slice(&vertex_data));
        }
        if indicies.len() > 0 {
            engine.set_index_buffer(mesh_id, bytemuck::cast_slice(&indicies));
        }
        Mesh {
            id: mesh_id.to_string(),
            vertex_data: vertex_data,
            index_data: indicies,
        }
    }
}

pub struct Mesh {
    id: String,
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
}

impl Mesh {
    pub fn new(id: &str) -> Mesh {
        Mesh {
            id: id.to_string(),
            vertex_data: Vec::new(),
            index_data: Vec::new(),
        }
    }
    pub fn get_mesh_id(&self) -> &str {
        self.id.as_str()
    }
    pub fn get_index_count(&self) -> u32 {
        self.index_data.len() as u32
    }
    pub fn get_vertex_data(&self) -> &[Vertex] {
        &self.vertex_data
    }
    pub fn set_vertex_data(&mut self, vertex_data: &[Vertex]) {
        self.vertex_data = vertex_data.to_vec();
    }

    pub fn set_index_data(&mut self, index_data: &[u32]) {
        self.index_data = index_data.to_vec();
    }
}
