"""Tests for encrypted-UCS support: AES, the OpenPGP decryptor, gpg
shell-out, passphrase resolution, and the CLI wiring.

The two embedded vectors below are real GnuPG output, so the bundled
pure-Python decryptor is exercised on every platform — including hosts
with no ``gpg`` binary, which is exactly the scenario it exists for.
Tests that need to *create* fresh ciphertext skip when gpg is absent.
"""

from __future__ import annotations

import base64
import contextlib
import io
import os
import subprocess
import sys
import tempfile
import types
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tooling.f5.f5_remote import _openpgp, ucs  # noqa: E402
from tooling.f5.f5_remote._aes import AES  # noqa: E402
from tooling.f5.main import main  # noqa: E402

# --- Embedded vectors (passphrase "s3cr3t") ---------------------------------
# Generated with, e.g.:
#   gpg --batch --pinentry-mode loopback --passphrase s3cr3t \
#       --cipher-algo AES128 --symmetric -o out.ucs plain.tar.gz
# V1 is a literal-only AES-128 archive; V2 is AES-256 with inner ZLIB
# compression (so it exercises Compressed-Data + partial body lengths).
_PASSPHRASE = "s3cr3t"
_V1_LITERAL_AES128 = (
    "jA0EBwMK49Jsi2M7ANn/0sAUAfhH+tnwvBMY/kTzjzu0JueMYQFBH+yLElYT55hJ1AbQiFK6MnXC"
    "ZShykjGKNqVOMnzfEOjTUOXdCGT8V04JzIl5jmm+yaXycvyDDzv0QZzcnHVHHHmBom/Fk4DGPdCo"
    "OUl0+/8AeP2jD3zi4RhFxKwYXl8Z96xGxPFRKTtU7FlWdk1hlK3pUNgFJ6qnYBtYNEJCSsJ4Wlk"
    "V/rE7OnAbHoZa4/p3b6lSgeu8DFQBqhr5Ar90gXaMCNBr3h/e6PLUP0U8G0GxSrMs9jXJDQSb9Q"
    "2Kbs0="
)
_V2_COMPRESSED_AES256 = (
    "jA0ECQMKbu2Gk5VnuDD/0sB2ARIJMlAFKui1mqgf6rPak2/E7jUqZ0htu3OG/9R+yMQAc1PSqynV"
    "nXvnL57CRMozTvrjtz4aCv6jwGeHk4kfvBF26g3H+3kcBxapkrJef0PBSQEwvmpOIaFcf8f5MILA"
    "wLfWcHgGi1iDPCRYgJ1tvxKMX2qXRdh0hFJfEJaRWtb7kcTqOEasQkKaaMllIj3R+U1FImUGA6qQ"
    "u+YCGnOS93cYDd3prF4Lzz+W5oG0fHLOb86N4DqvQ1PHGdpfjBRvvq6kPdbIFGkbFPuQNnvhL1AB"
    "VyvVoSf9bIDzm2p6Ca4r7Uk+K3PRLHKW00VGKE5SP1n+8PFMNHdwVdCTg9nYVbmoCIbzbKH3xcOw"
    "xi4SAQzFcGXaX4tUyPKkBYVcvdRKeCZijBD0vT4/hpA3PO+TNN1ltGlQdA=="
)
_V1 = base64.b64decode(_V1_LITERAL_AES128)
_V2 = base64.b64decode(_V2_COMPRESSED_AES256)

_GPG = ucs._resolve_gpg() is not None
requires_gpg = pytest.mark.skipif(not _GPG, reason="gpg/gpg2 not on PATH")


def _gpg_encrypt(plaintext: bytes, passphrase: str, *flags: str) -> bytes:
    """Symmetric-encrypt *plaintext* with gpg in a throwaway home dir."""
    home = tempfile.mkdtemp(prefix="gpgtest-")
    os.chmod(home, 0o700)
    env = {**os.environ, "GNUPGHOME": home}
    inp = Path(home) / "in"
    out = Path(home) / "out.pgp"
    inp.write_bytes(plaintext)
    subprocess.run(
        [
            "gpg", "--batch", "--quiet", "--yes", "--pinentry-mode", "loopback",
            "--passphrase", passphrase, *flags, "--symmetric", "-o", str(out), str(inp),
        ],
        env=env, check=True, capture_output=True,
    )
    data = out.read_bytes()
    import shutil

    shutil.rmtree(home, ignore_errors=True)
    return data


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


