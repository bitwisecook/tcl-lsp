//! UCS (BIG-IP backup archive) handling, including encrypted archives.
//!
//! Faithful port of the pure-Python parts of `tooling/f5/f5_remote/ucs.py`. A
//! UCS file is a gzip-compressed tar of a snapshot of `/config`; this module is
//! the inverse of `tmsh load sys ucs` — it reassembles a single SCF text by
//! concatenating the relevant `bigip*.conf` members in a deterministic order.
//!
//! BIG-IP can also encrypt a UCS with a passphrase (`tmsh save sys ucs <name>
//! passphrase <pass>`). Per F5 KB K5437 the archive is then a `GnuPG` *symmetric*
//! `OpenPGP` message whose plaintext is the ordinary gzip tar. Unlike the Python
//! (which prefers shelling out to `gpg`), this port decrypts entirely in pure
//! Rust via [`crate::openpgp`] — and, like the Python, the decrypted UCS lives
//! only in memory: decrypt → gunzip → untar all happen on in-memory cursors, so
//! the cleartext (a UCS routinely holds SSL private keys) never touches disk.

use std::io::{Cursor, Read};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::openpgp::decrypt_symmetric;

/// Order matters: base must come first so partition declarations exist before
/// objects that reference them (mirrors `_SCF_MEMBER_ORDER`).
const SCF_MEMBER_ORDER: &[&str] = &[
    "config/bigip_base.conf",
    "config/bigip.conf",
    "config/bigip_gtm.conf",
    "config/bigip_user.conf",
    "config/bigip_script.conf",
];

/// Default environment variable consulted for a UCS decryption passphrase.
pub const DEFAULT_PASSPHRASE_ENV: &str = "F5_UCS_PASSPHRASE";

/// A UCS error, carrying the human-facing message. Mirrors the Python
/// `UcsPassphraseError` / `UcsDecryptionError` / `ValueError` surface, all of
/// which the CLI renders as `error: {msg}`; here a single type carries the
/// matching text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct UcsError(pub String);

impl UcsError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        UcsError(msg.into())
    }
}

/// A zero-argument provider that yields the passphrase on demand. Used so the
/// (possibly interactive / env-var) resolution happens lazily — only when an
/// encrypted archive is actually encountered.
pub type PassphraseProvider<'a> = dyn Fn() -> Result<String, UcsError> + 'a;

/// Return true if `data` looks like a plain UCS archive (gzip magic).
#[must_use]
pub fn is_ucs_bytes(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1F && data[1] == 0x8B
}

/// Return true if `data` looks like an `OpenPGP` message (encrypted UCS).
///
/// Recognises both ASCII-armored messages and the binary form BIG-IP emits: an
/// `OpenPGP` packet header whose tag begins an encrypted message. An F5 encrypted
/// UCS starts with a Symmetric-Key Encrypted Session Key packet (tag 3 → first
/// byte `0x8C`).
#[must_use]
pub fn is_pgp_bytes(data: &[u8]) -> bool {
    let prefix_len = data.len().min(64);
    let trimmed = {
        let p = &data[..prefix_len];
        let start = p
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(p.len());
        &p[start..]
    };
    if trimmed.starts_with(b"-----BEGIN PGP") {
        return true;
    }
    let Some(&first) = data.first() else {
        return false;
    };
    if first & 0x80 == 0 {
        return false; // not an OpenPGP packet header
    }
    let tag = if first & 0x40 != 0 {
        first & 0x3F
    } else {
        (first >> 2) & 0x0F
    };
    // 1 = PKESK, 3 = SKESK, 9 = SED, 18 = SEIPD.
    matches!(tag, 1 | 3 | 9 | 18)
}

/// Strip leading `.`/`/` characters (mirrors Python `str.lstrip("./")`).
fn lstrip_dot_slash(name: &str) -> &str {
    name.trim_start_matches(['.', '/'])
}

/// Read every regular-file member of a gzip-tar into `(name, bytes)`, with the
/// leading `./` / `/` stripped from each name. Members keep archive order.
fn read_members(ucs_bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, UcsError> {
    let gz = GzDecoder::new(Cursor::new(ucs_bytes));
    let mut archive = Archive::new(gz);
    let entries = archive
        .entries()
        .map_err(|e| UcsError::new(format!("invalid UCS archive: {e}")))?;
    let mut members = Vec::new();
    for entry in entries {
        let mut entry = entry.map_err(|e| UcsError::new(format!("invalid UCS archive: {e}")))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .map_err(|e| UcsError::new(format!("invalid UCS archive: {e}")))?
            .to_string_lossy()
            .into_owned();
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|e| UcsError::new(format!("invalid UCS archive: {e}")))?;
        members.push((lstrip_dot_slash(&name).to_owned(), data));
    }
    Ok(members)
}

