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

//! Minimal, dependency-free `OpenPGP` *symmetric* (passphrase) decryptor.
//!
//! The pure-Rust fallback used to decrypt an encrypted BIG-IP UCS archive without shelling
//! out to `gpg`. BIG-IP encrypts UCS files with `GnuPG` using a passphrase and a
//! 128-bit AES symmetric cipher (F5 KB K5437), producing an ordinary `OpenPGP`
//! message:
//!
//! ```text
//! SKESK (tag 3)  → S2K-derived session key
//! SEIPD (tag 18) → AES-CFB ciphertext + SHA-1 MDC
//! (inner)        → optional Compressed Data (tag 8) wrapping
//!                  Literal Data (tag 11) — the gzip tar.
//! ```
//!
//! Only what that path needs is implemented:
//!
//! * AES-128/192/256 (via [`crate::aes_cfb`]) — the only ciphers UCS uses.
//! * S2K simple / salted / iterated-and-salted, any supported digest.
//! * OpenPGP-CFB for SEIPD v1, with the quick-check and SHA-1 MDC verify.
//! * ZIP / ZLIB inner compression (raw / zlib-wrapped DEFLATE via `flate2`).
//! * Old- and new-format packet headers including partial body lengths.
//!
//! Out of scope (returns [`OpenPgpError`], pointing at gpg): public-key
//! encryption, non-AES ciphers, AEAD / SEIPD v2, BZIP2
//! inner compression, and the obsolete MDC-less SED packet (tag 9).

use std::io::Read;

use digest::Digest;
use flate2::read::{DeflateDecoder, ZlibDecoder};
use tcl_core_types::RecursionLimit;

use crate::aes_cfb::{Aes, cfb_decrypt};

const BLOCK: usize = 16;

/// Maximum nesting depth of `OpenPGP` Compressed Data (tag 8) packets
/// [`extract_literal`] will unwrap before giving up — issue #996:
/// `extract_literal` recurses natively once per nested Compressed Data
/// packet, fully attacker/generator-controlled via a crafted `.ucs`/`.scf`
/// archive, with no cap before this fix. A legitimate BIG-IP UCS never
/// nests Compressed Data more than one level deep (SEIPD → one Compressed
/// Data → Literal Data); 16 is deliberately generous relative to that (a
/// handful of levels of headroom for any oddly-repacked archive) while
/// staying small enough to be cheap to check and to leave comfortable
/// margin on the smallest stack this code runs on — the in-browser
/// `bigip-report-gen/wasm` target, whose host-controlled stack budget this
/// repo does not control (unlike the native `bigip-report-gen` CLI, which
/// this repo's own entry point could give a generous stack, WASM cannot).
const MAX_COMPRESSED_PACKET_DEPTH: RecursionLimit = RecursionLimit(16);

/// The message could not be parsed or uses an unsupported feature, or
/// decryption failed (almost always a wrong passphrase). A single type
/// carries the human-facing text for every failure mode.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct OpenPgpError(pub String);

impl OpenPgpError {
    fn new(msg: impl Into<String>) -> Self {
        OpenPgpError(msg.into())
    }
}

/// True if `cipher_id` is an AES variant we can key, returning its key length.
fn cipher_keylen(cipher_id: u8) -> Option<usize> {
    match cipher_id {
        7 => Some(16),
        8 => Some(24),
        9 => Some(32),
        _ => None,
    }
}

// Packet framing

/// Read `n` big-endian bytes at `off`, erroring on truncation (a read past
/// the end of `data` yields `OpenPgpError`).
fn be(data: &[u8], off: usize, n: usize) -> Result<u32, OpenPgpError> {
    if off + n > data.len() {
        return Err(OpenPgpError::new("malformed or truncated OpenPGP message"));
    }
    let mut v = 0u32;
    for &b in &data[off..off + n] {
        v = (v << 8) | u32::from(b);
    }
    Ok(v)
}

