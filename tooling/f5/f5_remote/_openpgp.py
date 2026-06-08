"""Minimal, dependency-free OpenPGP *symmetric* (passphrase) decryptor.

This is the pure-Python fallback used by :mod:`.ucs` when no
``gpg``/``gpg2`` binary is available to decrypt an encrypted BIG-IP UCS
archive.  BIG-IP encrypts UCS files with GnuPG using a passphrase and a
128-bit AES symmetric cipher (F5 KB K5437), producing an ordinary
OpenPGP message:

    SKESK (tag 3)  → S2K-derived session key
    SEIPD (tag 18) → AES-CFB ciphertext + SHA-1 MDC
    (inner)        → optional Compressed Data (tag 8) wrapping
                     Literal Data (tag 11) — the gzip tar.

Only what that path needs is implemented:

* AES-128/192/256 (via :mod:`._aes`) — the only ciphers UCS uses.
* S2K simple / salted / iterated-and-salted, any :mod:`hashlib` digest.
* OpenPGP-CFB for SEIPD v1, with the quick-check and SHA-1 MDC verify.
* ZIP / ZLIB / BZIP2 inner compression (stdlib ``zlib`` / ``bz2``).
* Old- and new-format packet headers including partial body lengths.

Out of scope (raises :class:`OpenPGPError`, pointing at gpg): public-key
encryption, non-AES ciphers, AEAD / SEIPD v2, and the obsolete
MDC-less SED packet (tag 9).  The gpg path covers those when present.
"""

from __future__ import annotations

import bz2
import hashlib
import struct
import zlib

_BLOCK = 16

# OpenPGP symmetric-cipher algorithm IDs we can handle → AES key length.
_CIPHER_KEYLEN = {7: 16, 8: 24, 9: 32}

# OpenPGP hash algorithm IDs → hashlib constructor names.
_HASH_NAME = {
    1: "md5",
    2: "sha1",
    3: "ripemd160",
    8: "sha256",
    9: "sha384",
    10: "sha512",
    11: "sha224",
}


class OpenPGPError(ValueError):
    """The message could not be parsed or uses an unsupported feature."""


class OpenPGPDecryptError(OpenPGPError):
    """Decryption failed — almost always a wrong passphrase."""


# ---------------------------------------------------------------------------
# Packet framing
# ---------------------------------------------------------------------------


def _read_new_length(data: bytes, off: int) -> tuple[int, int, bool]:
    o1 = data[off]
    off += 1
    if o1 < 192:
        return o1, off, False
    if o1 < 224:
        return ((o1 - 192) << 8) + data[off] + 192, off + 1, False
    if o1 < 255:
        return 1 << (o1 & 0x1F), off, True  # partial body length
    length = struct.unpack_from(">I", data, off)[0]
    return length, off + 4, False


def _read_old_length(data: bytes, ctb: int, off: int) -> tuple[int, int]:
    length_type = ctb & 0x03
    if length_type == 0:
        return data[off], off + 1
    if length_type == 1:
        return struct.unpack_from(">H", data, off)[0], off + 2
    if length_type == 2:
        return struct.unpack_from(">I", data, off)[0], off + 4
    return len(data) - off, off  # indeterminate: to end of input


def _parse_packets(data: bytes) -> list[tuple[int, bytes]]:
    """Split an OpenPGP stream into ``(tag, body)`` pairs."""
    packets: list[tuple[int, bytes]] = []
    off = 0
    n = len(data)
    while off < n:
        ctb = data[off]
        off += 1
        if not ctb & 0x80:
            raise OpenPGPError("not an OpenPGP message (bad packet header)")
        if ctb & 0x40:  # new format
            tag = ctb & 0x3F
            body = bytearray()
            while True:
                length, off, partial = _read_new_length(data, off)
                body += data[off : off + length]
                off += length
                if not partial:
                    break
            packets.append((tag, bytes(body)))
        else:  # old format
            tag = (ctb >> 2) & 0x0F
            length, off = _read_old_length(data, ctb, off)
            packets.append((tag, data[off : off + length]))
            off += length
    return packets


# ---------------------------------------------------------------------------
# String-to-key and ciphers
# ---------------------------------------------------------------------------


