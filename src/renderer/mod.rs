use glam::{Mat4, Quat, Vec3};
use std::f32::consts::TAU;

mod bumpalloc_buffer;
mod draw_calls;
pub mod gl;
pub mod gltf;

pub use draw_calls::DrawCalls;

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
        let gltf_shader = gltf::create_program();
        let draw_calls = DrawCalls::new();
        Renderer {
            test_model,
            gltf_shader,
            draw_calls,
        }
    }

    pub fn render(&mut self, aspect_ratio: f32) {
        self.draw_calls.clear();
        self.test_model.draw(
            &mut self.draw_calls,
            Mat4::from_scale_rotation_translation(
                Vec3::splat(100.0), // Apparently the model is just tiny, in Blender too
                Quat::IDENTITY,
                Vec3::new(0.0, 0.0, 5.0),
            ),
        );

        gl::call!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
        gl::call!(gl::ClearDepthf(0.0));
        gl::call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        gl::call!(gl::Enable(gl::DEPTH_TEST));
        gl::call!(gl::DepthFunc(gl::GREATER));

        let view_from_world = Mat4::IDENTITY;
        // OpenGL clip space: right-handed, +X right, +Y up, +Z backward (out of screen).
        // GLTF:              right-handed, +X left, +Y up, +Z forward (into the screen).
        let to_opengl_basis = Mat4::from_rotation_y(TAU / 2.0);
        let proj_from_view = Mat4::perspective_rh_gl(74f32.to_radians(), aspect_ratio, 100.0, 0.3);
        let proj_view_matrix = (proj_from_view * to_opengl_basis * view_from_world).to_cols_array();

        // Draw glTFs:
        gl::call!(gl::UseProgram(self.gltf_shader.program));
        gl::call!(gl::UniformMatrix4fv(
            self.gltf_shader.proj_view_matrix_location,
            1,
            gl::FALSE,
            proj_view_matrix.as_ptr()
        ));
        self.draw_calls.draw(gltf::ATTR_LOC_MODEL_TRANSFORM_COLUMNS);
    }
}