fn at(data: &[u8], off: usize) -> Result<u8, OpenPgpError> {
    data.get(off)
        .copied()
        .ok_or_else(|| OpenPgpError::new("malformed or truncated OpenPGP message"))
}

/// `(length, new_off, partial)` — new-format body length (RFC 4880 §4.2.2).
fn read_new_length(data: &[u8], off: usize) -> Result<(usize, usize, bool), OpenPgpError> {
    let o1 = at(data, off)?;
    let off = off + 1;
    if o1 < 192 {
        Ok((o1 as usize, off, false))
    } else if o1 < 224 {
        let o2 = at(data, off)?;
        Ok((
            (((o1 as usize) - 192) << 8) + o2 as usize + 192,
            off + 1,
            false,
        ))
    } else if o1 < 255 {
        Ok((1usize << (o1 & 0x1F), off, true)) // partial body length
    } else {
        Ok((be(data, off, 4)? as usize, off + 4, false))
    }
}

/// `(length, new_off)` — old-format body length (RFC 4880 §4.2.1).
fn read_old_length(data: &[u8], ctb: u8, off: usize) -> Result<(usize, usize), OpenPgpError> {
    match ctb & 0x03 {
        0 => Ok((at(data, off)? as usize, off + 1)),
        1 => Ok((be(data, off, 2)? as usize, off + 2)),
        2 => Ok((be(data, off, 4)? as usize, off + 4)),
        _ => Ok((data.len() - off, off)), // indeterminate: to end of input
    }
}

/// Saturating slice (lenient out-of-range indexing).
fn slice_sat(data: &[u8], off: usize, len: usize) -> &[u8] {
    let start = off.min(data.len());
    let end = off.saturating_add(len).min(data.len());
    &data[start..end]
}

/// Split an `OpenPGP` stream into `(tag, body)` pairs.
fn parse_packets(data: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, OpenPgpError> {
    let mut packets = Vec::new();
    let mut off = 0;
    let n = data.len();
    while off < n {
        let ctb = data[off];
        off += 1;
        if ctb & 0x80 == 0 {
            return Err(OpenPgpError::new(
                "not an OpenPGP message (bad packet header)",
            ));
        }
        if ctb & 0x40 != 0 {
            // New format.
            let tag = ctb & 0x3F;
            let mut body = Vec::new();
            loop {
                let (length, new_off, partial) = read_new_length(data, off)?;
                off = new_off;
                body.extend_from_slice(slice_sat(data, off, length));
                off += length;
                if !partial {
                    break;
                }
            }
            packets.push((tag, body));
        } else {
            // Old format.
            let tag = (ctb >> 2) & 0x0F;
            let (length, new_off) = read_old_length(data, ctb, off)?;
            off = new_off;
            packets.push((tag, slice_sat(data, off, length).to_vec()));
            off += length;
        }
    }
    Ok(packets)
}

// String-to-key

struct S2k {
    hash_id: u8,
    salt: Vec<u8>,
    count: usize,
    next_off: usize,
}

/// Parse an S2K specifier (RFC 4880 §3.7.1). `count == 0` for non-iterated.
fn parse_s2k(buf: &[u8], off: usize) -> Result<S2k, OpenPgpError> {
    let s2k_type = at(buf, off)?;
    let hash_id = at(buf, off + 1)?;
    let mut off = off + 2;
    if hash_name(hash_id).is_none() {
        return Err(OpenPgpError::new(format!(
            "unsupported S2K hash algorithm {hash_id}"
        )));
    }
    let mut salt = Vec::new();
    let mut count = 0usize;
    if s2k_type == 1 || s2k_type == 3 {
        salt = slice_sat(buf, off, 8).to_vec();
        off += 8;
    }
    if s2k_type == 3 {
        let c = at(buf, off)?;
        off += 1;
        count = (16 + (c as usize & 0x0F)) << ((c >> 4) as usize + 6);
    } else if s2k_type != 0 && s2k_type != 1 {
        return Err(OpenPgpError::new(format!(
            "unsupported S2K type {s2k_type}"
        )));
    }
    Ok(S2k {
        hash_id,
        salt,
        count,
        next_off: off,
    })
}

/// Friendly name for a supported `OpenPGP` hash algorithm ID, else `None`.
fn hash_name(hash_id: u8) -> Option<&'static str> {
    match hash_id {
        1 => Some("md5"),
        2 => Some("sha1"),
        3 => Some("ripemd160"),
        8 => Some("sha256"),
        9 => Some("sha384"),
        10 => Some("sha512"),
        11 => Some("sha224"),
        _ => None,
    }
}

