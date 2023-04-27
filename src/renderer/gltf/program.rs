use std::mem::size_of;

use crate::renderer::gl;
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};

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
pub const UNIFORM_BLOCK_LIGHTS: u32 = 1;

pub const MAX_LIGHTS: usize = 32;

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct UniformBlockMaterial {
    pub base_color_factor: Vec4,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive_factor: Vec4,
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct UniformBlockLights {
    /// XRGB encoded with the most significant byte X representing the light
    /// type: directional = 1, point = 2, spot = 3, light list terminator = 0.
    pub kind_and_color: [i32; MAX_LIGHTS],
    pub intensity: [f32; MAX_LIGHTS],
    pub light_angle_scale: [f32; MAX_LIGHTS],
    pub light_angle_offset: [f32; MAX_LIGHTS],
    pub position: [Vec3; MAX_LIGHTS],
    pub direction: [Vec3; MAX_LIGHTS],
}

pub struct ShaderProgram {
    pub program: gl::types::GLuint,
    pub proj_from_view_location: gl::types::GLint,
    pub view_from_world_location: gl::types::GLint,
}

/// Compiles and returns the shader program which should be used to render the
/// glTF models.
pub fn create_program() -> ShaderProgram {
    let vertex_shader = gl::create_shader(gl::VERTEX_SHADER, include_str!("gltf_vertex.glsl"));
    let fragment_shader =
        gl::create_shader(gl::FRAGMENT_SHADER, include_str!("gltf_fragment.glsl"));
    let program = gl::create_program(&[vertex_shader, fragment_shader]);
    gl::call!(gl::DeleteShader(vertex_shader));
    gl::call!(gl::DeleteShader(fragment_shader));
    gl::call!(gl::UseProgram(program));
    let proj_from_view_location = gl::get_uniform_location(program, "proj_from_view").unwrap();
    let view_from_world_location = gl::get_uniform_location(program, "view_from_world").unwrap();

    let mut textures = 0;
    let mut uniform_blocks = 0;
    let mut max_ubo_size = 0;
    if let Some(location) = gl::get_uniform_location(program, "base_color_tex") {
        gl::call!(gl::Uniform1i(location, TEX_UNIT_BASE_COLOR as i32));
        textures += 1;
    }
    if let Some(location) = gl::get_uniform_location(program, "metallic_roughness_tex") {
        gl::call!(gl::Uniform1i(location, TEX_UNIT_METALLIC_ROUGHNESS as i32,));
        textures += 1;
    }
    if let Some(location) = gl::get_uniform_location(program, "normal_tex") {
        gl::call!(gl::Uniform1i(location, TEX_UNIT_NORMAL as i32));
        textures += 1;
    }
    if let Some(location) = gl::get_uniform_location(program, "occlusion_tex") {
        gl::call!(gl::Uniform1i(location, TEX_UNIT_OCCLUSION as i32));
        textures += 1;
    }
    if let Some(location) = gl::get_uniform_location(program, "emissive_tex") {
        gl::call!(gl::Uniform1i(location, TEX_UNIT_EMISSIVE as i32));
        textures += 1;
    }
    if let Some(loc) = gl::get_uniform_block_index(program, "Material") {
        let binding = UNIFORM_BLOCK_MATERIAL;
        gl::call!(gl::UniformBlockBinding(program, loc, binding));
        uniform_blocks += 1;
        max_ubo_size = max_ubo_size.max(size_of::<UniformBlockMaterial>() as i32);
    }
    if let Some(loc) = gl::get_uniform_block_index(program, "Lights") {
        let binding = UNIFORM_BLOCK_LIGHTS;
        gl::call!(gl::UniformBlockBinding(program, loc, binding));
        uniform_blocks += 1;
        max_ubo_size = max_ubo_size.max(size_of::<UniformBlockLights>() as i32);
    }

    let assert_limit = |name: &str, pname: gl::types::GLenum, debug_limit: i32, req: i32| {
        let mut driver_max = 0;
        gl::call!(gl::GetIntegerv(pname, &mut driver_max));
        assert!(
            req <= driver_max,
            "the graphics driver's limit for {name} is too low ({driver_max} < {req})",
        );
        debug_assert!(
            req <= debug_limit,
            "the debug limit for {name} is too low ({debug_limit} < {req})",
        );
    };

    let limit = gl::MAX_UNIFORM_BLOCK_SIZE;
    assert_limit("uniform block size", limit, 16_384, max_ubo_size);
    let limit = gl::MAX_UNIFORM_BUFFER_BINDINGS;
    assert_limit("uniform buffer bindings", limit, 24, uniform_blocks);
    let limit = gl::MAX_TEXTURE_IMAGE_UNITS;
    assert_limit("texture image units", limit, 16, textures);

    ShaderProgram {
        program,
        proj_from_view_location,
        view_from_world_location,
    }
}
