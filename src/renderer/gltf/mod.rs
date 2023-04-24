use crate::renderer::draw_calls::DrawCalls;
use crate::renderer::gl;
use glam::Mat4;

mod loader;

pub use loader::load_gltf;

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
    // Pretty sure accessors and buffer views can just be used during
    // initialization to make VAOs which can be stored in the primitives.
    primitives: Vec<Primitive>,
    // Buffers actually only also need tracking usage-wise during
    // initialization, as "which buffer should we bind" is stored in the VAOs,
    // in the primitives. However, some data needs to be saved so we can delete
    // the buffers in Drop. Don't know yet what's the wise move, maybe just a
    // u32 for each buffer object (named like, gl_buffer_objects).
    gl_buffers: Vec<gl::types::GLuint>,
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
}

pub struct Material {}

impl Gltf {
    pub fn collect_draw_calls(&self, draw_calls: &mut DrawCalls) {
        let scene = &self.scenes[self.scene];
        let mut node_queue = scene
            .node_indices
            .iter()
            .map(|&i| (Mat4::IDENTITY, &self.nodes[i]))
            .collect::<Vec<_>>();
        while let Some((parent_transform, node)) = node_queue.pop() {
            let transform = parent_transform * node.transform;
            if let Some(mesh_index) = node.mesh_index {
                for &primitive_index in &self.meshes[mesh_index].primitive_indices {
                    let primitive = &self.primitives[primitive_index];
                    draw_calls.add(transform);
                }
            }
            for &child_index in &node.child_node_indices {
                node_queue.push((transform, &self.nodes[child_index]));
            }
        }
    }
}
