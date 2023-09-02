pub mod mesh_instance;
pub mod node;

pub trait BaseNode {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get_node_name(&self) -> &str;
    fn update(&mut self, delta: f64);
    fn get_transformation_matrix(&self) -> &glam::Mat4;
    fn add_node(&mut self, node: Box<dyn BaseNode + 'static>);
    fn get_node_mut(&mut self, name: &str) -> Option<&mut Box<dyn BaseNode + 'static>>;
    fn get_children(&self) -> &Vec<Box<dyn BaseNode + 'static>>;
    fn get_children_mut(&mut self) -> &mut Vec<Box<dyn BaseNode + 'static>>;
    fn remove_node(&mut self, name: &str);
}
