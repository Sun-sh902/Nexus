//! Safe, bounded, opt-in network reachability checks for AI Model Providers (Security §7).
//! Plaintext HTTP is allowed ONLY for loopback addresses (127.0.0.1, localhost, [::1]);
//! public endpoints MUST use HTTPS with standard TLS verification.
//! Automatic redirects are disabled (0 redirects) to prevent credential leakage or SSRF.
//! Timeouts are strictly bounded to 5 seconds. Responses are bounded to 64 KiB.
//! No user prompts, chat completions, or project data are ever sent.

use crate::core::{redact, NexusError, NexusResult};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::IpAddr;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderReachabilityResult {
    pub provider_id: String,
    pub status: String, // "connected", "authFailed", "unreachable", "invalidEndpoint", "notConfigured"
    pub http_status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub message: String,
    pub models_discovered: Vec<String>,
}

/// Validates that a provider URL complies with Security §7:
/// - Scheme must be http or https
/// - If http, host MUST be loopback
/// - Disallows link-local, cloud metadata, and private IP redirects
pub fn validate_provider_url(raw: &str) -> NexusResult<url::Url> {
    let parsed = url::Url::parse(raw).map_err(|_| NexusError::new("provider.invalid_url"))?;
    match parsed.scheme() {
        "https" => {}
        "http" => {
            let host_str = parsed.host_str().unwrap_or("");
            let is_loopback = host_str.eq_ignore_ascii_case("localhost")
                || host_str == "127.0.0.1"
                || host_str == "::1"
                || host_str == "[::1]";
            if !is_loopback {
                return Err(NexusError::new("provider.plaintext_forbidden"));
            }
        }
        _ => return Err(NexusError::new("provider.invalid_scheme")),
    }

    if let Some(host) = parsed.host_str() {
        if host.eq_ignore_ascii_case("metadata.google.internal")
            || host == "169.254.169.254"
            || host == "100.100.100.200"
        {
            return Err(NexusError::new("provider.metadata_forbidden"));
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
            if ip.is_loopback() {
                // loopback is allowed
            } else if match ip {
                IpAddr::V4(v4) => v4.is_link_local(),
                IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) == 0xfe80,
            } {
                return Err(NexusError::new("provider.metadata_forbidden"));
            }
        }
    }

    Ok(parsed)
}

/// Performs a read-only reachability and credential check for a provider endpoint.
pub fn check_provider_reachability(
    provider_id: &str,
    base_url: Option<&str>,
    secret: Option<&[u8]>,
) -> ProviderReachabilityResult {
    let Some(raw_url) = base_url else {
        return ProviderReachabilityResult {
            provider_id: provider_id.to_string(),
            status: "notConfigured".to_string(),
            http_status: None,
            latency_ms: None,
            message: "No endpoint URL configured".to_string(),
            models_discovered: vec![],
        };
    };

    let target_url = match validate_provider_url(raw_url) {
        Ok(u) => {
            let mut s = u.to_string();
            if !s.ends_with('/') {
                s.push('/');
            }
            if s.ends_with("/v1/") {
                format!("{s}models")
            } else {
                format!("{s}v1/models")
            }
        }
        Err(e) => {
            return ProviderReachabilityResult {
                provider_id: provider_id.to_string(),
                status: "invalidEndpoint".to_string(),
                http_status: None,
                latency_ms: None,
                message: e.code.to_string(),
                models_discovered: vec![],
            };
        }
    };

    let config = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .max_redirects(0)
        .build();
    let agent = config.new_agent();

    let mut req = agent
        .get(&target_url)
        .header("User-Agent", "Project-Nexus/0.2.2")
        .header("Accept", "application/json");

    if let Some(token_bytes) = secret {
        if let Ok(token_str) = std::str::from_utf8(token_bytes) {
            req = req.header("Authorization", &format!("Bearer {}", token_str.trim()));
        }
    }

    let started = Instant::now();
    match req.call() {
        Ok(response) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            let status_code = response.status().as_u16();
            let mut body_bytes = Vec::new();
            let _ = response
                .into_body()
                .into_reader()
                .take(65536)
                .read_to_end(&mut body_bytes);

            let mut models_discovered = Vec::new();
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                    for item in data.iter().take(64) {
                        if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                            let clean: String = redact(id).chars().take(64).collect();
                            if !clean.is_empty() {
                                models_discovered.push(clean);
                            }
                        }
                    }
                } else if let Some(items) = val.get("models").and_then(|m| m.as_array()) {
                    for item in items.iter().take(64) {
                        if let Some(name) = item
                            .get("name")
                            .and_then(|n| n.as_str())
                            .or_else(|| item.as_str())
                        {
                            let clean: String = redact(name).chars().take(64).collect();
                            if !clean.is_empty() {
                                models_discovered.push(clean);
                            }
                        }
                    }
                }
            }

            ProviderReachabilityResult {
                provider_id: provider_id.to_string(),
                status: "connected".to_string(),
                http_status: Some(status_code),
                latency_ms: Some(latency_ms),
                message: "Endpoint reachable and verified".to_string(),
                models_discovered,
            }
        }
        Err(err) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            let (status, http_status, message) = match err {
                ureq::Error::StatusCode(code) => {
                    if code == 401 || code == 403 {
                        ("authFailed", Some(code), format!("HTTP {code}"))
                    } else {
                        ("unreachable", Some(code), format!("HTTP {code}"))
                    }
                }
                other => ("unreachable", None, redact(&other.to_string())),
            };
            ProviderReachabilityResult {
                provider_id: provider_id.to_string(),
                status: status.to_string(),
                http_status,
                latency_ms: Some(latency_ms),
                message,
                models_discovered: vec![],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_http_is_permitted() {
        assert!(validate_provider_url("http://127.0.0.1:11434").is_ok());
        assert!(validate_provider_url("http://localhost:8000/v1").is_ok());
        assert!(validate_provider_url("http://[::1]:9999").is_ok());
    }

    #[test]
    fn public_http_is_rejected() {
        let err = validate_provider_url("http://api.openai.com/v1").unwrap_err();
        assert_eq!(err.code, "provider.plaintext_forbidden");
        let err = validate_provider_url("http://example.com/models").unwrap_err();
        assert_eq!(err.code, "provider.plaintext_forbidden");
    }

    #[test]
    fn https_public_is_permitted() {
        assert!(validate_provider_url("https://api.openai.com/v1").is_ok());
        assert!(validate_provider_url("https://api.anthropic.com").is_ok());
    }

    #[test]
    fn metadata_and_link_local_are_rejected() {
        let err = validate_provider_url("http://169.254.169.254/latest/meta-data").unwrap_err();
        assert_eq!(err.code, "provider.plaintext_forbidden"); // also fails loopback
        let err = validate_provider_url("https://169.254.169.254/latest").unwrap_err();
        assert_eq!(err.code, "provider.metadata_forbidden");
        let err = validate_provider_url("https://metadata.google.internal/computeMetadata/v1")
            .unwrap_err();
        assert_eq!(err.code, "provider.metadata_forbidden");
    }

    #[test]
    fn unreachable_loopback_returns_structured_result_without_panic() {
        // Port 59199 is not listening
        let res = check_provider_reachability("test_prov", Some("http://127.0.0.1:59199"), None);
        assert_eq!(res.provider_id, "test_prov");
        assert_eq!(res.status, "unreachable");
        assert!(res.latency_ms.is_some());
        assert!(res.models_discovered.is_empty());
    }
}
