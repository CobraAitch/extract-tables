use thiserror::Error;

/// Errors returned by this crate.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Input image had zero width or height.
    #[error("input image is empty (width or height is zero)")]
    EmptyImage,

    /// A field on [`crate::DetectOptions`] was outside its valid range.
    #[error("invalid options: {0}")]
    InvalidOptions(&'static str),

    /// I/O failure while loading or rasterizing input.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Wraps any error from the pdfium rasterizer.
    #[cfg(feature = "pdfium")]
    #[error("pdfium error: {0}")]
    Pdfium(#[from] pdfium_render::prelude::PdfiumError),

    /// Page index passed to [`crate::Rasterizer`] doesn't exist in the PDF.
    #[cfg(feature = "pdfium")]
    #[error("page index {requested} out of range (document has {total} page(s))")]
    PageOutOfRange {
        /// Page index that was requested (zero-based).
        requested: i32,
        /// Number of pages in the document.
        total: i32,
    },
}

/// Shorthand for [`std::result::Result`] with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