/// Extract `ucs_bytes` and return a concatenated SCF text.
///
/// When `include_extras` is true, any additional `config/*.conf` files not in
/// the canonical order are appended at the end (in deterministic alphabetical
/// order). Members not present in the archive are silently skipped.
pub fn ucs_to_scf(ucs_bytes: &[u8], include_extras: bool) -> Result<String, UcsError> {
    let members = read_members(ucs_bytes)?;
    // Build a name → last-occurrence index map (Python dict-comprehension
    // semantics: a later member with the same name wins).
    let mut by_name: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, (name, _)) in members.iter().enumerate() {
        by_name.insert(name.as_str(), i);
    }

    let decode = |bytes: &[u8]| -> String { String::from_utf8_lossy(bytes).trim_end().to_owned() };

    let mut chunks: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for &canonical in SCF_MEMBER_ORDER {
        if let Some(&idx) = by_name.get(canonical) {
            let text = decode(&members[idx].1);
            chunks.push(format!("#\n# {canonical}\n#\n{text}\n"));
            seen.insert(canonical);
        }
    }

    if include_extras {
        let mut names: Vec<&str> = by_name.keys().copied().collect();
        names.sort_unstable();
        for name in names {
            if seen.contains(name) {
                continue;
            }
            // Case-sensitive `.conf` match, mirroring Python's `endswith(".conf")`.
            #[allow(clippy::case_sensitive_file_extension_comparisons)]
            if !name.starts_with("config/") || !name.ends_with(".conf") {
                continue;
            }
            let idx = by_name[name];
            let text = decode(&members[idx].1);
            chunks.push(format!("#\n# {name}\n#\n{text}\n"));
        }
    }

    Ok(chunks.join("\n"))
}

/// Return plaintext UCS bytes, decrypting `raw` first if it is `OpenPGP`.
///
/// Plain (gzip) input is returned unchanged. Encrypted input is decrypted via
/// the pure-Rust [`decrypt_symmetric`], resolving the passphrase through
/// `provider`. The result is validated to be a gzip archive so a wrong
/// passphrase that somehow slips past the integrity check still errors clearly.
pub fn decrypt_if_encrypted(
    raw: &[u8],
    provider: &PassphraseProvider<'_>,
    label: &str,
) -> Result<Vec<u8>, UcsError> {
    if !is_pgp_bytes(raw) {
        return Ok(raw.to_vec());
    }
    let passphrase = provider()?;
    let plaintext =
        decrypt_symmetric(raw, passphrase.as_bytes()).map_err(|e| UcsError::new(e.to_string()))?;
    if !is_ucs_bytes(&plaintext) {
        return Err(UcsError::new(format!(
            "{label}: decrypted data is not a UCS archive (gzip magic missing) — wrong passphrase?"
        )));
    }
    Ok(plaintext)
}

/// Turn raw UCS bytes — encrypted or plain — into SCF text.
///
/// Convenience wrapper that decrypts (when needed) and then extracts, keeping
/// the decrypted archive entirely in memory.
pub fn ucs_archive_to_scf(
    raw: &[u8],
    provider: &PassphraseProvider<'_>,
    include_extras: bool,
    label: &str,
) -> Result<String, UcsError> {
    let data = decrypt_if_encrypted(raw, provider, label)?;
    if !is_ucs_bytes(&data) {
        return Err(UcsError::new(format!("{label}: not a valid UCS archive")));
    }
    ucs_to_scf(&data, include_extras)
}

/// Return the bytes of a single file member from a UCS archive.
///
/// Decrypts `raw` first when it is OpenPGP-encrypted, then extracts the member
/// named by `member_path` — typically a BIG-IP filestore `cache-path`. Matching
/// is tolerant: the leading `/` is optional, and the `:partition:` filename
/// prefix is matched even when one side omits it. Returns [`UcsError`] when no
/// member matches or the archive is corrupt.
pub fn read_ucs_member(
    raw: &[u8],
    member_path: &str,
    provider: &PassphraseProvider<'_>,
    label: &str,
) -> Result<Vec<u8>, UcsError> {
    let data = decrypt_if_encrypted(raw, provider, label)?;
    if !is_ucs_bytes(&data) {
        return Err(UcsError::new(format!("{label}: not a valid UCS archive")));
    }
    let want = lstrip_dot_slash(member_path.trim_start_matches('/'));
    let base = want.rsplit('/').next().unwrap_or(want);
    // The `:partition:` prefix is informational for matching — compare the
    // bare leaf so a stanza cache-path and the on-disk filename agree.
    let leaf = base.rsplit(':').next().unwrap_or(base);

    let members = read_members(&data)?;
    // Exact match first.
    if let Some((_, bytes)) = members.iter().find(|(n, _)| n == want) {
        return Ok(bytes.clone());
    }
    let mut candidates: Vec<&(String, Vec<u8>)> = members
        .iter()
        .filter(|(n, _)| {
            let mbase = n.rsplit('/').next().unwrap_or(n);
            mbase == base || mbase.rsplit(':').next().unwrap_or(mbase) == leaf
        })
        .collect();
    if candidates.len() > 1 {
        // Prefer a filestore path so a same-named pair in different stores
        // doesn't collide.
        let narrowed: Vec<&(String, Vec<u8>)> = candidates
            .iter()
            .copied()
            .filter(|(n, _)| n.contains("/filestore/"))
            .collect();
        if !narrowed.is_empty() {
            candidates = narrowed;
        }
    }
    match candidates.first() {
        Some((_, bytes)) => Ok(bytes.clone()),
        None => Err(UcsError::new(format!(
            "member not found in UCS archive: {member_path}"
        ))),
    }
}
