"""``tclpkg.lock`` lockfile — canonical JSON representation.

The lockfile records the *exact* resolved dependency graph: every package
name, version, source URL, SHA-256 integrity hash, and transitive
dependency list.  Two invocations of ``tcl pkg install`` against the same
manifest on the same registry snapshot produce byte-identical lockfiles
(the ``generated`` timestamp is the only non-deterministic field, and
``--frozen`` preserves it).

Canonical JSON rules (so both Python and the future pure-Tcl port agree
byte-for-byte):

- 2-space indent
- Keys sorted alphabetically
- LF line endings, no trailing whitespace
- Final newline
- ``packages[]`` sorted by ``(name, version)``

The schema version (``"version": 1``) is bumped only on breaking changes.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .errors import ManifestError, TclPkgError


@dataclass(frozen=True)
class SourceSpec:
    """Where a package was fetched from."""

    type: str  # "tarball" | "git" | "path"
    url: str
    subdir: str = ""
    rev: str | None = None  # git commit SHA when type=git

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"type": self.type, "url": self.url}
        d["subdir"] = self.subdir
        d["rev"] = self.rev
        return d

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> SourceSpec:
        return cls(
            type=d["type"],
            url=d["url"],
            subdir=d.get("subdir", ""),
            rev=d.get("rev"),
        )


@dataclass
class LockedPackage:
    """A single resolved package in the lockfile."""

    name: str
    version: str
    source: SourceSpec
    integrity: str  # "sha256-<base64url>"
    size: int = 0
    requires: list[str] = field(default_factory=list)  # "name@minver"
    provides: list[str] = field(default_factory=list)
    license: str = ""
    dev: bool = False

    def to_dict(self) -> dict[str, Any]:
        return {
            "dev": self.dev,
            "integrity": self.integrity,
            "license": self.license,
            "name": self.name,
            "provides": sorted(self.provides),
            "requires": sorted(self.requires),
            "size": self.size,
            "source": self.source.to_dict(),
            "version": self.version,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> LockedPackage:
        return cls(
            name=d["name"],
            version=d["version"],
            source=SourceSpec.from_dict(d["source"]),
            integrity=d.get("integrity", ""),
            size=d.get("size", 0),
            requires=list(d.get("requires", [])),
            provides=list(d.get("provides", [])),
            license=d.get("license", ""),
            dev=d.get("dev", False),
        )


@dataclass
class ReplaceEntry:
    """A ``replace`` directive that was applied during resolution."""

    name: str
    from_version: str
    to_version: str

    def to_dict(self) -> dict[str, str]:
        return {"from": self.from_version, "name": self.name, "to": self.to_version}

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> ReplaceEntry:
        return cls(name=d["name"], from_version=d["from"], to_version=d["to"])


@dataclass
class ExcludeEntry:
    """An ``exclude`` directive that was honoured."""

    name: str
    version: str

    def to_dict(self) -> dict[str, str]:
        return {"name": self.name, "version": self.version}

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> ExcludeEntry:
        return cls(name=d["name"], version=d["version"])


# Schema version — bumped on incompatible lockfile changes.
LOCKFILE_VERSION = 1

# Generator string embedded in the lockfile.
_GENERATOR = "tcl pkg"


@dataclass
class LockFile:
    """In-memory representation of a ``tclpkg.lock`` file."""

    version: int = LOCKFILE_VERSION
    name: str = ""
    tcl: str = ">=8.6"
    generated: str = ""
    generator: str = _GENERATOR
    root_hash: str = ""
    packages: list[LockedPackage] = field(default_factory=list)
    replaces: list[ReplaceEntry] = field(default_factory=list)
    excludes: list[ExcludeEntry] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "excludes": [e.to_dict() for e in sorted(self.excludes, key=lambda e: e.name)],
            "generated": self.generated,
            "generator": self.generator,
            "name": self.name,
            "packages": [
                p.to_dict() for p in sorted(self.packages, key=lambda p: (p.name, p.version))
            ],
            "replaces": [r.to_dict() for r in sorted(self.replaces, key=lambda r: r.name)],
            "root_hash": self.root_hash,
            "tcl": self.tcl,
            "version": self.version,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> LockFile:
        lf = cls(
            version=d.get("version", LOCKFILE_VERSION),
            name=d.get("name", ""),
            tcl=d.get("tcl", ">=8.6"),
            generated=d.get("generated", ""),
            generator=d.get("generator", _GENERATOR),
            root_hash=d.get("root_hash", ""),
        )
        for p in d.get("packages", []):
            lf.packages.append(LockedPackage.from_dict(p))
        for r in d.get("replaces", []):
            lf.replaces.append(ReplaceEntry.from_dict(r))
        for e in d.get("excludes", []):
            lf.excludes.append(ExcludeEntry.from_dict(e))
        return lf

    def lookup(self, name: str) -> LockedPackage | None:
        """Return the locked entry for *name*, or ``None``."""
        for pkg in self.packages:
            if pkg.name == name:
                return pkg
        return None

    def stamp(self) -> None:
        """Set ``generated`` to now (UTC ISO-8601)."""
        self.generated = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def serialise(lockfile: LockFile) -> str:
    """Produce canonical JSON for a lockfile.

    The output is deterministic: sorted keys, 2-space indent, LF
    endings, final newline.  Two calls with the same data produce
    byte-identical output.
    """
    return json.dumps(lockfile.to_dict(), indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def deserialise(text: str) -> LockFile:
    """Parse a lockfile JSON string."""
    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        raise TclPkgError(f"invalid lockfile JSON: {exc}") from exc
    if not isinstance(data, dict):
        raise TclPkgError("lockfile must be a JSON object")
    schema_version = data.get("version")
    if schema_version is not None and schema_version > LOCKFILE_VERSION:
        raise TclPkgError(
            f"lockfile schema version {schema_version} is newer than supported "
            f"({LOCKFILE_VERSION}); upgrade tcl-lsp",
        )
    return LockFile.from_dict(data)


def write_lockfile(lockfile: LockFile, path: str | Path) -> None:
    """Write the lockfile to disk atomically."""
    dest = Path(path)
    text = serialise(lockfile)
    # Write to a temp file then rename for atomicity.
    tmp = dest.with_suffix(".lock.tmp")
    try:
        tmp.write_text(text, encoding="utf-8")
        tmp.replace(dest)
    except Exception:
        tmp.unlink(missing_ok=True)
        raise


def read_lockfile(path: str | Path) -> LockFile:
    """Read a lockfile from disk."""
    file_path = Path(path)
    if not file_path.is_file():
        raise ManifestError(
            f"lockfile not found: {file_path}",
            hint="run 'tcl pkg install' to create one",
        )
    text = file_path.read_text(encoding="utf-8")
    return deserialise(text)


__all__ = [
    "ExcludeEntry",
    "LOCKFILE_VERSION",
    "LockFile",
    "LockedPackage",
    "ReplaceEntry",
    "SourceSpec",
    "deserialise",
    "read_lockfile",
    "serialise",
    "write_lockfile",
]
