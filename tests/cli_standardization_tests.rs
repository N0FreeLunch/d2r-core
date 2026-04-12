use std::process::Command;
use serde_json::Value;
use std::fs;

const FIXTURE_PATH: &str = "tests/fixtures/savegames/original/amazon_10_scrolls.d2s";

struct RunResult {
    stdout: String,
    stderr: String,
    status: std::process::ExitStatus,
}

fn run_bin(bin: &str, args: &[&str]) -> RunResult {
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("Failed to execute binary");
    
    RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status,
    }
}

#[test]
fn test_sba_help() {
    let bin = env!("CARGO_BIN_EXE_sba");
    let res = run_bin(bin, &["--help"]);
    assert!(res.status.success());
    assert!(res.stdout.contains("Usage: sba") || res.stderr.contains("Usage: sba"));
}

#[test]
fn test_bit_peek_no_args() {
    let bin = env!("CARGO_BIN_EXE_d2item_bit_peek");
    let res = run_bin(bin, &[]);
    // bit_peek prints usage on any parse error
    assert!(res.stderr.contains("Usage: d2item_bit_peek"));
}

#[test]
fn test_d2save_verify_help() {
    let bin = env!("CARGO_BIN_EXE_d2save_verify");
    let res = run_bin(bin, &["--help"]);
    assert!(res.status.success());
    assert!(res.stdout.contains("Usage: d2save_verify") || res.stderr.contains("Usage: d2save_verify"));
}

#[test]
fn test_bit_peek_json_discipline() {
    let bin = env!("CARGO_BIN_EXE_d2item_bit_peek");
    let res = run_bin(bin, &[FIXTURE_PATH, "0", "64", "--json"]);
    
    assert!(res.status.success(), "Exit status should be successful. stderr: {}", res.stderr);
    
    // Parse stdout as JSON
    let json: Value = serde_json::from_str(&res.stdout).expect("Stdout should be parseable JSON");
    
    // Verify stderr does not contain root JSON payload
    assert!(!res.stderr.trim().starts_with('{'), "Stderr should not contain JSON payload");
    
    // Check some metadata
    assert_eq!(json["metadata"]["tool"], "d2item_bit_peek");
}

#[test]
fn test_d2save_verify_json_discipline() {
    let bin = env!("CARGO_BIN_EXE_d2save_verify");
    let res = run_bin(bin, &[FIXTURE_PATH, "--json"]);
    
    // In JSON mode, even if verification fails (exit code 1), it should output valid JSON
    let json: Value = serde_json::from_str(&res.stdout).expect("Stdout should be parseable JSON");
    assert!(!res.stderr.trim().starts_with('{'), "Stderr should not contain JSON payload");
    assert_eq!(json["metadata"]["tool"], "d2save_verify");
}

#[test]
fn test_sba_json_discipline() {
    let bin = env!("CARGO_BIN_EXE_sba");
    let baseline_path = "tmp/test_sba_baseline.json";
    
    // Ensure tmp dir exists
    let _ = fs::create_dir_all("tmp");

    // 1. Generate baseline
    let res_gen = run_bin(bin, &["--fixture", FIXTURE_PATH, "--baseline", baseline_path, "--generate"]);
    assert!(res_gen.status.success(), "Baseline generation failed: {}", res_gen.stderr);
    
    // 2. Verify with JSON
    let res_ver = run_bin(bin, &["--fixture", FIXTURE_PATH, "--baseline", baseline_path, "--verify", "--json"]);
    assert!(res_ver.status.success(), "SBA verification failed: {}", res_ver.stderr);
    
    let json: Value = serde_json::from_str(&res_ver.stdout).expect("Stdout should be parseable JSON");
    assert!(!res_ver.stderr.trim().starts_with('{'), "Stderr should not contain JSON payload");
    assert_eq!(json["metadata"]["tool"], "sba");
    
    // Cleanup
    let _ = fs::remove_file(baseline_path);
}

#[test]
fn test_bit_peek_positional_defaults() {
    let bin = env!("CARGO_BIN_EXE_d2item_bit_peek");
    // Only one positional save_file, others defaulted
    let res = run_bin(bin, &[FIXTURE_PATH]);
    assert!(res.status.success(), "Bit peek with defaults failed: {}", res.stderr);
    assert!(res.stdout.contains("JM at byte"), "Should show JM info");
}

#[test]
fn test_d2save_verify_dump_bits() {
    let bin = env!("CARGO_BIN_EXE_d2save_verify");
    let res = run_bin(bin, &[FIXTURE_PATH, "--dump-bits", "0", "16"]);
    assert!(res.status.success(), "Dump bits failed: {}", res.stderr);
    assert!(res.stdout.contains("Dumping 16 bits starting at 0:"), "Should show dump header");
}
