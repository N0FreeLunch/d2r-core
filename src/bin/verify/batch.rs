use d2r_core::verify::args::{ArgError, ArgParser, ArgSpec};
use d2r_core::verify::save_integrity::verify_save_integrity;
use d2r_core::verify::symmetry::calculate_symmetry_diff;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
struct BatchEntry {
    file: String,
    integrity_ok: bool,
    symmetry_ok: bool,
    integrity_issues: usize,
}

#[derive(Debug, Serialize)]
struct BatchReport {
    dir: String,
    total_files: usize,
    success_files: usize,
    failed_files: usize,
    entries: Vec<BatchEntry>,
}

fn main() -> anyhow::Result<()> {
    let mut parser = ArgParser::new("d2save_batch");
    parser.add_spec(ArgSpec::option("dir", None, Some("dir"), "Directory that contains .d2s files"));

    let parsed = match parser.parse(std::env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            return Ok(());
        }
        Err(ArgError::Error(e)) => anyhow::bail!("error: {}\n\n{}", e, parser.usage()),
    };

    let dir = parsed
        .get("dir")
        .ok_or_else(|| anyhow::anyhow!("--dir <PATH> is required"))?;
    let mut entries = Vec::new();

    for entry in fs::read_dir(Path::new(dir))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("d2s") {
            continue;
        }

        let bytes = fs::read(&path)?;
        let (integrity_report, integrity_failed) =
            verify_save_integrity(path.to_string_lossy().as_ref(), &bytes);
        let symmetry = calculate_symmetry_diff(&bytes, None, true)?;
        let ok = !integrity_failed && symmetry.success;
        entries.push(BatchEntry {
            file: path.to_string_lossy().to_string(),
            integrity_ok: !integrity_failed,
            symmetry_ok: symmetry.success,
            integrity_issues: integrity_report.issues.len(),
        });
        if !ok {
            continue;
        }
    }

    let total_files = entries.len();
    let success_files = entries
        .iter()
        .filter(|e| e.integrity_ok && e.symmetry_ok)
        .count();
    let report = BatchReport {
        dir: dir.to_string(),
        total_files,
        success_files,
        failed_files: total_files.saturating_sub(success_files),
        entries,
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
