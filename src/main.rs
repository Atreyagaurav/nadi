use clap::{Parser, Subcommand};

mod cliargs;
mod connection;
mod network;
mod timeseries;
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
    /// Visualize network
    Network(network::CliArgs),
    /// Connection
    Connection(connection::CliArgs),
    /// Timeseries
    Timeseries(timeseries::CliArgs),
}

impl CliAction for Action {
    fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Usgs(v) => v.run(),
            Self::Network(v) => v.run(),
            Self::Connection(v) => v.run(),
            Self::Timeseries(v) => v.run(),
        }
    }
}

fn main() {
    let args = Cli::parse();
    if let Err(e) = args.action.run() {
        eprintln!("{:?}", e);
    }
}
