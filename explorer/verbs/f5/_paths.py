"""Shared file-input helpers for f5 verbs.

The cleanup/grep verbs duplicated this trivially; centralising it
keeps every new verb consistent.
"""

from __future__ import annotations

import sys
from pathlib import Path

from core.bigip.model import BigipConfig
from core.bigip.parser import parse_bigip_conf


def read_path(path_str: str) -> tuple[str, str]:
    """Return ``(uri, source)`` for *path_str*.  ``-`` reads stdin."""
    if path_str == "-":
        return ("stdin://input", sys.stdin.read())
    path = Path(path_str).resolve()
    if not path.is_file():
        raise FileNotFoundError(f"not a file: {path_str}")
    return (path.as_uri(), path.read_text(encoding="utf-8", errors="replace"))


def load_paths(paths: list[str]) -> tuple[dict[str, str], dict[str, BigipConfig]]:
    sources: dict[str, str] = {}
    configs: dict[str, BigipConfig] = {}
    for p in paths:
        uri, src = read_path(p)
        sources[uri] = src
        configs[uri] = parse_bigip_conf(src)
    return sources, configs
