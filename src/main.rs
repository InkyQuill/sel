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
    let show_filename = cli.with_filename || files.len() > 1;
    for path in &files {
        if path.as_os_str() == "-" {
            sel::pipeline::run(cli.into_app_for_stdin(show_filename)?)?;
        } else {
            sel::pipeline::run(cli.into_app_for_file(path, show_filename)?)?;
        }
    }
    Ok(())
}
