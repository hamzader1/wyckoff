//! Refuse to send secrets to a third party.
//!
//! This is not paranoia: people stage `.env` files and paste API keys into
//! config all the time, and wyckoff's whole job is to ship that diff somewhere.
//! We scan the *added* lines only, and we only report the kind of secret and
//! the file — never the value.

mod patterns;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use patterns::{detects_assignment, detects_key_material, looks_like_assignment};

use crate::diff::ParsedDiff;

#[derive(Debug, Clone)]
pub struct Finding {
    pub path: String,
    pub kind: &'static str,
}

impl Finding {
    pub fn describe(&self) -> String {
        format!("  - {} in {}", self.kind, self.path)
    }
}

pub const PLACEHOLDERS: &[&str] = &[
    "changeme",
    "change_me",
    "example",
    "placeholder",
    "your_",
    "your-",
    "xxx",
    "yyy",
    "redacted",
    "dummy",
    "fake",
    "todo",
    "<",
    "process.env",
    "std::env",
    "os.environ",
    "getenv",
    "secret_here",
    "test_key",
    "sample",
];

pub const SENSITIVE_NAMES: &[&str] = &[
    "api_key",
    "apikey",
    "api_secret",
    "secret_key",
    "secretkey",
    "access_key",
    "access_token",
    "auth_token",
    "client_secret",
    "private_key",
    "password",
    "passwd",
    "token",
    "bearer",
    "credential",
];

pub fn scan(diff: &ParsedDiff) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();

    for file in &diff.files {
        let lower_path = file.path.to_lowercase();
        let file_name = lower_path
            .rsplit('/')
            .next()
            .unwrap_or(&lower_path)
            .to_string();
        let is_env_file = file_name.starts_with(".env")
            && !file_name.ends_with(".example")
            && !file_name.ends_with(".sample")
            && !file_name.ends_with(".template");

        for line in file.added_lines() {
            let trimmed = line.trim();
            let kind = if let Some(kind) = detects_key_material(trimmed) {
                kind
            } else if is_env_file && looks_like_assignment(trimmed) {
                "an environment value"
            } else if let Some(kind) = detects_assignment(trimmed) {
                kind
            } else {
                continue;
            };
            if !findings
                .iter()
                .any(|f| f.path == file.path && f.kind == kind)
            {
                findings.push(Finding {
                    path: file.path.clone(),
                    kind,
                });
            }
        }
    }

    findings
}
