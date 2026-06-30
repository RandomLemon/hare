mod cli;
mod hardware;
mod tui;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        Some(command) => cli::run(command),
        None => tui::run(),
    }
}