# ---------------------------------------------------------------------------
# AES core
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "key,plain,cipher",
    [
        ("000102030405060708090a0b0c0d0e0f",
         "00112233445566778899aabbccddeeff", "69c4e0d86a7b0430d8cdb78070b4c55a"),
        ("000102030405060708090a0b0c0d0e0f1011121314151617",
         "00112233445566778899aabbccddeeff", "dda97ca4864cdfe06eaf70a0ec0d7191"),
        ("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
         "00112233445566778899aabbccddeeff", "8ea2b7ca516745bfeafc49904b496089"),
    ],
)
def test_aes_fips197_known_answers(key, plain, cipher):
    assert AES(bytes.fromhex(key)).encrypt_block(bytes.fromhex(plain)).hex() == cipher


# ---------------------------------------------------------------------------
# OpenPGP detection
# ---------------------------------------------------------------------------


def test_is_pgp_bytes_recognises_encrypted_and_rejects_plain():
    assert ucs.is_pgp_bytes(_V1)  # binary SKESK (tag 3 → 0x8C)
    assert ucs.is_pgp_bytes(b"-----BEGIN PGP MESSAGE-----\n\nfoo\n")
    plain = ucs.make_test_ucs({"config/bigip.conf": "ltm pool /Common/p { }"})
    assert ucs.is_ucs_bytes(plain) and not ucs.is_pgp_bytes(plain)
    assert not ucs.is_pgp_bytes(b"ltm pool /Common/p { }")
    assert not ucs.is_pgp_bytes(b"")


# ---------------------------------------------------------------------------
# Pure-Python decryptor (runs everywhere via the embedded vectors)
# ---------------------------------------------------------------------------


def test_pure_python_decrypts_literal_vector():
    plain = _openpgp.decrypt_symmetric(_V1, _PASSPHRASE)
    assert ucs.is_ucs_bytes(plain)
    assert "ltm pool /Common/encp" in ucs.ucs_to_scf(plain)


def test_pure_python_decrypts_compressed_vector():
    # Exercises inner Compressed-Data + partial body lengths.
    plain = _openpgp.decrypt_symmetric(_V2, _PASSPHRASE)
    assert ucs.is_ucs_bytes(plain)
    assert "ltm rule /Common/big" in ucs.ucs_to_scf(plain)


def test_pure_python_wrong_passphrase_raises():
    with pytest.raises(_openpgp.OpenPGPDecryptError):
        _openpgp.decrypt_symmetric(_V1, "not-the-passphrase")


def test_pure_python_rejects_legacy_sed_packet():
    # Hand-built: a simple-S2K SKESK followed by an MDC-less SED packet.
    skesk = bytes([0x8C, 0x04, 0x04, 0x07, 0x00, 0x02])
    sed = bytes([0xA4, 0x01, 0x00])
    with pytest.raises(_openpgp.OpenPGPError, match="SED|gpg"):
        _openpgp.decrypt_symmetric(skesk + sed, "x")


def test_pure_python_rejects_non_pgp():
    with pytest.raises(_openpgp.OpenPGPError):
        _openpgp.decrypt_symmetric(b"this is not openpgp", "x")


@requires_gpg
@pytest.mark.parametrize(
    "flags",
    [
        ("--cipher-algo", "AES128"),
        ("--cipher-algo", "AES192"),
        ("--cipher-algo", "AES256"),
        ("--cipher-algo", "AES128", "--s2k-mode", "0"),
        ("--cipher-algo", "AES128", "--s2k-mode", "1"),
        ("--cipher-algo", "AES128", "--s2k-digest-algo", "SHA1"),
        ("--cipher-algo", "AES256", "--compress-algo", "ZIP", "--compress-level", "9"),
    ],
)
def test_pure_python_matches_gpg(flags):
    payload = ucs.make_test_ucs({"config/bigip.conf": "ltm node /Common/n { }\n" * 80})
    ciphertext = _gpg_encrypt(payload, "pw", *flags)
    assert _openpgp.decrypt_symmetric(ciphertext, "pw") == payload


