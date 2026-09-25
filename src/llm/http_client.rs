//! One HTTP helper, shared by every provider.
//!
//! `http_status_as_error(false)` matters: it lets us read the provider's error
//! body and show the real reason ("model not found", "invalid key") instead of
//! a bare 400. We never log headers, so keys cannot leak into output.

use std::time::Duration;

use crate::error::{Error, Result};

#[derive(Debug)]
pub struct HttpResponse {
    pub body: String,
}

fn agent(timeout_secs: u64) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_secs.max(1))))
        .http_status_as_error(false)
        .user_agent(concat!("wyckoff/", env!("CARGO_PKG_VERSION")))
        .build();
    config.into()
}

pub fn post_json(
    url: &str,
    headers: &[(String, String)],
    body: &serde_json::Value,
    timeout_secs: u64,
    provider: &str,
) -> Result<HttpResponse> {
    let agent = agent(timeout_secs);
    let mut request = agent.post(url);
    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let mut response = request.send_json(body).map_err(|e| Error::Http {
        url: url.to_string(),
        msg: describe(&e),
    })?;

    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::Http {
            url: url.to_string(),
            msg: describe(&e),
        })?;

    if status >= 400 {
        return Err(Error::ProviderHttp {
            provider: provider.to_string(),
            status,
            body: clip(&text),
        });
    }
    Ok(HttpResponse { body: text })
}

pub fn get_json(
    url: &str,
    headers: &[(String, String)],
    timeout_secs: u64,
) -> Result<HttpResponse> {
    let agent = agent(timeout_secs);
    let mut request = agent.get(url);
    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let mut response = request.call().map_err(|e| Error::Http {
        url: url.to_string(),
        msg: describe(&e),
    })?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::Http {
            url: url.to_string(),
            msg: describe(&e),
        })?;
    if status >= 400 {
        return Err(Error::ProviderHttp {
            provider: url.to_string(),
            status,
            body: clip(&text),
        });
    }
    Ok(HttpResponse { body: text })
}

fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Timeout(_) => "timed out".to_string(),
        ureq::Error::HostNotFound => "host not found (check the base url and your network)".into(),
        ureq::Error::ConnectionFailed => "connection failed (is the server running?)".into(),
        ureq::Error::StatusCode(code) => format!("HTTP {code}"),
        other => other.to_string(),
    }
}

/// Keep error bodies readable but bounded.
fn clip(text: &str) -> String {
    let trimmed = text.trim();
    let cut: String = trimmed.chars().take(800).collect();
    if trimmed.chars().count() > 800 {
        format!("{cut}…")
    } else {
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_long_error_bodies() {
        let long = "x".repeat(2000);
        assert!(clip(&long).chars().count() <= 801);
        assert_eq!(clip("short"), "short");
    }

    #[test]
    fn timeout_descriptions_are_human() {
        assert!(describe(&ureq::Error::HostNotFound).contains("host not found"));
    }

    #[test]
    fn unreachable_host_is_a_clean_error() {
        let error = get_json("http://127.0.0.1:9/models", &[], 1).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("127.0.0.1:9"), "{text}");
    }
}
