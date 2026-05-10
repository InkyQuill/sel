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
    let mut sink = cli.make_sink()?;
    for path in &files {
        let app_sink = sink;
        sink = if path.as_os_str() == "-" {
            let app = cli.into_app_for_stdin_with_sink(show_filename, app_sink)?;
            sel::pipeline::run_unfinished(app)?
        } else {
            let app = cli.into_app_for_file_with_sink(path, show_filename, app_sink)?;
            sel::pipeline::run_unfinished(app)?
        };
    }
    sel::pipeline::finish_sink(sink)
}
