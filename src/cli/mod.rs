pub mod args;
mod handler;

pub use args::{Cli, Commands};

pub fn run(command: Commands) -> anyhow::Result<()> {
    handler::dispatch(command)
}
