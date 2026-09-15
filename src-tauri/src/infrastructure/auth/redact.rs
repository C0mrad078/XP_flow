/// Centralized secret redaction (section 86). Every place a provider/
/// broker payload might end up in a log line, an `Activity` event, or a
/// diagnostics export must pass through this first — never format a raw
/// token/secret/code/verifier by hand at the call site, since that's
/// exactly the kind of one-off that leaks under a signature change no one
/// notices.
const SENSITIVE_KEYS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "client_secret",
    "code",
    "code_verifier",
    "authorization",
    "app_secret",
    "token",
];

/// Redacts known-sensitive keys out of a flat JSON object, in place.
/// Nested objects/arrays are walked too. Non-object/array values are
/// returned unchanged.
pub fn redact_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, entry) in map.iter_mut() {
                let lower = key.to_ascii_lowercase();
                if SENSITIVE_KEYS.iter().any(|k| lower.contains(k)) {
                    *entry = serde_json::Value::String("[redacted]".to_string());
                } else {
                    redact_json(entry);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                redact_json(item);
            }
        }
        _ => {}
    }
}

/// Redacts a bearer/Authorization-style header value for logging, keeping
/// only enough to distinguish "present" from "absent" in a support log.
pub fn redact_secret_str(value: &str) -> String {
    if value.is_empty() {
        return "[empty]".to_string();
    }
    "[redacted]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_top_level_sensitive_fields() {
        let mut value = json!({
            "access_token": "secret-value",
            "refresh_token": "another-secret",
            "expires_in": 3600,
        });
        redact_json(&mut value);
        assert_eq!(value["access_token"], "[redacted]");
        assert_eq!(value["refresh_token"], "[redacted]");
        assert_eq!(value["expires_in"], 3600);
    }

    #[test]
    fn redacts_nested_sensitive_fields() {
        let mut value = json!({
            "data": { "client_secret": "shh", "provider": "tiktok" }
        });
        redact_json(&mut value);
        assert_eq!(value["data"]["client_secret"], "[redacted]");
        assert_eq!(value["data"]["provider"], "tiktok");
    }

    #[test]
    fn redacts_within_arrays() {
        let mut value = json!([{ "code_verifier": "verifier-value" }]);
        redact_json(&mut value);
        assert_eq!(value[0]["code_verifier"], "[redacted]");
    }

    #[test]
    fn leaves_non_sensitive_values_untouched() {
        let mut value = json!({ "display_name": "Football Cuts", "scopes": ["a", "b"] });
        redact_json(&mut value);
        assert_eq!(value["display_name"], "Football Cuts");
        assert_eq!(value["scopes"], json!(["a", "b"]));
    }

    #[test]
    fn redact_secret_str_never_echoes_the_value() {
        let redacted = redact_secret_str("Bearer abc123");
        assert!(!redacted.contains("abc123"));
    }
}
