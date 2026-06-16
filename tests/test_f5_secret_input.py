"""Tests for the shared secret-input resolver used by the ``f5`` CLI."""

from __future__ import annotations

import getpass
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tooling.f5.f5_remote import secret_input  # noqa: E402
from tooling.f5.f5_remote.secret_input import (  # noqa: E402
    SecretInputError,
    resolve_secret,
    secret_source_available,
)


def test_explicit_wins(monkeypatch):
    monkeypatch.setenv("SECRET", "from-env")
    assert resolve_secret(explicit="from-flag", env_var="SECRET") == "from-flag"


def test_file_before_env(tmp_path, monkeypatch):
    keyfile = tmp_path / "k"
    keyfile.write_text("from-file\n", encoding="utf-8")
    monkeypatch.setenv("SECRET", "from-env")
    # strip=True trims the trailing newline a key file picks up.
    assert resolve_secret(file=str(keyfile), env_var="SECRET", strip=True) == "from-file"


def test_env_used_when_no_explicit_or_file(monkeypatch):
    monkeypatch.setenv("SECRET", "from-env")
    assert resolve_secret(env_var="SECRET") == "from-env"


def test_strip_false_preserves_whitespace(monkeypatch):
    monkeypatch.setenv("SECRET", "  spaced  ")
    assert resolve_secret(env_var="SECRET", strip=False) == "  spaced  "


def test_prompt_used_as_last_resort(monkeypatch):
    monkeypatch.delenv("SECRET", raising=False)
    monkeypatch.setattr(secret_input, "_has_tty", lambda: True)
    monkeypatch.setattr(getpass, "getpass", lambda prompt="": "typed")
    assert resolve_secret(env_var="SECRET", prompt="X: ") == "typed"


def test_no_prompt_returns_none_when_not_interactive(monkeypatch):
    monkeypatch.delenv("SECRET", raising=False)
    monkeypatch.setattr(secret_input, "_has_tty", lambda: False)
    assert resolve_secret(env_var="SECRET") is None


def test_allow_prompt_false_skips_prompt(monkeypatch):
    monkeypatch.setattr(secret_input, "_has_tty", lambda: True)

    def _boom(prompt=""):
        raise AssertionError("must not prompt when allow_prompt=False")

    monkeypatch.setattr(getpass, "getpass", _boom)
    assert resolve_secret(allow_prompt=False) is None


def test_cancelled_prompt_raises(monkeypatch):
    monkeypatch.setattr(secret_input, "_has_tty", lambda: True)

    def _cancel(prompt=""):
        raise KeyboardInterrupt

    monkeypatch.setattr(getpass, "getpass", _cancel)
    with pytest.raises(SecretInputError):
        resolve_secret()


def test_source_available(tmp_path, monkeypatch):
    monkeypatch.setattr(secret_input, "_has_tty", lambda: False)
    monkeypatch.delenv("SECRET", raising=False)
    assert not secret_source_available(env_var="SECRET")
    assert secret_source_available(explicit="x")
    assert secret_source_available(file=str(tmp_path / "anything"))
    monkeypatch.setattr(secret_input, "_has_tty", lambda: True)
    assert secret_source_available(env_var="SECRET")
