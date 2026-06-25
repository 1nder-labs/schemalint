use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

use clap::Parser;
use schemalint::cli::args::{Cli, Commands, OutputFormat};
use schemalint::python::PythonError;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

include!("python_tests/part_01.rs");
include!("python_tests/part_02.rs");
include!("python_tests/part_03.rs");
