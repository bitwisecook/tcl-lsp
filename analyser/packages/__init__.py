"""Core package resolution APIs."""

from __future__ import annotations

from .auto_index import AutoIndexEntry, parse_tcl_index
from .resolver import PackageInfo, PackageResolver

__all__ = ["AutoIndexEntry", "PackageInfo", "PackageResolver", "parse_tcl_index"]
