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

//! `OpenPGP` symmetric (passphrase) decryption for UCS archives.
//!
//! BIG-IP encrypts UCS files with a passphrase and AES (F5 KB K5437).
//! The session-key packet supplies an S2K-derived or wrapped key. The encrypted
//! packet uses SEIPD v1 and protects the compressed or literal data with an MDC.
//!
//! `glomar-pgp` owns packet framing, S2K and authenticated decryption. This
//! adapter retains the UCS reader's treatment of legacy partial chunks,
//! noncanonical fields, encrypted-data errors and permissive ASCII armor.

use std::io::{self, Read};

use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::{DecodeError, Engine as _, alphabet};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use glomar_pgp::composed::RawSessionKey;
use glomar_pgp::crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm};
use glomar_pgp::packet::{
    LiteralDataHeader, PacketHeader, SymEncryptedProtectedData, SymKeyEncryptedSessionKey,
};
use glomar_pgp::types::{PacketLength, Seipdv1ReadMode, StringToKey, Tag};
use tcl_core_types::RecursionLimit;

const BLOCK: usize = 16;
const IV: [u8; BLOCK] = [0; BLOCK];
const MDC_LEN: usize = 22;
const TRUNCATED: &str = "malformed or truncated OpenPGP message";
const CHECK_FAILED: &str = "incorrect passphrase (quick-check failed)";
const INTEGRITY_FAILED: &str = "integrity check failed (wrong passphrase or corrupt archive)";
const MDC_MISSING: &str = "incorrect passphrase or corrupt archive (MDC packet missing)";
const INVALID_KEYLEN: &str = "session key length does not match cipher";
const ARMOR_BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::RequireNone)
        .with_decode_allow_trailing_bits(true),
);

// The existing zero-based limit permits wrappers at depths 0 through 16.
// Keeping that bound also protects the browser's host-controlled native stack.
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

fn malformed() -> OpenPgpError {
    OpenPgpError::new(TRUNCATED)
}

fn decrypt_cfb(
    cipher: SymmetricKeyAlgorithm,
    key: &[u8],
    ciphertext: &mut [u8],
) -> Result<(), OpenPgpError> {
    cipher
        .decrypt_with_iv_regular(key, &IV, ciphertext)
        .map_err(|_| OpenPgpError::new(INVALID_KEYLEN))
}

// The packet library owns the binary header and length decoding. Read the segments with
// the archive reader's legacy rules: a short initial partial chunk is accepted, and a
// fixed body is bounded by the remaining bytes if its advertised length is larger.
fn parse_packets(mut input: &[u8]) -> Result<Vec<(PacketHeader, Vec<u8>)>, OpenPgpError> {
    let mut packets = Vec::new();
    while !input.is_empty() {
        let header = PacketHeader::try_from_reader(&mut input).map_err(|err| {
            if err.kind() == io::ErrorKind::InvalidData {
                OpenPgpError::new("not an OpenPGP message (bad packet header)")
            } else {
                malformed()
            }
        })?;
        let mut length = header.packet_length();
        let mut body = Vec::new();
        loop {
            let size = length.maybe_len().map_or(input.len(), |len| {
                usize::try_from(len).unwrap_or(usize::MAX)
            });
            let (chunk, remaining) = input.split_at(size.min(input.len()));
            body.extend_from_slice(chunk);
            input = remaining;
            if !matches!(length, PacketLength::Partial(_)) {
                break;
            }
            length = PacketLength::try_from_reader(&mut input).map_err(|_| malformed())?;
        }
        packets.push((header, body));
    }
    Ok(packets)
}

fn cipher(id: u8) -> Result<SymmetricKeyAlgorithm, OpenPgpError> {
    match SymmetricKeyAlgorithm::from(id) {
        cipher @ (SymmetricKeyAlgorithm::AES128
        | SymmetricKeyAlgorithm::AES192
        | SymmetricKeyAlgorithm::AES256) => Ok(cipher),
        _ => Err(OpenPgpError::new(format!(
            "unsupported symmetric cipher (algorithm {id}); the bundled decryptor only handles AES — install gpg/gpg2"
        ))),
    }
}

