use glam::Mat4;
use std::f32::consts::TAU;
use std::ffi::c_void;

mod draw_calls;
pub mod gl;
pub mod gltf;

pub use draw_calls::DrawCalls;

const ATTR_POSITION: u32 = 1;

const TEST_TRIANGLE_VERTEX_SHADER: &str = r#"#version 300 es
layout(location = 1) in vec3 ATTR_POSITION;
out vec3 vertex_color;
uniform mat4 projViewMatrix;
void main() {
    if (ATTR_POSITION.x > 0.0) {
        vertex_color = vec3(1.0, 0.0, 0.0);
    } else if (ATTR_POSITION.y > 0.0) {
        vertex_color = vec3(0.0, 1.0, 0.0);
    } else {
        vertex_color = vec3(0.0, 0.0, 1.0);
    }
    gl_Position = projViewMatrix * vec4(ATTR_POSITION, 1.0);
}
"#;
const TEST_TRIANGLE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
out vec4 COLOR;
in vec3 vertex_color;
void main() {
    // The framebuffer is not SRGB - Firefox at least does not support this.
    COLOR = vec4(pow(vertex_color, vec3(1.0 / 2.2)), 1.0);
}
"#;

pub struct Renderer {
    test_model: gltf::Gltf,
    vao: u32,
    vbo: u32,
    program: u32,
    proj_view_matrix_location: i32,
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut vao = 0;
        let mut vbo = 0;
        gl::call!(gl::GenVertexArrays(1, &mut vao));
        gl::call!(gl::GenBuffers(1, &mut vbo));
        gl::call!(gl::BindVertexArray(vao));
        gl::call!(gl::EnableVertexAttribArray(ATTR_POSITION));
        gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
        let data: [f32; 9] = [0.5, -0.5, 2.0, -0.5, -0.5, 2.0, 0.0, 0.5, 2.0];
        gl::buffer_data_f32(gl::ARRAY_BUFFER, &data, gl::STATIC_DRAW);
        gl::call!(gl::VertexAttribPointer(
            ATTR_POSITION,
            3,
            gl::FLOAT,
            gl::FALSE,
            0,
            0 as *const c_void
        ));

        let vertex_shader = gl::create_shader(gl::VERTEX_SHADER, TEST_TRIANGLE_VERTEX_SHADER);
        let fragment_shader = gl::create_shader(gl::FRAGMENT_SHADER, TEST_TRIANGLE_FRAGMENT_SHADER);
        let program = gl::create_program(&[vertex_shader, fragment_shader]);
        gl::call!(gl::UseProgram(program));
        let proj_view_matrix_location =
            gl::get_uniform_location(program, "projViewMatrix").unwrap();

        gl::call!(gl::DeleteShader(vertex_shader));
        gl::call!(gl::DeleteShader(fragment_shader));

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

        Renderer {
            test_model,
            vao,
            vbo,
            program,
            proj_view_matrix_location,
        }
    }

    pub fn render(&mut self, aspect_ratio: f32) {
        let mut draw_calls = DrawCalls::new();
        self.test_model.collect_draw_calls(&mut draw_calls);

        gl::call!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
        gl::call!(gl::ClearDepthf(0.0));
        gl::call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        gl::call!(gl::Enable(gl::DEPTH_TEST));
        gl::call!(gl::DepthFunc(gl::GREATER));

        // TODO: Draw the draw calls

        gl::call!(gl::UseProgram(self.program));
        let view_from_world = Mat4::IDENTITY;
        // OpenGL clip space: right-handed, +X right, +Y up, +Z backward (out of screen).
        // GLTF:              right-handed, +X left, +Y up, +Z forward (into the screen).
        let to_opengl_basis = Mat4::from_rotation_y(TAU / 2.0);
        let proj_from_view = Mat4::perspective_rh_gl(74f32.to_radians(), aspect_ratio, 100.0, 0.3);
        let proj_view_matrix = (proj_from_view * to_opengl_basis * view_from_world).to_cols_array();
        gl::call!(gl::UniformMatrix4fv(
            self.proj_view_matrix_location,
            1,
            gl::FALSE,
            proj_view_matrix.as_ptr()
        ));
        gl::call!(gl::BindVertexArray(self.vao));
        gl::call!(gl::DrawArrays(gl::TRIANGLES, 0, 3));
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        gl::call!(gl::DeleteVertexArrays(1, &self.vao));
        gl::call!(gl::DeleteBuffers(1, &self.vbo));
        gl::call!(gl::DeleteProgram(self.program));
    }
}
