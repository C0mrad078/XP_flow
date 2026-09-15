use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// RFC 7636 PKCE (Proof Key for Code Exchange) verifier/challenge pair.
/// `verifier` is a cryptographically random 43-128 character string sent
/// only at token-exchange time; `challenge` is `BASE64URL(SHA256(verifier))`
/// (no padding) sent at the *authorization* step, per S256 (section 6/section
/// 11 of the Phase 4 brief).
#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// Generates a fresh, cryptographically secure PKCE pair — never reused
/// across authorization attempts (section 6).
pub fn generate_pkce() -> Pkce {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = challenge_from_verifier(&verifier);
    Pkce {
        verifier,
        challenge,
    }
}

/// `BASE64URL(SHA256(verifier))`, no padding — the S256 PKCE transform.
///
/// This is the canonical, correct implementation every provider strategy
/// delegates to today. It is kept as its own function (rather than inlined
/// into a single "the" OAuth provider) specifically so a future provider
/// whose documented behavior actually diverges from RFC 7636 can override
/// it without touching this one (section 11: "do not assume every provider
/// interprets S256 identically").
pub fn challenge_from_verifier(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Google's PKCE strategy — RFC 7636 S256, as documented for the OAuth
/// installed-app flow.
pub struct GooglePkceStrategy;

impl GooglePkceStrategy {
    pub fn generate() -> Pkce {
        generate_pkce()
    }
}

/// TikTok's PKCE strategy, kept as its own type per section 11 even though
/// TikTok's Login Kit documentation currently specifies the same RFC 7636
/// S256 transform as Google — a provider-specific quirk (encoding,
/// verifier length/charset) can change here independently without risking
/// Google's flow.
pub struct TikTokPkceStrategy;

impl TikTokPkceStrategy {
    pub fn generate() -> Pkce {
        generate_pkce()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7636 Appendix B's official S256 test vector.
    #[test]
    fn matches_the_rfc_7636_appendix_b_test_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(challenge_from_verifier(verifier), expected_challenge);
    }

    #[test]
    fn generated_pairs_are_never_reused() {
        let a = generate_pkce();
        let b = generate_pkce();
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.challenge, b.challenge);
    }

    #[test]
    fn verifier_length_is_within_rfc_bounds() {
        let pkce = generate_pkce();
        assert!(pkce.verifier.len() >= 43 && pkce.verifier.len() <= 128);
    }

    #[test]
    fn challenge_is_deterministic_for_a_given_verifier() {
        let pkce = generate_pkce();
        assert_eq!(challenge_from_verifier(&pkce.verifier), pkce.challenge);
    }

    #[test]
    fn google_and_tiktok_strategies_both_produce_rfc_compliant_pairs() {
        let google = GooglePkceStrategy::generate();
        let tiktok = TikTokPkceStrategy::generate();
        assert_eq!(challenge_from_verifier(&google.verifier), google.challenge);
        assert_eq!(challenge_from_verifier(&tiktok.verifier), tiktok.challenge);
    }
}
