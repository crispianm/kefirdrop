//! GPU rendering: the wgpu pipeline, uniform layout and shader assembly.

mod renderer;
mod shader;
pub mod uniforms;

pub use renderer::Renderer;
pub use uniforms::Uniforms;
