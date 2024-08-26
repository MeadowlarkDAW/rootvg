#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[cfg(feature = "text")]
    #[error("glyphon prepare error: {0}")]
    GlyphonPrepareError(#[from] crate::text::glyphon::PrepareError),

    #[cfg(feature = "text")]
    #[error("glyphon render error: {0}")]
    GlyphonRenderError(#[from] crate::text::glyphon::RenderError),

    #[cfg(feature = "custom-primitive")]
    #[error("custom pipeline with ID {0:?} does not exist")]
    InvalidCustomPipelineID(rootvg_core::pipeline::CustomPipelineID),

    #[cfg(feature = "custom-primitive")]
    #[error("custom pipeline prepare error: {0}")]
    CustomPipelinePrepareError(Box<dyn std::error::Error>),

    #[cfg(feature = "custom-primitive")]
    #[error("custom pipeline render error: {0}")]
    CustomPipelineRenderError(Box<dyn std::error::Error>),

    #[error("unkown render error")]
    Unkown,
}
