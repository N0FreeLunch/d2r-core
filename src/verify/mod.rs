pub mod bit_diff;

pub trait Verifier {
    fn verify(&self, fixture: &[u8], reproduced: &[u8]) -> VerificationReport;
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct VerificationReport {
    pub is_success: bool,
    pub issues: Vec<VerificationIssue>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct VerificationIssue {
    pub bit_offset: u64,
    pub bit_length: u64,
    pub expected: Vec<u8>,
    pub actual: Vec<u8>,
    pub label: Option<String>,
    pub message: String,
}

impl VerificationReport {
    pub fn success() -> Self {
        Self {
            is_success: true,
            issues: Vec::new(),
        }
    }

    pub fn failure(issues: Vec<VerificationIssue>) -> Self {
        Self {
            is_success: false,
            issues,
        }
    }
}
