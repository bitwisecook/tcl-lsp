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

//! Offline, deterministic verification of a stored password hash against a
//! known candidate password — the primitive the Security tab's default-
//! credential check is built on (see [`crate::security`]).
//!
//! Never calls the platform `crypt(3)`: every scheme below is a pure-Rust
//! digest computation, so the native binary, the wasm build and any future
//! backend derive byte-identical results — no `libcrypt`, no `unsafe`, no
//! OS/library version skew. Nothing here logs, renders, or returns the
//! stored hash, the salt, the candidate password, or the derived hash; the
//! only thing a caller ever sees is a [`Verdict`].
//!
//! # Supported schemes
//!
//! | Prefix | Scheme | Source |
//! |---|---|---|
//! | `$6$` | SHA-512-crypt | [`sha_crypt`] (`RustCrypto`) |
//! | `$5$` | SHA-256-crypt | [`sha_crypt`] (`RustCrypto`) |
//! | `$1$` | MD5-crypt (FreeBSD/glibc) | implemented below, on the plain MD5 digest |
//!
//! `$6$`/`$5$` are what a current BIG-IP (TMOS 11+, glibc `libcrypt`) stores
//! for `auth user … encrypted-password` and for `/etc/shadow` entries in a
//! UCS; `$1$` remains for older estates and third-party accounts migrated
//! from earlier Linux/BSD systems.
//!
//! # Deliberately unsupported (fail closed, never guessed)
//!
//! Traditional Seventh-Edition Unix DES-crypt (bare 13-character hashes with
//! no `$scheme$` prefix), `BSDi` extended DES-crypt, Apache's `$apr1$`
//! variant, bcrypt (`$2a$`/`$2b$`/`$2y$`) and any other/unrecognised
//! representation are reported as [`Verdict::Unsupported`] — the finding is
//! "could not inspect", never a guess. See the issue's acceptance criteria:
//! *"Unsupported or malformed formats fail closed and do not produce false
//! positives."* Classic DES-crypt in particular is rare on a modern BIG-IP
//! (TMOS defaults to SHA-512-crypt) and its crypt(3) variant folds the salt
//! into a modified DES expansion permutation rather than just keying
//! standard DES, so — lacking an audited pure-Rust implementation — it is
//! left uninspected rather than hand-rolled.
//!
//! F5's own `$M$…` master-key envelope (see [`tcl_f5mku`]) is a different
//! thing entirely: it wraps *secrets* (RADIUS/LDAP/monitor passwords, key
//! passphrases), never the local-account login hash, so it never appears in
//! `encrypted-password` and is out of scope here.

use md5::{Digest, Md5};
use sha_crypt::password_hash;
use sha_crypt::{PasswordVerifier, ShaCrypt};

/// The outcome of comparing a stored password-hash string against a known
/// candidate password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The candidate password verifies against the stored hash.
    Match,
    /// The stored hash is in a supported scheme, but the candidate does not
    /// verify against it.
    NoMatch,
    /// The stored value marks the account as locked / passwordless (`*`,
    /// `!`, `!!`) or empty — there is no credential to compare a candidate
    /// against.
    NotApplicable,
    /// The stored value is in a scheme this module does not implement (or is
    /// not a recognisable hash at all). Never treated as a match or a
    /// non-match — the caller must render this as "could not inspect".
    Unsupported,
}

/// Shadow/tmsh sentinels meaning "this account has no usable password" —
/// never a hash to compare against.
const LOCKED_SENTINELS: [&str; 4] = ["*", "!", "!!", "x"];

/// Verify `candidate` against `stored` (a `auth user … encrypted-password` or
/// `/etc/shadow` field value), returning a [`Verdict`] and never the stored
/// hash, its salt, or the derived hash.
#[must_use]
pub fn verify(stored: &str, candidate: &str) -> Verdict {
    let stored = stored.trim();
    if stored.is_empty() || LOCKED_SENTINELS.contains(&stored) {
        return Verdict::NotApplicable;
    }

    if let Some(rest) = stored.strip_prefix("$1$") {
        return match md5_crypt_verify(rest, candidate) {
            Some(true) => Verdict::Match,
            Some(false) => Verdict::NoMatch,
            None => Verdict::Unsupported,
        };
    }

    if stored.starts_with("$5$") || stored.starts_with("$6$") {
        let sha = ShaCrypt::default();
        return match sha.verify_password(candidate.as_bytes(), stored) {
            Ok(()) => Verdict::Match,
            Err(password_hash::Error::PasswordInvalid) => Verdict::NoMatch,
            // Any other error (bad params, malformed field count, …) means
            // the stored text isn't actually a well-formed sha-crypt hash —
            // fail closed rather than guess.
            Err(_) => Verdict::Unsupported,
        };
    }

    Verdict::Unsupported
}

