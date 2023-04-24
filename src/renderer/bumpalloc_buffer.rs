use std::ffi::c_void;
use std::ptr;

use crate::renderer::gl;

pub struct BumpAllocatedBuffer {
    buffer: gl::types::GLuint,
    offset: usize,
    size: usize,
    data: Vec<u8>,
}

impl BumpAllocatedBuffer {
    pub fn new() -> BumpAllocatedBuffer {
        let mut buffer = 0;
        gl::call!(gl::GenBuffers(1, &mut buffer));
        BumpAllocatedBuffer {
            buffer,
            offset: 0,
            size: 0,
            data: Vec::new(),
        }
    }

    /// Writes the given bytes to a temporary buffer, and returns the buffer and
    /// offset where the data has been written to.
    pub fn allocate_buffer(&mut self, bytes: &[u8]) -> (gl::types::GLuint, *const c_void) {
        if self.offset + bytes.len() >= self.size {
            let additional = bytes.len() + self.size;
            self.size += additional;
            self.data.reserve_exact(additional);
            gl::call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer));
            // Allocate the new space
            gl::call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                self.size as isize,
                ptr::null(),
                gl::DYNAMIC_DRAW
            ));
            // Upload back the existing bytes
            gl::call!(gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                self.data.len() as isize,
                self.data.as_ptr() as *const c_void,
            ));
        }
        let upload_ptr = unsafe { ptr::null::<c_void>().add(self.offset) };
        gl::call!(gl::BufferSubData(
            gl::ARRAY_BUFFER,
            self.offset as isize,
            bytes.len() as isize,
            bytes.as_ptr() as *const c_void,
        ));
        self.data.extend_from_slice(bytes);
        self.offset += bytes.len();
        (self.buffer, upload_ptr)
    }

    pub fn clear(&mut self) {
        self.offset = 0;
        self.data.clear();
    }
}

impl Drop for BumpAllocatedBuffer {
    fn drop(&mut self) {
        gl::call!(gl::DeleteBuffers(1, &self.buffer));
    }
}
