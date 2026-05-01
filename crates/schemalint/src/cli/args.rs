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
    /// Check Pydantic models via Python subprocess
    CheckPython(CheckPythonArgs),
    /// Check Zod schemas via Node.js subprocess
    CheckNode(CheckNodeArgs),
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

#[derive(Parser)]
pub struct CheckPythonArgs {
    /// Python package names to discover Pydantic models from (repeatable)
    #[arg(short = 'P', long = "package")]
    pub packages: Vec<String>,

    /// Path to TOML capability profile (repeatable; overrides pyproject.toml)
    #[arg(short, long = "profile")]
    pub profiles: Vec<PathBuf>,

    /// Path to pyproject.toml (default: ./pyproject.toml)
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// Path to Python executable (default: python3)
    #[arg(long = "python-path")]
    pub python_path: Option<String>,

    /// Output format
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Parser)]
pub struct CheckNodeArgs {
    /// TypeScript source globs to discover Zod schemas from (repeatable)
    #[arg(short = 'S', long = "source")]
    pub sources: Vec<String>,

    /// Path to TOML capability profile (repeatable; overrides package.json)
    #[arg(short, long = "profile")]
    pub profiles: Vec<PathBuf>,

    /// Path to package.json (default: ./package.json)
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// Path to Node/tsx executable (default: auto-detect tsx)
    #[arg(long = "node-path")]
    pub node_path: Option<String>,

    /// Output format
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable rustc-style output
    Human,
    /// Structured JSON output
    Json,
    /// SARIF v2.1.0 output
    Sarif,
    /// GitHub Actions workflow commands
    Gha,
    /// JUnit XML output
    Junit,
}