def _parse_s2k(buf: bytes, off: int) -> tuple[str, bytes, int, int]:
    """Parse an S2K specifier; return ``(hash_name, salt, count, new_off)``.

    *count* is 0 for the non-iterated modes.  The S2K *type* is folded
    into the returned ``(salt, count)`` shape (salt empty ⇒ simple).
    """
    s2k_type = buf[off]
    hash_id = buf[off + 1]
    off += 2
    hash_name = _HASH_NAME.get(hash_id)
    if hash_name is None:
        raise OpenPGPError(f"unsupported S2K hash algorithm {hash_id}")
    salt = b""
    count = 0
    if s2k_type in (1, 3):
        salt = buf[off : off + 8]
        off += 8
    if s2k_type == 3:
        c = buf[off]
        off += 1
        count = (16 + (c & 0x0F)) << ((c >> 4) + 6)
    elif s2k_type not in (0, 1):
        raise OpenPGPError(f"unsupported S2K type {s2k_type}")
    return hash_name, salt, count, off


def _s2k_derive(passphrase: bytes, hash_name: str, salt: bytes, count: int, keylen: int) -> bytes:
    """Derive *keylen* key bytes from *passphrase* via OpenPGP S2K."""
    base = salt + passphrase
    if count and count > len(base):
        full, rem = divmod(count, len(base))
    else:
        full, rem = 1, 0  # simple/salted (or count below one full pass)
    out = bytearray()
    context = 0
    while len(out) < keylen:
        h = hashlib.new(hash_name)
        if context:
            h.update(b"\x00" * context)
        for _ in range(full):
            h.update(base)
        if rem:
            h.update(base[:rem])
        out += h.digest()
        context += 1
    return bytes(out[:keylen])


def _make_cipher(cipher_id: int, key: bytes):
    from ._aes import AES

    keylen = _CIPHER_KEYLEN.get(cipher_id)
    if keylen is None:
        raise OpenPGPError(
            f"unsupported symmetric cipher (algorithm {cipher_id}); "
            "the bundled decryptor only handles AES — install gpg/gpg2"
        )
    if len(key) != keylen:
        raise OpenPGPError("session key length does not match cipher")
    return AES(key)


def _cfb_decrypt(cipher, iv: bytes, data: bytes) -> bytes:
    """Standard CFB-128 decryption (OpenPGP SEIPD v1 uses an all-zero IV)."""
    out = bytearray()
    feedback = iv
    for i in range(0, len(data), _BLOCK):
        block = data[i : i + _BLOCK]
        keystream = cipher.encrypt_block(feedback)
        out += bytes(b ^ k for b, k in zip(block, keystream))
        feedback = block
    return bytes(out)


# ---------------------------------------------------------------------------
# Packet handlers
# ---------------------------------------------------------------------------


def _decode_skesk(body: bytes, passphrase: bytes) -> tuple[int, bytes]:
    """Symmetric-Key Encrypted Session Key packet → (cipher_id, session_key)."""
    version = body[0]
    if version not in (4, 5, 6):
        raise OpenPGPError(f"unsupported SKESK version {version}")
    cipher_id = body[1]
    hash_name, salt, count, off = _parse_s2k(body, 2)
    keylen = _CIPHER_KEYLEN.get(cipher_id)
    if keylen is None:
        raise OpenPGPError(
            f"unsupported symmetric cipher (algorithm {cipher_id}); "
            "the bundled decryptor only handles AES — install gpg/gpg2"
        )
    s2k_key = _s2k_derive(passphrase, hash_name, salt, count, keylen)
    encrypted_key = body[off:]
    if not encrypted_key:
        # No wrapped key: the S2K output is the session key directly.
        return cipher_id, s2k_key
    # Wrapped session key: CFB-decrypt with an all-zero IV; the first
    # plaintext byte is the real cipher algorithm, the rest is the key.
    decrypted = _cfb_decrypt(_make_cipher(cipher_id, s2k_key), b"\x00" * _BLOCK, encrypted_key)
    return decrypted[0], decrypted[1:]


def _decrypt_seipd(body: bytes, cipher_id: int, session_key: bytes) -> bytes:
    """Decrypt a Sym. Encrypted & Integrity Protected Data packet (tag 18)."""
    version = body[0]
    if version != 1:
        raise OpenPGPError("unsupported encrypted-data packet (AEAD/SEIPD v2); install gpg/gpg2")
    cipher = _make_cipher(cipher_id, session_key)
    plain = _cfb_decrypt(cipher, b"\x00" * _BLOCK, body[1:])
    if len(plain) < _BLOCK + 2 + 22:
        raise OpenPGPDecryptError("encrypted data too short (corrupt archive)")
    # Quick check: the two bytes after the random prefix repeat its tail.
    if plain[_BLOCK - 2 : _BLOCK] != plain[_BLOCK : _BLOCK + 2]:
        raise OpenPGPDecryptError("incorrect passphrase (quick-check failed)")
    mdc = plain[-22:]
    if mdc[0] != 0xD3 or mdc[1] != 0x14:
        raise OpenPGPDecryptError("incorrect passphrase or corrupt archive (MDC packet missing)")
    if hashlib.sha1(plain[:-20]).digest() != plain[-20:]:
        raise OpenPGPDecryptError("integrity check failed (wrong passphrase or corrupt archive)")
    return plain[_BLOCK + 2 : -22]


