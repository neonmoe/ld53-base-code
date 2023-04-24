use crate::renderer::gl;

/// The vertex attribute location of the POSITION attribute of glTF models.
pub const ATTR_LOC_POSITION: gl::types::GLuint = 0;
/// The vertex attribute location of the NORMAL attribute of glTF models.
pub const ATTR_LOC_NORMAL: gl::types::GLuint = 1;
/// The vertex attribute location of the TANGENT attribute of glTF models.
pub const ATTR_LOC_TANGENT: gl::types::GLuint = 2;
/// The vertex attribute location of the TEXCOORD0 attribute of glTF models.
pub const ATTR_LOC_TEXCOORD_0: gl::types::GLuint = 3;
/// The vertex attribute location of the TEXCOORD1 attribute of glTF models.
pub const ATTR_LOC_TEXCOORD_1: gl::types::GLuint = 4;
/// The vertex attribute location of the COLOR0 attribute of glTF models.
pub const ATTR_LOC_COLOR_0: gl::types::GLuint = 5;
/// The vertex attribute locations of the individual columns of the MODEL_TRANSFORM mat4 attribute of glTF models.
pub const ATTR_LOC_MODEL_TRANSFORM_COLUMNS: [gl::types::GLuint; 4] = [6, 7, 8, 9];

const VERTEX_SHADER: &str = r#"#version 300 es
layout(location = 0) in vec3 POSITION;
layout(location = 1) in vec3 NORMAL;
layout(location = 2) in vec4 TANGENT;
layout(location = 3) in vec2 TEXCOORD_0;
layout(location = 4) in vec2 TEXCOORD_1;
layout(location = 5) in vec3 COLOR_0;
layout(location = 6) in mat4 MODEL_TRANSFORM;
out vec3 vertex_color;
uniform mat4 projViewMatrix;
void main() {
    vertex_color = NORMAL;
    gl_Position = projViewMatrix * MODEL_TRANSFORM * vec4(POSITION, 1.0);
}
"#;
const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
out vec4 FRAG_COLOR;
in vec3 vertex_color;
void main() {
    vec3 output_linear_color = vec3(0.5) + vertex_color * 0.3;

    // The framebuffer is not SRGB, so we transform the linear color to close-enough-to-srgb.
    FRAG_COLOR = vec4(pow(output_linear_color, vec3(1.0 / 2.2)), 1.0);
}
"#;

pub struct ShaderProgram {
    pub program: gl::types::GLuint,
    pub proj_view_matrix_location: gl::types::GLint,
}

/// Compiles and returns the shader program which should be used to render the
/// glTF models.
pub fn create_program() -> ShaderProgram {
    let vertex_shader = gl::create_shader(gl::VERTEX_SHADER, VERTEX_SHADER);
    let fragment_shader = gl::create_shader(gl::FRAGMENT_SHADER, FRAGMENT_SHADER);
    let program = gl::create_program(&[vertex_shader, fragment_shader]);
    gl::call!(gl::UseProgram(program));
    let proj_view_matrix_location = gl::get_uniform_location(program, "projViewMatrix").unwrap();
    gl::call!(gl::DeleteShader(vertex_shader));
    gl::call!(gl::DeleteShader(fragment_shader));
    ShaderProgram {
        program,
        proj_view_matrix_location,
    }
}
