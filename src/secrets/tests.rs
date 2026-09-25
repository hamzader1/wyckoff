//! Tests for the secret scanner.

use super::scan;
use crate::diff::ParsedDiff;

fn diff_of(path: &str, header_extra: &str, added: &[&str]) -> ParsedDiff {
    let mut text =
        format!("diff --git a/{path} b/{path}\n{header_extra}--- a/{path}\n+++ b/{path}\n");
    text.push_str(&format!("@@ -1 +1,{} @@\n", added.len() + 1));
    text.push_str(" context\n");
    for line in added {
        text.push('+');
        text.push_str(line);
        text.push('\n');
    }
    ParsedDiff::parse(&text)
}

#[test]
fn catches_known_key_material() {
    let diff = diff_of(
        "src/a.rs",
        "",
        &[
            "const K: &str = \"AKIAIOSFODNN7EXAMPLE\";",
            "let t = \"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123\";",
            "let n = \"nvapi-abcdefghijklmnopqrstuvwxyz123456\";",
        ],
    );
    let findings = scan(&diff);
    assert_eq!(findings.len(), 3, "{findings:?}");
    assert!(findings.iter().any(|f| f.kind.contains("AWS")));
    assert!(findings.iter().any(|f| f.kind.contains("GitHub")));
    assert!(findings.iter().any(|f| f.kind.contains("NVIDIA")));
}

#[test]
fn catches_private_key_blocks() {
    let diff = diff_of(
        "id_rsa",
        "new file mode 100644\n",
        &["-----BEGIN OPENSSH PRIVATE KEY-----"],
    );
    assert!(scan(&diff).iter().any(|f| f.kind.contains("private key")));
}

#[test]
fn catches_env_files_with_real_values() {
    let diff = diff_of(".env", "", &["OPENAI_API_KEY=abcdef123456789"]);
    assert!(!scan(&diff).is_empty());
}

#[test]
fn catches_hardcoded_credentials_in_code() {
    let diff = diff_of("src/cfg.rs", "", &["let api_key = \"9f2b7c41aa55e013\";"]);
    let findings = scan(&diff);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].kind.contains("credential"));
}

#[test]
fn ignores_placeholders_env_examples_and_env_lookups() {
    let diff = diff_of(
        ".env.example",
        "",
        &["OPENAI_API_KEY=your_key_here", "SECRET=changeme"],
    );
    assert!(scan(&diff).is_empty(), "{:?}", scan(&diff));

    let diff = diff_of(
        "src/config.rs",
        "",
        &["let api_key = std::env::var(\"API_KEY\")?;"],
    );
    assert!(scan(&diff).is_empty(), "{:?}", scan(&diff));
}

#[test]
fn does_not_flag_ordinary_lines() {
    let diff = diff_of(
        "src/lib.rs",
        "",
        &[
            "pub fn token_count(text: &str) -> usize { 0 }",
            "// the access token is refreshed by the caller",
            "let timeout = 30;",
            "let auth_token = request.headers().get(\"x-token\");",
        ],
    );
    assert!(scan(&diff).is_empty(), "{:?}", scan(&diff));
}

#[test]
fn only_added_lines_matter() {
    let text = "diff --git a/.env b/.env\n--- a/.env\n+++ b/.env\n@@ -1,2 +1,2 @@\n-OPENAI_API_KEY=old1234567890\n REMOVED=1\n";
    let diff = ParsedDiff::parse(text);
    assert!(scan(&diff).is_empty(), "{:?}", scan(&diff));
}

#[test]
fn reports_each_secret_once_per_file() {
    let diff = diff_of(
        "src/a.rs",
        "",
        &[
            "let a = \"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123\";",
            "let b = \"ghp_ZYXWVUTSRQPONMLKJIHGFEDCBA9876\";",
        ],
    );
    assert_eq!(scan(&diff).len(), 1);
}
