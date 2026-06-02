"""Pure-Python AES block cipher (encryption primitive only).

This is a self-contained, dependency-free implementation used as a
*fallback* for decrypting encrypted (OpenPGP) UCS archives when no
``gpg``/``gpg2`` binary is available — see :mod:`._openpgp`.  The
production path shells out to GnuPG (exactly what BIG-IP itself uses);
this module exists purely so the zipapp can still decrypt a UCS on a
host with no GnuPG installed, because the zipapp builder strips C
extensions and therefore cannot bundle ``cryptography``/PGPy.

Only the *forward* (encryption) transform is implemented: OpenPGP CFB
mode — the only mode UCS encryption uses — runs the block cipher in the
encryption direction for both encrypting and decrypting, so a decryptor
never needs ``InvSubBytes``/``InvMixColumns``.

Correctness is anchored by the FIPS-197 known-answer vectors and by
round-trip tests against GnuPG output (see ``tests/test_f5_ucs_crypto.py``).
Performance is adequate for the fallback role (a few MB/s in CPython via
the standard T-table formulation); the gpg path is used whenever it is
available.
"""

from __future__ import annotations

import struct

# AES forward S-box (FIPS-197, Figure 7).
_SBOX = (
    0x63,
    0x7C,
    0x77,
    0x7B,
    0xF2,
    0x6B,
    0x6F,
    0xC5,
    0x30,
    0x01,
    0x67,
    0x2B,
    0xFE,
    0xD7,
    0xAB,
    0x76,
    0xCA,
    0x82,
    0xC9,
    0x7D,
    0xFA,
    0x59,
    0x47,
    0xF0,
    0xAD,
    0xD4,
    0xA2,
    0xAF,
    0x9C,
    0xA4,
    0x72,
    0xC0,
    0xB7,
    0xFD,
    0x93,
    0x26,
    0x36,
    0x3F,
    0xF7,
    0xCC,
    0x34,
    0xA5,
    0xE5,
    0xF1,
    0x71,
    0xD8,
    0x31,
    0x15,
    0x04,
    0xC7,
    0x23,
    0xC3,
    0x18,
    0x96,
    0x05,
    0x9A,
    0x07,
    0x12,
    0x80,
    0xE2,
    0xEB,
    0x27,
    0xB2,
    0x75,
    0x09,
    0x83,
    0x2C,
    0x1A,
    0x1B,
    0x6E,
    0x5A,
    0xA0,
    0x52,
    0x3B,
    0xD6,
    0xB3,
    0x29,
    0xE3,
    0x2F,
    0x84,
    0x53,
    0xD1,
    0x00,
    0xED,
    0x20,
    0xFC,
    0xB1,
    0x5B,
    0x6A,
    0xCB,
    0xBE,
    0x39,
    0x4A,
    0x4C,
    0x58,
    0xCF,
    0xD0,
    0xEF,
    0xAA,
    0xFB,
    0x43,
    0x4D,
    0x33,
    0x85,
    0x45,
    0xF9,
    0x02,
    0x7F,
    0x50,
    0x3C,
    0x9F,
    0xA8,
    0x51,
    0xA3,
    0x40,
    0x8F,
    0x92,
    0x9D,
    0x38,
    0xF5,
    0xBC,
    0xB6,
    0xDA,
    0x21,
    0x10,
    0xFF,
    0xF3,
    0xD2,
    0xCD,
    0x0C,
    0x13,
    0xEC,
    0x5F,
    0x97,
    0x44,
    0x17,
    0xC4,
    0xA7,
    0x7E,
    0x3D,
    0x64,
    0x5D,
    0x19,
    0x73,
    0x60,
    0x81,
    0x4F,
    0xDC,
    0x22,
    0x2A,
    0x90,
    0x88,
    0x46,
    0xEE,
    0xB8,
    0x14,
    0xDE,
    0x5E,
    0x0B,
    0xDB,
    0xE0,
    0x32,
    0x3A,
    0x0A,
    0x49,
    0x06,
    0x24,
    0x5C,
    0xC2,
    0xD3,
    0xAC,
    0x62,
    0x91,
    0x95,
    0xE4,
    0x79,
    0xE7,
    0xC8,
    0x37,
    0x6D,
    0x8D,
    0xD5,
    0x4E,
    0xA9,
    0x6C,
    0x56,
    0xF4,
    0xEA,
    0x65,
    0x7A,
    0xAE,
    0x08,
    0xBA,
    0x78,
    0x25,
    0x2E,
    0x1C,
    0xA6,
    0xB4,
    0xC6,
    0xE8,
    0xDD,
    0x74,
    0x1F,
    0x4B,
    0xBD,
    0x8B,
    0x8A,
    0x70,
    0x3E,
    0xB5,
    0x66,
    0x48,
    0x03,
    0xF6,
    0x0E,
    0x61,
    0x35,
    0x57,
    0xB9,
    0x86,
    0xC1,
    0x1D,
    0x9E,
    0xE1,
    0xF8,
    0x98,
    0x11,
    0x69,
    0xD9,
    0x8E,
    0x94,
    0x9B,
    0x1E,
    0x87,
    0xE9,
    0xCE,
    0x55,
    0x28,
    0xDF,
    0x8C,
    0xA1,
    0x89,
    0x0D,
    0xBF,
    0xE6,
    0x42,
    0x68,
    0x41,
    0x99,
    0x2D,
    0x0F,
    0xB0,
    0x54,
    0xBB,
    0x16,
)

_RCON = (
    0x01000000,
    0x02000000,
    0x04000000,
    0x08000000,
    0x10000000,
    0x20000000,
    0x40000000,
    0x80000000,
    0x1B000000,
    0x36000000,
    0x6C000000,
    0xD8000000,
    0xAB000000,
    0x4D000000,
)


