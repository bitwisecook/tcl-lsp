//! Locate and rewrite the master-key-encrypted secret values in a config.
//!
//! Exposes [`rewrite_secrets`], [`ENCRYPTED_SECRET_KEYS`], and
//! [`SecretRewriteReport`]. This is a text-level rewrite — comments,
//! whitespace, and ordering survive — and it stays free of any crypto
//! dependency: the caller (`f5 encrypt-secrets` / `f5 decrypt-secrets`) supplies
//! the actual cipher as a `transform` closure.
//!
//! # Which keys
//!
//! [`ENCRYPTED_SECRET_KEYS`] is a deliberately tighter set than the redactor's
//! `_SECRET_KEYS`:
//!
//! - Redaction also blanks SNMP community strings and monitor receive strings,
//!   but those are kept in clear text on the device and are never master-key
//!   encrypted, so feeding them through the cipher would corrupt the config.
//! - `encrypted-password` is intentionally absent: in `auth user` stanzas it
//!   holds the operating-system crypt(3) hash (`$6$…`), not a master-key `$M$`
//!   secret, so encrypting it would wreck the local-user credential. (The APM
//!   `*-encrypted-password` variants are prefixed and so never match the
//!   word-boundary key pattern.)
//!
//! `SNMPv3` secrets are `auth-password` / `privacy-password` — the names the F5
//! SNMP registry uses for trap and user stanzas (NOT `priv-password`).

use regex::Regex;

/// BIG-IP fields whose values it stores encrypted under the unit master key
/// (the `$M$...` envelope produced by `f5mku`).
pub const ENCRYPTED_SECRET_KEYS: [&str; 6] = [
    "passphrase",
    "password",
    "secret",
    "shared-secret",
    "auth-password",
    "privacy-password",
];

/// Values BIG-IP uses as a sentinel for "no secret set" — never crypto.
const SECRET_SENTINELS: [&str; 2] = ["none", "<REDACTED>"];

/// The outcome of a [`rewrite_secrets`] pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRewriteReport {
    /// Number of secret values `transform` actually rewrote.
    pub transformed: usize,
    /// Number of secret-bearing values left untouched (`transform` returned
    /// `None` — e.g. already in the target form, or a `none` sentinel).
    pub skipped: usize,
    /// The rewritten source text.
    pub new_source: String,
}

/// Build the per-key matcher: word-boundary KEY, the run of spaces/tabs between
/// key and value, then the value token (a double-quoted string or a run of
/// non-whitespace, non-brace characters). Group 1 = key, 2 = gap, 3 = value.
///
/// The `regex` crate has no look-around, so the caller checks the preceding
/// byte manually to emulate the `(?<![\w-])` boundary.
fn key_pattern(key: &str) -> Regex {
    // `"(?:[^"\\]|\\.)*"|[^\s{}]+` — quoted string OR bare token.
    let value = r#""(?:[^"\\]|\\.)*"|[^\s{}]+"#;
    Regex::new(&format!(r"({})([ \t]+)({value})", regex::escape(key))).expect("valid secret regex")
}

/// For ASCII config text the key-boundary guard `(?<![\w-])` means the
/// preceding byte must not be a word byte (`[A-Za-z0-9_]`) or a hyphen (keeps
/// `admin-encrypted-password` from matching `password`).
fn key_boundary_ok(bytes: &[u8], start: usize) -> bool {
    start == 0 || {
        let b = bytes[start - 1];
        !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }
}

/// Return `(value, was_quoted)` for a secret-value `token`.
fn unquote(token: &str) -> (String, bool) {
    let bytes = token.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        let inner = &token[1..token.len() - 1];
        (inner.replace("\\\"", "\"").replace("\\\\", "\\"), true)
    } else {
        (token.to_owned(), false)
    }
}

