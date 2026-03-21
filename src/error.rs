use thiserror::Error;

/// A structured error for parsing and verification tasks.
/// Designed to provide AI-friendly, actionable diagnostic information.
#[derive(Debug, Clone, Error)]
#[error(r#"{{"error": "DiagnosticError", "offset": {offset}, "expected": "{expected}", "actual": "{actual}", "hint": "{hint}"}}"#)]
pub struct DiagnosticError {
    pub offset: usize,
    pub expected: String,
    pub actual: String,
    pub hint: String,
}

impl DiagnosticError {
    pub fn new(
        offset: usize,
        expected: impl Into<String>,
        actual: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            offset,
            expected: expected.into(),
            actual: actual.into(),
            hint: hint.into(),
        }
    }
}
