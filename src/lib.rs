//! Detect table, row, column, and cell bounding boxes in rasterized images of
//! bordered tables. Classical CV  no ML, no OCR.
//!
//! Built for clean digital tables with visible borderlines. Multiple tables
//! per image are supported as long as their line skeletons stay disconnected.
//!
//! ```no_run
//! use extract_tables::{detect_tables, DetectOptions};
//!
//! let img = image::open("page.png")?;
//! let detection = detect_tables(&img, &DetectOptions::default())?;
//! println!("{}", serde_json::to_string_pretty(&detection)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! With the `pdfium` feature (on by default), [`Rasterizer`] turns a PDF page
//! into the input image. See [`detect_tables`] for the detection itself.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod detect;
mod error;
mod types;

#[cfg(feature = "pdfium")]
mod rasterize;

pub use detect::detect_tables;
pub use error::{Error, Result};
pub use types::{BBox, Cell, DetectOptions, Detection, Table};

#[cfg(feature = "pdfium")]
pub use rasterize::{Rasterizer, RasterizeOptions};
