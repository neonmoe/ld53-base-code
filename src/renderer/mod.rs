use glam::{Mat4, Vec3, Vec4};

mod bumpalloc_buffer;
mod draw_calls;
pub mod gl;
pub mod gltf;

pub use draw_calls::DrawCalls;

/// The "up" vector in world-space (which is in glTF's coordinate system, for
/// now).
pub const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
/// The "right" vector in world-space (which is in glTF's coordinate system, for
/// now).
pub const RIGHT: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
/// The "forward" vector in world-space (which is in glTF's coordinate system,
/// for now).
pub const FORWARD: Vec3 = Vec3::new(0.0, 0.0, 1.0);

pub struct Renderer {
    test_model: gltf::Gltf,
    gltf_shader: gltf::ShaderProgram,
    draw_calls: DrawCalls,
}

impl Renderer {
    pub fn new() -> Renderer {
        macro_rules! boom_box_path {
            ($lit:literal) => {
                concat!(
                    "../../resources/models/testing-static/BoomBoxWithAxes",
                    $lit
                )
            };
        }
        macro_rules! boom_box_resource {
            ($lit:literal) => {
                (
                    concat!("BoomBoxWithAxes", $lit),
                    include_bytes!(boom_box_path!($lit)),
                )
            };
        }
        let gltf_shader = gltf::create_program();
        let test_model = gltf::load_gltf(
            include_str!(boom_box_path!(".gltf")),
            &[
                boom_box_resource!(".bin"),
                boom_box_resource!("_baseColor.png"),
                boom_box_resource!("_baseColor1.png"),
                boom_box_resource!("_emissive.png"),
                boom_box_resource!("_normal.png"),
                boom_box_resource!("_roughnessMetallic.png"),
            ],
        );
        let draw_calls = DrawCalls::new();
        Renderer {
            test_model,
            gltf_shader,
            draw_calls,
        }
    }

    pub fn render(&mut self, aspect_ratio: f32) {
        self.draw_calls.clear();
        self.test_model.draw(&mut self.draw_calls, Mat4::IDENTITY);

        fn aces_filmic(x: Vec3) -> Vec3 {
            let a = Vec3::splat(2.51);
            let b = Vec3::splat(0.03);
            let c = Vec3::splat(2.43);
            let d = Vec3::splat(0.59);
            let e = Vec3::splat(0.14);
            (x * (a * x + b) / (x * (c * x + d) + e)).clamp(Vec3::ZERO, Vec3::ONE)
        }
        fn srgb(linear: Vec3) -> Vec3 {
            linear.powf(1.0 / 2.0)
        }
        let ambient = srgb(aces_filmic(Vec3::splat(0.1)));
        gl::call!(gl::ClearColor(ambient.x, ambient.y, ambient.z, 1.0));
        gl::call!(gl::ClearDepthf(0.0));
        gl::call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        gl::call!(gl::Enable(gl::CULL_FACE));
        gl::call!(gl::Enable(gl::DEPTH_TEST));
        gl::call!(gl::DepthFunc(gl::GREATER));

        let view_matrix = Mat4::IDENTITY.to_cols_array();
        // OpenGL clip space: right-handed, +X right, +Y up, +Z backward (out of screen).
        // GLTF:              right-handed, +X left, +Y up, +Z forward (into the screen).
        let to_opengl_basis = Mat4::from_cols(
            (RIGHT, 0.0).into(),    // +X is right in OpenGL clip space
            (UP, 0.0).into(),       // +Y is up in OpenGL clip space
            (-FORWARD, 0.0).into(), // +Z is backward in OpenGL clip space
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );
        let proj_matrix = (Mat4::perspective_rh_gl(74f32.to_radians(), aspect_ratio, 100.0, 0.3)
            * to_opengl_basis)
            .to_cols_array();

        // Draw glTFs:
        gl::call!(gl::UseProgram(self.gltf_shader.program));
        gl::call!(gl::UniformMatrix4fv(
            self.gltf_shader.proj_from_view_location,
            1,
            gl::FALSE,
            proj_matrix.as_ptr(),
        ));
        gl::call!(gl::UniformMatrix4fv(
            self.gltf_shader.view_from_world_location,
            1,
            gl::FALSE,
            view_matrix.as_ptr(),
        ));
        self.draw_calls.draw(gltf::ATTR_LOC_MODEL_TRANSFORM_COLUMNS);
    }
}