/// Split a `$1$salt` remainder (after the `$1$` prefix has been stripped)
/// into `(salt, has_trailing_hash)`. The salt is at most 8 characters, up to
/// the next `$` (or end of string).
fn split_md5_salt(rest: &str) -> (&str, bool) {
    match rest.split_once('$') {
        Some((salt, _)) => (&salt[..salt.len().min(8)], true),
        None => (&rest[..rest.len().min(8)], false),
    }
}

/// Verify `candidate` against a `$1$salt$hash` MD5-crypt string (with the
/// `$1$` prefix already stripped). Returns `None` when `rest` doesn't carry a
/// trailing `$hash` field (malformed — fail closed).
fn md5_crypt_verify(rest: &str, candidate: &str) -> Option<bool> {
    let (salt, has_hash) = split_md5_salt(rest);
    if !has_hash {
        return None;
    }
    let computed = md5_crypt(candidate.as_bytes(), salt.as_bytes());
    let stored_hash = rest.split_once('$').map(|(_, h)| h)?;
    // Plain, non-secret-length string comparison: the *inputs* being compared
    // (a derived hash and a stored hash) are not the secret being protected
    // (the password) — this mirrors `sha_crypt`'s own non-constant-time
    // dev-dependency-free path for the `$5$`/`$6$` MCF string compare and the
    // module doc's "no timing hardening needed for a local, offline batch
    // compare" scope note.
    Some(computed == stored_hash)
}

/// The `crypt(3)`/FreeBSD/glibc MD5-crypt algorithm (RFC-less but
/// long-standing de facto standard, `$1$` scheme), computing only the final
/// hash field (the caller already has the salt).
///
/// This is the well-known Poul-Henning Kamp algorithm shipped by FreeBSD and
/// glibc since the 1990s; there is no maintained pure-Rust crate for it (see
/// the module doc), so it is implemented here directly on the plain MD5
/// digest and validated in tests against real `crypt(3)` output (`python3
/// -c 'import crypt; crypt.crypt(...)'` on glibc).
fn md5_crypt(password: &[u8], salt: &[u8]) -> String {
    const MAGIC: &[u8] = b"$1$";

    let mut ctx = Md5::new();
    ctx.update(password);
    ctx.update(MAGIC);
    ctx.update(salt);

    let mut ctx1 = Md5::new();
    ctx1.update(password);
    ctx1.update(salt);
    ctx1.update(password);
    let mut alt: [u8; 16] = ctx1.finalize().into();

    let mut remaining = password.len();
    while remaining > 0 {
        let take = remaining.min(16);
        ctx.update(&alt[..take]);
        remaining -= take;
    }

    let mut i = password.len();
    while i != 0 {
        if i & 1 != 0 {
            ctx.update([0u8]);
        } else {
            ctx.update(&password[..1]);
        }
        i >>= 1;
    }
    alt = ctx.finalize().into();

    for i in 0..1000 {
        let mut c = Md5::new();
        if i & 1 != 0 {
            c.update(password);
        } else {
            c.update(alt);
        }
        if i % 3 != 0 {
            c.update(salt);
        }
        if i % 7 != 0 {
            c.update(password);
        }
        if i & 1 != 0 {
            c.update(alt);
        } else {
            c.update(password);
        }
        alt = c.finalize().into();
    }

    // The final permutation + custom base64 ("itoa64") encoding, in the
    // fixed byte-triple order the reference implementation uses.
    let mut out = String::with_capacity(22);
    let groups: [(usize, usize, usize); 5] =
        [(0, 6, 12), (1, 7, 13), (2, 8, 14), (3, 9, 15), (4, 10, 5)];
    for (a, b, c) in groups {
        let v = (u32::from(alt[a]) << 16) | (u32::from(alt[b]) << 8) | u32::from(alt[c]);
        to64(v, 4, &mut out);
    }
    to64(u32::from(alt[11]), 2, &mut out);
    out
}

