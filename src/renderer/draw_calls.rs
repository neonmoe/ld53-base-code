use glam::Mat4;

/// Stores the required information for rendering a set of primitives with
/// various materials, in a form that's optimized for minimum state changes
/// during rendering.
pub struct DrawCalls {}

impl DrawCalls {
    pub fn new() -> DrawCalls {
        DrawCalls {}
    }

    pub fn add(&mut self, transform: Mat4) {}
}
