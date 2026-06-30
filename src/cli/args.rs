use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hare")]
// TODO: about
#[command(about = "A lightweight hardware information CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show CPU frequency
    Cpu,
}