/// Render `value` back as a SCF token, quoting when it needs it.
fn requote(value: &str, was_quoted: bool) -> String {
    let needs_quote =
        was_quoted || value.is_empty() || value.chars().any(|c| " \t\"{}#;".contains(c));
    if !needs_quote {
        return value.to_owned();
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Apply `transform` to every encrypted-secret-bearing value in `source`.
///
/// For each `<key> <value>` whose key is in [`ENCRYPTED_SECRET_KEYS`],
/// `transform` is called with the decoded value and returns the replacement
/// value, or `None` to leave it untouched. The literals `none` / `<REDACTED>`
/// are always skipped without calling `transform`.
pub fn rewrite_secrets<F>(source: &str, mut transform: F) -> SecretRewriteReport
where
    F: FnMut(&str) -> Option<String>,
{
    let mut transformed = 0usize;
    let mut skipped = 0usize;
    let mut out = source.to_owned();

    for key in ENCRYPTED_SECRET_KEYS {
        let re = key_pattern(key);
        let mut result = String::with_capacity(out.len());
        let mut last = 0usize;
        let bytes = out.as_bytes();
        for caps in re.captures_iter(&out) {
            let whole = caps.get(0).expect("group 0");
            // Emulate the `(?<![\w-])` look-behind: skip a match whose key is
            // glued to a preceding word byte / hyphen (e.g. the `password` tail
            // of `admin-encrypted-password`).
            if !key_boundary_ok(bytes, whole.start()) {
                continue;
            }
            result.push_str(&out[last..whole.start()]);
            let key_tok = caps.get(1).expect("key group").as_str();
            let gap = caps.get(2).expect("gap group").as_str();
            let value_tok = caps.get(3).expect("value group").as_str();
            let (value, was_quoted) = unquote(value_tok);
            let replacement = if SECRET_SENTINELS.contains(&value.as_str()) {
                None
            } else {
                transform(&value)
            };
            if let Some(new_value) = replacement {
                transformed += 1;
                result.push_str(key_tok);
                result.push_str(gap);
                result.push_str(&requote(&new_value, was_quoted));
            } else {
                skipped += 1;
                result.push_str(whole.as_str());
            }
            last = whole.end();
        }
        result.push_str(&out[last..]);
        out = result;
    }

    SecretRewriteReport {
        transformed,
        skipped,
        new_source: out,
    }
}

/// One secret-bearing field found in a config, with the object it lives in.
///
/// `value` is the raw stored value — an `$M$…` ciphertext when `encrypted`, or
/// the clear text once the config has been run through a decrypting
/// [`rewrite_secrets`] pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundSecret {
    /// The enclosing top-level stanza header, e.g. `sys file ssl-key /Common/x`.
    pub object: String,
    /// The secret field name (one of [`ENCRYPTED_SECRET_KEYS`]).
    pub field: &'static str,
    /// The stored value (ciphertext or, post-decryption, clear text).
    pub value: String,
    /// Whether `value` is still an `f5mku` `$M$…` ciphertext.
    pub encrypted: bool,
}

impl FoundSecret {
    /// Serialise to the `{object, field, value, encrypted}` JSON the report's
    /// Secrets tab consumes.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "object": self.object,
            "field": self.field,
            "value": self.value,
            "encrypted": self.encrypted,
        })
    }
}

/// Inventory every secret-bearing field in `source`, tagged with the object it
/// lives in and whether it is still encrypted.
///
/// Powers the report's Secrets tab: it lists *what* secrets exist regardless of
/// whether a master key was supplied. The scan tracks the enclosing top-level
/// stanza header so each secret shows which object it belongs to. It shares the
/// [`ENCRYPTED_SECRET_KEYS`] / [`SECRET_SENTINELS`] policy with
/// [`rewrite_secrets`], so the two always agree on what counts as a secret.
#[must_use]
pub fn collect_secrets(source: &str) -> Vec<FoundSecret> {
    let mut out = Vec::new();
    let mut depth: usize = 0;
    let mut object = String::new();

    for raw_line in source.lines() {
        let line = raw_line.trim();

        if depth >= 1
            && let Some((field, value)) = match_secret_line(line)
            && !SECRET_SENTINELS.contains(&value.as_str())
        {
            let encrypted = value.starts_with("$M$");
            out.push(FoundSecret {
                object: object.clone(),
                field,
                value,
                encrypted,
            });
        }

        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        if depth == 0 && opens > closes {
            object.clear();
            object.push_str(line.split('{').next().unwrap_or("").trim());
        }
        depth = (depth + opens).saturating_sub(closes);
        if depth == 0 {
            object.clear();
        }
    }
    out
}

