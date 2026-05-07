# extract-tables

Detects table, row, column, and cell bounding boxes in rasterized images of
bordered tables. No machine learning, no OCR just classical computer vision
applied to the line skeleton of each table.

Built for clean, digital tables with visible borderlines. Multiple tables per
image work fine, including tables that visually touch as long as their line
skeletons stay disconnected.

## What it does

Given a PDF page (or any image), it returns:

- The bounding box of every table on the page
- The bounding box of every row and every column
- The bounding box of every cell, including merged cells with `row_span` and
 `col_span`

Output is plain JSON in image-pixel coordinates.

## What it doesn't do

- No OCR. Cell contents aren't extracted, only their geometry. Run the cells
 through Tesseract or whatever fits if you need text.
- No support for borderless tables (whitespace-aligned columns). The pipeline
 needs visible lines.
- No layout analysis beyond the table grid.

## Install

```toml
[dependencies]
extract-tables = "0.1"
```

PDF support is on by default. If you've already got images, drop the pdfium
dependency:

```toml
[dependencies]
extract-tables = { version = "0.1", default-features = false }
```

### pdfium

PDF rasterization needs the [pdfium](https://pdfium.googlesource.com/pdfium/)
shared library at runtime. Grab a prebuilt binary from
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases)
and put it on the library search path:

- **Windows** `pdfium.dll` next to your binary or anywhere on `PATH`
- **Linux** `libpdfium.so` on `LD_LIBRARY_PATH` or in a system lib dir
- **macOS** `libpdfium.dylib` likewise

You can also point at a specific directory with
`Rasterizer::with_library_path("./vendor/pdfium")`.

## Usage

```rust
use extract_tables::{detect_tables, DetectOptions, RasterizeOptions, Rasterizer};

let bytes = std::fs::read("doc.pdf")?;
let rasterizer = Rasterizer::new()?;
let image = rasterizer.rasterize_page(&bytes, 0, &RasterizeOptions::default())?;
let detection = detect_tables(&image, &DetectOptions::default())?;

println!("{}", serde_json::to_string_pretty(&detection)?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

If you've already got an image from somewhere else, skip the rasterizer:

```rust
let image = image::open("page.png")?;
let detection = extract_tables::detect_tables(&image, &Default::default())?;
```

## Output

```json
{
 "image_width": 2000,
 "image_height": 2587,
 "coordinate_space": "image_pixels",
 "tables": [
  {
   "bbox": { "x1": 120, "y1": 340, "x2": 1880, "y2": 1620 },
   "rows": [
    { "x1": 120, "y1": 340, "x2": 1880, "y2": 460 }
   ],
   "cols": [
    { "x1": 120, "y1": 340, "x2": 320, "y2": 1620 }
   ],
   "cells": [
    {
     "row": 0, "col": 0, "row_span": 1, "col_span": 1,
     "bbox": { "x1": 120, "y1": 340, "x2": 320, "y2": 460 }
    }
   ]
  }
 ]
}
```

Coordinates are image pixels with the origin at the top-left. `x2` and `y2`
are exclusive width is `x2 - x1`, height is `y2 - y1`.

## How it works

1. Rasterize the PDF page through pdfium-render.
2. Convert to grayscale and binarize with Otsu.
3. Open the binary image with a horizontal structuring element to keep only
  horizontal lines, then again with a vertical one for vertical lines.
4. OR the two masks together. That's the table skeleton.
5. Run connected-component labeling on the skeleton. Each component is a
  candidate table, which means two tables on the same page stay separate as
  long as their borderlines don't touch.
6. For each component, recover row and column positions via projection
  profiles and build the cell grid.
7. Detect merged cells: for every pair of adjacent grid cells, check whether
  the inner border between them is actually present in the line mask. If
  not, merge via union-find.

Single-digit milliseconds per page on a typical CPU. No GPU, no model files,
deterministic output.

## Tuning

`DetectOptions` exposes a few knobs, all expressed as ratios so they work
across DPIs without manual tuning:

| Field | Default | Notes |
|---|---|---|
| `min_horizontal_line_ratio` | `0.02` | Min horizontal line length, as a fraction of image width |
| `min_vertical_line_ratio` | `0.02` | Min vertical line length, as a fraction of image height |
| `min_table_area_ratio` | `0.001` | Min table bbox area, as a fraction of image area |
| `min_line_coverage_ratio` | `0.5` | Fraction of a row/column that must be filled by line pixels to register as a border |
| `min_grid_lines` | `2` | Minimum number of rows and columns for a candidate to be reported |

Lower the line ratios if you have small tables on a large page; raise them if
short, table-unrelated lines elsewhere on the page are getting picked up as
phantom borders.

## Example

`examples/extract_pdf.rs` rasterizes a page, runs detection, writes the
JSON to disk, and produces a PNG with the bounding boxes overlaid for visual
sanity-checking.

```sh
cargo run --release --example extract_pdf -- doc.pdf 0 ./output
```

The overlay uses red for the outer table bbox, blue for cells, and a faint
gray for row and column strips.

## License

Licensed under either of [MIT](LICENSE) or
[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) at your option.
