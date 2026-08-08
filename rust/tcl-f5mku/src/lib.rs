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

//! F5 master-key (`f5mku`) secret encryption / decryption.
//!
//! BIG-IP stores credential-bearing values (SSL key passphrases, monitor
//! passwords, RADIUS / TACACS shared secrets, …) in its configuration encrypted
//! under the unit master key. On a device that key is what `f5mku -K` prints — a
//! base64-encoded AES key — and the stored secrets look like
//! `$M$<salt>$<base64-ciphertext>`.
//!
//! The transform is AES in ECB mode with PKCS#7 padding: a two-character salt is
//! prepended to the plaintext, the result is padded to a 16-byte boundary,
//! encrypted block-by-block with the master key, and base64 encoded. The block
//! transform is delegated to the audited pure-Rust [`aes`] crate; this module
//! provides only the thin salt / PKCS#7 / base64 envelope logic on top of the
//! cipher.
//!
//! `decrypt`, `is_ciphertext` and `extract_salt` need no entropy and are always
//! available (so the crate builds for wasm32); `encrypt` with a random salt
//! requires the default `rand` feature.
//!
//! ```
//! # use tcl_f5mku::{decrypt, extract_salt};
//! let key = "BHDLd0bbao1VlwpTk1sioQ==";
//! assert_eq!(decrypt("$M$iP$rr0su9oHn9J9p1t3nRzydA==", key).unwrap(), "KEY45678");
//! assert_eq!(extract_salt("$M$iP$rr0su9oHn9J9p1t3nRzydA==").unwrap(), "iP");
//! ```

// `aes` 0.9 moved off `generic-array` onto `hybrid-array`'s `Array`, and split
// the block transforms out of `BlockEncrypt`/`BlockDecrypt` into
// `BlockCipherEncrypt`/`BlockCipherDecrypt`. Same operations, new names.
use aes::cipher::array::Array;
use aes::cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

const BLOCK_SIZE: usize = 16;
// Only the random-salt `encrypt` path uses these.
#[cfg(feature = "rand")]
const SALT_LENGTH: usize = 2;
#[cfg(feature = "rand")]
const SALT_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Raised for malformed master keys or ciphertext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F5MkuError(pub String);

impl std::fmt::Display for F5MkuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for F5MkuError {}

/// An AES cipher keyed for one of the three supported key lengths, exposing both
/// block transforms (ECB needs encrypt and decrypt).
enum Aes {
    A128(Box<Aes128>),
    A192(Box<Aes192>),
    A256(Box<Aes256>),
}

impl Aes {
    fn encrypt_block(&self, block: &[u8; BLOCK_SIZE]) -> [u8; BLOCK_SIZE] {
        let mut b = Array(*block);
        match self {
            Aes::A128(c) => c.encrypt_block(&mut b),
            Aes::A192(c) => c.encrypt_block(&mut b),
            Aes::A256(c) => c.encrypt_block(&mut b),
        }
        b.into()
    }

    fn decrypt_block(&self, block: &[u8; BLOCK_SIZE]) -> [u8; BLOCK_SIZE] {
        let mut b = Array(*block);
        match self {
            Aes::A128(c) => c.decrypt_block(&mut b),
            Aes::A192(c) => c.decrypt_block(&mut b),
            Aes::A256(c) => c.decrypt_block(&mut b),
        }
        b.into()
    }
}

/// Encrypt `plaintext` under the `f5mku` master key.
///
/// `f5mku` is the base64 key printed by `f5mku -K`. `salt`, when given, is
/// either a bare two-character string (no `$`) or a full F5 ciphertext to copy
/// the salt from; otherwise a fresh random salt is generated. Returns the
/// `$M$<salt>$<base64>` form found in bigip.conf.
///
/// # Errors
/// Returns [`F5MkuError`] when `f5mku` is not a valid base64 AES key, or when a
/// `salt` carrying a `$` is not a parseable ciphertext.
pub fn encrypt(plaintext: &str, f5mku: &str, salt: Option<&str>) -> Result<String, F5MkuError> {
    let cipher = load_key(f5mku)?;
    let salt_bytes = resolve_salt(salt)?;
    let mut data = salt_bytes.clone();
    data.extend_from_slice(plaintext.as_bytes());
    let padded = pkcs7_pad(&data);
    let body = ecb_encrypt(&cipher, &padded);
    Ok(format_ciphertext(&salt_bytes, &body))
}

