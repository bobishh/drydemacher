use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage: migrate_history_payloads <history.sqlite>";

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let _program = args.next();
    let Some(database_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    let database_path = PathBuf::from(database_path);
    match ecky_cad_lib::db::migrate_history_payload_storage(&database_path) {
        Ok(()) => {
            println!(
                "CAD payload migration complete: {}",
                database_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("CAD payload migration failed: {error}");
            ExitCode::from(1)
        }
    }
}