/// Derive `keylen` key bytes from `passphrase` via `OpenPGP` S2K (the simple /
/// salted / iterated digest-folding loop). Generic over the digest so the
/// dispatch below stays a flat match.
fn s2k_derive_with<D: Digest>(
    passphrase: &[u8],
    salt: &[u8],
    count: usize,
    keylen: usize,
) -> Vec<u8> {
    let mut base = Vec::with_capacity(salt.len() + passphrase.len());
    base.extend_from_slice(salt);
    base.extend_from_slice(passphrase);
    let (full, rem) = if count != 0 && count > base.len() {
        (count / base.len(), count % base.len())
    } else {
        (1usize, 0usize) // simple/salted (or count below one full pass)
    };
    let mut out: Vec<u8> = Vec::new();
    let mut context = 0usize;
    while out.len() < keylen {
        let mut h = D::new();
        if context > 0 {
            h.update(vec![0u8; context]);
        }
        for _ in 0..full {
            h.update(&base);
        }
        if rem > 0 {
            h.update(&base[..rem]);
        }
        out.extend_from_slice(&h.finalize());
        context += 1;
    }
    out.truncate(keylen);
    out
}

fn s2k_derive(hash_id: u8, passphrase: &[u8], salt: &[u8], count: usize, keylen: usize) -> Vec<u8> {
    match hash_id {
        1 => s2k_derive_with::<md5::Md5>(passphrase, salt, count, keylen),
        2 => s2k_derive_with::<sha1::Sha1>(passphrase, salt, count, keylen),
        3 => s2k_derive_with::<ripemd::Ripemd160>(passphrase, salt, count, keylen),
        8 => s2k_derive_with::<sha2::Sha256>(passphrase, salt, count, keylen),
        9 => s2k_derive_with::<sha2::Sha384>(passphrase, salt, count, keylen),
        10 => s2k_derive_with::<sha2::Sha512>(passphrase, salt, count, keylen),
        11 => s2k_derive_with::<sha2::Sha224>(passphrase, salt, count, keylen),
        // Validated already by parse_s2k; unreachable in practice.
        _ => Vec::new(),
    }
}

fn make_cipher(cipher_id: u8, key: &[u8]) -> Result<Aes, OpenPgpError> {
    let Some(keylen) = cipher_keylen(cipher_id) else {
        return Err(OpenPgpError::new(format!(
            "unsupported symmetric cipher (algorithm {cipher_id}); the bundled decryptor only handles AES — install gpg/gpg2"
        )));
    };
    if key.len() != keylen {
        return Err(OpenPgpError::new(
            "session key length does not match cipher",
        ));
    }
    Aes::new(key).map_err(OpenPgpError::new)
}

// Packet handlers

