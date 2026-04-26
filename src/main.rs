use dioxus::prelude::*;

mod app;
mod cache;
mod components;
mod duplicate_cleaner;
mod hash_algorithm;
mod icons;
mod quarantine;
mod reporting;
mod scan_mode;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let paths = std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();

    if !paths.is_empty() {
        let result = if paths.len() == 1 {
            duplicate_cleaner::clean_duplicates(paths.into_iter().next().unwrap())
        } else {
            duplicate_cleaner::clean_duplicate_paths(
                paths,
                hash_algorithm::HashAlgorithm::default(),
                scan_mode::ScanMode::default(),
            )
        };

        match result {
            Ok(report) => {
                reporting::print_report(&report);
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }

    use dioxus::desktop::tao::dpi::LogicalSize;
    use dioxus::desktop::{Config, WindowBuilder};

    let size = LogicalSize::new(1200.0, 750.0);

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("hash-killer")
                    .with_inner_size(size)
                    .with_min_inner_size(size)
                    .with_max_inner_size(size)
                    .with_resizable(false)
                    .with_always_on_top(false),
            ),
        )
        .launch(app::App);
}

#[cfg(target_arch = "wasm32")]
fn main() {
    LaunchBuilder::web().launch(app::App);
}
