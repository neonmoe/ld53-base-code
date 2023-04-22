// Rendering plan for gltfs:
// rendering state for each frame HashMap<Material, HashMap<Primitive, Vec<Transform>>>
// which map to opengl state changes as follows:
// - each material corresponds to uniform changes
// - each primitive corresponds to a VAO bind
// - transforms are collected into a vertex buffer, which is indexed per-instance (glVertexAttribDivisor)
// - finally, glDrawElementsInstanced() with the instance count being the length of the transform vec

struct Gltf {
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
    buffers: Vec<Buffer>,
}

struct Scene {
    node_indices: Vec<usize>,
}

struct Node {
    mesh_index: Option<usize>,
}

struct Mesh {
    primitive_indices: Vec<usize>,
}

struct Primitive {
    material_index: usize,
}

struct Material {}

struct Buffer {
    buffer_object: u32,
}
