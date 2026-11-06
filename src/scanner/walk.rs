#![allow(dead_code)]

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::models::BookFormat;

#[derive(Debug, Clone)]
pub struct DiscoveredBookFile {
    pub path: PathBuf,
    pub format: BookFormat,
    pub file_size: i64,
}

pub fn discover_books_in_directory(dir: &Path) -> Vec<DiscoveredBookFile> {
    if !dir.exists() || !dir.is_dir() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Ignore hidden and temporary files
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            if file_name.starts_with('.')
                || file_name.ends_with(".tmp")
                || file_name.contains("__MACOSX")
            {
                continue;
            }
        }

        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if let Some(format) = BookFormat::from_extension(ext) {
                let file_size = entry.metadata().map(|m| m.len() as i64).unwrap_or(0);
                results.push(DiscoveredBookFile {
                    path: path.to_path_buf(),
                    format,
                    file_size,
                });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dir() {
        let files = discover_books_in_directory(Path::new("/nonexistent_directory_for_test_12345"));
        assert!(files.is_empty());
    }
}
