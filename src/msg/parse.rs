//! Getting a [`CommitMsg`] out of whatever the provider actually returned.

use super::CommitMsg;
use super::clean::clean_subject;

/// Tolerant JSON extraction: raw, fenced, or embedded in prose.
pub fn extract_json(text: &str) -> Option<serde_json::Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed)
        && value.is_object()
    {
        return Some(value);
    }
    for block in fenced_blocks(trimmed) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(block.trim())
            && value.is_object()
        {
            return Some(value);
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&trimmed[start..=end])
        .ok()
        .filter(|v| v.is_object())
}

fn fenced_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        let after = &rest[open + 3..];
        let after = after.strip_prefix("json").unwrap_or(after);
        let Some(close) = after.find("```") else {
            break;
        };
        blocks.push(&after[..close]);
        rest = &after[close + 3..];
    }
    blocks
}

/// Map a JSON object into a message, accepting the field-name variations that
/// different models reach for.
pub fn from_value(value: &serde_json::Value) -> CommitMsg {
    let object = value.as_object().cloned().unwrap_or_default();

    let subject = ["subject", "message", "commit", "summary", "title"]
        .iter()
        .find_map(|key| object.get(*key).and_then(|v| v.as_str()))
        .map(clean_subject)
        .unwrap_or_default();

    let body = object
        .get("body")
        .map(strings_from)
        .unwrap_or_default()
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();

    CommitMsg {
        subject,
        body,
        breaking: object
            .get("breaking")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        footers: object.get("footers").map(strings_from).unwrap_or_default(),
        confidence: object
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32),
        notes: ["notes", "notes_to_user", "note"]
            .iter()
            .find_map(|key| object.get(*key).and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string(),
    }
}

/// Fall back to treating a line of prose as the subject.
pub fn plain_subject(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("```")
            || line.starts_with('{')
            || line.starts_with('}')
        {
            continue;
        }
        if line.chars().count() > super::DEFAULT_MAX_SUBJECT * 2 {
            return None;
        }
        let cleaned = clean_subject(line);
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }
    None
}

fn strings_from(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Object(map) => map
                    .get("text")
                    .or_else(|| map.get("value"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                _ => None,
            })
            .collect(),
        serde_json::Value::String(s) => s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn truncate_for_error(text: &str) -> String {
    let trimmed = text.trim();
    let cut: String = trimmed.chars().take(600).collect();
    if trimmed.chars().count() > 600 {
        format!("{cut}…")
    } else {
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::CommitMsg;

    #[test]
    fn reads_fenced_and_prose_wrapped_json() {
        let text = "Sure! Here it is:\n```json\n{\"subject\": \"Add cache warming\", \"body\": [\"why not\"]}\n```";
        let msg = CommitMsg::from_provider_output(text).unwrap();
        assert_eq!(msg.subject, "Add cache warming");
        assert_eq!(msg.body, vec!["why not"]);
    }

    #[test]
    fn reads_bare_json_and_embedded_json() {
        assert_eq!(
            CommitMsg::from_provider_output("{\"subject\": \"Add x\"}")
                .unwrap()
                .subject,
            "Add x"
        );
        assert_eq!(
            CommitMsg::from_provider_output("blah {\"subject\": \"Add y\"} trailing")
                .unwrap()
                .subject,
            "Add y"
        );
    }

    #[test]
    fn falls_back_to_plain_text_output() {
        let msg = CommitMsg::from_provider_output("Refactor the page access path").unwrap();
        assert_eq!(msg.subject, "Refactor the page access path");
    }

    #[test]
    fn reads_alternate_field_names() {
        let value = serde_json::json!({"message": "feat: x", "notes_to_user": "unsure"});
        let msg = from_value(&value);
        assert_eq!(msg.subject, "X");
        assert_eq!(msg.notes, "unsure");
    }

    #[test]
    fn body_as_a_string_becomes_lines() {
        let value = serde_json::json!({"subject": "Add x", "body": "one\ntwo"});
        assert_eq!(from_value(&value).body, vec!["one", "two"]);
    }

    #[test]
    fn junk_output_is_an_error_not_a_message() {
        assert!(CommitMsg::from_provider_output("").is_err());
        assert!(CommitMsg::from_provider_output("#").is_err());
    }
}