fn decode_skesk(
    header: PacketHeader,
    body: &[u8],
    passphrase: &[u8],
) -> Result<(u8, RawSessionKey), OpenPgpError> {
    let version = *body.first().ok_or_else(malformed)?;
    if !matches!(version, 4..=6) {
        return Err(OpenPgpError::new(format!(
            "unsupported SKESK version {version}"
        )));
    }
    let [_, cipher_id, s2k_type, hash_id, contents @ ..] = body else {
        return Err(malformed());
    };
    let (cipher_id, s2k_type, hash_id) = (*cipher_id, *s2k_type, *hash_id);
    if !matches!(hash_id, 1..=3 | 8..=11) {
        return Err(OpenPgpError::new(format!(
            "unsupported S2K hash algorithm {hash_id}"
        )));
    }
    if !matches!(s2k_type, 0 | 1 | 3) {
        return Err(OpenPgpError::new(format!(
            "unsupported S2K type {s2k_type}"
        )));
    }
    if s2k_type == 3 && contents.len() < 9 {
        return Err(malformed());
    }
    let cipher = cipher(cipher_id)?;

    if s2k_type == 1 && contents.len() < 8 {
        // Historically available salt bytes were all used, even for truncated salt.
        let s2k = StringToKey::Simple {
            hash_alg: HashAlgorithm::from(hash_id),
        };
        let key = s2k
            .derive_key(&[contents, passphrase].concat(), cipher.key_size())
            .map_err(|_| malformed())?;
        return Ok((cipher_id, key));
    }
    // Older archive readers interpreted versions 4–6 with the v4 layout.
    let as_v4 = [4u8].as_slice().chain(&body[1..]);
    let packet =
        SymKeyEncryptedSessionKey::try_from_reader(header, as_v4).map_err(|_| malformed())?;
    let s2k = packet.s2k().ok_or_else(malformed)?;
    let key = s2k
        .derive_key(passphrase, cipher.key_size())
        .map_err(|_| malformed())?;
    let Some(encrypted_key) = packet.encrypted_key().filter(|key| !key.is_empty()) else {
        return Ok((cipher_id, key));
    };
    // The archive reader has historically deferred the inner cipher and key-length
    // check until the encrypted body is present. Use the same library primitive as
    // `packet.decrypt()` and retain that validation order.
    let mut plain = encrypted_key.to_vec();
    decrypt_cfb(cipher, key.as_ref(), &mut plain)?;
    let (&id, session_key) = plain.split_first().ok_or_else(malformed)?;
    Ok((id, RawSessionKey::from(session_key)))
}

