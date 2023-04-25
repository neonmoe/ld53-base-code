use crate::renderer::bumpalloc_buffer::BumpAllocatedBuffer;
use crate::renderer::gl;
use glam::{Mat4, Vec4};
use std::collections::HashMap;
use std::ffi::c_void;
use std::{mem, ptr};

#[derive(PartialEq, Eq, Hash)]
struct Uniforms {}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawCall {
    pub vao: gl::types::GLuint,
    pub mode: gl::types::GLenum,
    pub index_buffer: gl::types::GLuint,
    pub index_type: gl::types::GLuint,
    pub index_byte_offset: usize,
    pub index_count: gl::types::GLint,
    /// If vertex colors aren't provided, they should default to 1, 1, 1, 1
    /// instead of the default 0, 0, 0, 1. The problem is, the default value
    /// needs to be provided at draw-time, and can't be saved in the VAO. So
    /// this holds the location of the vertex color attribute, if it's disabled.
    pub disabled_all_ones_vertex_attribute: Option<gl::types::GLuint>,
    pub front_face: gl::types::GLenum,
}

#[derive(Default)]
struct InstanceData {
    transforms: Vec<Mat4>,
    count: gl::types::GLsizei,
}

/// Stores the required information for rendering a set of primitives with
/// various materials, in a form that's optimized for minimum state changes
/// during rendering.
pub struct DrawCalls {
    draws: HashMap<Uniforms, HashMap<DrawCall, InstanceData>>,
    temp_buffer: BumpAllocatedBuffer,
}

impl DrawCalls {
    pub fn new() -> DrawCalls {
        DrawCalls {
            draws: HashMap::new(),
            temp_buffer: BumpAllocatedBuffer::new(gl::ARRAY_BUFFER, gl::STREAM_DRAW),
        }
    }

    pub fn add(&mut self, draw_call: DrawCall, transform: Mat4) {
        let draw = self.draws.entry(Uniforms {}).or_default();
        let mut draw_call = draw.entry(draw_call).or_default();
        draw_call.count += 1;
        draw_call.transforms.push(transform);
    }

    pub fn draw(&mut self, model_transform_attrib_locations: [u32; 4]) {
        for (uniforms, draw_calls) in &self.draws {
            let empty_draw = draw_calls
                .values()
                .all(|instance| instance.transforms.is_empty());
            if empty_draw {
                continue;
            }

            // TODO: Update uniforms
            let _ = uniforms;

            for (draw_call, instance_data) in draw_calls {
                gl::call!(gl::BindVertexArray(draw_call.vao));
                // Setup the transform vertex attribute
                let transforms = bytemuck::cast_slice(&instance_data.transforms);
                let (transforms_buffer, transforms_offset) =
                    self.temp_buffer.allocate_buffer(transforms);
                gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, transforms_buffer));
                for i in 0..4 {
                    let attrib_location = model_transform_attrib_locations[i];
                    let offset = transforms_offset + mem::size_of::<Vec4>() * i;
                    gl::call!(gl::EnableVertexAttribArray(attrib_location));
                    gl::call!(gl::VertexAttribPointer(
                        attrib_location,
                        4,
                        gl::FLOAT,
                        gl::FALSE,
                        mem::size_of::<Mat4>() as i32,
                        ptr::null::<c_void>().add(offset)
                    ));
                    gl::call!(gl::VertexAttribDivisor(attrib_location, 1));
                }
                // Set color vertex attribute default value
                if let Some(location) = draw_call.disabled_all_ones_vertex_attribute {
                    gl::call!(gl::VertexAttrib4f(location, 1.0, 1.0, 1.0, 1.0));
                }
                // Set the front face
                gl::call!(gl::FrontFace(draw_call.front_face));
                // Bind the index buffer
                gl::call!(gl::BindBuffer(
                    gl::ELEMENT_ARRAY_BUFFER,
                    draw_call.index_buffer
                ));
                gl::call!(gl::DrawElementsInstanced(
                    draw_call.mode,
                    draw_call.index_count,
                    draw_call.index_type,
                    ptr::null::<c_void>().add(draw_call.index_byte_offset),
                    instance_data.count
                ));
            }
        }
    }

    pub fn clear(&mut self) {
        for draw_calls in self.draws.values_mut() {
            for instance_data in draw_calls.values_mut() {
                instance_data.transforms.clear();
                instance_data.count = 0;
            }
        }
        self.temp_buffer.clear();
    }
}
