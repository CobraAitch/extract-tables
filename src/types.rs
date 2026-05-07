use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in image pixels.
///
/// Top-left inclusive, bottom-right exclusive: `width = x2 - x1`,
/// `height = y2 - y1`. Origin is the top-left of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BBox {
    /// Left edge, inclusive.
    pub x1: u32,
    /// Top edge, inclusive.
    pub y1: u32,
    /// Right edge, exclusive.
    pub x2: u32,
    /// Bottom edge, exclusive.
    pub y2: u32,
}

impl BBox {
    /// Construct a new bounding box. Caller must ensure `x2 >= x1`, `y2 >= y1`.
    #[must_use]
    pub const fn new(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.x2.saturating_sub(self.x1)
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.y2.saturating_sub(self.y1)
    }

    /// Pixel area.
    #[must_use]
    pub const fn area(&self) -> u64 {
        (self.width() as u64) * (self.height() as u64)
    }

    /// True if width or height is zero.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }
}

/// A cell in a detected table.
///
/// `row_span` and `col_span` are always at least one. Values greater than one
/// mean the cell was merged across multiple grid positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// Row index of the cell's top-left grid position (zero-based).
    pub row: u32,
    /// Column index of the cell's top-left grid position (zero-based).
    pub col: u32,
    /// Number of grid rows the cell spans.
    pub row_span: u32,
    /// Number of grid columns the cell spans.
    pub col_span: u32,
    /// Pixel bounding box.
    pub bbox: BBox,
}

/// A detected table: outer bbox plus its row, column, and cell grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    /// Outer bounding box.
    pub bbox: BBox,
    /// Row strips, top to bottom.
    pub rows: Vec<BBox>,
    /// Column strips, left to right.
    pub cols: Vec<BBox>,
    /// Cells in row-major order.
    pub cells: Vec<Cell>,
}

/// Result of running detection on a single image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detection {
    /// Source image width.
    pub image_width: u32,
    /// Source image height.
    pub image_height: u32,
    /// Always `"image_pixels"`. Kept explicit so JSON consumers don't have
    /// to guess at the unit.
    pub coordinate_space: &'static str,
    /// Detected tables. Order is implementation-defined.
    pub tables: Vec<Table>,
}

/// Knobs for [`crate::detect_tables`]. All ratios are fractions of the image
/// width or height so the defaults work across DPIs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectOptions {
    /// Minimum horizontal line length, as a fraction of image width.
    pub min_horizontal_line_ratio: f32,
    /// Minimum vertical line length, as a fraction of image height.
    pub min_vertical_line_ratio: f32,
    /// Minimum table bbox area, as a fraction of image area. Filters out
    /// spurious tiny components.
    pub min_table_area_ratio: f32,
    /// Fraction of a row's width (or column's height) that must be filled
    /// by line pixels for it to register as a border.
    pub min_line_coverage_ratio: f32,
    /// Tables with fewer rows or columns than this are dropped.
    pub min_grid_lines: u32,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self {
            min_horizontal_line_ratio: 0.02,
            min_vertical_line_ratio: 0.02,
            min_table_area_ratio: 0.001,
            min_line_coverage_ratio: 0.5,
            min_grid_lines: 2,
        }
    }
}
