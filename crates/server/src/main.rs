use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "roadboard-server", version, about = "Roadboard HTTP + MCP server")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the HTTP server (reads ROADBOARD_BIND, default 127.0.0.1:8787)
    Serve,
    /// Run pending database migrations
    Migrate,
    /// Print version and exit
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Serve => roadboard_server::run().await,
        Command::Migrate => {
            println!("TODO M1: database migrations not yet implemented");
            Ok(())
        }
        Command::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
