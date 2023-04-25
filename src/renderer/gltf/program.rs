use glam::Vec4;

use crate::renderer::gl;

pub const ATTR_LOC_POSITION: gl::types::GLuint = 0;
pub const ATTR_LOC_NORMAL: gl::types::GLuint = 1;
pub const ATTR_LOC_TANGENT: gl::types::GLuint = 2;
pub const ATTR_LOC_TEXCOORD_0: gl::types::GLuint = 3;
pub const ATTR_LOC_TEXCOORD_1: gl::types::GLuint = 4;
pub const ATTR_LOC_COLOR_0: gl::types::GLuint = 5;
pub const ATTR_LOC_MODEL_TRANSFORM_COLUMNS: [gl::types::GLuint; 4] = [6, 7, 8, 9];

pub const TEX_UNIT_BASE_COLOR: u32 = 0;
pub const TEX_UNIT_METALLIC_ROUGHNESS: u32 = 1;
pub const TEX_UNIT_NORMAL: u32 = 2;
pub const TEX_UNIT_OCCLUSION: u32 = 3;
pub const TEX_UNIT_EMISSIVE: u32 = 4;

pub const UNIFORM_BLOCK_MATERIAL: u32 = 0;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct UniformBlockMaterial {
    pub base_color_factor: Vec4,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive_factor: Vec4,
}
unsafe impl bytemuck::Zeroable for UniformBlockMaterial {}
unsafe impl bytemuck::Pod for UniformBlockMaterial {}

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
    let material_ub_location = gl::get_uniform_block_index(program, "Material").unwrap();
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
    gl::call!(gl::UniformBlockBinding(
        program,
        material_ub_location,
        UNIFORM_BLOCK_MATERIAL,
    ));
    gl::call!(gl::DeleteShader(vertex_shader));
    gl::call!(gl::DeleteShader(fragment_shader));
    ShaderProgram {
        program,
        proj_view_matrix_location,
    }
}