# ---------------------------------------------------------------------------
# decrypt_ucs_bytes dispatcher (gpg path + forced pure-Python path)
# ---------------------------------------------------------------------------


def test_decrypt_ucs_bytes_forced_pure_python(monkeypatch):
    monkeypatch.setenv("F5_UCS_PYPGP", "1")
    plain = ucs.decrypt_ucs_bytes(_V1, _PASSPHRASE)
    assert ucs.is_ucs_bytes(plain)


@requires_gpg
def test_decrypt_ucs_bytes_uses_gpg(monkeypatch):
    monkeypatch.delenv("F5_UCS_PYPGP", raising=False)
    plain = ucs.decrypt_ucs_bytes(_V1, _PASSPHRASE)
    assert ucs.is_ucs_bytes(plain)


def test_decrypt_ucs_bytes_wrong_passphrase(monkeypatch):
    monkeypatch.setenv("F5_UCS_PYPGP", "1")
    with pytest.raises(ucs.UcsDecryptionError):
        ucs.decrypt_ucs_bytes(_V1, "wrong")


# ---------------------------------------------------------------------------
# Passphrase resolution
# ---------------------------------------------------------------------------


def test_resolve_passphrase_explicit_then_env(monkeypatch):
    monkeypatch.delenv("F5_UCS_PASSPHRASE", raising=False)
    assert ucs.resolve_passphrase(explicit="abc") == "abc"
    monkeypatch.setenv("F5_UCS_PASSPHRASE", "from-env")
    assert ucs.resolve_passphrase() == "from-env"


def test_resolve_passphrase_missing_is_clear_error(monkeypatch):
    monkeypatch.delenv("F5_UCS_PASSPHRASE", raising=False)
    with pytest.raises(ucs.UcsPassphraseError, match="passphrase"):
        ucs.resolve_passphrase(allow_prompt=False)


def test_passphrase_provider_is_lazy_and_cached(monkeypatch):
    calls = {"n": 0}

    def fake_resolve(**_kw):
        calls["n"] += 1
        return "cached"

    monkeypatch.setattr(ucs, "resolve_passphrase", fake_resolve)
    provider = ucs.make_passphrase_provider()
    assert calls["n"] == 0  # nothing resolved until first call
    assert provider() == "cached"
    assert provider() == "cached"
    assert calls["n"] == 1  # resolved exactly once


# ---------------------------------------------------------------------------
# High-level archive helper + temp-file staging (Windows path)
# ---------------------------------------------------------------------------


def test_ucs_archive_to_scf_decrypts_with_provider():
    scf = ucs.ucs_archive_to_scf(
        _V1, passphrase_provider=ucs.make_passphrase_provider(explicit=_PASSPHRASE)
    )
    assert "ltm pool /Common/encp" in scf


def test_staged_ciphertext_warns_and_cleans_up():
    err = io.StringIO()
    with contextlib.redirect_stderr(err):
        with ucs._staged_ciphertext(b"ENCRYPTED") as tmp:
            assert os.path.exists(tmp)
            assert Path(tmp).read_bytes() == b"ENCRYPTED"
    assert not os.path.exists(tmp)
    message = err.getvalue()
    assert "warning" in message and tmp in message


def test_staged_ciphertext_cleans_up_on_exception():
    captured = {}
    with contextlib.suppress(RuntimeError), contextlib.redirect_stderr(io.StringIO()):
        with ucs._staged_ciphertext(b"X") as tmp:
            captured["tmp"] = tmp
            raise RuntimeError("boom")
    assert not os.path.exists(captured["tmp"])


