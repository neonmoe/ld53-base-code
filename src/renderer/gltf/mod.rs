use crate::renderer::draw_calls::{DrawCall, DrawCalls};
use crate::renderer::gl;
use glam::Mat4;

mod loader;
mod program;

pub use loader::load_gltf;
pub use program::*;

// Rendering plan for gltfs:
// rendering state for each frame HashMap<Material, HashMap<Primitive, Vec<Transform>>>
// which map to opengl state changes as follows:
// - each material corresponds to uniform changes
// - each primitive corresponds to a VAO bind
// - transforms are collected into a vertex buffer, which is indexed per-instance (glVertexAttribDivisor)
// - finally, glDrawElementsInstanced() with the instance count being the length of the transform vec

pub struct Gltf {
    pub scene: usize,
    scenes: Vec<Scene>,
    nodes: Vec<Node>,
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    primitives: Vec<Primitive>,

    gl_vaos: Vec<gl::types::GLuint>,
    gl_buffers: Vec<gl::types::GLuint>,
    gl_textures: Vec<gl::types::GLuint>,
}

pub struct Scene {
    pub node_indices: Vec<usize>,
}

pub struct Node {
    pub mesh_index: Option<usize>,
    pub child_node_indices: Vec<usize>,
    pub transform: Mat4,
}

pub struct Mesh {
    pub primitive_indices: Vec<usize>,
}

pub struct Primitive {
    pub material_index: usize,
    pub draw_call: DrawCall,
}

pub struct Material {}

impl Gltf {
    pub fn draw(&self, draw_calls: &mut DrawCalls, model_transform: Mat4) {
        let scene = &self.scenes[self.scene];
        let mut node_queue = scene
            .node_indices
            .iter()
            .map(|&i| (model_transform, &self.nodes[i]))
            .collect::<Vec<_>>();
        while let Some((parent_transform, node)) = node_queue.pop() {
            let transform = parent_transform * node.transform;
            if let Some(mesh_index) = node.mesh_index {
                for &primitive_index in &self.meshes[mesh_index].primitive_indices {
                    let primitive = &self.primitives[primitive_index];
                    let mut draw_call = primitive.draw_call.clone();
                    // glTF spec section 3.7.4:
                    draw_call.front_face = (transform.determinant() > 0.0)
                        .then_some(gl::CCW)
                        .unwrap_or(gl::CW);
                    draw_calls.add(draw_call, transform);
                }
            }
            for &child_index in &node.child_node_indices {
                node_queue.push((transform, &self.nodes[child_index]));
            }
        }
    }
}

impl Drop for Gltf {
    fn drop(&mut self) {
        gl::call!(gl::DeleteVertexArrays(
            self.gl_vaos.len() as i32,
            self.gl_vaos.as_ptr(),
        ));
        gl::call!(gl::DeleteBuffers(
            self.gl_buffers.len() as i32,
            self.gl_buffers.as_ptr(),
        ));
        gl::call!(gl::DeleteTextures(
            self.gl_textures.len() as i32,
            self.gl_textures.as_ptr(),
        ));
    }
}
