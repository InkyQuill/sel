//! `sel` — Select Slices from Text Files

use clap::Parser;
use sel::cli::Cli;
use std::process;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = cli.validate() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> sel::Result<()> {
    let files = cli.get_files();
    if files.is_empty() {
        return Err(sel::SelError::InvalidSelector(
            "no input files specified".to_string(),
        ));
    }
    let show_filename = cli.with_filename || files.len() > 1;
    for path in &files {
        let app = cli.into_app(path, show_filename)?;
        sel::pipeline::run(app)?;
    }
    Ok(())
}
