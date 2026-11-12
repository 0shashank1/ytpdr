//! Binary entry point for `ytpdr`.

use std::process::ExitCode;

use clap::Parser;
use console::style;
use ytpdr::cli::{Cli, run};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{} {error}", style("Error:").red().bold());
            ExitCode::FAILURE
        }
    }
}
