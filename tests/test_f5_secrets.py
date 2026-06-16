"""Tests for F5 master-key (``f5mku``) secret crypto: the AES inverse
transform, the :mod:`tooling.f5.f5mku` primitive, the secret-locating
rewrite, and the ``encrypt-secrets`` / ``decrypt-secrets`` CLI verbs.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dialects.f5.bigip.rewrite import rewrite_secrets  # noqa: E402
from tooling.f5 import f5mku  # noqa: E402
from tooling.f5.f5_remote._aes import AES  # noqa: E402
from tooling.f5.main import main  # noqa: E402

# A real F5 master key + its known-answer ciphertext, from the original
# f5mkupy implementation's docstrings.
_KEY = "BHDLd0bbao1VlwpTk1sioQ=="


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


# --- AES inverse transform --------------------------------------------------


@pytest.mark.parametrize(
    "key,plain,cipher",
    [
        (
            "000102030405060708090a0b0c0d0e0f",
            "00112233445566778899aabbccddeeff",
            "69c4e0d86a7b0430d8cdb78070b4c55a",
        ),
        (
            "000102030405060708090a0b0c0d0e0f1011121314151617",
            "00112233445566778899aabbccddeeff",
            "dda97ca4864cdfe06eaf70a0ec0d7191",
        ),
        (
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            "00112233445566778899aabbccddeeff",
            "8ea2b7ca516745bfeafc49904b496089",
        ),
    ],
)
def test_aes_decrypt_block_fips197(key, plain, cipher):
    assert AES(bytes.fromhex(key)).decrypt_block(bytes.fromhex(cipher)).hex() == plain


def test_aes_decrypt_rejects_short_block():
    with pytest.raises(ValueError):
        AES(bytes(16)).decrypt_block(b"short")


# --- f5mku primitive --------------------------------------------------------


def test_f5mku_known_answers():
    assert f5mku.decrypt("$M$iP$rr0su9oHn9J9p1t3nRzydA==", _KEY) == "KEY45678"
    assert f5mku.encrypt("KEY45678", _KEY, salt="ab") == "$M$ab$mmIL9xEWGe7pbNtvS/QAQA=="
    assert f5mku.extract_salt("$M$iP$rr0su9oHn9J9p1t3nRzydA==") == "iP"


def test_f5mku_salt_from_ciphertext():
    # A full ciphertext may be reused as the salt source.
    assert f5mku.encrypt("KEY45678", _KEY, salt="$M$ab$mmIL9xEWGe7pbNtvS/QAQA==") == (
        "$M$ab$mmIL9xEWGe7pbNtvS/QAQA=="
    )


@pytest.mark.parametrize("plaintext", ["", "x", "hunter2", "spaces and $ymbol$", "é—utf8"])
def test_f5mku_round_trip_random_salt(plaintext):
    ct = f5mku.encrypt(plaintext, _KEY)
    assert f5mku.is_ciphertext(ct)
    assert f5mku.decrypt(ct, _KEY) == plaintext


def test_f5mku_bad_key_raises():
    with pytest.raises(f5mku.F5MkuError):
        f5mku.encrypt("x", "not base64!!!")


def test_f5mku_wrong_key_raises():
    ct = f5mku.encrypt("topsecret", _KEY)
    with pytest.raises(f5mku.F5MkuError):
        f5mku.decrypt(ct, "AAAAAAAAAAAAAAAAAAAAAA==")


@pytest.mark.parametrize("bad", ["plain", "$M$$body==", "$M$ab$", "$X$ab$body=="])
def test_f5mku_malformed_ciphertext(bad):
    assert not f5mku.is_ciphertext(bad)
    with pytest.raises(f5mku.F5MkuError):
        f5mku.decrypt(bad, _KEY)


# --- secret-locating rewrite ------------------------------------------------

_CONF = """\
ltm monitor https /Common/mon {
    password clearpw
    username admin
}
auth radius-server /Common/rad {
    secret "my radius secret"
}
sys snmp {
    communities {
        public { community-name public }
    }
}
"""


def test_rewrite_only_touches_encrypted_secret_keys():
    report = rewrite_secrets(_CONF, lambda v: f"<{v}>")
    # password + secret are rewritten; the SNMP community-name and the
    # username are not.
    assert report.transformed == 2
    assert "password <clearpw>" in report.new_source
    assert "<my radius secret>" in report.new_source
    assert "community-name public" in report.new_source
    assert "username admin" in report.new_source


def test_rewrite_skips_sentinels():
    report = rewrite_secrets("password none\nsecret <REDACTED>\n", lambda v: "X")
    assert report.transformed == 0
    assert report.skipped == 2


def test_rewrite_transform_none_is_left_untouched():
    report = rewrite_secrets(_CONF, lambda v: None)
    assert report.transformed == 0
    assert report.new_source == _CONF


# --- CLI verbs --------------------------------------------------------------


def test_cli_round_trip(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")

    sealed = tmp_path / "sealed.conf"
    code, _, err = _run(
        ["encrypt-secrets", str(conf), "-k", _KEY, "--salt", "ab", "-o", str(sealed)], capsys
    )
    assert code == 0
    assert "encrypted: 2 secret(s)" in err
    sealed_text = sealed.read_text(encoding="utf-8")
    assert "clearpw" not in sealed_text
    assert "$M$ab$" in sealed_text

    code, out, err = _run(["decrypt-secrets", str(sealed), "-k", _KEY], capsys)
    assert code == 0
    assert "decrypted: 2 secret(s)" in err
    assert "password clearpw" in out
    assert "my radius secret" in out


def test_cli_encrypt_is_idempotent(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    code, out, _ = _run(["encrypt-secrets", str(conf), "-k", _KEY], capsys)
    assert code == 0
    sealed = tmp_path / "sealed.conf"
    sealed.write_text(out, encoding="utf-8")
    code, _, err = _run(["encrypt-secrets", str(sealed), "-k", _KEY], capsys)
    assert code == 0
    assert "encrypted: 0 secret(s); 2 left untouched" in err


def test_cli_key_from_env(tmp_path, capsys, monkeypatch):
    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    monkeypatch.setenv("F5MKU", _KEY)
    code, out, _ = _run(["encrypt-secrets", str(conf)], capsys)
    assert code == 0
    assert "$M$" in out


def test_cli_key_from_file(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    keyfile = tmp_path / "key.txt"
    keyfile.write_text(_KEY + "\n", encoding="utf-8")
    code, out, _ = _run(["encrypt-secrets", str(conf), "--f5mku-file", str(keyfile)], capsys)
    assert code == 0
    assert "$M$" in out


def test_cli_missing_key_errors(tmp_path, capsys, monkeypatch):
    monkeypatch.delenv("F5MKU", raising=False)
    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    code, _, err = _run(["encrypt-secrets", str(conf)], capsys)
    assert code == 2
    assert "no master key" in err


def test_cli_bad_key_errors(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    code, _, err = _run(["encrypt-secrets", str(conf), "-k", "not base64!!!"], capsys)
    assert code == 2
    assert "f5mku" in err


def test_cli_key_from_prompt(tmp_path, capsys, monkeypatch):
    import getpass

    from tooling.f5.verbs import secrets

    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    monkeypatch.delenv("F5MKU", raising=False)
    monkeypatch.setattr(secrets, "_interactive", lambda: True)
    monkeypatch.setattr(getpass, "getpass", lambda prompt="": _KEY)
    code, out, _ = _run(["decrypt-secrets", str(conf)], capsys)
    assert code == 0
    assert out  # decrypt ran with the prompted key


def test_cli_no_key_prompt_skips_prompt(tmp_path, capsys, monkeypatch):
    import getpass

    from tooling.f5.verbs import secrets

    conf = tmp_path / "bigip.conf"
    conf.write_text(_CONF, encoding="utf-8")
    monkeypatch.delenv("F5MKU", raising=False)
    monkeypatch.setattr(secrets, "_interactive", lambda: True)

    def _boom(prompt=""):
        raise AssertionError("should not prompt when --no-key-prompt is set")

    monkeypatch.setattr(getpass, "getpass", _boom)
    code, _, err = _run(["encrypt-secrets", str(conf), "--no-key-prompt"], capsys)
    assert code == 2
    assert "no master key" in err
