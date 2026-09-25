use super::*;

// ---------------------------------------------------------------------------
// Additional CLI argument parsing edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_check_node_all_formats() {
    for (flag, expected) in [
        ("human", OutputFormat::Human),
        ("json", OutputFormat::Json),
        ("sarif", OutputFormat::Sarif),
        ("gha", OutputFormat::Gha),
        ("junit", OutputFormat::Junit),
    ] {
        let cli = Cli::parse_from([
            "schemalint",
            "check-node",
            "-S",
            "src/**/*.ts",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            flag,
        ]);
        match cli.command {
            Commands::CheckNode(args) => {
                assert_eq!(args.format, Some(expected), "format flag -f {flag}");
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn parse_check_node_with_output_flag() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "src/app.ts",
        "-p",
        "openai.so.2026-04-30",
        "-o",
        "results.json",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.output, Some(std::path::PathBuf::from("results.json")));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_with_output_and_format() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "src/app.ts",
        "-p",
        "openai.so.2026-04-30",
        "-f",
        "sarif",
        "-o",
        "results.sarif",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.format, Some(OutputFormat::Sarif));
            assert_eq!(args.output, Some(std::path::PathBuf::from("results.sarif")));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_empty_source_list() {
    let cli = Cli::parse_from(["schemalint", "check-node", "-p", "openai.so.2026-04-30"]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert!(args.sources.is_empty());
            assert!(!args.profiles.is_empty());
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_minimal_args() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "schema.ts",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.sources, vec!["schema.ts"]);
            assert_eq!(args.profiles.len(), 1);
            assert!(args.format.is_none());
            assert!(args.config.is_none());
            assert!(args.node_path.is_none());
            assert!(args.output.is_none());
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Format validity (CLI handles invalid format before Node spawns)
// ---------------------------------------------------------------------------

#[test]
fn check_node_invalid_format_cli_rejects() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-node",
            "-S",
            "src/**/*.ts",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "invalidfmt",
        ])
        .output()
        .unwrap();
    // clap rejects invalid ValueEnum variant
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalidfmt"));
}
