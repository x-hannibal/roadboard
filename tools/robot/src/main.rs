mod bump;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "robot", version, about = "Roadboard devloop CLI")]
struct Cli {
    /// Enable debug logging (RUST_LOG=debug)
    #[arg(short = 'd', long, global = true)]
    debug: bool,

    /// Enable trace logging — overrides --debug (RUST_LOG=trace)
    #[arg(short = 't', long, global = true)]
    trace: bool,

    /// Quiet mode (RUST_LOG=warn)
    #[arg(short = 'q', long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: RobotCommand,
}

#[derive(Subcommand)]
enum RobotCommand {
    /// Run the application (Tauri dev or Axum server)
    Run {
        /// Run Axum standalone server instead of Tauri
        #[arg(long)]
        server: bool,

        /// Also run pnpm dev for the web app (server mode only)
        #[arg(long)]
        with_web: bool,

        /// Override server bind port
        #[arg(long)]
        port: Option<u16>,
    },

    /// Bump version, update changelog, tag and push
    Bump {
        /// Target version in MAJOR.MINOR.PATCH format (default: patch increment)
        version: Option<String>,

        /// Print what would happen without making any changes
        #[arg(long)]
        dry_run: bool,

        /// Skip git push
        #[arg(long)]
        no_push: bool,

        /// Skip git tag creation
        #[arg(long)]
        no_tag: bool,
    },
}

fn log_level(cli: &Cli) -> &'static str {
    if cli.trace {
        "trace"
    } else if cli.debug {
        "debug"
    } else if cli.quiet {
        "warn"
    } else {
        "info"
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let level = log_level(&cli);
    match &cli.command {
        RobotCommand::Run {
            server,
            with_web,
            port,
        } => run::run_command(*server, *with_web, *port, level),
        RobotCommand::Bump {
            version,
            dry_run,
            no_push,
            no_tag,
        } => bump::bump_command(version.as_deref(), *dry_run, *no_push, *no_tag),
    }
}
