use std::ffi::c_void;

use sdl2::video::Window;
use sdl2::VideoSubsystem;

mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

macro_rules! gl_call {
    ($expr:expr) => {{
        let result = unsafe { $expr };
        if cfg!(debug_assertions) {
            let error = unsafe { gl::GetError() };
            if error != gl::NO_ERROR {
                let error_number_stringified;
                let error_name = match error {
                    gl::INVALID_ENUM => "INVALID_ENUM",
                    gl::INVALID_VALUE => "INVALID_VALUE",
                    gl::INVALID_OPERATION => "INVALID_OPERATION",
                    gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
                    gl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
                    _ => {
                        error_number_stringified = format!("{error}");
                        &error_number_stringified
                    }
                };
                panic!(
                    "OpenGL error {error_name} at {}:{}:{}",
                    file!(),
                    line!(),
                    column!(),
                );
            }
        }
        result
    }};
}

const POSITION: u32 = 1;
const VERTEX_SHADER: &str = r#"#version 300 es
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
const FRAGMENT_SHADER: &str = r#"#version 300 es
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
    pub fn new(video: &VideoSubsystem, window: &Window) -> Renderer {
        gl::load_with(|s| video.gl_get_proc_address(s) as *const core::ffi::c_void);
        video.gl_set_swap_interval(1).unwrap();
        let (w, h) = window.drawable_size();
        gl_call!(gl::Viewport(0, 0, w as i32, h as i32));

        let mut vao = 0;
        let mut vbo = 0;
        gl_call!(gl::GenVertexArrays(1, &mut vao));
        gl_call!(gl::GenBuffers(1, &mut vbo));
        gl_call!(gl::BindVertexArray(vao));
        gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
        let data: [f32; 6] = [-0.5, -0.5, 0.5, -0.5, 0.0, 0.5];
        let data: &[u8] = bytemuck::cast_slice(&data);
        gl_call!(gl::BufferData(
            gl::ARRAY_BUFFER,
            data.len() as isize,
            data.as_ptr() as *const c_void,
            gl::STATIC_DRAW,
        ));
        gl_call!(gl::VertexAttribPointer(
            POSITION,
            2,
            gl::FLOAT,
            gl::FALSE,
            0,
            0 as *const c_void
        ));

        let vertex_shader = gl_call!(gl::CreateShader(gl::VERTEX_SHADER));
        let vertex_sources = [VERTEX_SHADER.as_bytes().as_ptr() as *const i8];
        let vertex_source_lens = [VERTEX_SHADER.len() as i32];
        gl_call!(gl::ShaderSource(
            vertex_shader,
            1,
            vertex_sources.as_ptr(),
            vertex_source_lens.as_ptr(),
        ));
        gl_call!(gl::CompileShader(vertex_shader));
        let mut compile_status = 0;
        gl_call!(gl::GetShaderiv(
            vertex_shader,
            gl::COMPILE_STATUS,
            &mut compile_status
        ));
        if compile_status == gl::FALSE as i32 {
            let mut info_log = [0u8; 4096];
            let mut length = 0;
            gl_call!(gl::GetShaderInfoLog(
                vertex_shader,
                4096,
                &mut length,
                info_log.as_mut_ptr() as *mut i8,
            ));
            let info_log = std::str::from_utf8(&info_log[..length as usize]).unwrap();
            panic!("Compiling vertex shader failed: {info_log}");
        }

        let fragment_shader = gl_call!(gl::CreateShader(gl::FRAGMENT_SHADER));
        let fragment_sources = [FRAGMENT_SHADER.as_bytes().as_ptr() as *const i8];
        let fragment_source_lens = [FRAGMENT_SHADER.len() as i32];
        gl_call!(gl::ShaderSource(
            fragment_shader,
            1,
            fragment_sources.as_ptr(),
            fragment_source_lens.as_ptr(),
        ));
        gl_call!(gl::CompileShader(fragment_shader));
        let mut compile_status = 0;
        gl_call!(gl::GetShaderiv(
            fragment_shader,
            gl::COMPILE_STATUS,
            &mut compile_status
        ));
        if compile_status == gl::FALSE as i32 {
            let mut info_log = [0u8; 4096];
            let mut length = 0;
            gl_call!(gl::GetShaderInfoLog(
                fragment_shader,
                4096,
                &mut length,
                info_log.as_mut_ptr() as *mut i8,
            ));
            let info_log = std::str::from_utf8(&info_log[..length as usize]).unwrap();
            panic!("Compiling fragment shader failed: {info_log}");
        }

        let program = gl_call!(gl::CreateProgram());
        gl_call!(gl::AttachShader(program, vertex_shader));
        gl_call!(gl::AttachShader(program, fragment_shader));
        gl_call!(gl::LinkProgram(program));
        let mut link_status = 0;
        gl_call!(gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_status));
        if link_status == gl::FALSE as i32 {
            let mut info_log = [0u8; 4096];
            let mut length = 0;
            gl_call!(gl::GetProgramInfoLog(
                program,
                4096,
                &mut length,
                info_log.as_mut_ptr() as *mut i8,
            ));
            let info_log = std::str::from_utf8(&info_log[..length as usize]).unwrap();
            panic!("Linking shader program failed: {info_log}");
        }

        gl_call!(gl::DeleteShader(vertex_shader));
        gl_call!(gl::DeleteShader(fragment_shader));

        Renderer { vao, vbo, program }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        gl_call!(gl::Viewport(0, 0, width, height));
    }

    pub fn render(&mut self) {
        gl_call!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT));
        gl_call!(gl::UseProgram(self.program));
        gl_call!(gl::BindVertexArray(self.vao));
        gl_call!(gl::EnableVertexAttribArray(POSITION));
        gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 3));
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        gl_call!(gl::DeleteVertexArrays(1, &self.vao));
        gl_call!(gl::DeleteBuffers(1, &self.vbo));
        gl_call!(gl::DeleteProgram(self.program));
    }
}
