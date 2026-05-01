use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "schemalint")]
#[command(
    about = "Static analysis tool for JSON Schema compatibility with LLM structured-output providers"
)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check JSON Schemas against a capability profile
    Check(CheckArgs),
    /// Start JSON-RPC server mode
    Server(ServerArgs),
}

#[derive(Parser)]
pub struct CheckArgs {
    /// Path to the TOML capability profile (may be given multiple times)
    #[arg(short, long = "profile", required = true)]
    pub profiles: Vec<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Schema files or directories to check
    pub paths: Vec<String>,
}

#[derive(Parser)]
pub struct ServerArgs {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable rustc-style output
    Human,
    /// Structured JSON output
    Json,
}
