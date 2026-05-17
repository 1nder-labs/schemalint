#[test]
fn parse_check_python_with_output_flag() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp.models",
        "-p",
        "openai.so.2026-04-30",
        "-o",
        "results.json",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.output, Some(std::path::PathBuf::from("results.json")));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_with_output_and_format() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp.models",
        "-p",
        "openai.so.2026-04-30",
        "-f",
        "sarif",
        "-o",
        "results.sarif",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.format, Some(OutputFormat::Sarif));
            assert_eq!(args.output, Some(std::path::PathBuf::from("results.sarif")));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_empty_package_list() {
    let cli = Cli::parse_from(["schemalint", "check-python", "-p", "openai.so.2026-04-30"]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert!(args.packages.is_empty());
            assert!(!args.profiles.is_empty());
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_minimal_args() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.packages, vec!["myapp"]);
            assert_eq!(args.profiles.len(), 1);
            assert!(args.format.is_none());
            assert!(args.config.is_none());
            assert!(args.python_path.is_none());
            assert!(args.output.is_none());
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// CLI errors without Python (argument-level validation)
// ---------------------------------------------------------------------------

#[test]
fn check_python_missing_required_package_flag() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args(["check-python", "-p", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no packages specified."));
}

#[test]
fn check_python_invalid_profile_name() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-python",
            "-P",
            "myapp.models",
            "-p",
            "nonexistent-zzz-profile",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown built-in profile"));
}

#[test]
fn check_python_invalid_format_cli_rejects() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "-P",
            "myapp.models",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "invalidfmt",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalidfmt"));
}
