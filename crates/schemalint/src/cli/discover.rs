use std::fs;
use std::path::{Path, PathBuf};

/// Discover all `.json` schema files from the given paths.
///
/// - If a path is a file ending with `.json`, it is included directly.
/// - If a path is a directory, it is recursively scanned for `.json` files.
/// - Symlinks are not followed.
/// - Results are sorted lexicographically for deterministic output.
pub fn discover(paths: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path_str in paths {
        let path = Path::new(path_str);
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            files.push(path.to_path_buf());
        } else if path.is_dir() {
            collect_json_files(path, &mut files);
        }
    }
    files.sort();
    files.dedup();
    files
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let metadata = entry.metadata();
        let is_symlink = metadata.as_ref().is_ok_and(|m| m.file_type().is_symlink());
        if is_symlink {
            continue;
        }
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(path);
        }
    }
}
