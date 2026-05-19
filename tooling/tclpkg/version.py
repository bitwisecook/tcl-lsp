"""Semantic version type with Tcl-friendly parsing.

tclpkg needs version ordering that agrees with both semver 2.0 (for
compatibility with community registries) and with C Tcl's own
``package require`` ordering.  In practice the only differences between
the two are:

- Missing patch digits default to zero: ``1.2`` -> ``1.2.0``.
- A leading ``v`` is tolerated: ``v1.2.0`` -> ``1.2.0``.
- Pre-release tags accept Tcl-style ``a1`` / ``b2`` alongside semver's
  ``-alpha.1`` / ``-beta.2``.  ``8.6.3a1`` sorts before ``8.6.3``.

This module is intentionally tiny and dependency-free so the pure-Tcl
port can reproduce its behaviour byte-for-byte in
``tclpkg-tcl/lib/tclpkg/version.tcl``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any


class VersionError(ValueError):
    """Raised when a version string cannot be parsed."""


# Core semver + lax trailing-zero + optional v-prefix + Tcl-style alpha/beta.
_SEMVER_RE = re.compile(
    r"""^
    v?                                      # optional leading 'v'
    (?P<major>0|[1-9]\d*)
    (?:\.(?P<minor>0|[1-9]\d*))?
    (?:\.(?P<patch>0|[1-9]\d*))?
    (?:
        (?P<prerelease>
            -[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*   # semver ``-rc.1``
          | [ab]\d+                                # Tcl ``a1`` / ``b2``
          | rc\d+                                  # Tcl ``rc1``
        )
    )?
    (?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?
    $""",
    re.VERBOSE,
)

# Order for short-form pre-release prefixes (Tcl-style).  Anything not in
# this table ranks after these but before the final release.
_PRE_ORDER: dict[str, int] = {"a": 0, "alpha": 0, "b": 1, "beta": 1, "rc": 2, "pre": 2}


def _parse_prerelease(raw: str) -> tuple[tuple[Any, ...], str]:
    """Return a comparable tuple for a prerelease tag plus its canonical form.

    Semver prereleases compare piece-wise with numeric pieces beating
    alphabetic ones.  Tcl-style ``a1``/``b2``/``rc1`` are expanded into
    ``("alpha", 1)`` / ``("beta", 2)`` / ``("rc", 1)`` so they collate
    naturally.
    """
    if raw.startswith("-"):
        pieces = raw[1:].split(".")
        canonical = "-" + raw[1:]
        comparable: list[Any] = []
        for piece in pieces:
            if piece.isdigit():
                comparable.append((0, int(piece)))
            else:
                comparable.append((1, piece))
        return tuple(comparable), canonical

    match = re.match(r"^(a|b|rc)(\d+)$", raw)
    if match is None:
        return ((1, raw),), raw
    label = {"a": "alpha", "b": "beta", "rc": "rc"}[match.group(1)]
    number = int(match.group(2))
    return ((0, _PRE_ORDER[label]), (0, number)), f"-{label}.{number}"


@dataclass(frozen=True)
class Version:
    """A semver-compatible version string with Tcl-friendly parsing.

    ``Version`` instances are immutable, hashable, and totally ordered.
    Comparison follows semver 2.0 with Tcl-style short prereleases
    mapped onto their canonical semver form before comparison.
    """

    major: int
    minor: int
    patch: int
    prerelease: tuple[Any, ...]
    prerelease_raw: str
    build: str
    original: str

    @classmethod
    def parse(cls, raw: str) -> Version:
        if not isinstance(raw, str):
            raise VersionError(f"version must be a string, got {type(raw).__name__}")
        stripped = raw.strip()
        if not stripped:
            raise VersionError("empty version string")
        match = _SEMVER_RE.match(stripped)
        if match is None:
            raise VersionError(f"invalid version: {raw!r}")
        major = int(match.group("major"))
        minor = int(match.group("minor") or 0)
        patch = int(match.group("patch") or 0)
        pre_raw = match.group("prerelease") or ""
        if pre_raw:
            pre_tuple, pre_canon = _parse_prerelease(pre_raw)
        else:
            pre_tuple = ()
            pre_canon = ""
        build = match.group("build") or ""
        return cls(
            major=major,
            minor=minor,
            patch=patch,
            prerelease=pre_tuple,
            prerelease_raw=pre_canon,
            build=build,
            original=stripped,
        )

    def canonical(self) -> str:
        """Return the canonical ``MAJOR.MINOR.PATCH[-pre][+build]`` form."""
        base = f"{self.major}.{self.minor}.{self.patch}"
        if self.prerelease_raw:
            base += self.prerelease_raw
        if self.build:
            base += f"+{self.build}"
        return base

    def __str__(self) -> str:
        return self.canonical()

    def _compare_key(self) -> tuple[Any, ...]:
        # A tuple that sorts correctly: pre-release versions come BEFORE
        # the corresponding release.  We represent "no prerelease" as a
        # sentinel that compares greater than any prerelease tuple.
        pre = self.prerelease if self.prerelease else ((2,),)
        return (self.major, self.minor, self.patch, pre)

    def __lt__(self, other: Any) -> bool:
        if not isinstance(other, Version):
            return NotImplemented
        return self._compare_key() < other._compare_key()

    def __le__(self, other: Any) -> bool:
        if not isinstance(other, Version):
            return NotImplemented
        return self._compare_key() <= other._compare_key()

    def __gt__(self, other: Any) -> bool:
        if not isinstance(other, Version):
            return NotImplemented
        return self._compare_key() > other._compare_key()

    def __ge__(self, other: Any) -> bool:
        if not isinstance(other, Version):
            return NotImplemented
        return self._compare_key() >= other._compare_key()

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Version):
            return NotImplemented
        return self._compare_key() == other._compare_key()

    def __hash__(self) -> int:
        return hash(self._compare_key())


def parse_version(raw: str) -> Version:
    """Convenience alias for :meth:`Version.parse`."""
    return Version.parse(raw)


def max_version(versions: list[Version]) -> Version:
    """Return the maximum of a non-empty list of versions."""
    if not versions:
        raise ValueError("max_version requires a non-empty list")
    return max(versions)


__all__ = ["Version", "VersionError", "max_version", "parse_version"]
