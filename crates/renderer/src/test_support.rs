//! Shared helpers for the crate's headless tests.

use wgpu::{VertexAttribute, VertexFormat};

/// A vertex attribute as the shader's `@location` sees it, with the offset
/// taken from `offset_of!` on the struct field that feeds it.
pub(crate) fn vertex_attribute(
    shader_location: u32,
    offset: usize,
    format: VertexFormat,
) -> VertexAttribute {
    VertexAttribute { shader_location, offset: offset as u64, format }
}
