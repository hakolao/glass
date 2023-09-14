use glam::{Vec2, Vec3};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

pub const Z_POS: f32 = -10.0;

#[derive(Copy, Clone)]
pub struct Camera {
    pub pos: Vec2,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
    scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new([1920.0, 1080.0])
    }
}

impl Camera {
    pub fn new(screen_size: [f32; 2]) -> Camera {
        let half_width = screen_size[0] / 2.0;
        let half_height = screen_size[1] / 2.0;
        Camera {
            pos: Vec2::ZERO,
            left: -half_width,
            right: half_width,
            bottom: -half_height,
            top: half_height,
            near: 0.0,
            far: 1000.0,
            scale: 1.0,
        }
    }

    pub fn centered_projection(&self) -> glam::Mat4 {
        OPENGL_TO_WGPU
            * glam::Mat4::orthographic_rh(
                self.left,
                self.right,
                self.bottom,
                self.top,
                self.near,
                self.far,
            )
    }

    #[allow(unused)]
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    #[allow(unused)]
    pub fn translate(&mut self, delta: Vec2) {
        self.pos += delta;
    }

    pub fn update(&mut self, screen_size: &[f32; 2]) {
        let half_width = screen_size[0] / 2.0;
        let half_height = screen_size[1] / 2.0;
        self.left = -half_width;
        self.right = half_width;
        self.bottom = -half_height;
        self.top = half_height;
    }

    pub fn view(&self) -> glam::Mat4 {
        OPENGL_TO_WGPU
    }

    pub fn translation(&self) -> glam::Mat4 {
        glam::Mat4::look_to_rh(
            Vec2::new(self.pos.x, self.pos.y).extend(Z_POS),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }

    pub fn proj(&self) -> glam::Mat4 {
        glam::Mat4::orthographic_rh(
            self.left * self.scale,
            self.right * self.scale,
            self.bottom * self.scale,
            self.top * self.scale,
            self.near,
            self.far,
        )
    }

    pub fn view_proj(&self) -> glam::Mat4 {
        self.view() * self.proj() * self.translation()
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }
}
