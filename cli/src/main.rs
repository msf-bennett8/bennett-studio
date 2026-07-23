mod commands;

use clap::{Parser, Subcommand};
use commands::keys::KeysCommand;

#[derive(Debug, Parser)]
#[command(name = "bennett", about = "Bennett Studio CLI", version)]
struct Cli {
    /// Base URL of the local engine
    #[arg(long, global = true, env = "BENNETT_ENGINE_URL", default_value = "http://localhost:3001")]
    engine_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manage durable API keys for external app access
    #[command(subcommand)]
    Keys(KeysCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keys(cmd) => commands::keys::handle(cmd, &cli.engine_url).await?,
    }

    Ok(())
}