/// Symmetric-Key Encrypted Session Key packet → `(cipher_id, session_key)`.
fn decode_skesk(body: &[u8], passphrase: &[u8]) -> Result<(u8, Vec<u8>), OpenPgpError> {
    let version = at(body, 0)?;
    if !matches!(version, 4..=6) {
        return Err(OpenPgpError::new(format!(
            "unsupported SKESK version {version}"
        )));
    }
    let cipher_id = at(body, 1)?;
    let s2k = parse_s2k(body, 2)?;
    let Some(keylen) = cipher_keylen(cipher_id) else {
        return Err(OpenPgpError::new(format!(
            "unsupported symmetric cipher (algorithm {cipher_id}); the bundled decryptor only handles AES — install gpg/gpg2"
        )));
    };
    let s2k_key = s2k_derive(s2k.hash_id, passphrase, &s2k.salt, s2k.count, keylen);
    let encrypted_key = &body[s2k.next_off.min(body.len())..];
    if encrypted_key.is_empty() {
        // No wrapped key: the S2K output is the session key directly.
        return Ok((cipher_id, s2k_key));
    }
    // Wrapped session key: CFB-decrypt with an all-zero IV; the first
    // plaintext byte is the real cipher algorithm, the rest is the key.
    let cipher = make_cipher(cipher_id, &s2k_key)?;
    let decrypted = cfb_decrypt(&cipher, &[0u8; BLOCK], encrypted_key);
    let real_id = decrypted[0];
    Ok((real_id, decrypted[1..].to_vec()))
}

/// Decrypt a Sym. Encrypted & Integrity Protected Data packet (tag 18).
fn decrypt_seipd(body: &[u8], cipher_id: u8, session_key: &[u8]) -> Result<Vec<u8>, OpenPgpError> {
    let version = at(body, 0)?;
    if version != 1 {
        return Err(OpenPgpError::new(
            "unsupported encrypted-data packet (AEAD/SEIPD v2); install gpg/gpg2",
        ));
    }
    let cipher = make_cipher(cipher_id, session_key)?;
    let plain = cfb_decrypt(&cipher, &[0u8; BLOCK], &body[1..]);
    if plain.len() < BLOCK + 2 + 22 {
        return Err(OpenPgpError::new(
            "encrypted data too short (corrupt archive)",
        ));
    }
    // Quick check: the two bytes after the random prefix repeat its tail.
    if plain[BLOCK - 2..BLOCK] != plain[BLOCK..BLOCK + 2] {
        return Err(OpenPgpError::new(
            "incorrect passphrase (quick-check failed)",
        ));
    }
    let mdc = &plain[plain.len() - 22..];
    if mdc[0] != 0xD3 || mdc[1] != 0x14 {
        return Err(OpenPgpError::new(
            "incorrect passphrase or corrupt archive (MDC packet missing)",
        ));
    }
    let mut h = sha1::Sha1::new();
    h.update(&plain[..plain.len() - 20]);
    if h.finalize().as_slice() != &plain[plain.len() - 20..] {
        return Err(OpenPgpError::new(
            "integrity check failed (wrong passphrase or corrupt archive)",
        ));
    }
    Ok(plain[BLOCK + 2..plain.len() - 22].to_vec())
}

fn decompress(algo: u8, data: &[u8]) -> Result<Vec<u8>, OpenPgpError> {
    match algo {
        0 => Ok(data.to_vec()),
        1 => inflate(DeflateDecoder::new(data)), // raw DEFLATE (OpenPGP "ZIP")
        2 => inflate(ZlibDecoder::new(data)),    // zlib-wrapped DEFLATE ("ZLIB")
        3 => Err(OpenPgpError::new(
            "BZIP2 inner compression is unsupported by the bundled decryptor — install gpg/gpg2 to decrypt this archive",
        )),
        other => Err(OpenPgpError::new(format!(
            "unsupported compression algorithm {other}"
        ))),
    }
}

fn inflate<R: Read>(mut dec: R) -> Result<Vec<u8>, OpenPgpError> {
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| OpenPgpError::new(format!("decompression failed: {e}")))?;
    Ok(out)
}

