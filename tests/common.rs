use std::path::PathBuf;

pub fn repo_path(relative: &str) -> PathBuf {
    let _ = dotenvy::dotenv();
    let base = std::env::var("D2R_CORE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    base.join(relative)
}
