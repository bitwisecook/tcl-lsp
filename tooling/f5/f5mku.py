"""F5 master-key (``f5mku``) secret encryption / decryption.

BIG-IP stores credential-bearing values (SSL key passphrases, monitor
passwords, RADIUS/TACACS shared secrets, …) in its configuration
encrypted under the unit master key.  On a device that key is what
``f5mku -K`` prints — a base64-encoded AES key — and the stored secrets
look like ``$M$<salt>$<base64-ciphertext>``.

The transform is AES in ECB mode with PKCS#7 padding: a two-character
salt is prepended to the plaintext, the result is padded to a 16-byte
boundary, encrypted block-by-block with the master key, and base64
encoded.  This module is a self-contained reimplementation of that
scheme so the ``f5`` CLI can encrypt and decrypt secrets offline, given
the master key.

It deliberately avoids the optional ``cryptography`` dependency: the
``f5`` CLI ships as a zipapp that strips C extensions, so the crypto
runs on the bundled pure-Python :class:`tooling.f5.f5_remote._aes.AES`
block cipher instead.

Examples:
    >>> decrypt("$M$iP$rr0su9oHn9J9p1t3nRzydA==", "BHDLd0bbao1VlwpTk1sioQ==")
    'KEY45678'
    >>> encrypt("KEY45678", "BHDLd0bbao1VlwpTk1sioQ==", salt="ab")
    '$M$ab$mmIL9xEWGe7pbNtvS/QAQA=='
"""

from __future__ import annotations

import secrets
import string
from base64 import b64decode, b64encode
from dataclasses import dataclass

from .f5_remote._aes import AES

__all__ = ["encrypt", "decrypt", "extract_salt", "F5MkuError"]

_BLOCK_SIZE = 16
_SALT_LENGTH = 2
_SALT_ALPHABET = string.ascii_letters + string.digits


class F5MkuError(ValueError):
    """Raised for malformed master keys or ciphertext."""


@dataclass(frozen=True, slots=True)
class _Ciphertext:
    salt: bytes
    ciphertext: bytes


def encrypt(plaintext: str, f5mku: str, salt: str | None = None) -> str:
    """Encrypt *plaintext* under the *f5mku* master key.

    *f5mku* is the base64 key printed by ``f5mku -K``.  *salt*, when
    given, is either a bare two-character string (no ``$``) or a full F5
    ciphertext to copy the salt from; otherwise a fresh random salt is
    generated.  Returns the ``$M$<salt>$<base64>`` form found in
    bigip.conf.
    """
    cipher = _load_key(f5mku)
    salt_bytes = _resolve_salt(salt)
    padded = _pkcs7_pad(salt_bytes + plaintext.encode("utf-8"))
    body = _ecb_encrypt(cipher, padded)
    return _format_ciphertext(salt_bytes, body)


def decrypt(ciphertext: str, f5mku: str) -> str:
    """Decrypt an ``$M$<salt>$<base64>`` *ciphertext* under *f5mku*.

    *f5mku* is the base64 key printed by ``f5mku -K``.  Returns the
    recovered plaintext.
    """
    parsed = _deconstruct_ciphertext(ciphertext)
    cipher = _load_key(f5mku)
    decrypted = _pkcs7_unpad(_ecb_decrypt(cipher, parsed.ciphertext))
    return _strip_salt(decrypted, parsed.salt).decode("utf-8")


def extract_salt(ciphertext: str) -> str:
    """Return the salt embedded in an F5 *ciphertext* string."""
    return _deconstruct_ciphertext(ciphertext).salt.decode("utf-8")


def is_ciphertext(value: str) -> bool:
    """Whether *value* looks like an F5 ``$M$<salt>$<body>`` secret."""
    parts = value.split("$")
    return len(parts) == 4 and parts[0] == "" and parts[1] == "M" and bool(parts[2] and parts[3])


def _load_key(f5mku: str) -> AES:
    try:
        return AES(b64decode(f5mku))
    except Exception as exc:  # noqa: BLE001
        raise F5MkuError(
            "decoding of f5mku failed. Make sure you are using the base64 "
            "formatted key returned by: f5mku -K"
        ) from exc


def _resolve_salt(salt: str | None) -> bytes:
    if salt is None:
        return "".join(secrets.choice(_SALT_ALPHABET) for _ in range(_SALT_LENGTH)).encode("utf-8")
    if "$" in salt:
        # Accept a full F5 ciphertext and reuse its salt.
        salt = extract_salt(salt)
    return salt.encode("utf-8")


def _ecb_encrypt(cipher: AES, data: bytes) -> bytes:
    return b"".join(
        cipher.encrypt_block(data[i : i + _BLOCK_SIZE]) for i in range(0, len(data), _BLOCK_SIZE)
    )


def _ecb_decrypt(cipher: AES, data: bytes) -> bytes:
    if not data or len(data) % _BLOCK_SIZE != 0:
        raise F5MkuError("ciphertext length is not a multiple of the AES block size")
    return b"".join(
        cipher.decrypt_block(data[i : i + _BLOCK_SIZE]) for i in range(0, len(data), _BLOCK_SIZE)
    )


def _pkcs7_pad(data: bytes) -> bytes:
    pad = _BLOCK_SIZE - (len(data) % _BLOCK_SIZE)
    return data + bytes([pad]) * pad


def _pkcs7_unpad(data: bytes) -> bytes:
    if not data:
        raise F5MkuError("empty plaintext after decryption")
    pad = data[-1]
    if pad < 1 or pad > _BLOCK_SIZE or data[-pad:] != bytes([pad]) * pad:
        raise F5MkuError("invalid PKCS#7 padding (wrong master key?)")
    return data[:-pad]


def _strip_salt(plaintext: bytes, salt: bytes) -> bytes:
    if not plaintext.startswith(salt):
        raise F5MkuError("decrypted text does not start with its salt (wrong master key?)")
    return plaintext[len(salt) :]


def _deconstruct_ciphertext(ciphertext: str) -> _Ciphertext:
    parts = ciphertext.split("$")
    if len(parts) != 4 or (parts[0], parts[1]) != ("", "M"):
        raise F5MkuError(
            f"unrecognised ciphertext: expected '$M$<salt>$<base64>', got {ciphertext!r}"
        )
    _, _, salt, body = parts
    if not salt:
        raise F5MkuError("unrecognised ciphertext: empty salt")
    if not body:
        raise F5MkuError("unrecognised ciphertext: empty ciphertext")
    try:
        decoded = b64decode(body, validate=True)
    except Exception as exc:  # noqa: BLE001
        raise F5MkuError(f"unrecognised ciphertext: malformed base64 {body!r}") from exc
    return _Ciphertext(salt=salt.encode("utf-8"), ciphertext=decoded)


def _format_ciphertext(salt: bytes, body: bytes) -> str:
    return "$".join(["", "M", salt.decode("utf-8"), b64encode(body).decode("ascii")])
