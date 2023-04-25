use crate::renderer::bumpalloc_buffer::BumpAllocatedBuffer;
use crate::renderer::draw_calls::{DrawCall, Uniforms};
use crate::renderer::{gl, gltf};
use glam::{Mat4, Quat, Vec3};
use image::imageops::FilterType;
use image::DynamicImage;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use tinyjson::JsonValue;

pub fn load_gltf(gltf: &str, resources: &[(&str, &[u8])]) -> gltf::Gltf {
    let gltf: JsonValue = gltf.parse().unwrap();
    let gltf = gltf.get::<HashMap<_, _>>().unwrap();

    // TODO: Measure how much of the buffers is unused after load (i.e. used by textures and index buffers)
    let buffers_json = gltf["buffers"].get::<Vec<_>>().unwrap();
    let mut gl_buffers = vec![0; buffers_json.len()];
    let mut buffer_slices = Vec::with_capacity(buffers_json.len());
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
        buffer_slices.push(*buffer_data);
    }
    gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
    let get_buffer_slice = |buffer: usize, offset: usize, length: usize| {
        &buffer_slices[buffer][offset..offset + length]
    };

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
    let mut index_buffer_allocator =
        BumpAllocatedBuffer::new(gl::ELEMENT_ARRAY_BUFFER, gl::STATIC_DRAW);
    gl_buffers.push(index_buffer_allocator.get_buffer(true));
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

                let buffer = take_usize(&buffer_view["buffer"]);
                let byte_offset = accessor.get("byteOffset").map(take_usize).unwrap_or(0)
                    + buffer_view.get("byteOffset").map(take_usize).unwrap_or(0);
                let count = take_usize(&accessor["count"]) as gl::types::GLint;
                assert!(
                    !buffer_view.contains_key("byteStride"),
                    "byteStride is not supported for attributes"
                );
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
            let mut disabled_all_ones_vertex_attribute = Some(gltf::ATTR_LOC_COLOR_0);
            let attribute_accessors = primitive_json["attributes"].get::<HashMap<_, _>>().unwrap();
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
                gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, gl_buffers[buffer]));
                gl::call!(gl::VertexAttribPointer(
                    location,
                    size,
                    type_,
                    if normalized { gl::TRUE } else { gl::FALSE },
                    0,
                    ptr::null::<c_void>().add(offset),
                ));
                if location == gltf::ATTR_LOC_COLOR_0 {
                    disabled_all_ones_vertex_attribute = None;
                }
            }

            let indices_accessor = take_usize(&primitive_json["indices"]);
            let (index_buffer, index_byte_offset, index_count, size, index_type, _) =
                unpack_accessor(indices_accessor);
            let index_type_byte_size = match index_type {
                gl::UNSIGNED_BYTE => 1,
                gl::UNSIGNED_SHORT => 2,
                gl::UNSIGNED_INT => 4,
                type_ => panic!("invalid index buffer type {type_}"),
            };
            let index_byte_length = (index_count * size * index_type_byte_size) as usize;
            let index_buffer = get_buffer_slice(index_buffer, index_byte_offset, index_byte_length);
            let (index_buffer, index_byte_offset) =
                index_buffer_allocator.allocate_buffer(index_buffer);

            primitives.push(gltf::Primitive {
                material_index,
                draw_call: DrawCall {
                    mode,
                    vao,
                    index_type,
                    index_buffer,
                    index_byte_offset,
                    index_count,
                    disabled_all_ones_vertex_attribute,
                    front_face: gl::CCW,
                },
            });
            primitive_indices.push(primitive_index);
        }
        meshes.push(gltf::Mesh { primitive_indices });
    }

    let materials_json = gltf["materials"].get::<Vec<_>>().unwrap();
    let textures_json = gltf["textures"].get::<Vec<_>>().unwrap();
    let images_json = gltf["images"].get::<Vec<_>>().unwrap();
    let mut is_srgb = vec![None; images_json.len()];
    for material in materials_json {
        let material = material.get::<HashMap<_, _>>().unwrap();
        let pbr_image = |name: &str| {
            let pbr = material
                .get("pbrMetallicRoughness")?
                .get::<HashMap<_, _>>()
                .unwrap();
            let texture = take_usize(&pbr.get(name)?["index"]);
            Some(take_usize(&textures_json[texture]["source"]))
        };
        let additional_image = |name: &str| {
            let texture = take_usize(&material.get(name)?["index"]);
            Some(take_usize(&textures_json[texture]["source"]))
        };
        let set_srgb_status = |is_srgb: &mut [Option<bool>], index: usize, expected: bool| {
            assert!(
                is_srgb[index] != Some(!expected),
                "images[{}] is used both as srgb and not",
                index,
            );
            is_srgb[index] = Some(expected);
        };
        if let Some(image) = pbr_image("baseColorTexture") {
            set_srgb_status(&mut is_srgb, image, true);
        }
        if let Some(image) = pbr_image("metallicRoughnessTexture") {
            set_srgb_status(&mut is_srgb, image, false);
        }
        if let Some(image) = additional_image("normalTexture") {
            set_srgb_status(&mut is_srgb, image, false);
        }
        if let Some(image) = additional_image("occlusionTexture") {
            set_srgb_status(&mut is_srgb, image, false);
        }
        if let Some(image) = additional_image("emissiveTexture") {
            set_srgb_status(&mut is_srgb, image, true);
        }
    }

    let mut gl_textures = vec![0; images_json.len() + 4];
    gl::call!(gl::GenTextures(
        gl_textures.len() as i32,
        gl_textures.as_mut_ptr()
    ));
    let white_tex = gl_textures[gl_textures.len() - 1];
    let blue_tex = gl_textures[gl_textures.len() - 2];
    let black_tex = gl_textures[gl_textures.len() - 3];
    let gray_tex = gl_textures[gl_textures.len() - 4];
    let make_pixel_tex = |tex: u32, color: [u8; 3]| {
        let target = gl::TEXTURE_2D;
        let ifmt = gl::RGBA as i32;
        let fmt = gl::RGBA;
        let type_ = gl::UNSIGNED_BYTE;
        let pixels = color.as_ptr() as *const c_void;
        gl::call!(gl::BindTexture(target, tex));
        gl::call!(gl::TexImage2D(target, 0, ifmt, 1, 1, 0, fmt, type_, pixels));
    };
    make_pixel_tex(white_tex, [0xFF, 0xFF, 0xFF]);
    make_pixel_tex(blue_tex, [0, 0, 0xFF]);
    make_pixel_tex(black_tex, [0, 0, 0]);
    make_pixel_tex(gray_tex, [0x7F, 0x7F, 0x7F]);
    for (i, image) in images_json.into_iter().enumerate() {
        let Some(is_srgb) = is_srgb[i] else {
            continue; // Not used by any material.
        };

        let image = image.get::<HashMap<_, _>>().unwrap();
        let image_data = if let Some(uri) = image.get("uri") {
            let uri = uri.get::<String>().unwrap().as_str();
            match resources
                .iter()
                .find(|(name, _)| *name == uri)
                .map(|(_, data)| *data)
            {
                Some(data) => data,
                None => panic!("the uri of image {i} ({uri}) is not included in resources"),
            }
        } else {
            let buffer_view = image["bufferView"].get::<HashMap<_, _>>().unwrap();
            let buffer = take_usize(&buffer_view["buffer"]);
            let offset = buffer_view.get("byteOffset").map(take_usize).unwrap_or(0);
            let length = take_usize(&buffer_view["byteLength"]);
            assert!(
                !buffer_view.contains_key("byteStride"),
                "byteStride is not supported for image data"
            );
            get_buffer_slice(buffer, offset, length)
        };

        let mut parsed_image = image::load_from_memory(image_data).unwrap();
        let (format, type_, bpp) = match parsed_image {
            DynamicImage::ImageRgb8(_) => (gl::RGB, gl::UNSIGNED_BYTE, 3),
            DynamicImage::ImageRgba8(_) => (gl::RGBA, gl::UNSIGNED_BYTE, 4),
            DynamicImage::ImageRgb16(_) => (gl::RGB, gl::UNSIGNED_SHORT, 6),
            DynamicImage::ImageRgba16(_) => (gl::RGBA, gl::UNSIGNED_SHORT, 8),
            img => panic!("image {img:?} is of an unsupported format"),
        };
        let internal_format = match (is_srgb, format) {
            (true, gl::RGBA) => gl::SRGB8_ALPHA8,
            (true, gl::RGB) => gl::SRGB8,
            (false, format) => format,
            _ => unreachable!(),
        };
        gl::call!(gl::BindTexture(gl::TEXTURE_2D, gl_textures[i]));
        let size = parsed_image.width().min(parsed_image.height());
        let mip_levels = (size as f32).log2().floor() as i32 + 1;
        for mip_level in 0..mip_levels {
            let (width, height, data) = (
                parsed_image.width() as i32,
                parsed_image.height() as i32,
                parsed_image.as_bytes(),
            );
            assert_eq!(width * height * bpp, data.len() as i32);
            gl::call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                mip_level,
                internal_format as i32,
                width,
                height,
                0,
                format,
                type_,
                data.as_ptr() as *const c_void,
            ));
            if mip_level < mip_levels - 1 {
                parsed_image = parsed_image.resize_exact(
                    width as u32 / 2,
                    height as u32 / 2,
                    if is_srgb {
                        FilterType::CatmullRom
                    } else {
                        FilterType::Triangle
                    },
                );
            }
        }
    }

    // TODO: Make the required uniforms from each material
    // - included fields: materials, textures
    // - would probably be wise to batch up e.g. all baseColorFactors into one UBO, etc.,
    //   then store offsets into that in the materials

    let samplers_json_fallback = Vec::with_capacity(0);
    let samplers_json = gltf
        .get("samplers")
        .map(|v| v.get::<Vec<_>>().unwrap())
        .unwrap_or(&samplers_json_fallback);
    let mut gl_samplers = vec![0; samplers_json.len() + 1];
    gl::call!(gl::GenSamplers(
        gl_samplers.len() as i32,
        gl_samplers.as_mut_ptr()
    ));
    let default_sampler = gl_samplers[gl_samplers.len() - 1];
    gl::call!(gl::SamplerParameteri(
        default_sampler,
        gl::TEXTURE_MAG_FILTER,
        gl::LINEAR as i32,
    ));
    gl::call!(gl::SamplerParameteri(
        default_sampler,
        gl::TEXTURE_MIN_FILTER,
        gl::LINEAR_MIPMAP_LINEAR as i32,
    ));
    gl::call!(gl::SamplerParameteri(
        default_sampler,
        gl::TEXTURE_WRAP_S,
        gl::REPEAT as i32,
    ));
    gl::call!(gl::SamplerParameteri(
        default_sampler,
        gl::TEXTURE_WRAP_T,
        gl::REPEAT as i32,
    ));
    for (i, sampler) in samplers_json.into_iter().enumerate() {
        let sampler = sampler.get::<HashMap<_, _>>().unwrap();
        gl::call!(gl::SamplerParameteri(
            gl_samplers[i],
            gl::TEXTURE_MAG_FILTER,
            sampler
                .get("magFilter")
                .map(take_usize)
                .unwrap_or(gl::LINEAR as usize) as i32,
        ));
        gl::call!(gl::SamplerParameteri(
            gl_samplers[i],
            gl::TEXTURE_MIN_FILTER,
            sampler
                .get("minFilter")
                .map(take_usize)
                .unwrap_or(gl::LINEAR_MIPMAP_LINEAR as usize) as i32,
        ));
        gl::call!(gl::SamplerParameteri(
            gl_samplers[i],
            gl::TEXTURE_WRAP_S,
            sampler.get("wrapS").map(take_usize).unwrap_or(10497) as i32,
        ));
        gl::call!(gl::SamplerParameteri(
            gl_samplers[i],
            gl::TEXTURE_WRAP_T,
            sampler.get("wrapT").map(take_usize).unwrap_or(10497) as i32,
        ));
    }

    let materials_json = gltf["materials"].get::<Vec<_>>().unwrap();
    let mut materials = Vec::with_capacity(materials_json.len());
    for material in materials_json {
        let unpack_texture_info = |texture_info: &JsonValue| {
            let texture_info = texture_info.get::<HashMap<_, _>>().unwrap();
            // TODO: Support TEXCOORD_1
            assert!(matches!(
                texture_info.get("texCoord").map(take_usize),
                None | Some(0)
            ));
            let texture = &textures_json[take_usize(&texture_info["index"])];
            let texture = texture.get::<HashMap<_, _>>().unwrap();
            let sampler = texture
                .get("sampler")
                .map(take_usize)
                .unwrap_or(gl_samplers.len() - 1);
            let source = take_usize(&texture["source"]);
            (gl_textures[source], gl_samplers[sampler])
        };

        let material = material.get::<HashMap<_, _>>().unwrap();
        let mut textures = [None; 5];
        if let Some(pbr) = material.get("pbrMetallicRoughness") {
            let pbr = pbr.get::<HashMap<_, _>>().unwrap();
            if let Some(texture_info) = pbr.get("baseColorTexture") {
                let (texture, sampler) = unpack_texture_info(texture_info);
                textures[0] = Some((gltf::TEX_UNIT_BASE_COLOR, texture, sampler));
            } else {
                textures[0] = Some((gltf::TEX_UNIT_BASE_COLOR, white_tex, default_sampler));
            }
            if let Some(texture_info) = pbr.get("metallicRoughnessTexture") {
                let (texture, sampler) = unpack_texture_info(texture_info);
                textures[1] = Some((gltf::TEX_UNIT_METALLIC_ROUGHNESS, texture, sampler));
            } else {
                textures[1] = Some((gltf::TEX_UNIT_METALLIC_ROUGHNESS, gray_tex, default_sampler));
            }
        }
        if let Some(texture_info) = material.get("normalTexture") {
            // TODO: normal_texture_info.scale (should be included in ubo)
            let (texture, sampler) = unpack_texture_info(texture_info);
            textures[2] = Some((gltf::TEX_UNIT_NORMAL, texture, sampler));
        } else {
            textures[2] = Some((gltf::TEX_UNIT_NORMAL, blue_tex, default_sampler));
        }
        if let Some(texture_info) = material.get("occlusionTexture") {
            // TODO: occlusion_texture_info.strength (should be included in ubo)
            let (texture, sampler) = unpack_texture_info(texture_info);
            textures[3] = Some((gltf::TEX_UNIT_OCCLUSION, texture, sampler));
        } else {
            textures[3] = Some((gltf::TEX_UNIT_OCCLUSION, white_tex, default_sampler));
        }
        if let Some(texture_info) = material.get("emissiveTexture") {
            let (texture, sampler) = unpack_texture_info(texture_info);
            textures[4] = Some((gltf::TEX_UNIT_EMISSIVE, texture, sampler));
        } else {
            textures[4] = Some((gltf::TEX_UNIT_EMISSIVE, black_tex, default_sampler));
        }
        materials.push(gltf::Material {
            uniforms: Uniforms { textures },
        });
    }

    gltf::Gltf {
        scene,
        scenes,
        nodes,
        meshes,
        materials,
        primitives,
        gl_vaos,
        gl_buffers,
        gl_textures,
        gl_samplers,
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
