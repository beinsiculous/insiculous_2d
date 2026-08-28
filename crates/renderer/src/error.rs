//! Error types for the renderer.

use crate::texture::TextureError;

/// Errors that can occur in the renderer
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("Failed to create window: {0}")]
    WindowCreationError(String),

    #[error("Failed to create surface: {0}")]
    SurfaceCreationError(String),

    #[error("Failed to create adapter: {0}")]
    AdapterCreationError(String),

    #[error("Failed to create device: {0}")]
    DeviceCreationError(String),

    #[error("Rendering error: {0}")]
    RenderingError(String),

    #[error("Surface error: {0}")]
    SurfaceError(String),

    /// The GPU device was reported lost (driver reset, browser reclaim).
    /// Fatal: the render path refuses all further queue/surface work — the
    /// loss reason and message are logged at the callback site.
    #[error("Graphics device lost")]
    DeviceLost,

    #[error("Texture error: {0}")]
    TextureError(#[from] TextureError),
}
