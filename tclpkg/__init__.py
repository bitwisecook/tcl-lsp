"""Tcl package manager and virtual-environment system.

``tclpkg`` is a deterministic, MVS-based dependency manager for Tcl
projects, integrated into the ``tcl`` CLI at ``explorer/tcl_cli.py``.  It
evaluates a ``tclpkg.tcl`` manifest in a sandboxed Tcl interpreter,
resolves the dependency graph using Go-style Minimum Version Selection,
fetches packages into a content-addressable cache keyed by SHA-256, and
materialises them into ``./lib/<pkg>-<ver>/`` or a virtual environment's
``lib/`` directory.

See ``docs/kcs/kcs-tclpkg-overview.md`` for the architecture overview and
the design plan at ``docs/kcs/kcs-tclpkg-manifest-contracts.md`` for the
manifest grammar.
"""

from __future__ import annotations

from .errors import (
    IntegrityError,
    ManifestError,
    RegistryError,
    ResolutionError,
    TclPkgError,
)
from .version import Version, VersionError

__all__ = [
    "IntegrityError",
    "ManifestError",
    "RegistryError",
    "ResolutionError",
    "TclPkgError",
    "Version",
    "VersionError",
]
