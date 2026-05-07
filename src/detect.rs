//! Classical-CV table detection.
//!
//! Pipeline: Otsu binarize → horizontal/vertical morphological opening to
//! extract the line skeleton → connected components separate touching tables
//! → per-table projection profiles recover row and column positions →
//! union-find on the grid detects merged cells from missing inner borders.

use image::{DynamicImage, GenericImageView, GrayImage, Luma};
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use imageproc::definitions::Image;
use imageproc::region_labelling::{Connectivity, connected_components};

use crate::error::{Error, Result};
use crate::types::{BBox, Cell, DetectOptions, Detection, Table};

/// Detect tables in a rasterized image.
///
/// Tables must be bordered  every cell enclosed by visible lines. Two tables
/// on the same image stay separate as long as their line skeletons remain
/// disconnected; tables that visually touch through a shared borderline will
/// be reported as one.
///
/// Coordinates in the returned [`Detection`] are image pixels.
pub fn detect_tables(image: &DynamicImage, options: &DetectOptions) -> Result<Detection> {
    validate_options(options)?;

    let (image_width, image_height) = image.dimensions();
    if image_width == 0 || image_height == 0 {
        return Err(Error::EmptyImage);
    }

    let gray = image.to_luma8();
    let binary = binarize(&gray);

    let h_kernel = ratio_to_pixels(options.min_horizontal_line_ratio, image_width);
    let v_kernel = ratio_to_pixels(options.min_vertical_line_ratio, image_height);

    let h_mask = horizontal_open(&binary, h_kernel);
    let v_mask = vertical_open(&binary, v_kernel);
    let skeleton = or_masks(&h_mask, &v_mask);

    let labels = connected_components(&skeleton, Connectivity::Eight, Luma([0u8]));
    let candidates = component_bboxes(&labels);

    let min_area = (f64::from(image_width) * f64::from(image_height)
        * f64::from(options.min_table_area_ratio)) as u64;

    let mut tables = Vec::new();
    for bbox in candidates {
        if bbox.area() < min_area {
            continue;
        }
        if let Some(table) = build_table(bbox, &h_mask, &v_mask, options) {
            tables.push(table);
        }
    }

    Ok(Detection {
        image_width,
        image_height,
        coordinate_space: "image_pixels",
        tables,
    })
}

fn validate_options(options: &DetectOptions) -> Result<()> {
    let in_unit = |x: f32| (0.0..=1.0).contains(&x);
    if !in_unit(options.min_horizontal_line_ratio) {
        return Err(Error::InvalidOptions(
            "min_horizontal_line_ratio must be in [0, 1]",
        ));
    }
    if !in_unit(options.min_vertical_line_ratio) {
        return Err(Error::InvalidOptions(
            "min_vertical_line_ratio must be in [0, 1]",
        ));
    }
    if !in_unit(options.min_table_area_ratio) {
        return Err(Error::InvalidOptions(
            "min_table_area_ratio must be in [0, 1]",
        ));
    }
    if !in_unit(options.min_line_coverage_ratio) {
        return Err(Error::InvalidOptions(
            "min_line_coverage_ratio must be in [0, 1]",
        ));
    }
    Ok(())
}

fn ratio_to_pixels(ratio: f32, dimension: u32) -> u32 {
    ((ratio * dimension as f32).round() as u32).max(3)
}

fn binarize(gray: &GrayImage) -> GrayImage {
    let level = otsu_level(gray);
    threshold(gray, level, ThresholdType::BinaryInverted)
}

fn horizontal_erode(img: &GrayImage, length: u32) -> GrayImage {
    let length = length.max(1);
    let half = length / 2;
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    let mut prefix = vec![0u32; w as usize + 1];
    for y in 0..h {
        prefix[0] = 0;
        for x in 0..w {
            let v = img.get_pixel(x, y).0[0];
            prefix[x as usize + 1] = prefix[x as usize] + u32::from(v < 255);
        }
        for x in 0..w {
            let lo = x.saturating_sub(half) as usize;
            let hi = (x + half + 1).min(w) as usize;
            let bg = prefix[hi] - prefix[lo];
            out.put_pixel(x, y, Luma([if bg == 0 { 255 } else { 0 }]));
        }
    }
    out
}

fn horizontal_dilate(img: &GrayImage, length: u32) -> GrayImage {
    let length = length.max(1);
    let half = length / 2;
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    let mut prefix = vec![0u32; w as usize + 1];
    for y in 0..h {
        prefix[0] = 0;
        for x in 0..w {
            let v = img.get_pixel(x, y).0[0];
            prefix[x as usize + 1] = prefix[x as usize] + u32::from(v == 255);
        }
        for x in 0..w {
            let lo = x.saturating_sub(half) as usize;
            let hi = (x + half + 1).min(w) as usize;
            let fg = prefix[hi] - prefix[lo];
            out.put_pixel(x, y, Luma([if fg == 0 { 0 } else { 255 }]));
        }
    }
    out
}

