// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! UCS (BIG-IP backup archive) handling, including encrypted archives.
//!
//! A UCS file is a gzip-compressed tar of a snapshot of `/config`; this module is
//! the inverse of `tmsh load sys ucs` — it reassembles a single SCF text by
//! concatenating the relevant `bigip*.conf` members in a deterministic order.
//!
//! BIG-IP can also encrypt a UCS with a passphrase (`tmsh save sys ucs <name>
//! passphrase <pass>`). Per F5 KB K5437 the archive is then a `GnuPG` *symmetric*
//! `OpenPGP` message whose plaintext is the ordinary gzip tar. Rather than
//! shelling out to `gpg`, this module decrypts entirely in pure
//! Rust via [`crate::openpgp`] — and the decrypted UCS lives
//! only in memory: decrypt → gunzip → untar all happen on in-memory cursors, so
//! the cleartext (a UCS routinely holds SSL private keys) never touches disk.

use std::io::{Cursor, Read};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::openpgp::decrypt_symmetric;

/// The built-in default profile / monitor definitions (the `/Common` defaults
/// that user objects reference, e.g. `/Common/tcp`, `/Common/http`). Emitted
/// first so those references resolve; present only in full device archives
/// (qkview / a UCS taken with defaults), silently skipped otherwise.
const DEFAULT_MEMBERS: &[&str] = &["config/low_profile_base.conf", "config/profile_base.conf"];

/// Order matters: base must come first so partition declarations exist before
/// objects that reference them. These are the `/Common` (root) members.
///
/// `bigip_script.conf` is deliberately excluded here and emitted *last* (see
/// [`is_script_member`]): it holds `cli script` / iApp templates whose embedded
/// Tcl bodies can carry brace shapes the config parser trips on, and any object
/// *after* such a body in the stream is lost. It contributes no LTM/GTM objects
/// the report projects, so deferring it to the end keeps real objects intact.
const SCF_MEMBER_ORDER: &[&str] = &[
    "config/bigip_base.conf",
    "config/bigip.conf",
    "config/bigip_gtm.conf",
    "config/bigip_user.conf",
];

/// Per-partition member order within `config/partitions/<name>/` — base first,
/// mirroring [`SCF_MEMBER_ORDER`]. Scripts are deferred (see [`is_script_member`]).
const PARTITION_MEMBER_ORDER: &[&str] = &[
    "bigip_base.conf",
    "bigip.conf",
    "bigip_gtm.conf",
    "bigip_user.conf",
];

/// A `cli script` / iApp-template member (`bigip_script.conf`), emitted last so a
/// brace shape its embedded Tcl carries can't swallow real objects downstream.
fn is_script_member(name: &str) -> bool {
    name.ends_with("bigip_script.conf")
}

/// If `name` is `config/partitions/<partition>/<file>`, return `(partition,
/// file)`; otherwise `None`.
fn split_partition_member(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix("config/partitions/")?;
    let (partition, file) = rest.split_once('/')?;
    if partition.is_empty() || file.is_empty() || file.contains('/') {
        return None;
    }
    Some((partition, file))
}

/// Default environment variable consulted for a UCS decryption passphrase.
pub const DEFAULT_PASSPHRASE_ENV: &str = "F5_UCS_PASSPHRASE";

/// A UCS error, carrying the human-facing message. A single type carries the
/// text for every failure mode (bad passphrase, decryption failure, invalid
/// input); the CLI renders it as `error: {msg}`.
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

/// Strip leading `.`/`/` characters.
fn lstrip_dot_slash(name: &str) -> &str {
    name.trim_start_matches(['.', '/'])
}

/// Case-sensitive `.conf` suffix test, mirroring `endswith(".conf")`.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn ends_with_conf(name: &str) -> bool {
    name.ends_with(".conf")
}