def _xtime(a: int) -> int:
    a <<= 1
    return (a ^ 0x11B) & 0xFF if a & 0x100 else a


def _build_te() -> tuple[list[int], list[int], list[int], list[int]]:
    """Build the four 256-entry AES encryption T-tables."""
    te0: list[int] = [0] * 256
    te1: list[int] = [0] * 256
    te2: list[int] = [0] * 256
    te3: list[int] = [0] * 256
    for x in range(256):
        s = _SBOX[x]
        s2 = _xtime(s)
        s3 = s2 ^ s
        te0[x] = ((s2 << 24) | (s << 16) | (s << 8) | s3) & 0xFFFFFFFF
        # Te1/2/3 are Te0 rotated right by 8/16/24 bits.
        te1[x] = ((te0[x] >> 8) | (te0[x] << 24)) & 0xFFFFFFFF
        te2[x] = ((te0[x] >> 16) | (te0[x] << 16)) & 0xFFFFFFFF
        te3[x] = ((te0[x] >> 24) | (te0[x] << 8)) & 0xFFFFFFFF
    return te0, te1, te2, te3


_TE0, _TE1, _TE2, _TE3 = _build_te()


def _sub_word(w: int) -> int:
    return (
        (_SBOX[(w >> 24) & 0xFF] << 24)
        | (_SBOX[(w >> 16) & 0xFF] << 16)
        | (_SBOX[(w >> 8) & 0xFF] << 8)
        | _SBOX[w & 0xFF]
    )


class AES:
    """AES-128/192/256 with only the forward (encrypt) block transform."""

    __slots__ = ("_rk", "_rounds")

    def __init__(self, key: bytes) -> None:
        nk = len(key) // 4
        if len(key) not in (16, 24, 32):
            raise ValueError(f"AES key must be 16/24/32 bytes, got {len(key)}")
        self._rounds = nk + 6
        words = list(struct.unpack(f">{nk}I", key))
        total = 4 * (self._rounds + 1)
        for i in range(nk, total):
            temp = words[i - 1]
            if i % nk == 0:
                temp = _sub_word(((temp << 8) | (temp >> 24)) & 0xFFFFFFFF) ^ _RCON[i // nk - 1]
            elif nk > 6 and i % nk == 4:
                temp = _sub_word(temp)
            words.append((words[i - nk] ^ temp) & 0xFFFFFFFF)
        self._rk = words

    def encrypt_block(self, block: bytes) -> bytes:
        """Encrypt a single 16-byte block."""
        rk = self._rk
        s0, s1, s2, s3 = struct.unpack(">4I", block)
        s0 ^= rk[0]
        s1 ^= rk[1]
        s2 ^= rk[2]
        s3 ^= rk[3]
        te0, te1, te2, te3 = _TE0, _TE1, _TE2, _TE3
        k = 4
        for _ in range(self._rounds - 1):
            t0 = (
                te0[(s0 >> 24) & 0xFF]
                ^ te1[(s1 >> 16) & 0xFF]
                ^ te2[(s2 >> 8) & 0xFF]
                ^ te3[s3 & 0xFF]
                ^ rk[k]
            )
            t1 = (
                te0[(s1 >> 24) & 0xFF]
                ^ te1[(s2 >> 16) & 0xFF]
                ^ te2[(s3 >> 8) & 0xFF]
                ^ te3[s0 & 0xFF]
                ^ rk[k + 1]
            )
            t2 = (
                te0[(s2 >> 24) & 0xFF]
                ^ te1[(s3 >> 16) & 0xFF]
                ^ te2[(s0 >> 8) & 0xFF]
                ^ te3[s1 & 0xFF]
                ^ rk[k + 2]
            )
            t3 = (
                te0[(s3 >> 24) & 0xFF]
                ^ te1[(s0 >> 16) & 0xFF]
                ^ te2[(s1 >> 8) & 0xFF]
                ^ te3[s2 & 0xFF]
                ^ rk[k + 3]
            )
            s0, s1, s2, s3 = t0, t1, t2, t3
            k += 4
        # Final round: SubBytes + ShiftRows + AddRoundKey (no MixColumns).
        sbox = _SBOX
        o0 = (
            (sbox[(s0 >> 24) & 0xFF] << 24)
            | (sbox[(s1 >> 16) & 0xFF] << 16)
            | (sbox[(s2 >> 8) & 0xFF] << 8)
            | sbox[s3 & 0xFF]
        ) ^ rk[k]
        o1 = (
            (sbox[(s1 >> 24) & 0xFF] << 24)
            | (sbox[(s2 >> 16) & 0xFF] << 16)
            | (sbox[(s3 >> 8) & 0xFF] << 8)
            | sbox[s0 & 0xFF]
        ) ^ rk[k + 1]
        o2 = (
            (sbox[(s2 >> 24) & 0xFF] << 24)
            | (sbox[(s3 >> 16) & 0xFF] << 16)
            | (sbox[(s0 >> 8) & 0xFF] << 8)
            | sbox[s1 & 0xFF]
        ) ^ rk[k + 2]
        o3 = (
            (sbox[(s3 >> 24) & 0xFF] << 24)
            | (sbox[(s0 >> 16) & 0xFF] << 16)
            | (sbox[(s1 >> 8) & 0xFF] << 8)
            | sbox[s2 & 0xFF]
        ) ^ rk[k + 3]
        return struct.pack(
            ">4I", o0 & 0xFFFFFFFF, o1 & 0xFFFFFFFF, o2 & 0xFFFFFFFF, o3 & 0xFFFFFFFF
        )