fn horizontal_open(img: &GrayImage, length: u32) -> GrayImage {
    horizontal_dilate(&horizontal_erode(img, length), length)
}

fn vertical_erode(img: &GrayImage, length: u32) -> GrayImage {
    let length = length.max(1);
    let half = length / 2;
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    let mut prefix = vec![0u32; h as usize + 1];
    for x in 0..w {
        prefix[0] = 0;
        for y in 0..h {
            let v = img.get_pixel(x, y).0[0];
            prefix[y as usize + 1] = prefix[y as usize] + u32::from(v < 255);
        }
        for y in 0..h {
            let lo = y.saturating_sub(half) as usize;
            let hi = (y + half + 1).min(h) as usize;
            let bg = prefix[hi] - prefix[lo];
            out.put_pixel(x, y, Luma([if bg == 0 { 255 } else { 0 }]));
        }
    }
    out
}

fn vertical_dilate(img: &GrayImage, length: u32) -> GrayImage {
    let length = length.max(1);
    let half = length / 2;
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    let mut prefix = vec![0u32; h as usize + 1];
    for x in 0..w {
        prefix[0] = 0;
        for y in 0..h {
            let v = img.get_pixel(x, y).0[0];
            prefix[y as usize + 1] = prefix[y as usize] + u32::from(v == 255);
        }
        for y in 0..h {
            let lo = y.saturating_sub(half) as usize;
            let hi = (y + half + 1).min(h) as usize;
            let fg = prefix[hi] - prefix[lo];
            out.put_pixel(x, y, Luma([if fg == 0 { 0 } else { 255 }]));
        }
    }
    out
}

fn vertical_open(img: &GrayImage, length: u32) -> GrayImage {
    vertical_dilate(&vertical_erode(img, length), length)
}

fn or_masks(a: &GrayImage, b: &GrayImage) -> GrayImage {
    debug_assert_eq!(a.dimensions(), b.dimensions());
    let (w, h) = a.dimensions();
    let mut out = GrayImage::new(w, h);
    for (px, (pa, pb)) in out.pixels_mut().zip(a.pixels().zip(b.pixels())) {
        px.0[0] = pa.0[0].max(pb.0[0]);
    }
    out
}

fn component_bboxes(labels: &Image<Luma<u32>>) -> Vec<BBox> {
    let (w, h) = labels.dimensions();
    let mut by_label: std::collections::HashMap<u32, [u32; 4]> = std::collections::HashMap::new();
    for y in 0..h {
        for x in 0..w {
            let label = labels.get_pixel(x, y).0[0];
            if label == 0 {
                continue;
            }
            let entry = by_label.entry(label).or_insert([x, y, x, y]);
            entry[0] = entry[0].min(x);
            entry[1] = entry[1].min(y);
            entry[2] = entry[2].max(x);
            entry[3] = entry[3].max(y);
        }
    }
    let mut keys: Vec<u32> = by_label.keys().copied().collect();
    keys.sort_unstable();
    keys.into_iter()
        .map(|k| {
            let [x1, y1, x2, y2] = by_label[&k];
            BBox::new(x1, y1, x2 + 1, y2 + 1)
        })
        .collect()
}

fn build_table(
    bbox: BBox,
    h_mask: &GrayImage,
    v_mask: &GrayImage,
    options: &DetectOptions,
) -> Option<Table> {
    let row_ys = horizontal_line_centers(h_mask, bbox, options.min_line_coverage_ratio);
    let col_xs = vertical_line_centers(v_mask, bbox, options.min_line_coverage_ratio);

    let n_rows = row_ys.len().checked_sub(1)?;
    let n_cols = col_xs.len().checked_sub(1)?;

    if n_rows < options.min_grid_lines as usize || n_cols < options.min_grid_lines as usize {
        return None;
    }

    let rows: Vec<BBox> = row_ys
        .windows(2)
        .map(|w| BBox::new(bbox.x1, w[0], bbox.x2, w[1]))
        .collect();
    let cols: Vec<BBox> = col_xs
        .windows(2)
        .map(|w| BBox::new(w[0], bbox.y1, w[1], bbox.y2))
        .collect();

    let cells = build_cells(&row_ys, &col_xs, h_mask, v_mask, options.min_line_coverage_ratio);

    Some(Table { bbox, rows, cols, cells })
}