/// Decrypt an `$M$<salt>$<base64>` `ciphertext` under `f5mku`.
///
/// `f5mku` is the base64 key printed by `f5mku -K`. Returns the recovered
/// plaintext.
///
/// # Errors
/// Returns [`F5MkuError`] when the key is invalid, the envelope is malformed, or
/// the PKCS#7 padding / leading salt does not check out (the wrong-key signal).
pub fn decrypt(ciphertext: &str, f5mku: &str) -> Result<String, F5MkuError> {
    let parsed = deconstruct_ciphertext(ciphertext)?;
    let cipher = load_key(f5mku)?;
    let decrypted = pkcs7_unpad(&ecb_decrypt(&cipher, &parsed.ciphertext)?)?;
    let stripped = strip_salt(&decrypted, &parsed.salt)?;
    String::from_utf8(stripped).map_err(|e| F5MkuError(format!("decrypted text is not UTF-8: {e}")))
}

/// Return the salt embedded in an F5 `ciphertext` string.
///
/// # Errors
/// Returns [`F5MkuError`] when `ciphertext` is not a valid `$M$<salt>$<base64>`
/// envelope.
pub fn extract_salt(ciphertext: &str) -> Result<String, F5MkuError> {
    let parsed = deconstruct_ciphertext(ciphertext)?;
    String::from_utf8(parsed.salt).map_err(|e| F5MkuError(format!("salt is not UTF-8: {e}")))
}

/// Whether `value` looks like an F5 `$M$<salt>$<body>` secret.
#[must_use]
pub fn is_ciphertext(value: &str) -> bool {
    let parts: Vec<&str> = value.split('$').collect();
    parts.len() == 4
        && parts[0].is_empty()
        && parts[1] == "M"
        && !parts[2].is_empty()
        && !parts[3].is_empty()
}

struct Ciphertext {
    salt: Vec<u8>,
    ciphertext: Vec<u8>,
}

fn load_key(f5mku: &str) -> Result<Aes, F5MkuError> {
    let key = B64.decode(f5mku).map_err(|_| bad_key())?;
    match key.len() {
        16 => Ok(Aes::A128(Box::new(Aes128::new(
            &Array::try_from(&key[..]).map_err(|_| bad_key())?,
        )))),
        24 => Ok(Aes::A192(Box::new(Aes192::new(
            &Array::try_from(&key[..]).map_err(|_| bad_key())?,
        )))),
        32 => Ok(Aes::A256(Box::new(Aes256::new(
            &Array::try_from(&key[..]).map_err(|_| bad_key())?,
        )))),
        _ => Err(bad_key()),
    }
}

fn bad_key() -> F5MkuError {
    F5MkuError(
        "decoding of f5mku failed. Make sure you are using the base64 \
         formatted key returned by: f5mku -K"
            .to_owned(),
    )
}

fn resolve_salt(salt: Option<&str>) -> Result<Vec<u8>, F5MkuError> {
    match salt {
        None => {
            // A fresh random two-char salt from `[A-Za-z0-9]`.
            #[cfg(feature = "rand")]
            {
                let mut buf = [0u8; SALT_LENGTH];
                getrandom::fill(&mut buf).map_err(|e| {
                    F5MkuError(format!("could not gather randomness for salt: {e}"))
                })?;
                Ok(buf
                    .iter()
                    .map(|b| SALT_ALPHABET[(*b as usize) % SALT_ALPHABET.len()])
                    .collect())
            }
            #[cfg(not(feature = "rand"))]
            Err(F5MkuError(
                "random-salt encryption requires the 'rand' feature".to_owned(),
            ))
        }
        Some(s) if s.contains('$') => {
            // Accept a full F5 ciphertext and reuse its salt.
            Ok(extract_salt(s)?.into_bytes())
        }
        Some(s) => Ok(s.as_bytes().to_vec()),
    }
}

fn ecb_encrypt(cipher: &Aes, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(BLOCK_SIZE) {
        let block: [u8; BLOCK_SIZE] = chunk.try_into().expect("chunks_exact yields full blocks");
        out.extend_from_slice(&cipher.encrypt_block(&block));
    }
    out
}

fn ecb_decrypt(cipher: &Aes, data: &[u8]) -> Result<Vec<u8>, F5MkuError> {
    if data.is_empty() || !data.len().is_multiple_of(BLOCK_SIZE) {
        return Err(F5MkuError(
            "ciphertext length is not a multiple of the AES block size".to_owned(),
        ));
    }
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(BLOCK_SIZE) {
        let block: [u8; BLOCK_SIZE] = chunk.try_into().expect("chunks_exact yields full blocks");
        out.extend_from_slice(&cipher.decrypt_block(&block));
    }
    Ok(out)
}

fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad = BLOCK_SIZE - (data.len() % BLOCK_SIZE); // 1..=16
    let pad_byte = u8::try_from(pad).expect("pad is in 1..=16");
    let mut out = data.to_vec();
    out.extend(std::iter::repeat_n(pad_byte, pad));
    out
}

fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, F5MkuError> {
    let Some(&pad) = data.last() else {
        return Err(F5MkuError("empty plaintext after decryption".to_owned()));
    };
    let pad = usize::from(pad);
    if !(1..=BLOCK_SIZE).contains(&pad)
        || data.len() < pad
        || data[data.len() - pad..]
            .iter()
            .any(|&b| usize::from(b) != pad)
    {
        return Err(F5MkuError(
            "invalid PKCS#7 padding (wrong master key?)".to_owned(),
        ));
    }
    Ok(data[..data.len() - pad].to_vec())
}

fn strip_salt(plaintext: &[u8], salt: &[u8]) -> Result<Vec<u8>, F5MkuError> {
    if !plaintext.starts_with(salt) {
        return Err(F5MkuError(
            "decrypted text does not start with its salt (wrong master key?)".to_owned(),
        ));
    }
    Ok(plaintext[salt.len()..].to_vec())
}

fn deconstruct_ciphertext(ciphertext: &str) -> Result<Ciphertext, F5MkuError> {
    let parts: Vec<&str> = ciphertext.split('$').collect();
    if parts.len() != 4 || !parts[0].is_empty() || parts[1] != "M" {
        return Err(F5MkuError(format!(
            "unrecognised ciphertext: expected '$M$<salt>$<base64>', got {ciphertext:?}"
        )));
    }
    let salt = parts[2];
    let body = parts[3];
    if salt.is_empty() {
        return Err(F5MkuError("unrecognised ciphertext: empty salt".to_owned()));
    }
    if body.is_empty() {
        return Err(F5MkuError(
            "unrecognised ciphertext: empty ciphertext".to_owned(),
        ));
    }
    let decoded = B64.decode(body).map_err(|_| {
        F5MkuError(format!(
            "unrecognised ciphertext: malformed base64 {body:?}"
        ))
    })?;
    Ok(Ciphertext {
        salt: salt.as_bytes().to_vec(),
        ciphertext: decoded,
    })
}

fn format_ciphertext(salt: &[u8], body: &[u8]) -> String {
    let salt = String::from_utf8_lossy(salt);
    format!("$M${salt}${}", B64.encode(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "BHDLd0bbao1VlwpTk1sioQ==";

    #[test]
    fn known_answers() {
        assert_eq!(
            decrypt("$M$iP$rr0su9oHn9J9p1t3nRzydA==", KEY).unwrap(),
            "KEY45678"
        );
        assert_eq!(
            encrypt("KEY45678", KEY, Some("ab")).unwrap(),
            "$M$ab$mmIL9xEWGe7pbNtvS/QAQA=="
        );
        assert_eq!(
            extract_salt("$M$iP$rr0su9oHn9J9p1t3nRzydA==").unwrap(),
            "iP"
        );
    }

    #[test]
    fn salt_from_ciphertext() {
        // A full ciphertext may be reused as the salt source.
        assert_eq!(
            encrypt("KEY45678", KEY, Some("$M$ab$mmIL9xEWGe7pbNtvS/QAQA==")).unwrap(),
            "$M$ab$mmIL9xEWGe7pbNtvS/QAQA=="
        );
    }

    #[test]
    fn round_trip_random_salt() {
        for plaintext in ["", "x", "hunter2", "spaces and $ymbol$", "é—utf8"] {
            let ct = encrypt(plaintext, KEY, None).unwrap();
            assert!(is_ciphertext(&ct), "{ct} should look like ciphertext");
            assert_eq!(decrypt(&ct, KEY).unwrap(), plaintext);
        }
    }

    #[test]
    fn bad_key_raises() {
        assert!(encrypt("x", "not base64!!!", None).is_err());
    }

    #[test]
    fn wrong_key_raises() {
        let ct = encrypt("topsecret", KEY, None).unwrap();
        assert!(decrypt(&ct, "AAAAAAAAAAAAAAAAAAAAAA==").is_err());
    }

    #[test]
    fn malformed_ciphertext() {
        for bad in ["plain", "$M$$body==", "$M$ab$", "$X$ab$body=="] {
            assert!(!is_ciphertext(bad), "{bad} should not look like ciphertext");
            assert!(decrypt(bad, KEY).is_err());
        }
    }
}
