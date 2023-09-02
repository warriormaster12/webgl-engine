pub struct Transform {
    translation: glam::Vec3,
    rotation: glam::Vec3,
    scale: glam::Vec3,
    values_changed: bool,
}

impl Transform {
    pub fn new() -> Transform {
        Transform {
            translation: glam::Vec3::ZERO,
            rotation: glam::Vec3::ZERO,
            scale: glam::Vec3::ONE,
            values_changed: true,
        }
    }
    pub fn get_translation(&self) -> &glam::Vec3 {
        return &self.translation;
    }
    pub fn set_translation(&mut self, input: glam::Vec3) {
        self.translation = input;
        self.values_changed = true;
    }
    pub fn get_scale(&self) -> &glam::Vec3 {
        return &self.scale;
    }
    pub fn set_scale(&mut self, input: glam::Vec3) {
        self.scale = input;
        self.values_changed = true;
    }
    pub fn get_rotation(&self) -> &glam::Vec3 {
        return &self.rotation;
    }
    pub fn set_rotation(&mut self, input: glam::Vec3) {
        self.rotation = input;
        self.values_changed = true;
    }
    pub fn get_values_changed(&self) -> bool {
        return self.values_changed;
    }
    pub fn set_values_changed(&mut self, input: bool) {
        self.values_changed = input;
    }
    pub fn generate_transform_matrix(&self) -> glam::Mat4 {
        let rot_quat = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x.to_radians(),
            self.rotation.y.to_radians(),
            self.rotation.z.to_radians(),
        );
        return glam::Mat4::from_scale_rotation_translation(self.scale, rot_quat, self.translation);
    }
}
