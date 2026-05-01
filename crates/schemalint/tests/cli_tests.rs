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
            assert_eq!(args.profile, PathBuf::from("openai.toml"));
            assert_eq!(args.paths, vec!["schema.json"]);
            assert!(args.format.is_none());
        }
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
    }
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
