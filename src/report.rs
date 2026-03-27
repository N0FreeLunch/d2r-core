use serde::Serialize;
use std::path::Path;

/// Standardized metadata for all verifier reports.
#[derive(Serialize, Debug, Clone)]
pub struct ReportMetadata {
    pub file: String,
}

impl ReportMetadata {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            file: path.as_ref()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }
}

/// A generic report wrapper that ensures consistent JSON structure.
#[derive(Serialize, Debug)]
pub struct Report<T> {
    #[serde(flatten)]
    pub metadata: ReportMetadata,
    pub scan_results: Vec<T>,
}

impl<T> Report<T> {
    pub fn new(file_path: &str, scan_results: Vec<T>) -> Self {
        Self {
            metadata: ReportMetadata::from_path(file_path),
            scan_results,
        }
    }
}
