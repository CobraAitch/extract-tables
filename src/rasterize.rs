//! PDF rasterization via pdfium.
//!
//! pdfium-render does not bundle the pdfium binary. Drop `pdfium.dll` /
//! `libpdfium.so` / `libpdfium.dylib` on the library search path, point at
//! it explicitly with [`Rasterizer::with_library_path`], or enable the
//! pdfium-render `static` feature to link it in.

use std::path::Path;

use image::DynamicImage;
use pdfium_render::prelude::{PdfPageRenderRotation, PdfRenderConfig, Pdfium};

use crate::error::{Error, Result};

/// Knobs controlling how a PDF page is rasterized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterizeOptions {
    /// Target width in pixels. Height is derived from the page aspect ratio.
    /// `2000` is roughly 250 DPI for Letter and a sweet spot for clean
    /// digital tables.
    pub target_width: i32,
    /// Rotation applied before rasterization.
    pub rotation: PdfPageRenderRotation,
    /// Render form fields and annotations. Off by default; annotations rarely
    /// help table detection and can introduce noise in the line masks.
    pub render_annotations: bool,
}

impl Default for RasterizeOptions {
    fn default() -> Self {
        Self {
            target_width: 2000,
            rotation: PdfPageRenderRotation::None,
            render_annotations: false,
        }
    }
}

/// Stateful PDF rasterizer. Initializing pdfium is non-trivial; reuse the
/// same `Rasterizer` across many calls.
pub struct Rasterizer {
    pdfium: Pdfium,
}

impl Rasterizer {
    /// Bind to the system pdfium library (or the statically-linked one if
    /// pdfium-render's `static` feature is enabled).
    pub fn new() -> Result<Self> {
        let bindings = Pdfium::bind_to_system_library()?;
        Ok(Self { pdfium: Pdfium::new(bindings) })
    }

    /// Load pdfium from a specific directory.
    pub fn with_library_path(dir: impl AsRef<Path>) -> Result<Self> {
        let path = dir.as_ref().to_string_lossy().into_owned();
        let bindings =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&path))?;
        Ok(Self { pdfium: Pdfium::new(bindings) })
    }

    /// Use a pre-built [`Pdfium`] (e.g. with custom thread-safety wrappers).
    pub const fn from_pdfium(pdfium: Pdfium) -> Self {
        Self { pdfium }
    }

    /// Rasterize a single page from PDF bytes.
    pub fn rasterize_page(
        &self,
        pdf_bytes: &[u8],
        page_index: i32,
        options: &RasterizeOptions,
    ) -> Result<DynamicImage> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, None)?;
        let pages = document.pages();
        let total = pages.len();
        if page_index < 0 || page_index >= total {
            return Err(Error::PageOutOfRange { requested: page_index, total });
        }
        let page = pages.get(page_index)?;

        let config = build_render_config(*options);
        let bitmap = page.render_with_config(&config)?;
        Ok(bitmap.as_image()?)
    }

    /// Rasterize every page. Convenient when you don't know which page the
    /// table is on and want to scan them all.
    pub fn rasterize_all_pages(
        &self,
        pdf_bytes: &[u8],
        options: &RasterizeOptions,
    ) -> Result<Vec<DynamicImage>> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, None)?;
        let pages = document.pages();
        let total = pages.len();
        let mut images = Vec::with_capacity(total.max(0) as usize);
        let config = build_render_config(*options);
        for index in 0..total {
            let page = pages.get(index)?;
            let bitmap = page.render_with_config(&config)?;
            images.push(bitmap.as_image()?);
        }
        Ok(images)
    }
}

fn build_render_config(options: RasterizeOptions) -> PdfRenderConfig {
    PdfRenderConfig::new()
        .set_target_width(options.target_width)
        .rotate(options.rotation, false)
        .render_form_data(options.render_annotations)
        .render_annotations(options.render_annotations)
}
