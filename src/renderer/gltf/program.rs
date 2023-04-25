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

pub const TEX_UNIT_BASE_COLOR: u32 = 0;
pub const TEX_UNIT_METALLIC_ROUGHNESS: u32 = 1;
pub const TEX_UNIT_NORMAL: u32 = 2;
pub const TEX_UNIT_OCCLUSION: u32 = 3;
pub const TEX_UNIT_EMISSIVE: u32 = 4;

pub struct ShaderProgram {
    pub program: gl::types::GLuint,
    pub proj_view_matrix_location: gl::types::GLint,
}

/// Compiles and returns the shader program which should be used to render the
/// glTF models.
pub fn create_program() -> ShaderProgram {
    let vertex_shader = gl::create_shader(gl::VERTEX_SHADER, include_str!("gltf_vertex.glsl"));
    let fragment_shader =
        gl::create_shader(gl::FRAGMENT_SHADER, include_str!("gltf_fragment.glsl"));
    let program = gl::create_program(&[vertex_shader, fragment_shader]);
    let proj_view_matrix_location = gl::get_uniform_location(program, "proj_view_matrix").unwrap();
    let base_color_tex_location = gl::get_uniform_location(program, "base_color_tex").unwrap();
    let metallic_roughness_tex_location =
        gl::get_uniform_location(program, "metallic_roughness_tex").unwrap();
    let normal_tex_location = gl::get_uniform_location(program, "normal_tex").unwrap();
    let occlusion_tex_location = gl::get_uniform_location(program, "occlusion_tex").unwrap();
    let emissive_tex_location = gl::get_uniform_location(program, "emissive_tex").unwrap();
    gl::call!(gl::UseProgram(program));
    gl::call!(gl::Uniform1i(
        base_color_tex_location,
        TEX_UNIT_BASE_COLOR as i32
    ));
    gl::call!(gl::Uniform1i(
        metallic_roughness_tex_location,
        TEX_UNIT_METALLIC_ROUGHNESS as i32,
    ));
    gl::call!(gl::Uniform1i(normal_tex_location, TEX_UNIT_NORMAL as i32));
    gl::call!(gl::Uniform1i(
        occlusion_tex_location,
        TEX_UNIT_OCCLUSION as i32
    ));
    gl::call!(gl::Uniform1i(
        emissive_tex_location,
        TEX_UNIT_EMISSIVE as i32
    ));
    gl::call!(gl::DeleteShader(vertex_shader));
    gl::call!(gl::DeleteShader(fragment_shader));
    ShaderProgram {
        program,
        proj_view_matrix_location,
    }
}