def _decompress(algo: int, data: bytes) -> bytes:
    if algo == 0:
        return data
    if algo == 1:
        return zlib.decompress(data, -15)  # raw DEFLATE (OpenPGP "ZIP")
    if algo == 2:
        return zlib.decompress(data)  # zlib-wrapped DEFLATE ("ZLIB")
    if algo == 3:
        return bz2.decompress(data)
    raise OpenPGPError(f"unsupported compression algorithm {algo}")


def _extract_literal(data: bytes) -> bytes:
    """Unwrap Compressed (tag 8) / Literal (tag 11) packets to the payload."""
    for tag, body in _parse_packets(data):
        if tag == 8:  # Compressed Data
            return _extract_literal(_decompress(body[0], body[1:]))
        if tag == 11:  # Literal Data: [format][namelen][name][mtime(4)][payload]
            namelen = body[1]
            return body[2 + namelen + 4 :]
    raise OpenPGPError("no literal-data packet found after decryption")


# ---------------------------------------------------------------------------
# ASCII armor (binary UCS files are not armored, but be liberal in input)
# ---------------------------------------------------------------------------


def _maybe_dearmor(data: bytes) -> bytes:
    if not data.lstrip()[:14].startswith(b"-----BEGIN PGP"):
        return data
    import base64

    lines = data.decode("ascii", "replace").splitlines()
    collecting = False
    b64: list[str] = []
    for line in lines:
        if line.startswith("-----BEGIN PGP"):
            collecting = True
            continue
        if line.startswith("-----END PGP"):
            break
        if not collecting:
            continue
        stripped = line.strip()
        if not stripped:  # blank line separating armor headers from body
            continue
        if stripped.startswith("="):  # CRC24 checksum line
            continue
        if ": " in line:  # armor header (Version:, Comment:, …)
            continue
        b64.append(stripped)
    try:
        return base64.b64decode("".join(b64))
    except Exception as exc:  # noqa: BLE001
        raise OpenPGPError(f"malformed PGP ASCII armor: {exc}") from exc


def decrypt_symmetric(data: bytes, passphrase: str | bytes) -> bytes:
    """Decrypt a passphrase-encrypted OpenPGP message; return the plaintext.

    Raises :class:`OpenPGPDecryptError` for a wrong passphrase and
    :class:`OpenPGPError` for malformed input or unsupported features.
    """
    if isinstance(passphrase, str):
        passphrase = passphrase.encode("utf-8")
    # Packet parsing indexes into the byte string and unpacks length
    # fields; on truncated/malformed input that surfaces as IndexError or
    # struct.error.  Normalise those to OpenPGPError so the public error
    # surface (and the UcsDecryptionError wrapper in ucs.py) stays
    # consistent instead of leaking an unexpected exception type.
    try:
        packets = _parse_packets(_maybe_dearmor(data))

        cipher_id: int | None = None
        session_key: bytes | None = None
        enc_tag: int | None = None
        enc_body: bytes | None = None
        for tag, body in packets:
            if tag == 3:
                cipher_id, session_key = _decode_skesk(body, passphrase)
            elif tag == 1:
                raise OpenPGPError("public-key encrypted message — a passphrase cannot decrypt it")
            elif tag in (9, 18):
                enc_tag, enc_body = tag, body

        if session_key is None or cipher_id is None:
            raise OpenPGPError(
                "no symmetric-key session packet — this is not a passphrase-encrypted UCS"
            )
        if enc_body is None:
            raise OpenPGPError("no encrypted-data packet found in message")
        if enc_tag == 9:
            raise OpenPGPError(
                "legacy SED packet (no MDC) is unsupported by the bundled "
                "decryptor — install gpg/gpg2 to decrypt this archive"
            )
        return _extract_literal(_decrypt_seipd(enc_body, cipher_id, session_key))
    except (IndexError, struct.error) as exc:
        raise OpenPGPError(f"malformed or truncated OpenPGP message: {exc}") from exc
