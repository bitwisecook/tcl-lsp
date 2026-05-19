"""Tests for credential resolution in ``explorer.f5_remote.auth``."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from tooling.explorer.f5_remote.auth import (
    Credentials,
    hosts_config_path,
    resolve_credentials,
    xdg_cache_dir,
    xdg_config_dir,
)


@pytest.fixture
def isolated_xdg(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "config"))
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))
    for var in ("F5_HOST", "F5_USER", "F5_PASSWORD", "F5_PORT", "F5_SSH_PORT"):
        monkeypatch.delenv(var, raising=False)
    return tmp_path


def test_xdg_paths_respect_env(isolated_xdg):
    assert xdg_config_dir() == isolated_xdg / "config" / "f5"
    assert xdg_cache_dir() == isolated_xdg / "cache" / "f5"
    assert hosts_config_path() == isolated_xdg / "config" / "f5" / "hosts.toml"


def test_resolve_from_explicit_args(isolated_xdg):
    creds = resolve_credentials(host="1.2.3.4", user="admin", password="x", interactive=False)
    assert creds == Credentials(host="1.2.3.4", user="admin", password="x", port=443, ssh_port=22)


def test_resolve_from_env(isolated_xdg, monkeypatch):
    monkeypatch.setenv("F5_HOST", "device.local")
    monkeypatch.setenv("F5_USER", "ops")
    monkeypatch.setenv("F5_PASSWORD", "pw")
    monkeypatch.setenv("F5_PORT", "8443")

    creds = resolve_credentials(interactive=False)
    assert creds.host == "device.local"
    assert creds.user == "ops"
    assert creds.password == "pw"
    assert creds.port == 8443


def test_resolve_via_hosts_alias(isolated_xdg):
    config = xdg_config_dir()
    config.mkdir(parents=True)
    (config / "hosts.toml").write_text(
        '[hosts.lab]\nhost = "10.0.0.5"\nuser = "labadmin"\npassword = "labpw"\nport = 8443\n'
    )

    creds = resolve_credentials(host="lab", interactive=False)
    assert creds.host == "10.0.0.5"
    assert creds.user == "labadmin"
    assert creds.password == "labpw"
    assert creds.port == 8443


def test_resolve_explicit_port_overrides_env(isolated_xdg, monkeypatch):
    """CLI port must beat F5_PORT — see PR #392 review."""
    monkeypatch.setenv("F5_HOST", "h")
    monkeypatch.setenv("F5_USER", "u")
    monkeypatch.setenv("F5_PASSWORD", "p")
    monkeypatch.setenv("F5_PORT", "9999")
    monkeypatch.setenv("F5_SSH_PORT", "8888")

    creds = resolve_credentials(port=4443, ssh_port=2222, interactive=False)
    assert creds.port == 4443
    assert creds.ssh_port == 2222


def test_resolve_explicit_overrides_env(isolated_xdg, monkeypatch):
    monkeypatch.setenv("F5_HOST", "from-env")
    monkeypatch.setenv("F5_USER", "from-env")
    monkeypatch.setenv("F5_PASSWORD", "from-env")
    creds = resolve_credentials(
        host="from-cli", user="cli-user", password="cli-pw", interactive=False
    )
    assert creds.host == "from-cli"
    assert creds.user == "cli-user"
    assert creds.password == "cli-pw"


def test_resolve_no_prompt_raises_when_missing(isolated_xdg):
    with pytest.raises(ValueError, match="host"):
        resolve_credentials(interactive=False)


def test_resolve_malformed_hosts_file_is_ignored(isolated_xdg, capsys, monkeypatch):
    config = xdg_config_dir()
    config.mkdir(parents=True)
    (config / "hosts.toml").write_text("not = valid = toml")
    monkeypatch.setenv("F5_HOST", "fallback")
    monkeypatch.setenv("F5_USER", "u")
    monkeypatch.setenv("F5_PASSWORD", "p")
    creds = resolve_credentials(interactive=False)
    assert creds.host == "fallback"
    err = capsys.readouterr().err
    assert "warning" in err