/// Unwrap Compressed (tag 8) / Literal (tag 11) packets to the payload.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_COMPRESSED_PACKET_DEPTH`] this returns [`OpenPgpError`] instead of
/// unwrapping another Compressed Data packet, so pathologically deep
/// Compressed Data nesting in a crafted archive fails cleanly rather than
/// overflowing the native stack.
fn extract_literal(data: &[u8], depth: u32) -> Result<Vec<u8>, OpenPgpError> {
    for (tag, body) in parse_packets(data)? {
        if tag == 8 {
            if MAX_COMPRESSED_PACKET_DEPTH.exceeded(depth) {
                return Err(OpenPgpError::new(
                    "OpenPGP message nests Compressed Data packets too deeply (malformed or hostile archive)",
                ));
            }
            // Compressed Data: [algo][compressed...]
            let algo = at(&body, 0)?;
            return extract_literal(&decompress(algo, &body[1..])?, depth + 1);
        }
        if tag == 11 {
            // Literal Data: [format][namelen][name][mtime(4)][payload]
            let namelen = at(&body, 1)? as usize;
            let start = (2 + namelen + 4).min(body.len());
            return Ok(body[start..].to_vec());
        }
    }
    Err(OpenPgpError::new(
        "no literal-data packet found after decryption",
    ))
}

// ASCII armor (binary UCS files are not armored, but be liberal in input)

fn maybe_dearmor(data: &[u8]) -> Result<Vec<u8>, OpenPgpError> {
    let lead = &data[..data.len().min(64)];
    let trimmed_start = lead
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(lead.len());
    if !data[trimmed_start..].starts_with(b"-----BEGIN PGP") {
        return Ok(data.to_vec());
    }
    let text = String::from_utf8_lossy(data);
    let mut collecting = false;
    let mut b64 = String::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.starts_with("-----BEGIN PGP") {
            collecting = true;
            continue;
        }
        if line.starts_with("-----END PGP") {
            break;
        }
        if !collecting {
            continue;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue; // blank line separating armor headers from body
        }
        if stripped.starts_with('=') {
            continue; // CRC24 checksum line
        }
        if line.contains(": ") {
            continue; // armor header (Version:, Comment:, …)
        }
        b64.push_str(stripped);
    }
    base64_decode(&b64).map_err(|e| OpenPgpError::new(format!("malformed PGP ASCII armor: {e}")))
}

/// Minimal standard-alphabet base64 decoder (armor bodies only).
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = val(c).ok_or_else(|| format!("invalid base64 character {c:#x}"))?;
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            // Intentional low-byte extraction.
            #[allow(clippy::cast_possible_truncation)]
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

// Public API

