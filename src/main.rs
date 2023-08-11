use clap::{Parser, Subcommand};

mod cliargs;
mod network;
mod usgs;

use crate::cliargs::CliAction;

#[derive(Parser)]
struct Cli {
    /// Don't print the stderr outputs
    #[arg(short, long, action)]
    quiet: bool,
    /// Command to run
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Download data from USGS
    Usgs(usgs::CliArgs),
    Network(network::CliArgs),
}

impl CliAction for Action {
    fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Usgs(v) => v.run(),
            Self::Network(v) => v.run(),
        }
    }
}

fn main() {
    let args = Cli::parse();
    args.action.run().ok();
}
