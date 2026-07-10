use std::fs;
use std::path::{Path, PathBuf};

use super::glob::glob_match;
use super::report::{CoverageCounts, ReportMessage};

pub struct RawDiscoveryOutcome {
    pub files: Vec<PathBuf>,
    pub coverage: CoverageCounts,
    pub failures: Vec<ReportMessage>,
}

/// Discover all `.json` schema files from the given paths.
///
/// - If a path is a file ending with `.json`, it is included directly.
/// - If a path is a directory, it is recursively scanned for `.json` files.
/// - Symlinks are not followed.
/// - Results are sorted lexicographically for deterministic output.
pub fn discover(paths: &[String], exclusions: &[String]) -> RawDiscoveryOutcome {
    let mut files = Vec::new();
    let mut coverage = CoverageCounts::default();
    let mut failures = Vec::new();
    for path_str in paths {
        let path = Path::new(path_str);
        if is_excluded(path, exclusions) {
            coverage.excluded += 1;
            continue;
        }
        coverage.attempted += 1;
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            files.push(path.to_path_buf());
        } else if path.is_dir() {
            collect_json_files(path, exclusions, &mut files, &mut coverage, &mut failures);
        } else {
            coverage.failed += 1;
            failures.push(ReportMessage {
                target: path_str.clone(),
                message: "input does not exist or is not a JSON schema file".into(),
            });
        }
    }
    files.sort();
    files.dedup();
    coverage.discovered = files.len();
    RawDiscoveryOutcome {
        files,
        coverage,
        failures,
    }
}

fn collect_json_files(
    dir: &Path,
    exclusions: &[String],
    out: &mut Vec<PathBuf>,
    coverage: &mut CoverageCounts,
    failures: &mut Vec<ReportMessage>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            coverage.failed += 1;
            failures.push(ReportMessage {
                target: dir.display().to_string(),
                message: format!("failed to read directory: {error}"),
            });
            return;
        }
    };
    collect_json_entries(dir, entries, exclusions, out, coverage, failures);
}

fn collect_json_entries(
    dir: &Path,
    entries: impl Iterator<Item = std::io::Result<fs::DirEntry>>,
    exclusions: &[String],
    out: &mut Vec<PathBuf>,
    coverage: &mut CoverageCounts,
    failures: &mut Vec<ReportMessage>,
) {
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                coverage.failed += 1;
                failures.push(ReportMessage {
                    target: dir.display().to_string(),
                    message: format!("failed to read directory entry: {error}"),
                });
                continue;
            }
        };
        let path = entry.path();
        if is_excluded(&path, exclusions) {
            coverage.excluded += 1;
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                coverage.failed += 1;
                failures.push(ReportMessage {
                    target: path.display().to_string(),
                    message: format!("failed to read directory entry metadata: {error}"),
                });
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_json_files(&path, exclusions, out, coverage, failures);
        } else if metadata.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

fn is_excluded(path: &Path, exclusions: &[String]) -> bool {
    let path = path.display().to_string().replace('\\', "/");
    exclusions.iter().any(|pattern| glob_match(pattern, &path))
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn directory_entry_errors_are_retained_as_failed_coverage() {
        let mut files = vec![];
        let mut coverage = CoverageCounts::default();
        let mut failures = vec![];
        let entries = std::iter::once(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "entry denied",
        )));

        collect_json_entries(
            Path::new("schemas"),
            entries,
            &[],
            &mut files,
            &mut coverage,
            &mut failures,
        );

        assert!(files.is_empty());
        assert_eq!(coverage.failed, 1);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].target, "schemas");
        assert!(failures[0]
            .message
            .contains("failed to read directory entry: entry denied"));
    }
}