/// If `line` is `<secret-key> <value>`, return `(key, value)` unquoted.
fn match_secret_line(line: &str) -> Option<(&'static str, String)> {
    for &key in &ENCRYPTED_SECRET_KEYS {
        if let Some(rest) = line.strip_prefix(key)
            && rest.starts_with([' ', '\t'])
        {
            let value = rest.trim();
            if value.is_empty() {
                return None;
            }
            return Some((key, unquote_secret(value)));
        }
    }
    None
}

/// Strip surrounding double quotes (with backslash unescaping) from a token,
/// else take the value up to the first whitespace.
fn unquote_secret(tok: &str) -> String {
    if tok.len() >= 2 && tok.starts_with('"') && tok.ends_with('"') {
        tok[1..tok.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        tok.split_whitespace().next().unwrap_or(tok).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &str = "ltm monitor https /Common/mon {\n    password clearpw\n    username admin\n}\nauth radius-server /Common/rad {\n    secret \"my radius secret\"\n}\nsys snmp {\n    communities {\n        public { community-name public }\n    }\n}\n";

    #[test]
    fn only_touches_encrypted_secret_keys() {
        let report = rewrite_secrets(CONF, |v| Some(format!("<{v}>")));
        // password + secret are rewritten; the SNMP community-name and the
        // username are not.
        assert_eq!(report.transformed, 2);
        assert!(report.new_source.contains("password <clearpw>"));
        assert!(report.new_source.contains("<my radius secret>"));
        assert!(report.new_source.contains("community-name public"));
        assert!(report.new_source.contains("username admin"));
    }

    #[test]
    fn skips_sentinels() {
        let report = rewrite_secrets("password none\nsecret <REDACTED>\n", |_| {
            Some("X".to_owned())
        });
        assert_eq!(report.transformed, 0);
        assert_eq!(report.skipped, 2);
    }

    #[test]
    fn transform_none_is_left_untouched() {
        let report = rewrite_secrets(CONF, |_| None);
        assert_eq!(report.transformed, 0);
        assert_eq!(report.new_source, CONF);
    }

    #[test]
    fn ignores_auth_user_crypt_hash() {
        // `auth user ... encrypted-password $6$...` is an OS crypt hash, not an
        // f5mku $M$ secret — it must not be in the encrypted-secret key set.
        let conf = "auth user /Common/admin {\n    encrypted-password $6$abc123$def456\n}\n";
        let report = rewrite_secrets(conf, |v| Some(format!("<{v}>")));
        assert_eq!(report.transformed, 0);
        assert_eq!(report.new_source, conf);
    }

    #[test]
    fn handles_snmpv3_privacy_password() {
        let conf = "sys snmp {\n    users {\n        u1 {\n            auth-password authpw\n            privacy-password privpw\n        }\n    }\n}\n";
        let report = rewrite_secrets(conf, |v| Some(format!("<{v}>")));
        assert_eq!(report.transformed, 2);
        assert!(report.new_source.contains("auth-password <authpw>"));
        assert!(report.new_source.contains("privacy-password <privpw>"));
    }

    #[test]
    fn requotes_when_needed() {
        // A value with spaces is re-quoted; a bare one stays bare.
        let report = rewrite_secrets("password bare\n", |_| Some("has space".to_owned()));
        assert!(report.new_source.contains("password \"has space\""));
    }
}
