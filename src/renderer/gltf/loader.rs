use crate::renderer::draw_calls::DrawCall;
use crate::renderer::{gl, gltf};
use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use tinyjson::JsonValue;

pub fn load_gltf(gltf: &str, resources: &[(&str, &[u8])]) -> gltf::Gltf {
    let gltf: JsonValue = gltf.parse().unwrap();

    let buffers_json = gltf["buffers"].get::<Vec<_>>().unwrap();
    let mut gl_buffers = vec![0; buffers_json.len()];
    gl::call!(gl::GenBuffers(
        gl_buffers.len() as i32,
        gl_buffers.as_mut_ptr()
    ));
    for (i, buffer) in buffers_json.into_iter().enumerate() {
        let gl_buffer = gl_buffers[i];
        let buffer: &HashMap<_, _> = buffer.get().unwrap();
        let buffer_resource_name = if i != 0 || buffer.contains_key("uri") {
            buffer["uri"].get::<String>().unwrap()
        } else {
            "" // The BIN buffer of GLBs
        };
        let mut buffer_data = None;
        for (resource_name, data) in resources {
            if *resource_name == buffer_resource_name {
                buffer_data = Some(data);
            }
        }
        let Some(buffer_data) = buffer_data else {
            panic!("could not find buffer with uri \"{buffer_resource_name}\"");
        };
        let byte_length = take_usize(&buffer["byteLength"]);
        assert_eq!(byte_length, buffer_data.len());
        gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, gl_buffer));
        gl::call!(gl::BufferData(
            gl::ARRAY_BUFFER,
            byte_length as isize,
            buffer_data.as_ptr() as *const c_void,
            gl::STATIC_READ,
        ));
    }
    gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));

    let scenes_json = gltf["scenes"].get::<Vec<_>>().unwrap();
    let mut scenes = Vec::with_capacity(scenes_json.len());
    for scene in scenes_json {
        let node_indices = scene["nodes"].get::<Vec<_>>().unwrap();
        let node_indices = node_indices.iter().map(take_usize).collect::<Vec<_>>();
        scenes.push(gltf::Scene { node_indices });
    }
    let scene = take_usize(&gltf["scene"]);

    let nodes_json = gltf["nodes"].get::<Vec<_>>().unwrap();
    let mut nodes = Vec::with_capacity(nodes_json.len());
    for node in nodes_json {
        let node: &HashMap<_, _> = node.get().unwrap();
        let child_node_indices = if let Some(children) = node.get("children") {
            let children = children.get::<Vec<_>>().unwrap();
            children.iter().map(take_usize).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mesh_index = node.get("mesh").map(take_usize);
        let transform = if let Some(matrix_values) = node.get("matrix") {
            let matrix_values = matrix_values.get::<Vec<_>>().unwrap();
            let mut matrix: [f32; 16] = [0.0; 16];
            assert_eq!(16, matrix_values.len());
            for (i, value) in matrix_values.into_iter().enumerate() {
                matrix[i] = *value.get::<f64>().unwrap() as f32;
            }
            Mat4::from_cols_slice(&matrix)
        } else {
            let translation = node.get("translation").map(take_vec3).unwrap_or(Vec3::ZERO);
            let scale = node.get("scale").map(take_vec3).unwrap_or(Vec3::ONE);
            let rotation = node
                .get("rotation")
                .map(take_quat)
                .unwrap_or(Quat::IDENTITY);
            Mat4::from_scale_rotation_translation(scale, rotation, translation)
        };
        nodes.push(gltf::Node {
            mesh_index,
            child_node_indices,
            transform,
        });
    }

    let accessors_json = gltf["accessors"].get::<Vec<_>>().unwrap();
    let buffer_views_json = gltf["bufferViews"].get::<Vec<_>>().unwrap();
    let meshes_json = gltf["meshes"].get::<Vec<_>>().unwrap();
    let primitive_count = meshes_json
        .iter()
        .flat_map(|mesh| mesh["primitives"].get::<Vec<_>>())
        .count();
    let mut gl_vaos = vec![0; primitive_count];
    gl::call!(gl::GenVertexArrays(
        gl_vaos.len() as i32,
        gl_vaos.as_mut_ptr()
    ));
    let mut primitives = Vec::with_capacity(primitive_count);
    let mut meshes = Vec::with_capacity(meshes_json.len());
    for mesh in meshes_json {
        let primitives_json = mesh["primitives"].get::<Vec<_>>().unwrap();
        let mut primitive_indices = Vec::with_capacity(primitives_json.len());
        for primitive_json in primitives_json {
            let primitive_json = primitive_json.get::<HashMap<_, _>>().unwrap();
            let unpack_accessor = |accessor: usize| {
                let accessor = accessors_json[accessor].get::<HashMap<_, _>>().unwrap();
                let buffer_view = buffer_views_json[take_usize(&accessor["bufferView"])]
                    .get::<HashMap<_, _>>()
                    .unwrap();

                let buffer = gl_buffers[take_usize(&buffer_view["buffer"])];
                let byte_offset = accessor.get("byteOffset").map(take_usize).unwrap_or(0)
                    + buffer_view.get("byteOffset").map(take_usize).unwrap_or(0);
                let count = take_usize(&accessor["count"]) as gl::types::GLint;
                if accessor.contains_key("byteStride") {
                    panic!("this gltf loader does not support byteStride");
                }
                let size = match accessor["type"].get::<String>().unwrap().as_ref() {
                    "SCALAR" => 1,
                    "VEC2" => 2,
                    "VEC3" => 3,
                    "VEC4" => 4,
                    type_ => panic!("unexpected vertex attribute accessor type \"{type_}\""),
                };
                let type_ = take_usize(&accessor["componentType"]) as gl::types::GLuint;
                let normalized = accessor
                    .get("normalized")
                    .map(|v| *v.get::<bool>().unwrap())
                    .unwrap_or(false);

                (buffer, byte_offset, count, size, type_, normalized)
            };

            let primitive_index = primitives.len();
            let material_index = take_usize(&primitive_json["material"]);
            let mode = primitive_json.get("mode").map(take_usize).unwrap_or(4) as gl::types::GLuint;
            let vao = gl_vaos[primitive_index];
            let attribute_accessors = primitive_json["attributes"].get::<HashMap<_, _>>().unwrap();
            let indices_accessor = take_usize(&primitive_json["indices"]);
            let (index_buffer, index_byte_offset, index_count, _, index_type, _) =
                unpack_accessor(indices_accessor);
            gl::call!(gl::BindVertexArray(vao));
            for (attr_name, accessor) in attribute_accessors {
                let accessor = take_usize(accessor);
                let location = match attr_name.as_ref() {
                    "POSITION" => gltf::ATTR_LOC_POSITION,
                    "NORMAL" => gltf::ATTR_LOC_NORMAL,
                    "TANGENT" => gltf::ATTR_LOC_TANGENT,
                    "TEXCOORD_0" => gltf::ATTR_LOC_TEXCOORD_0,
                    "TEXCOORD_1" => gltf::ATTR_LOC_TEXCOORD_1,
                    "COLOR_0" => gltf::ATTR_LOC_COLOR_0,
                    attr => panic!("unsupported attribute semantic \"{attr}\""),
                };
                let (buffer, offset, _, size, type_, normalized) = unpack_accessor(accessor);
                gl::call!(gl::EnableVertexAttribArray(location));
                gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, buffer));
                gl::call!(gl::VertexAttribPointer(
                    location,
                    size,
                    type_,
                    if normalized { gl::TRUE } else { gl::FALSE },
                    0,
                    ptr::null::<c_void>().add(offset),
                ));
            }

            primitives.push(gltf::Primitive {
                material_index,
                draw_call: DrawCall {
                    mode,
                    vao,
                    index_type,
                    index_buffer,
                    index_byte_offset,
                    index_count,
                },
            });
            primitive_indices.push(primitive_index);
        }
        meshes.push(gltf::Mesh { primitive_indices });
    }

    // TODO: Make a texture from each image
    // - included fields: images, bufferviews

    // TODO: Make the required uniforms from each material
    // - included fields: materials, textures
    // - would probably be wise to batch up e.g. all baseColorFactors into one UBO, etc.,
    //   then store offsets into that in the materials

    gltf::Gltf {
        scene,
        scenes,
        nodes,
        meshes,
        materials: vec![],
        primitives,
        gl_vaos,
        gl_buffers,
    }
}

/// Return usize if JsonValue is a number, otherwise panic.
fn take_usize(json_value: &JsonValue) -> usize {
    let i: &f64 = json_value.get().unwrap();
    *i as usize
}

/// Return Vec3 if JsonValue is an array, otherwise None.
fn take_vec3(json_value: &JsonValue) -> Vec3 {
    let values: &Vec<JsonValue> = json_value.get().unwrap();
    assert_eq!(3, values.len());
    let x = *values[0].get::<f64>().unwrap() as f32;
    let y = *values[1].get::<f64>().unwrap() as f32;
    let z = *values[2].get::<f64>().unwrap() as f32;
    Vec3::new(x, y, z)
}

/// Return Quat if JsonValue is an array, otherwise None.
fn take_quat(json_value: &JsonValue) -> Quat {
    let values: &Vec<JsonValue> = json_value.get().unwrap();
    assert_eq!(4, values.len());
    let x = *values[0].get::<f64>().unwrap() as f32;
    let y = *values[1].get::<f64>().unwrap() as f32;
    let z = *values[2].get::<f64>().unwrap() as f32;
    let w = *values[3].get::<f64>().unwrap() as f32;
    Quat::from_xyzw(x, y, z, w)
}
