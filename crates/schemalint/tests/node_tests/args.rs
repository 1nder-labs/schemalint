use super::*;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_check_node_with_source_and_profile() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "src/**/*.ts",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.sources, vec!["src/**/*.ts"]);
            assert_eq!(args.profiles, vec![PathBuf::from("openai.so.2026-04-30")]);
            assert!(args.format.is_none());
            assert!(args.config.is_none());
            assert!(args.node_path.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_with_format() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "app.ts",
        "-p",
        "openai.so.2026-04-30",
        "-f",
        "json",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.format, Some(OutputFormat::Json));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_with_config_and_node_path() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "app.ts",
        "-p",
        "openai.so.2026-04-30",
        "--config",
        "custom-package.json",
        "--node-path",
        "/usr/local/bin/tsx",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.config, Some(PathBuf::from("custom-package.json")));
            assert_eq!(args.node_path, Some("/usr/local/bin/tsx".to_string()));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_multiple_sources() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "src/models.ts",
        "-S",
        "src/schemas.ts",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.sources, vec!["src/models.ts", "src/schemas.ts"]);
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Help output
// ---------------------------------------------------------------------------

#[test]
fn check_node_help_shows_subcommand() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd.args(["check-node", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check Zod schemas"));
    assert!(stdout.contains("--source"));
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--node-path"));
    assert!(output.status.success());
}
