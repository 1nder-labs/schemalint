use std::fs;
use std::path::PathBuf;

use clap::Parser;
use schemalint::cli::args::{Cli, Commands, OutputFormat};
use schemalint::cli::discover;

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_check_command_with_profile_and_files() {
    let cli = Cli::parse_from([
        "schemalint",
        "check",
        "--profile",
        "openai.toml",
        "schema.json",
    ]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.profiles, vec![PathBuf::from("openai.toml")]);
            assert_eq!(args.paths, vec!["schema.json"]);
            assert!(args.format.is_none());
        }
        Commands::Server(_) => unreachable!(),
        Commands::CheckPython(_) => unreachable!(),
    }
}

#[test]
fn parse_check_command_with_format_flag() {
    let cli = Cli::parse_from([
        "schemalint",
        "check",
        "--profile",
        "openai.toml",
        "--format",
        "json",
        "schema.json",
    ]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.format, Some(OutputFormat::Json));
        }
        Commands::Server(_) => unreachable!(),
        Commands::CheckPython(_) => unreachable!(),
    }
}

#[test]
fn parse_check_command_with_multiple_paths() {
    let cli = Cli::parse_from([
        "schemalint",
        "check",
        "--profile",
        "openai.toml",
        "schema1.json",
        "schema2.json",
        "dir/",
    ]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.paths, vec!["schema1.json", "schema2.json", "dir/"]);
        }
        Commands::Server(_) => unreachable!(),
        Commands::CheckPython(_) => unreachable!(),
    }
}

#[test]
fn parse_check_command_with_multiple_profiles() {
    let cli = Cli::parse_from([
        "schemalint",
        "check",
        "--profile",
        "openai.so.2026-04-30",
        "--profile",
        "anthropic.so.2026-04-30",
        "schema.json",
    ]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(
                args.profiles,
                vec![
                    PathBuf::from("openai.so.2026-04-30"),
                    PathBuf::from("anthropic.so.2026-04-30")
                ]
            );
        }
        Commands::Server(_) => unreachable!(),
        Commands::CheckPython(_) => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Profile resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_builtin_profile_openai() {
    let bytes = schemalint::cli::resolve_profile("openai.so.2026-04-30").unwrap();
    assert!(!bytes.is_empty());
    let profile = schemalint::profile::load(&bytes).unwrap();
    assert_eq!(profile.name, "openai.so.2026-04-30");
}

#[test]
fn resolve_builtin_profile_anthropic() {
    let bytes = schemalint::cli::resolve_profile("anthropic.so.2026-04-30").unwrap();
    assert!(!bytes.is_empty());
    let profile = schemalint::profile::load(&bytes).unwrap();
    assert_eq!(profile.name, "anthropic.so.2026-04-30");
}

#[test]
fn resolve_unknown_builtin_profile() {
    let result = schemalint::cli::resolve_profile("unknown-profile");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown built-in profile"));
}

#[test]
fn resolve_filesystem_profile() {
    let dir = tempfile::tempdir().unwrap();
    let profile_path = dir.path().join("custom.toml");
    fs::write(
        &profile_path,
        r#"
name = "custom"
version = "1.0"
type = "allow"

[structural]
require_object_root = false
"#,
    )
    .unwrap();

    let bytes = schemalint::cli::resolve_profile(profile_path.to_str().unwrap()).unwrap();
    let profile = schemalint::profile::load(&bytes).unwrap();
    assert_eq!(profile.name, "custom");
}

// ---------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------

#[test]
fn discover_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("schema.json");
    fs::write(&file, "{}").unwrap();
    let files = discover::discover(&[file.to_string_lossy().to_string()]);
    assert_eq!(files, vec![file]);
}

#[test]
fn discover_recursive_directory() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    let a = dir.path().join("a.json");
    let b = sub.join("b.json");
    fs::write(&a, "{}").unwrap();
    fs::write(&b, "{}").unwrap();
    let files = discover::discover(&[dir.path().to_string_lossy().to_string()]);
    assert_eq!(files, vec![a, b]);
}

#[test]
fn discover_ignores_non_json_files() {
    let dir = tempfile::tempdir().unwrap();
    let json = dir.path().join("schema.json");
    let txt = dir.path().join("readme.txt");
    fs::write(&json, "{}").unwrap();
    fs::write(&txt, "hello").unwrap();
    let files = discover::discover(&[dir.path().to_string_lossy().to_string()]);
    assert_eq!(files, vec![json]);
}

#[test]
fn discover_ignores_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.json");
    let link = dir.path().join("link.json");
    fs::write(&real, "{}").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&real, &link).unwrap();
    let files = discover::discover(&[dir.path().to_string_lossy().to_string()]);
    assert_eq!(files, vec![real]);
}

#[test]
fn discover_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let files = discover::discover(&[dir.path().to_string_lossy().to_string()]);
    assert!(files.is_empty());
}

#[test]
fn discover_deduplicates() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("schema.json");
    fs::write(&file, "{}").unwrap();
    let files = discover::discover(&[
        file.to_string_lossy().to_string(),
        file.to_string_lossy().to_string(),
    ]);
    assert_eq!(files.len(), 1);
}