/// Decrypt a passphrase-encrypted `OpenPGP` message; return the plaintext.
///
/// Returns [`OpenPgpError`] for a wrong passphrase, malformed input, or an
/// unsupported feature.
pub fn decrypt_symmetric(data: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, OpenPgpError> {
    let packets = parse_packets(&maybe_dearmor(data)?)?;

    let mut cipher_id: Option<u8> = None;
    let mut session_key: Option<Vec<u8>> = None;
    let mut enc_tag: Option<u8> = None;
    let mut enc_body: Option<Vec<u8>> = None;
    for (tag, body) in packets {
        match tag {
            3 => {
                let (cid, key) = decode_skesk(&body, passphrase)?;
                cipher_id = Some(cid);
                session_key = Some(key);
            }
            1 => {
                return Err(OpenPgpError::new(
                    "public-key encrypted message — a passphrase cannot decrypt it",
                ));
            }
            9 | 18 => {
                enc_tag = Some(tag);
                enc_body = Some(body);
            }
            _ => {}
        }
    }

    let (Some(cipher_id), Some(session_key)) = (cipher_id, session_key) else {
        return Err(OpenPgpError::new(
            "no symmetric-key session packet — this is not a passphrase-encrypted UCS",
        ));
    };
    let Some(enc_body) = enc_body else {
        return Err(OpenPgpError::new(
            "no encrypted-data packet found in message",
        ));
    };
    if enc_tag == Some(9) {
        return Err(OpenPgpError::new(
            "legacy SED packet (no MDC) is unsupported by the bundled decryptor — install gpg/gpg2 to decrypt this archive",
        ));
    }
    extract_literal(&decrypt_seipd(&enc_body, cipher_id, &session_key)?, 0)
}

#[cfg(test)]
mod recursion_tests {
    use super::*;

    /// New-format packet body-length header (RFC 4880 §4.2.2), the encode
    /// side of [`read_new_length`] — used only to build synthetic packets
    /// for the tests below.
    fn encode_new_length(len: usize) -> Vec<u8> {
        if len < 192 {
            vec![u8::try_from(len).expect("checked len < 192")]
        } else if len < 8384 {
            let l = len - 192;
            vec![
                u8::try_from(192 + (l >> 8)).expect("checked l < 8192, so 192 + (l>>8) < 224"),
                u8::try_from(l & 0xFF).expect("masked to a byte"),
            ]
        } else {
            let b = u32::try_from(len)
                .expect("test lengths fit u32")
                .to_be_bytes();
            vec![255, b[0], b[1], b[2], b[3]]
        }
    }

    /// A new-format `OpenPGP` packet with the given tag and body.
    fn new_format_packet(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![0xC0 | tag];
        out.extend(encode_new_length(body.len()));
        out.extend_from_slice(body);
        out
    }

    /// A minimal Literal Data packet (tag 11) wrapping `payload`.
    fn literal_packet(payload: &[u8]) -> Vec<u8> {
        let mut body = vec![b't', 0]; // format 't' (text), zero-length name
        body.extend_from_slice(&[0u8; 4]); // mtime
        body.extend_from_slice(payload);
        new_format_packet(11, &body)
    }

    /// Wrap `inner` in one Compressed Data packet (tag 8) using algorithm 0
    /// ("uncompressed" — [`decompress`] passes it through byte-for-byte), so
    /// nested test packets can be built without a real compressor.
    fn compressed_wrap(inner: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(inner.len() + 1);
        body.push(0); // algo 0: uncompressed passthrough
        body.extend_from_slice(inner);
        new_format_packet(8, &body)
    }

    /// A Literal Data packet wrapped in `levels` nested Compressed Data
    /// packets.
    fn nested_compressed(levels: usize, payload: &[u8]) -> Vec<u8> {
        let mut cur = literal_packet(payload);
        for _ in 0..levels {
            cur = compressed_wrap(&cur);
        }
        cur
    }

    /// Regression coverage for issue #996: `extract_literal` recurses
    /// natively once per nested Compressed Data (tag 8) packet, with no
    /// depth cap before this fix — depth is fully attacker/generator
    /// controlled via a crafted `.ucs`/`.scf` archive, reachable from the
    /// native `bigip-report-gen` CLI and the in-browser
    /// `bigip-report-gen/wasm` target alike. 5000 nested Compressed Data
    /// packets is comfortably past `MAX_COMPRESSED_PACKET_DEPTH` (16); the
    /// assertion is that `extract_literal` returns a clean error rather
    /// than crashing, not what the error says.
    #[test]
    fn deeply_nested_compressed_packets_return_error_not_crash() {
        const DEPTH: usize = 5000;
        let packet = nested_compressed(DEPTH, b"payload");
        let result = extract_literal(&packet, 0);
        assert!(
            result.is_err(),
            "expected a clean error past the depth cap, not a crash"
        );
    }

    /// A handful of nested Compressed Data packets (well under
    /// `MAX_COMPRESSED_PACKET_DEPTH`) still unwraps to the literal payload
    /// exactly as before this fix — the safety net must not fire on
    /// realistic nesting depths (a real BIG-IP UCS nests at most one level).
    #[test]
    fn moderately_nested_compressed_packets_still_extract_literal() {
        let packet = nested_compressed(3, b"hello");
        let extracted = extract_literal(&packet, 0).expect("well under the cap");
        assert_eq!(extracted, b"hello");
    }
}