fn decrypt_seipd(
    header: PacketHeader,
    body: &[u8],
    cipher_id: u8,
    key: &RawSessionKey,
) -> Result<Vec<u8>, OpenPgpError> {
    let (&version, ciphertext) = body.split_first().ok_or_else(malformed)?;
    if version != 1 {
        return Err(OpenPgpError::new(
            "unsupported encrypted-data packet (AEAD/SEIPD v2); install gpg/gpg2",
        ));
    }
    let cipher = cipher(cipher_id)?;
    if key.len() != cipher.key_size() {
        return Err(OpenPgpError::new(INVALID_KEYLEN));
    }
    let Some((prefix, _)) = ciphertext
        .split_at_checked(BLOCK + 2)
        .filter(|(_, rest)| rest.len() >= MDC_LEN)
    else {
        return Err(OpenPgpError::new(
            "encrypted data too short (corrupt archive)",
        ));
    };
    // The backend always verifies the MDC and no longer uses the advisory prefix.
    // Preserve the UCS reader's distinct failure and rejection for mismatched prefixes.
    let mut prefix: [u8; BLOCK + 2] = prefix.try_into().map_err(|_| malformed())?;
    decrypt_cfb(cipher, key.as_ref(), &mut prefix)?;
    if prefix[BLOCK - 2..BLOCK] != prefix[BLOCK..] {
        return Err(OpenPgpError::new(CHECK_FAILED));
    }
    let packet =
        SymEncryptedProtectedData::try_from_reader(header, body).map_err(|_| malformed())?;
    // The library collects the entire stream and returns it only after verifying the MDC.
    packet
        .decrypt(key.as_ref(), Some(cipher), Seipdv1ReadMode::Streaming)
        .map_err(|_| {
            // Preserve the old distinction between a missing and a mismatched MDC, without
            // duplicating successful packet decryption or returning unauthenticated bytes.
            let mut plain = ciphertext.to_vec();
            if decrypt_cfb(cipher, key.as_ref(), &mut plain).is_ok()
                && !plain[plain.len() - MDC_LEN..]
                    .starts_with(&[Tag::ModDetectionCode.encode(), 20])
            {
                OpenPgpError::new(MDC_MISSING)
            } else {
                OpenPgpError::new(INTEGRITY_FAILED)
            }
        })
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

fn extract_literal(data: &[u8], depth: u32) -> Result<Vec<u8>, OpenPgpError> {
    for (header, body) in parse_packets(data)? {
        match header.tag() {
            Tag::CompressedData => {
                if MAX_COMPRESSED_PACKET_DEPTH.exceeded(depth) {
                    return Err(OpenPgpError::new(
                        "OpenPGP message nests Compressed Data packets too deeply (malformed or hostile archive)",
                    ));
                }
                let (&algorithm, compressed) = body.split_first().ok_or_else(malformed)?;
                return extract_literal(&decompress(algorithm, compressed)?, depth + 1);
            }
            Tag::LiteralData => {
                let mut input = body.as_slice();
                return match LiteralDataHeader::try_from_reader(&mut input) {
                    Ok(_) => Ok(input.to_vec()),
                    // The archive reader accepts a truncated filename or timestamp as empty data.
                    Err(_) if body.len() >= 2 => Ok(Vec::new()),
                    Err(_) => Err(malformed()),
                };
            }
            _ => {}
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
    let mut encoded = Vec::new();
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
        encoded.extend(
            stripped
                .bytes()
                .filter(|byte| *byte != b'=' && !byte.is_ascii_whitespace()),
        );
    }
    // Historically a trailing lone symbol was validated but produced no byte.
    // Complete the group for the decoder, then discard its synthetic byte.
    let lone_symbol = encoded.len() % 4 == 1;
    if lone_symbol {
        encoded.push(b'A');
    }
    let mut body = ARMOR_BASE64.decode(encoded).map_err(|error| {
        let detail = match error {
            DecodeError::InvalidByte(_, byte) => format!("invalid base64 character {byte:#x}"),
            error => error.to_string(),
        };
        OpenPgpError::new(format!("malformed PGP ASCII armor: {detail}"))
    })?;
    if lone_symbol {
        body.pop();
    }
    Ok(body)
}

// Public API

/// Decrypt a passphrase-encrypted `OpenPGP` message; return the plaintext.
///
/// Returns [`OpenPgpError`] for a wrong passphrase, malformed input, or an
/// unsupported feature.
pub fn decrypt_symmetric(data: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, OpenPgpError> {
    let mut selected_key = None;
    let mut selected_packet = None;
    for (header, body) in parse_packets(&maybe_dearmor(data)?)? {
        match header.tag() {
            Tag::SymKeyEncryptedSessionKey => {
                selected_key = Some(decode_skesk(header, &body, passphrase)?);
            }
            Tag::PublicKeyEncryptedSessionKey => {
                return Err(OpenPgpError::new(
                    "public-key encrypted message — a passphrase cannot decrypt it",
                ));
            }
            Tag::SymEncryptedData | Tag::SymEncryptedProtectedData => {
                selected_packet = Some((header, body));
            }
            _ => {}
        }
    }
    let Some((cipher, key)) = selected_key else {
        return Err(OpenPgpError::new(
            "no symmetric-key session packet — this is not a passphrase-encrypted UCS",
        ));
    };
    let Some((header, body)) = selected_packet else {
        return Err(OpenPgpError::new(
            "no encrypted-data packet found in message",
        ));
    };
    if header.tag() == Tag::SymEncryptedData {
        return Err(OpenPgpError::new(
            "legacy SED packet (no MDC) is unsupported by the bundled decryptor — install gpg/gpg2 to decrypt this archive",
        ));
    }
    extract_literal(&decrypt_seipd(header, &body, cipher, &key)?, 0)
}

#[cfg(test)]
mod armor_tests {
    use super::maybe_dearmor;

    #[test]
    fn armor_keeps_the_legacy_separator_and_tail_rules() {
        let input = b"-----BEGIN PGP MESSAGE-----\r\nVersion: GnuPG\r\n\r\nT \t= R\r\n=abcd\r\n-----END PGP MESSAGE-----\r\n";
        assert_eq!(maybe_dearmor(input).unwrap(), b"M");

        let input = b"-----BEGIN PGP MESSAGE-----\nTWFuZ\n-----END PGP MESSAGE-----\n";
        assert_eq!(maybe_dearmor(input).unwrap(), b"Man");

        let input = b"-----BEGIN PGP MESSAGE-----\n!WFu?\n-----END PGP MESSAGE-----\n";
        assert_eq!(
            maybe_dearmor(input).unwrap_err().0,
            "malformed PGP ASCII armor: invalid base64 character 0x21"
        );
    }
}

#[cfg(test)]
mod cfb_tests {
    use super::{cipher, decrypt_cfb};

    #[test]
    fn decrypts_all_aes_widths_and_partial_final_blocks() {
        // OpenSSL aes-{128,192,256}-cfb with an all-zero IV and sequential key bytes.
        let key: Vec<u8> = (0..32).collect();
        let vectors = [
            (
                7,
                16,
                [
                    137, 209, 94, 89, 215, 200, 11, 162, 31, 46, 243, 22, 200, 169, 180, 84, 69,
                    77, 227, 144, 153, 94, 13, 80, 188, 227,
                ],
            ),
            (
                8,
                24,
                [
                    222, 18, 52, 236, 76, 52, 245, 2, 179, 247, 164, 83, 81, 96, 250, 42, 162, 21,
                    216, 134, 172, 178, 134, 184, 130, 110,
                ],
            ),
            (
                9,
                32,
                [
                    189, 224, 101, 216, 122, 14, 207, 240, 217, 146, 232, 30, 180, 79, 27, 173, 68,
                    79, 47, 211, 10, 113, 190, 176, 56, 94,
                ],
            ),
        ];
        for (cipher_id, keylen, mut ciphertext) in vectors {
            decrypt_cfb(cipher(cipher_id).unwrap(), &key[..keylen], &mut ciphertext).unwrap();
            assert_eq!(ciphertext.as_slice(), b"OpenPGP partial-block test");
        }
    }
}

#[cfg(test)]
mod recursion_tests {
    use super::*;

    use glomar_pgp::ser::Serialize;

    fn new_format_packet(tag: u8, body: &[u8]) -> Vec<u8> {
        let header = PacketHeader::new_fixed(Tag::from(tag), u32::try_from(body.len()).unwrap());
        let mut out = Vec::new();
        header.to_writer(&mut out).unwrap();
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn reads_short_initial_partial_length_and_bounds_maximum_fixed_length() {
        let partial = [0xcb, 224, b'b', 6, 0, 0, 0, 0, 0, 42];
        assert_eq!(extract_literal(&partial, 0).unwrap(), [42]);

        let fixed = [0xcb, 255, 255, 255, 255, 255, b'b', 0, 0, 0, 0, 0, 42];
        assert_eq!(extract_literal(&fixed, 0).unwrap(), [42]);
    }

    #[test]
    fn preserves_the_compressed_packet_limit() {
        let payload = b"payload";
        assert_eq!(
            extract_literal(&nested_compressed(17, payload), 0).unwrap(),
            payload
        );
        assert!(extract_literal(&nested_compressed(18, payload), 0).is_err());
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

    /// Regression coverage: `extract_literal` recurses
    /// natively once per nested Compressed Data (tag 8) packet, so it needs
    /// a depth cap — depth is fully attacker/generator
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
