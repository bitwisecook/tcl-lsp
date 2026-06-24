//! Secure interactive terminal prompts for the CLIs.
//!
//! Reads a secret (e.g. a UCS decryption passphrase) from the controlling
//! terminal without echoing it, via [`rpassword`] — which restores terminal
//! echo on every exit path and falls back to `/dev/tty` when stdin is
//! redirected. Returns `Err` when there is no usable terminal, so callers fall
//! back to a non-interactive error rather than hanging.

/// Securely read the UCS decryption passphrase from the terminal (no echo). The
/// prompt text matches `getpass` provider (`"Enter UCS passphrase: "`).
///
/// # Errors
/// Returns the underlying I/O error string when there is no controlling
/// terminal or the read fails — the caller then falls through to a
/// non-interactive error.
pub fn read_ucs_passphrase() -> Result<String, String> {
    rpassword::prompt_password("Enter UCS passphrase: ").map_err(|e| e.to_string())
}
