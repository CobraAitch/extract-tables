use std::env;
use std::fs;
use std::process::ExitCode;

use extract_tables::{DetectOptions, RasterizeOptions, Rasterizer, detect_tables};

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let Some(pdf_path) = args.get(1) else {
        eprintln!("usage: extract_pdf <pdf-path> [page-index]");
        return ExitCode::from(2);
    };
    let page_index: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let bytes = match fs::read(pdf_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {pdf_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

        let rasterizer = match Rasterizer::new() {
        Ok(r) => r,
        Err(_e) => {
            eprintln!(
                "failed to initialize pdfium: could not load pdfium library\n\
                 hint: download pdfium.dll from https://github.com/bblanchon/pdfium-binaries/releases\n\
                 and drop it in the project root, target/debug/, or any directory on PATH."
            );
            return ExitCode::FAILURE;
        }
    };

    let image = match rasterizer.rasterize_page(&bytes, page_index, &RasterizeOptions::default()) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("failed to rasterize page {page_index}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let detection = match detect_tables(&image, &DetectOptions::default()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("detection failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    match serde_json::to_string_pretty(&detection) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("failed to serialize detection: {e}");
            ExitCode::FAILURE
        }
    }
}
