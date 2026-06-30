use crate::cli::Commands;
use crate::hardware;
use anyhow::Result;

pub fn dispatch(command: Commands) -> Result<()> {
    match command {
        Commands::Cpu => hardware::cpu::run(),
    }
}
