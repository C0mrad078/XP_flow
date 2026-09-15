//! Secret redaction (section 86) — deliberately duplicated from the
//! desktop crate's `infrastructure::auth::redact` rather than shared via a
//! workspace dependency: the broker and desktop are independently
//! deployable services on purpose (see README.md), and this is a ~20-line
//! pure function, not enough shared surface to justify coupling their
//! release cycles together.

const SENSITIVE_KEYS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "client_secret",
    "app_secret",
    "code",
    "code_verifier",
    "authorization",
    "token",
];

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_sensitive_fields_anywhere_in_the_tree() {
        let mut value = json!({
            "connection": { "access_token": "abc", "refresh_token": "def" },
            "provider": "tiktok",
        });
        redact_json(&mut value);
        assert_eq!(value["connection"]["access_token"], "[redacted]");
        assert_eq!(value["connection"]["refresh_token"], "[redacted]");
        assert_eq!(value["provider"], "tiktok");
    }
}