/// macOS archive metadata that must never reach the parser: `AppleDouble`
/// resource forks (`._name`), the `__MACOSX/` shadow tree, and `.DS_Store`.
fn is_macos_cruft(name: &str) -> bool {
    let n = lstrip_dot_slash(name);
    n.starts_with("__MACOSX/")
        || n.split('/')
            .next_back()
            .is_some_and(|base| base.starts_with("._") || base == ".DS_Store")
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
        // Skip macOS archive cruft (AppleDouble `._foo` resource forks, the
        // `__MACOSX/` shadow tree, `.DS_Store`) that a UCS zipped/untarred on a
        // Mac carries — their binary bodies would otherwise be spliced into the
        // SCF and corrupt parsing.
        if is_macos_cruft(&name) {
            continue;
        }
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
    // Build a name → last-occurrence index map (last write wins:
    // a later member with the same name wins).
    let mut by_name: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, (name, _)) in members.iter().enumerate() {
        by_name.insert(name.as_str(), i);
    }

    let decode = |bytes: &[u8]| -> String { String::from_utf8_lossy(bytes).trim_end().to_owned() };

    let mut chunks: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let emit =
        |name: &str, chunks: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
            if seen.contains(name) {
                return;
            }
            if let Some(&idx) = by_name.get(name) {
                let text = decode(&members[idx].1);
                chunks.push(format!("#\n# {name}\n#\n{text}\n"));
                seen.insert(name.to_owned());
            }
        };

    // 1. Built-in defaults (profile/monitor bases) so `/Common/tcp` etc. resolve.
    for &canonical in DEFAULT_MEMBERS {
        emit(canonical, &mut chunks, &mut seen);
    }
    // 2. Root (`/Common`) members, base first.
    for &canonical in SCF_MEMBER_ORDER {
        emit(canonical, &mut chunks, &mut seen);
    }
    // 3. Per-partition members. Each partition's base first, then its bigip.conf,
    //    then any remaining `.conf` under it (sorted) — so objects in
    //    non-Common partitions (`/TenantA/...`) are part of the model.
    let mut partitions: Vec<&str> = by_name
        .keys()
        .filter_map(|n| split_partition_member(n).map(|(p, _)| p))
        .collect();
    partitions.sort_unstable();
    partitions.dedup();
    for partition in partitions {
        for &suffix in PARTITION_MEMBER_ORDER {
            let name = format!("config/partitions/{partition}/{suffix}");
            emit(&name, &mut chunks, &mut seen);
        }
        // Any other `.conf` under this partition, deterministically.
        let mut rest: Vec<&str> = by_name
            .keys()
            .copied()
            .filter(|n| {
                split_partition_member(n).is_some_and(|(p, _)| p == partition)
                    && ends_with_conf(n)
                    && !seen.contains(*n)
            })
            .collect();
        rest.sort_unstable();
        for name in rest {
            emit(name, &mut chunks, &mut seen);
        }
    }

    if include_extras {
        let mut names: Vec<&str> = by_name.keys().copied().collect();
        names.sort_unstable();
        for name in names {
            if seen.contains(name)
                || !name.starts_with("config/")
                || !ends_with_conf(name)
                || is_script_member(name)
            {
                continue;
            }
            emit(name, &mut chunks, &mut seen);
        }
    }

    // 4. Scripts (root + per-partition `bigip_script.conf`) last, so their
    //    embedded Tcl can't truncate the parse of any real object.
    let mut scripts: Vec<&str> = by_name
        .keys()
        .copied()
        .filter(|n| is_script_member(n) && !seen.contains(*n))
        .collect();
    scripts.sort_unstable();
    for name in scripts {
        emit(name, &mut chunks, &mut seen);
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

/// Which members [`list_ucs_members`] returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberScope {
    /// Only forensically-relevant members — the OS files an attacker tampers
    /// with for persistence, credential theft or log evasion (see
    /// [`is_forensic_path`]). The default for the report's forensic view.
    Forensic,
    /// Every regular-file member (macOS cruft already stripped). The full
    /// archive tree, for deep inspection.
    FullTree,
}

