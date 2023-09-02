use std::rc::Rc;

use crate::engine::scene::nodes::BaseNode;
use crate::engine::scene::utils::Transform;

pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub children: Vec<Box<dyn BaseNode>>,
    pub parent: Option<Rc<dyn BaseNode>>,
    transform_mx: glam::Mat4,
}

impl Node {
    pub fn new(name: &str) -> Node {
        Node {
            name: name.to_string(),
            transform: Transform::new(),
            children: Vec::new(),
            parent: None,
            transform_mx: glam::Mat4::IDENTITY,
        }
    }
}

impl BaseNode for Node {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn get_node_name(&self) -> &str {
        &self.name
    }
    fn update(&mut self, delta: f64) {
        if self.transform.get_values_changed() {
            self.transform_mx = self.transform.generate_transform_matrix();
            for i in 0..self.children.len() {
                self.children[i].update(delta);
                self.transform_mx = self.children[i]
                    .get_transformation_matrix()
                    .mul_mat4(&self.transform_mx);
            }
            self.transform.set_values_changed(false);
        }
    }
    fn get_transformation_matrix(&self) -> &glam::Mat4 {
        &self.transform_mx
    }
    fn add_node(&mut self, node: Box<dyn BaseNode + 'static>) {
        self.children.push(node);
    }
    fn get_node_mut(&mut self, name: &str) -> Option<&mut Box<dyn BaseNode + 'static>> {
        if let Some(node) = self
            .children
            .iter_mut()
            .find(|node| node.get_node_name() == name)
        {
            return Some(node);
        }
        println!("node: {} not found", name);
        None
    }
    fn get_children(&self) -> &Vec<Box<dyn BaseNode + 'static>> {
        &self.children
    }
    fn get_children_mut(&mut self) -> &mut Vec<Box<dyn BaseNode + 'static>> {
        &mut self.children
    }
    fn remove_node(&mut self, name: &str) {
        if let Some(index) = self
            .children
            .iter()
            .position(|node| node.get_node_name() == name)
        {
            self.children.remove(index);
        } else {
            println!(
                "couldn't find a node: {} or it has been already removed",
                name
            )
        }
    }
}