fn horizontal_line_centers(mask: &GrayImage, bbox: BBox, min_coverage: f32) -> Vec<u32> {
    let width = bbox.width();
    if width == 0 {
        return Vec::new();
    }
    let min_count = ((width as f32) * min_coverage) as u32;
    let mut centers = Vec::new();
    let mut run_start: Option<u32> = None;
    for y in bbox.y1..bbox.y2 {
        let mut count = 0u32;
        for x in bbox.x1..bbox.x2 {
            if mask.get_pixel(x, y).0[0] == 255 {
                count += 1;
            }
        }
        let is_line = count >= min_count;
        match (is_line, run_start) {
            (true, None) => run_start = Some(y),
            (false, Some(start)) => {
                centers.push(start.midpoint(y));
                run_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = run_start {
        centers.push(start.midpoint(bbox.y2));
    }
    centers
}

fn vertical_line_centers(mask: &GrayImage, bbox: BBox, min_coverage: f32) -> Vec<u32> {
    let height = bbox.height();
    if height == 0 {
        return Vec::new();
    }
    let min_count = ((height as f32) * min_coverage) as u32;
    let mut centers = Vec::new();
    let mut run_start: Option<u32> = None;
    for x in bbox.x1..bbox.x2 {
        let mut count = 0u32;
        for y in bbox.y1..bbox.y2 {
            if mask.get_pixel(x, y).0[0] == 255 {
                count += 1;
            }
        }
        let is_line = count >= min_count;
        match (is_line, run_start) {
            (true, None) => run_start = Some(x),
            (false, Some(start)) => {
                centers.push(start.midpoint(x));
                run_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = run_start {
        centers.push(start.midpoint(bbox.x2));
    }
    centers
}

fn build_cells(
    row_ys: &[u32],
    col_xs: &[u32],
    h_mask: &GrayImage,
    v_mask: &GrayImage,
    min_coverage: f32,
) -> Vec<Cell> {
    let n_rows = row_ys.len() - 1;
    let n_cols = col_xs.len() - 1;
    let n = n_rows * n_cols;
    let idx = |r: usize, c: usize| r * n_cols + c;

    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let union = |parent: &mut Vec<usize>, a: usize, b: usize| {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    };

    for r in 0..n_rows {
        for c in 0..n_cols.saturating_sub(1) {
            let border_x = col_xs[c + 1];
            let y1 = row_ys[r];
            let y2 = row_ys[r + 1];
            if !line_present_vertical(v_mask, border_x, y1, y2, min_coverage) {
                union(&mut parent, idx(r, c), idx(r, c + 1));
            }
        }
    }

    for r in 0..n_rows.saturating_sub(1) {
        for c in 0..n_cols {
            let border_y = row_ys[r + 1];
            let x1 = col_xs[c];
            let x2 = col_xs[c + 1];
            if !line_present_horizontal(h_mask, border_y, x1, x2, min_coverage) {
                union(&mut parent, idx(r, c), idx(r + 1, c));
            }
        }
    }

    let mut roots: std::collections::HashMap<usize, Vec<(usize, usize)>> =
        std::collections::HashMap::new();
    for r in 0..n_rows {
        for c in 0..n_cols {
            let root = find(&mut parent, idx(r, c));
            roots.entry(root).or_default().push((r, c));
        }
    }

    let mut emitted: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut cells = Vec::with_capacity(n);
    for r in 0..n_rows {
        for c in 0..n_cols {
            let root = find(&mut parent, idx(r, c));
            if !emitted.insert(root) {
                continue;
            }
            let members = &roots[&root];
            let min_r = members.iter().map(|m| m.0).min().unwrap();
            let max_r = members.iter().map(|m| m.0).max().unwrap();
            let min_c = members.iter().map(|m| m.1).min().unwrap();
            let max_c = members.iter().map(|m| m.1).max().unwrap();
            let bbox = BBox::new(
                col_xs[min_c],
                row_ys[min_r],
                col_xs[max_c + 1],
                row_ys[max_r + 1],
            );
            cells.push(Cell {
                row: min_r as u32,
                col: min_c as u32,
                row_span: (max_r - min_r + 1) as u32,
                col_span: (max_c - min_c + 1) as u32,
                bbox,
            });
        }
    }

    cells.sort_by_key(|c| (c.row, c.col));
    cells
}

fn line_present_horizontal(
    mask: &GrayImage,
    y: u32,
    x1: u32,
    x2: u32,
    min_coverage: f32,
) -> bool {
    if x2 <= x1 {
        return false;
    }
    let span = x2 - x1;
    let min_count = ((span as f32) * min_coverage) as u32;
    let mut count = 0u32;
    let (w, h) = mask.dimensions();
    if y >= h {
        return false;
    }
    let x_lo = x1.min(w);
    let x_hi = x2.min(w);
    for x in x_lo..x_hi {
        if mask.get_pixel(x, y).0[0] == 255 {
            count += 1;
        }
    }
    count >= min_count
}

fn line_present_vertical(
    mask: &GrayImage,
    x: u32,
    y1: u32,
    y2: u32,
    min_coverage: f32,
) -> bool {
    if y2 <= y1 {
        return false;
    }
    let span = y2 - y1;
    let min_count = ((span as f32) * min_coverage) as u32;
    let mut count = 0u32;
    let (w, h) = mask.dimensions();
    if x >= w {
        return false;
    }
    let y_lo = y1.min(h);
    let y_hi = y2.min(h);
    for y in y_lo..y_hi {
        if mask.get_pixel(x, y).0[0] == 255 {
            count += 1;
        }
    }
    count >= min_count
}
