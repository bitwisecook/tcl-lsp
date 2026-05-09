"""Credential resolution for ``f5`` remote verbs.

Resolution order (highest priority first):

1. Explicit CLI arguments (``--host``, ``--user``, ``--password``)
2. Environment variables (``F5_HOST``, ``F5_USER``, ``F5_PASSWORD``)
3. XDG config file (``$XDG_CONFIG_HOME/f5/hosts.toml`` or
   ``~/.config/f5/hosts.toml``) keyed by host alias
4. Interactive prompt (``getpass``)
"""

from __future__ import annotations

import getpass
import os
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class Credentials:
    host: str
    user: str
    password: str
    port: int = 443
    ssh_port: int = 22


def xdg_config_dir() -> Path:
    base = os.environ.get("XDG_CONFIG_HOME")
    if base:
        return Path(base) / "f5"
    return Path.home() / ".config" / "f5"


def xdg_cache_dir() -> Path:
    base = os.environ.get("XDG_CACHE_HOME")
    if base:
        return Path(base) / "f5"
    return Path.home() / ".cache" / "f5"


def hosts_config_path() -> Path:
    """Return the path to the XDG hosts.toml file (may not exist)."""
    return xdg_config_dir() / "hosts.toml"


def _load_hosts_config() -> dict[str, dict[str, object]]:
    path = hosts_config_path()
    if not path.is_file():
        return {}
    try:
        with path.open("rb") as fh:
            data = tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        print(f"warning: cannot read {path}: {exc}", file=sys.stderr)
        return {}
    hosts = data.get("hosts", {})
    if not isinstance(hosts, dict):
        return {}
    return {k: v for k, v in hosts.items() if isinstance(v, dict)}


def resolve_credentials(
    *,
    host: str | None = None,
    user: str | None = None,
    password: str | None = None,
    port: int | None = None,
    ssh_port: int | None = None,
    interactive: bool = True,
) -> Credentials:
    """Resolve a complete :class:`Credentials` from layered sources.

    *host* may name an alias defined in the hosts.toml file.  When the
    field cannot be resolved from CLI/env/file and *interactive* is
    true, the user is prompted on stdin (password via :func:`getpass`).
    """
    config = _load_hosts_config()

    # Look up alias entry if *host* matches a configured alias.
    alias_entry: dict[str, object] = {}
    if host and host in config:
        alias_entry = config[host]

    def pick(field_name: str, env_name: str, fallback: object | None) -> str | None:
        env_val = os.environ.get(env_name)
        if env_val:
            return env_val
        cfg_val = alias_entry.get(field_name)
        if cfg_val is not None:
            return str(cfg_val)
        if fallback is not None:
            return str(fallback)
        return None

    resolved_host = host or pick("host", "F5_HOST", None)
    if not resolved_host:
        if not interactive:
            raise ValueError("no host configured (set --host or F5_HOST)")
        resolved_host = input("F5 host: ").strip()
        if not resolved_host:
            raise ValueError("host is required")
    # If host is an alias, replace with the real address from the config.
    if resolved_host in config and "host" in config[resolved_host]:
        alias_entry = config[resolved_host]
        resolved_host = str(alias_entry["host"])

    resolved_user = user or pick("user", "F5_USER", None)
    if not resolved_user:
        if not interactive:
            raise ValueError("no user configured (set --user or F5_USER)")
        resolved_user = input(f"F5 user [{resolved_host}]: ").strip()
        if not resolved_user:
            raise ValueError("user is required")

    resolved_password = password or pick("password", "F5_PASSWORD", None)
    if not resolved_password:
        if not interactive:
            raise ValueError("no password configured (set --password or F5_PASSWORD)")
        resolved_password = getpass.getpass(f"Password for {resolved_user}@{resolved_host}: ")
        if not resolved_password:
            raise ValueError("password is required")

    resolved_port_raw = pick("port", "F5_PORT", port)
    resolved_port = int(resolved_port_raw) if resolved_port_raw is not None else 443

    resolved_ssh_port_raw = pick("ssh_port", "F5_SSH_PORT", ssh_port)
    resolved_ssh_port = int(resolved_ssh_port_raw) if resolved_ssh_port_raw is not None else 22

    return Credentials(
        host=resolved_host,
        user=resolved_user,
        password=resolved_password,
        port=resolved_port,
        ssh_port=resolved_ssh_port,
    )
