//! Shared secret-input resolution for the native `tcl` / `f5` CLIs.
//!
//! A single, idiomatic way to obtain a secret (passphrase, master key, password, …) from
//! the four sources the CLI supports, tried in a fixed order — the first
//! non-empty one wins:
//!
//! 1. an explicit value (e.g. a `--f5mku` flag),
//! 2. a file (e.g. `--f5mku-file key.txt`),
//! 3. an environment variable (e.g. `$F5MKU`), and
//! 4. a secure, no-echo terminal prompt via [`rpassword`].
//!
//! Centralising this keeps the tty detection correct in one place — the prompt
//! is offered whenever *either* stdin or stderr is a terminal, and `rpassword`
//! itself reads from the controlling `/dev/tty`, so a prompt still works when
//! the config is piped in on stdin (`f5 decrypt-secrets -`).
//! [`resolve_secret`] returns `Ok(None)` when no source yields a value, leaving
//! the caller to decide whether that is an error; a cancelled prompt (Ctrl-C /
//! EOF) returns [`SecretInputError`].

use std::io::IsTerminal;
use std::path::Path;

/// Raised when interactive secret entry is cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretInputError(pub String);

impl std::fmt::Display for SecretInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SecretInputError {}

/// The sources [`resolve_secret`] consults, in priority order.
#[derive(Debug, Default, Clone)]
pub struct SecretRequest<'a> {
    /// An explicit value (highest priority, e.g. a `--f5mku` flag).
    pub explicit: Option<&'a str>,
    /// A file to read the secret from (e.g. `--f5mku-file key.txt`).
    pub file: Option<&'a Path>,
    /// Environment variable consulted when no explicit value or file resolved.
    pub env_var: Option<&'a str>,
    /// Whether a secure terminal prompt is permitted as the last resort.
    pub allow_prompt: bool,
    /// Prompt text shown when prompting (e.g. `"F5 MKU Key: "`). `None` disables
    /// prompting regardless of `allow_prompt`.
    pub prompt: Option<&'a str>,
    /// Whitespace-trim the resolved value. Leave `false` for passphrases /
    /// passwords (surrounding whitespace can be significant); set `true` for
    /// tokens that never carry whitespace (a base64 key read from
    /// `f5mku -K > key.txt` picks up a trailing newline).
    pub strip: bool,
}

/// Whether a controlling terminal is available to prompt on: true when *either*
/// stdin or stderr is a terminal.
#[must_use]
pub fn has_tty() -> bool {
    std::io::stdin().is_terminal() || std::io::stderr().is_terminal()
}

fn norm(value: &str, strip: bool) -> String {
    if strip {
        value.trim().to_owned()
    } else {
        value.to_owned()
    }
}

/// Resolve a secret from an explicit value, a file, an env var, or a prompt.
///
/// Sources are tried in that fixed order and the first non-empty one wins.
/// Returns `Ok(None)` when nothing yields a value (the caller decides whether
/// that is fatal).
///
/// # Errors
/// Returns [`SecretInputError`] when a file cannot be read, or an interactive
/// prompt is cancelled (Ctrl-C / EOF).
pub fn resolve_secret(req: &SecretRequest) -> Result<Option<String>, SecretInputError> {
    if let Some(explicit) = req.explicit
        && !explicit.is_empty()
    {
        return Ok(Some(norm(explicit, req.strip)));
    }

    if let Some(file) = req.file {
        let text = std::fs::read_to_string(file).map_err(|e| {
            SecretInputError(format!("cannot read secret file {}: {e}", file.display()))
        })?;
        let text = norm(&text, req.strip);
        if !text.is_empty() {
            return Ok(Some(text));
        }
    }

    if let Some(env_var) = req.env_var
        && let Ok(val) = std::env::var(env_var)
        && !val.is_empty()
    {
        return Ok(Some(norm(&val, req.strip)));
    }

    if req.allow_prompt
        && let Some(prompt) = req.prompt
        && has_tty()
    {
        let entered = rpassword::prompt_password(prompt)
            .map_err(|_| SecretInputError("secret entry cancelled".to_owned()))?;
        let entered = norm(&entered, req.strip);
        if !entered.is_empty() {
            return Ok(Some(entered));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests cover the non-prompt resolution order. The prompt branch
    // needs a real controlling terminal (`rpassword` reads `/dev/tty`) and is
    // exercised by the CLI integration tests instead. Env-var mutation is
    // process-global and `unsafe` under the workspace lints, so the env cases go
    // through `temp_env::with_var` (a scoped, panic-safe set/restore).

    #[test]
    fn explicit_wins() {
        temp_env::with_var("SI_EXPLICIT", Some("from-env"), || {
            let req = SecretRequest {
                explicit: Some("from-flag"),
                env_var: Some("SI_EXPLICIT"),
                ..Default::default()
            };
            assert_eq!(resolve_secret(&req).unwrap().as_deref(), Some("from-flag"));
        });
    }

    #[test]
    fn file_before_env() {
        let dir = std::env::temp_dir().join(format!("si-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let keyfile = dir.join("k");
        std::fs::write(&keyfile, "from-file\n").unwrap();
        temp_env::with_var("SI_FILE", Some("from-env"), || {
            let req = SecretRequest {
                file: Some(&keyfile),
                env_var: Some("SI_FILE"),
                strip: true, // trims the trailing newline a key file picks up
                ..Default::default()
            };
            assert_eq!(resolve_secret(&req).unwrap().as_deref(), Some("from-file"));
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_used_when_no_explicit_or_file() {
        temp_env::with_var("SI_ENV", Some("from-env"), || {
            let req = SecretRequest {
                env_var: Some("SI_ENV"),
                ..Default::default()
            };
            assert_eq!(resolve_secret(&req).unwrap().as_deref(), Some("from-env"));
        });
    }

    #[test]
    fn strip_false_preserves_whitespace() {
        temp_env::with_var("SI_WS", Some("  spaced  "), || {
            let req = SecretRequest {
                env_var: Some("SI_WS"),
                strip: false,
                ..Default::default()
            };
            assert_eq!(resolve_secret(&req).unwrap().as_deref(), Some("  spaced  "));
        });
    }

    #[test]
    fn missing_file_is_error() {
        let req = SecretRequest {
            file: Some(Path::new("/nonexistent/si/key.txt")),
            ..Default::default()
        };
        assert!(resolve_secret(&req).is_err());
    }

    #[test]
    fn nothing_resolves_to_none() {
        temp_env::with_var_unset("SI_NONE", || {
            let req = SecretRequest {
                env_var: Some("SI_NONE"),
                allow_prompt: false,
                ..Default::default()
            };
            assert_eq!(resolve_secret(&req).unwrap(), None);
        });
    }
}