/// Metadata for one UCS member. Content is fetched separately (via
/// [`read_ucs_member`]) so an inventory stays light — a UCS can hold hundreds
/// of megabytes across thousands of members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcsFileEntry {
    /// Archive path, leading `./` / `/` stripped (e.g. `etc/passwd`).
    pub path: String,
    /// Uncompressed size in bytes.
    pub size: u64,
    /// Lowercase hex SHA-256 of the member's content — lets a reviewer diff a
    /// file against a known-good hash without shipping the bytes.
    pub sha256: String,
    /// Whether the content decodes as UTF-8 with no NUL bytes (i.e. safe to
    /// show as text rather than a binary blob).
    pub is_text: bool,
}

/// Lowercase hex SHA-256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Heuristic: is this content safe to display as text? Valid UTF-8 with no NUL
/// byte. Cheap and good enough to separate config / dotfiles / keys from binary
/// blobs (databases, certs' DER, images).
fn is_text_bytes(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

/// Is `name` a forensically-relevant UCS path?
///
/// These are the OS files outside `/config` that attackers modify on a
/// compromised BIG-IP — the same set the report's UCS forensic checklist maps
/// to MITRE ATT&CK: login-persistence dotfiles and SSH keys, the local account
/// databases, external-auth and PAM config, logging config, and cron. `name`
/// is expected already stripped of a leading `./` / `/` (as [`read_members`]
/// returns it). `bigip*.conf` is intentionally excluded — it is already
/// surfaced as the SCF config, not a forensic artefact.
fn is_forensic_path(name: &str) -> bool {
    // Directory subtrees commonly tampered with (prefix match).
    const FORENSIC_PREFIXES: &[&str] = &[
        "etc/cron",       // crontab + cron.d / cron.daily / …
        "etc/syslog-ng/", // T1562.006 logging evasion
        "etc/ssh/",       // sshd_config
        "etc/openldap/",  // T1556 external auth
        "etc/pam.d/",
        "etc/security/",
    ];
    // Individual files commonly tampered with (exact match).
    const FORENSIC_EXACT: &[&str] = &[
        "etc/passwd",
        "etc/shadow",
        "etc/group",
        "etc/gshadow",
        "etc/hosts",
        "etc/hosts.deny",
        "etc/hosts.allow",
        "etc/nsswitch.conf",
        "etc/krb5.conf",
        "etc/resolv.conf",
        "etc/ldap.conf",
        "etc/motd",
        "etc/issue",
    ];

    // SSH material anywhere (authorized_keys, id_*, known_hosts, config) —
    // T1098.004 SSH-key persistence.
    if name.contains("/.ssh/") || name.starts_with(".ssh/") {
        return true;
    }
    // Shell / user dotfiles under a home directory — T1546.004 login hooks.
    let dotfile = (name.starts_with("home/") || name.starts_with("root/"))
        && name
            .rsplit('/')
            .next()
            .is_some_and(|base| base.starts_with('.'));

    dotfile
        || FORENSIC_EXACT.contains(&name)
        || FORENSIC_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// List the members of a UCS archive with per-file metadata (path, size,
/// SHA-256, text/binary), decrypting first when the archive is encrypted.
///
/// `scope` selects the whole tree or just the forensically-relevant subset
/// ([`is_forensic_path`]). Content is not returned — fetch a single member's
/// bytes on demand with [`read_ucs_member`]. Everything happens in memory, so a
/// UCS's secrets never touch disk.
pub fn list_ucs_members(
    raw: &[u8],
    provider: &PassphraseProvider<'_>,
    label: &str,
    scope: MemberScope,
) -> Result<Vec<UcsFileEntry>, UcsError> {
    let data = decrypt_if_encrypted(raw, provider, label)?;
    if !is_ucs_bytes(&data) {
        return Err(UcsError::new(format!("{label}: not a valid UCS archive")));
    }
    let members = read_members(&data)?;
    let mut out = Vec::new();
    for (name, bytes) in &members {
        if matches!(scope, MemberScope::Forensic) && !is_forensic_path(name) {
            continue;
        }
        out.push(UcsFileEntry {
            path: name.clone(),
            size: bytes.len() as u64,
            sha256: sha256_hex(bytes),
            is_text: is_text_bytes(bytes),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pgp_and_ucs_byte_signatures() {
        // Binary SKESK packet — old-format header, tag 3 → first byte 0x8C.
        assert!(is_pgp_bytes(&[0x8C, 0x0D, 0x04]));
        // ASCII-armored OpenPGP message.
        assert!(is_pgp_bytes(b"-----BEGIN PGP MESSAGE-----\n\nfoo\n"));

        // A plain (unencrypted) UCS is a gzip stream: UCS magic, not PGP.
        let gzip = [0x1F, 0x8B, 0x08, 0x00];
        assert!(is_ucs_bytes(&gzip));
        assert!(!is_pgp_bytes(&gzip));

        // Plain config text and empty input are neither.
        assert!(!is_pgp_bytes(b"ltm pool /Common/p { }"));
        assert!(!is_pgp_bytes(b""));
        assert!(!is_ucs_bytes(b""));
    }

    /// Build a plain (gzip-tar) UCS archive from `(name, content)` members.
    fn build_ucs(members: &[(&str, &str)]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        let mut tar = tar::Builder::new(Vec::new());
        for (name, content) in members {
            let bytes = content.as_bytes();
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, bytes).unwrap();
        }
        let tar_bytes = tar.into_inner().unwrap();
        let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
        std::io::Write::write_all(&mut gz, &tar_bytes).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn ucs_to_scf_includes_defaults_and_partitions_in_order() {
        let ucs = build_ucs(&[
            ("config/bigip.conf", "ltm virtual /Common/vs_common { }"),
            (
                "config/profile_base.conf",
                "ltm profile tcp /Common/tcp { }",
            ),
            (
                "config/bigip_script.conf",
                "cli script /Common/brace_trap { proc script::run {} { } }",
            ),
            (
                "config/partitions/TenantB/bigip.conf",
                "ltm pool /TenantB/web_b { }",
            ),
            (
                "config/partitions/TenantA/bigip.conf",
                "ltm pool /TenantA/web_a { }",
            ),
            (
                "config/partitions/TenantA/bigip_base.conf",
                "auth partition TenantA { }",
            ),
        ]);
        let scf = ucs_to_scf(&ucs, false).unwrap();

        // Every real member is present.
        assert!(scf.contains("/Common/tcp"), "defaults included:\n{scf}");
        assert!(scf.contains("/Common/vs_common"));
        assert!(scf.contains("/TenantA/web_a"));
        assert!(scf.contains("/TenantB/web_b"));
        assert!(scf.contains("brace_trap"));

        // Order: defaults -> Common -> partitions (alpha), base before bigip.conf,
        // and the script member LAST so it can't truncate any real object.
        let pos = |needle: &str| scf.find(needle).expect(needle);
        assert!(pos("/Common/tcp") < pos("/Common/vs_common"));
        assert!(pos("/Common/vs_common") < pos("auth partition TenantA"));
        assert!(pos("auth partition TenantA") < pos("/TenantA/web_a"));
        assert!(pos("/TenantA/web_a") < pos("/TenantB/web_b"));
        assert!(
            pos("brace_trap") > pos("/TenantB/web_b"),
            "script emitted last"
        );
    }

    #[test]
    fn ucs_to_scf_skips_macos_cruft() {
        // A UCS zipped on a Mac carries AppleDouble `._` forks and __MACOSX
        // shadows whose binary bodies must never reach the config parser.
        let ucs = build_ucs(&[
            ("config/bigip.conf", "ltm virtual /Common/vs { }"),
            ("config/._bigip.conf", "\u{0}\u{5}\u{16}\u{7}garbage"),
            ("__MACOSX/config/._bigip.conf", "\u{0}more binary"),
            ("config/partitions/T/._bigip.conf", "\u{0}\u{1}applefork"),
            ("config/partitions/T/bigip.conf", "ltm pool /T/p { }"),
        ]);
        let scf = ucs_to_scf(&ucs, true).unwrap();
        assert!(scf.contains("/Common/vs") && scf.contains("/T/p"));
        assert!(
            !scf.contains("garbage"),
            "AppleDouble body excluded:\n{scf}"
        );
        assert!(!scf.contains("applefork"));
        assert!(!scf.contains("more binary"));
    }

    #[test]
    fn forensic_path_classification() {
        // Forensic artefacts.
        for p in [
            "etc/passwd",
            "etc/shadow",
            "etc/hosts.deny",
            "etc/nsswitch.conf",
            "etc/krb5.conf",
            "etc/motd",
            "etc/syslog-ng/syslog-ng.conf",
            "etc/openldap/ldap.conf",
            "etc/security/limits.conf",
            "etc/pam.d/sshd",
            "etc/crontab",
            "etc/cron.d/job",
            "root/.ssh/authorized_keys",
            "home/admin/.ssh/id_rsa",
            "home/admin/.bashrc",
            "root/.bash_history",
        ] {
            assert!(is_forensic_path(p), "{p} should be forensic");
        }
        // Not forensic — config (already the SCF) and ordinary runtime data.
        for p in [
            "config/bigip.conf",
            "config/bigip_base.conf",
            "var/lib/hsqldb/db.data",
            "SPEC-Manifest",
            "etc/hostname",
        ] {
            assert!(!is_forensic_path(p), "{p} should not be forensic");
        }
    }

    #[test]
    fn list_members_forensic_vs_full_tree() {
        use std::collections::BTreeSet;
        let ucs = build_ucs(&[
            ("config/bigip.conf", "ltm pool /Common/p { }"),
            ("etc/passwd", "root:x:0:0::/root:/bin/bash\n"),
            ("root/.ssh/authorized_keys", "ssh-rsa AAAAB3Nz attacker\n"),
            ("home/admin/.bashrc", "echo hi\n"),
            ("etc/motd", "welcome\n"),
            // A binary blob (embedded NUL) that must classify as non-text and
            // must NOT appear in the forensic subset.
            ("var/lib/hsqldb/db.data", "rows\u{0}\u{1}\u{2}binary"),
        ]);
        // Plain UCS: the provider is never consulted.
        let provider = || Ok(String::new());

        let all = list_ucs_members(&ucs, &provider, "t", MemberScope::FullTree).unwrap();
        assert_eq!(all.len(), 6, "full tree returns every member");

        let forensic = list_ucs_members(&ucs, &provider, "t", MemberScope::Forensic).unwrap();
        let paths: BTreeSet<&str> = forensic.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            BTreeSet::from([
                "etc/passwd",
                "root/.ssh/authorized_keys",
                "home/admin/.bashrc",
                "etc/motd",
            ]),
            "forensic subset excludes config + runtime data"
        );

        // Metadata: size, text/binary and a stable, correct SHA-256.
        let passwd = forensic.iter().find(|e| e.path == "etc/passwd").unwrap();
        assert_eq!(passwd.size, "root:x:0:0::/root:/bin/bash\n".len() as u64);
        assert!(passwd.is_text);
        // sha256("root:x:0:0::/root:/bin/bash\n") — independently reproducible.
        assert_eq!(
            passwd.sha256,
            sha256_hex(b"root:x:0:0::/root:/bin/bash\n"),
            "sha256 is lowercase hex of the content"
        );
        assert_eq!(passwd.sha256.len(), 64);

        let blob = all
            .iter()
            .find(|e| e.path == "var/lib/hsqldb/db.data")
            .unwrap();
        assert!(!blob.is_text, "NUL-bearing content is binary");
    }
}
