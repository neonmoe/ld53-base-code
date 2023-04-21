use std::ffi::c_void;

pub mod gl;

const POSITION: u32 = 1;
const TEST_TRIANGLE_VERTEX_SHADER: &str = r#"#version 300 es
layout(location = 1) in vec2 POSITION;
out vec3 vertex_color;
void main() {
    if (gl_VertexID == 0) {
        vertex_color = vec3(1.0, 0.0, 0.0);
    } else if (gl_VertexID == 1) {
        vertex_color = vec3(0.0, 1.0, 0.0);
    } else {
        vertex_color = vec3(0.0, 0.0, 1.0);
    }
    gl_Position = vec4(POSITION, 0.0, 1.0);
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
    vao: u32,
    vbo: u32,
    program: u32,
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut vao = 0;
        let mut vbo = 0;
        gl::call!(gl::GenVertexArrays(1, &mut vao));
        gl::call!(gl::GenBuffers(1, &mut vbo));
        gl::call!(gl::BindVertexArray(vao));
        gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
        let data: [f32; 6] = [-0.5, -0.5, 0.5, -0.5, 0.0, 0.5];
        gl::buffer_data_f32(gl::ARRAY_BUFFER, &data, gl::STATIC_DRAW);
        gl::call!(gl::VertexAttribPointer(
            POSITION,
            2,
            gl::FLOAT,
            gl::FALSE,
            0,
            0 as *const c_void
        ));

        let vertex_shader = gl::create_shader(gl::VERTEX_SHADER, TEST_TRIANGLE_VERTEX_SHADER);
        let fragment_shader = gl::create_shader(gl::FRAGMENT_SHADER, TEST_TRIANGLE_FRAGMENT_SHADER);
        let program = gl::create_program(&[vertex_shader, fragment_shader]);

        gl::call!(gl::DeleteShader(vertex_shader));
        gl::call!(gl::DeleteShader(fragment_shader));

        Renderer { vao, vbo, program }
    }

    pub fn render(&mut self) {
        gl::call!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
        gl::call!(gl::Clear(gl::COLOR_BUFFER_BIT));
        gl::call!(gl::UseProgram(self.program));
        gl::call!(gl::BindVertexArray(self.vao));
        gl::call!(gl::EnableVertexAttribArray(POSITION));
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