@requires_gpg
def test_gpg_windows_branch(monkeypatch):
    # Force the non-POSIX (temp-file) decrypt path and confirm it works,
    # warns with the temp path, and leaves nothing behind.
    monkeypatch.delenv("F5_UCS_PYPGP", raising=False)
    monkeypatch.setattr(ucs.os, "name", "nt")
    err = io.StringIO()
    with contextlib.redirect_stderr(err):
        plain = ucs.decrypt_ucs_bytes(_V1, _PASSPHRASE)
    assert ucs.is_ucs_bytes(plain)
    assert "f5ucs-" in err.getvalue()
    leftovers = [f for f in os.listdir(tempfile.gettempdir()) if f.startswith("f5ucs-")]
    assert not leftovers


def test_gpg_install_hint_is_nonempty_string():
    hint = ucs._gpg_install_hint()
    assert isinstance(hint, str) and hint


# ---------------------------------------------------------------------------
# CLI end-to-end
# ---------------------------------------------------------------------------


def test_extract_cli_decrypts_with_env_passphrase(tmp_path, capsys, monkeypatch):
    enc = tmp_path / "device.ucs"
    enc.write_bytes(_V1)
    monkeypatch.setenv("F5_UCS_PASSPHRASE", _PASSPHRASE)
    code, out, err = _run(["extract", str(enc)], capsys)
    assert code == 0, err
    assert "ltm pool /Common/encp" in out


def test_extract_cli_decrypts_via_bundled_decrypter(tmp_path, capsys, monkeypatch):
    # Simulate a host with no gpg: force the pure-Python fallback.
    enc = tmp_path / "device.ucs"
    enc.write_bytes(_V1)
    monkeypatch.setenv("F5_UCS_PASSPHRASE", _PASSPHRASE)
    monkeypatch.setenv("F5_UCS_PYPGP", "1")
    code, out, err = _run(["extract", str(enc)], capsys)
    assert code == 0, err
    assert "ltm pool /Common/encp" in out


def test_extract_cli_passphrase_env_flag(tmp_path, capsys, monkeypatch):
    enc = tmp_path / "device.ucs"
    enc.write_bytes(_V1)
    monkeypatch.delenv("F5_UCS_PASSPHRASE", raising=False)
    monkeypatch.setenv("MY_UCS_PASS", _PASSPHRASE)
    code, out, err = _run(["extract", str(enc), "--passphrase-env", "MY_UCS_PASS"], capsys)
    assert code == 0, err
    assert "ltm pool /Common/encp" in out


def test_extract_cli_wrong_passphrase_errors(tmp_path, capsys, monkeypatch):
    enc = tmp_path / "device.ucs"
    enc.write_bytes(_V1)
    monkeypatch.setenv("F5_UCS_PASSPHRASE", "definitely-wrong")
    code, _out, err = _run(["extract", str(enc)], capsys)
    assert code == 2
    assert "error" in err.lower()


def test_extract_cli_missing_passphrase_is_helpful(tmp_path, capsys, monkeypatch):
    enc = tmp_path / "device.ucs"
    enc.write_bytes(_V1)
    monkeypatch.delenv("F5_UCS_PASSPHRASE", raising=False)
    # Non-interactive stdin → no prompt → clear error naming the env var.
    monkeypatch.setattr(sys, "stdin", types.SimpleNamespace(isatty=lambda: False))
    code, _out, err = _run(["extract", str(enc), "--no-passphrase-prompt"], capsys)
    assert code == 2
    assert "passphrase" in err.lower() and "F5_UCS_PASSPHRASE" in err


def test_query_cli_reads_encrypted_ucs(tmp_path, capsys, monkeypatch):
    enc = tmp_path / "device.ucs"
    enc.write_bytes(_V1)
    monkeypatch.setenv("F5_UCS_PASSPHRASE", _PASSPHRASE)
    code, out, err = _run(["query", ".ltm.pool[].name", str(enc)], capsys)
    assert code == 0, err
    assert "encp" in out  # the pool name, read end-to-end out of an encrypted UCS