/// The MD5-crypt "itoa64" alphabet — not standard base64.
const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Append `count` `itoa64` characters (least-significant 6 bits first) of
/// `value` to `out`.
fn to64(mut value: u32, count: usize, out: &mut String) {
    for _ in 0..count {
        out.push(ITOA64[(value & 0x3f) as usize] as char);
        value >>= 6;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ground truth captured from glibc `crypt(3)` via
    // `python3 -c 'import crypt; print(crypt.crypt(pw, salt))'`.

    #[test]
    fn md5_crypt_matches_glibc() {
        assert_eq!(md5_crypt(b"default", b"abcdefgh"), "9aR4aM1iehMtluq/alSdu0");
        assert_eq!(md5_crypt(b"admin", b"saltsalt"), "Y3BMhxQHmg7scgLILa56u1");
        assert_eq!(md5_crypt(b"", b"abcdefgh"), "M55TzYaaccxVGbptZWaxX/");
        assert_eq!(md5_crypt(b"default", b""), "cVmNS3JBcYsfcdsZnqCys0");
    }

    #[test]
    fn md5_crypt_default_credentials_match() {
        assert_eq!(
            verify("$1$abcdefgh$9aR4aM1iehMtluq/alSdu0", "default"),
            Verdict::Match
        );
        assert_eq!(
            verify("$1$saltsalt$Y3BMhxQHmg7scgLILa56u1", "admin"),
            Verdict::Match
        );
    }

    #[test]
    fn md5_crypt_non_default_does_not_match() {
        assert_eq!(
            verify("$1$abcdefgh$9aR4aM1iehMtluq/alSdu0", "hunter2"),
            Verdict::NoMatch
        );
    }

    #[test]
    fn sha256_crypt_default_credentials() {
        assert_eq!(
            verify(
                "$5$rounds=5000$saltstring$JZi6s3UTY7yQ.mIUS72QzgoEF8T6QbJoTOftn/Vm7c1",
                "default"
            ),
            Verdict::Match
        );
        assert_eq!(
            verify(
                "$5$abcsalt12$egUJtD7PERzbcgR/nWzHxBZDwbk7PoR9QXn31CaUcq/",
                "admin"
            ),
            Verdict::Match
        );
    }

    #[test]
    fn sha512_crypt_default_credentials() {
        assert_eq!(
            verify(
                "$6$rounds=5000$saltstring$6p43fZYI.QjsRRE84rwofulMnyqp504VY9sFYK14/NXu6XfSJmmWs.6LU23LEbtyE.wFHWhcL9ahBHbNgfPCc1",
                "default"
            ),
            Verdict::Match
        );
        assert_eq!(
            verify(
                "$6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1",
                "admin"
            ),
            Verdict::Match
        );
    }

    #[test]
    fn sha_crypt_non_default_does_not_match() {
        assert_eq!(
            verify(
                "$6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1",
                "hunter2"
            ),
            Verdict::NoMatch
        );
        assert_eq!(
            verify(
                "$5$abcXYZ12$tffhf423Gq6U4KVT21AMqiHanCKTGd8MYH4vGUIgO/1",
                "admin"
            ),
            Verdict::NoMatch
        );
    }

    #[test]
    fn locked_and_empty_accounts_are_not_applicable() {
        for stored in ["", "*", "!", "!!", "x", "  "] {
            assert_eq!(
                verify(stored, "admin"),
                Verdict::NotApplicable,
                "stored={stored:?}"
            );
        }
    }

    #[test]
    fn unsupported_schemes_fail_closed() {
        // Classic 13-char Unix DES-crypt (no `$` prefix) — deliberately
        // unsupported (see module docs).
        assert_eq!(verify("xOAFZqRz5RduI", "password"), Verdict::Unsupported);
        // bcrypt — a different login-hash ecosystem, not implemented.
        assert_eq!(
            verify(
                "$2y$05$bvIG6Nmid91Mu9RcmmWZfO5HJIMCT8riNW0hEp8f6/FuA2/mHZFpe",
                "password"
            ),
            Verdict::Unsupported
        );
        // Apache's MD5 variant shares the `$1$`-like shape but a different
        // salt magic; garbage/malformed text also fails closed.
        assert_eq!(
            verify("$apr1$abcdefgh$whatever", "default"),
            Verdict::Unsupported
        );
        assert_eq!(verify("not-a-hash-at-all", "default"), Verdict::Unsupported);
        // A `$1$` string with no trailing `$hash` field is malformed.
        assert_eq!(verify("$1$onlysalt", "default"), Verdict::Unsupported);
    }
}
