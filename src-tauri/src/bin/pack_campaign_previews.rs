use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use ecky_cad_lib::models::PathResolver;

struct PackResolver {
    app_data_root: PathBuf,
}

impl PathResolver for PackResolver {
    fn app_config_dir(&self) -> PathBuf {
        self.app_data_root.join("config")
    }

    fn app_data_dir(&self) -> PathBuf {
        self.app_data_root.clone()
    }

    fn resource_path(&self, _path: &str) -> Option<PathBuf> {
        None
    }
}

fn main() -> ExitCode {
    let mut render_one_missing = false;
    let mut campaign_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/books/ecky-ir/missions");
    let mut runtime_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/campaign-preview-runtime");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--render-one-missing" => render_one_missing = true,
            "--campaign-root" => match args.next() {
                Some(path) => campaign_root = PathBuf::from(path),
                None => return usage("--campaign-root requires a path"),
            },
            "--runtime-root" => match args.next() {
                Some(path) => runtime_root = PathBuf::from(path),
                None => return usage("--runtime-root requires a path"),
            },
            "--help" | "-h" => return usage(""),
            _ => return usage(&format!("unknown argument: {arg}")),
        }
    }
    let resolver = PackResolver {
        app_data_root: runtime_root,
    };
    match ecky_cad_lib::campaign_definition::pack_previews(
        &campaign_root,
        render_one_missing,
        &resolver,
    ) {
        Ok(report) => {
            println!("reused: {}", report.reused_count);
            println!("rendered: {}", report.rendered_count);
            println!("missing: {}", report.missing_count);
            for step_id in report.missing_step_ids {
                println!("missingStep: {step_id}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("pack campaign previews: {}", error.message);
            ExitCode::FAILURE
        }
    }
}

fn usage(error: &str) -> ExitCode {
    if !error.is_empty() {
        eprintln!("{error}");
    }
    eprintln!("Usage: cargo run --bin pack_campaign_previews -- [--render-one-missing] [--campaign-root <path>] [--runtime-root <path>]");
    if error.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
