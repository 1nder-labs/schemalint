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
    /// List built-in capability profiles and their provider aliases
    Profiles(ProfilesArgs),
    /// Start JSON-RPC server mode
    Server(ServerArgs),
}

#[derive(Parser)]
pub struct CheckArgs {
    /// Built-in profile ID or path to a TOML capability profile (repeatable).
    /// Accepts bare provider names ("openai", "anthropic") and
    /// "<provider>.so.latest" as aliases for that provider's latest profile.
    /// Optional: if omitted, the provider is auto-detected from package.json
    /// dependencies near the schema path (falling back to openai.so.2026-04-30).
    /// Run `schemalint profiles` to list all built-in profiles.
    #[arg(short, long = "profile")]
    pub profiles: Vec<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Exclude matching files or directories before checking
    #[arg(long = "exclude")]
    pub excludes: Vec<String>,

    /// Schema files or directories to check
    pub paths: Vec<String>,
}

#[derive(Parser)]
pub struct ProfilesArgs {}

#[derive(Parser)]
pub struct ServerArgs {}

#[derive(Parser)]
pub struct CheckPythonArgs {
    /// Python package names to discover Pydantic models from (repeatable)
    #[arg(short = 'P', long = "package")]
    pub packages: Vec<String>,

    /// Built-in profile ID or TOML capability profile path (repeatable; overrides
    /// pyproject.toml). Accepts bare provider names ("openai", "anthropic") and
    /// "<provider>.so.latest" as aliases. Run `schemalint profiles` to list all.
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

    /// Exclude matching packages or modules before importing them
    #[arg(long = "exclude")]
    pub excludes: Vec<String>,

    /// Continue discovering remaining packages after a discovery failure
    #[arg(long)]
    pub continue_on_discovery_error: bool,
}

#[derive(Parser)]
pub struct CheckNodeArgs {
    /// TypeScript source globs to discover Zod schemas from (repeatable)
    #[arg(short = 'S', long = "source")]
    pub sources: Vec<String>,

    /// Built-in profile ID or TOML capability profile path (repeatable; overrides
    /// package.json). Accepts bare provider names ("openai", "anthropic") and
    /// "<provider>.so.latest" as aliases. Optional: if omitted, the provider is
    /// auto-detected from source imports or package.json dependencies (falling
    /// back to openai.so.2026-04-30). Run `schemalint profiles` to list all.
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

    /// Exclude matching source files before importing them
    #[arg(long = "exclude")]
    pub excludes: Vec<String>,

    /// Continue discovering remaining source globs after a discovery failure
    #[arg(long)]
    pub continue_on_discovery_error: bool,
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
