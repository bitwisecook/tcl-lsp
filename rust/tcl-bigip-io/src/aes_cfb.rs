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

//! AES block cipher + OpenPGP-style CFB, the cryptographic primitives the
//! encrypted-UCS path needs.
//!
//! Only the forward (encrypt) direction is needed. The block
//! transform is delegated to the audited pure-Rust [`aes`] crate, leaving only
//! the thin key-length dispatch + CFB feedback loop on top of
//! `AES.encrypt_block`.

// `aes` 0.9 moved off `generic-array` onto `hybrid-array`'s `Array`, and
// renamed `BlockEncrypt` to `BlockCipherEncrypt`. Same operation, new names.
use aes::cipher::array::Array;
use aes::cipher::{BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256};

const BLOCK: usize = 16;

/// An AES cipher keyed for one of the three supported key lengths, exposing
/// only the forward (encrypt) block transform — all OpenPGP-CFB needs.
pub(crate) enum Aes {
    A128(Box<Aes128>),
    A192(Box<Aes192>),
    A256(Box<Aes256>),
}

impl Aes {
    /// Key an AES cipher; `key` must be 16, 24, or 32 bytes (AES-128/192/256).
    pub(crate) fn new(key: &[u8]) -> Result<Self, String> {
        match key.len() {
            16 => Ok(Aes::A128(Box::new(Aes128::new(
                &Array::try_from(key).map_err(|_| "AES key length mismatch".to_owned())?,
            )))),
            24 => Ok(Aes::A192(Box::new(Aes192::new(
                &Array::try_from(key).map_err(|_| "AES key length mismatch".to_owned())?,
            )))),
            32 => Ok(Aes::A256(Box::new(Aes256::new(
                &Array::try_from(key).map_err(|_| "AES key length mismatch".to_owned())?,
            )))),
            n => Err(format!("AES key must be 16/24/32 bytes, got {n}")),
        }
    }

    /// Encrypt a single 16-byte block (the keystream generator for CFB).
    fn encrypt_block(&self, block: &[u8; BLOCK]) -> [u8; BLOCK] {
        let mut b = Array(*block);
        match self {
            Aes::A128(c) => c.encrypt_block(&mut b),
            Aes::A192(c) => c.encrypt_block(&mut b),
            Aes::A256(c) => c.encrypt_block(&mut b),
        }
        b.into()
    }
}

/// Standard CFB-128 decryption (`OpenPGP` SEIPD v1 uses an all-zero IV).
///
/// AES-CFB decryption: the keystream is `E(feedback)` and the feedback for
/// the next block is the *ciphertext* block just consumed. A short trailing
/// block is `XORed` against the leading keystream bytes (truncated to
/// the shorter operand) and never re-used as feedback.
pub(crate) fn cfb_decrypt(cipher: &Aes, iv: &[u8; BLOCK], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut feedback = *iv;
    let mut i = 0;
    while i < data.len() {
        let end = (i + BLOCK).min(data.len());
        let block = &data[i..end];
        let keystream = cipher.encrypt_block(&feedback);
        for (j, &b) in block.iter().enumerate() {
            out.push(b ^ keystream[j]);
        }
        let mut next = [0u8; BLOCK];
        next[..block.len()].copy_from_slice(block);
        feedback = next;
        i = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FIPS-197 Appendix B / C known-answer vectors for the forward transform.
    #[test]
    fn fips197_known_answers() {
        // AES-128 (FIPS-197 §C.1).
        let key = hex("000102030405060708090a0b0c0d0e0f");
        let pt = hex("00112233445566778899aabbccddeeff");
        let ct = hex("69c4e0d86a7b0430d8cdb78070b4c55a");
        assert_eq!(
            Aes::new(&key).unwrap().encrypt_block(&arr(&pt)).to_vec(),
            ct
        );

        // AES-192 (FIPS-197 §C.2).
        let key = hex("000102030405060708090a0b0c0d0e0f1011121314151617");
        let ct = hex("dda97ca4864cdfe06eaf70a0ec0d7191");
        assert_eq!(
            Aes::new(&key).unwrap().encrypt_block(&arr(&pt)).to_vec(),
            ct
        );

        // AES-256 (FIPS-197 §C.3).
        let key = hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
        let ct = hex("8ea2b7ca516745bfeafc49904b496089");
        assert_eq!(
            Aes::new(&key).unwrap().encrypt_block(&arr(&pt)).to_vec(),
            ct
        );
    }

    #[test]
    fn cfb_round_trips_against_self() {
        // CFB decrypt(encrypt(x)) == x using the same keystream construction.
        let cipher = Aes::new(&hex("000102030405060708090a0b0c0d0e0f")).unwrap();
        let iv = [0u8; BLOCK];
        let plain = b"the quick brown fox jumps over the lazy dog!!".to_vec();
        // Encrypt via the standard CFB construction (keystream xor plaintext).
        let mut ct = Vec::new();
        let mut fb = iv;
        let mut i = 0;
        while i < plain.len() {
            let end = (i + BLOCK).min(plain.len());
            let ks = cipher.encrypt_block(&fb);
            let mut blk = [0u8; BLOCK];
            for (j, &p) in plain[i..end].iter().enumerate() {
                let c = p ^ ks[j];
                ct.push(c);
                blk[j] = c;
            }
            fb = blk;
            i = end;
        }
        assert_eq!(cfb_decrypt(&cipher, &iv, &ct), plain);
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
    fn arr(v: &[u8]) -> [u8; BLOCK] {
        let mut a = [0u8; BLOCK];
        a.copy_from_slice(v);
        a
    }
}
