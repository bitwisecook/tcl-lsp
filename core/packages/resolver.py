"""Tcl package resolution via pkgIndex.tcl files.

Parses ``pkgIndex.tcl`` files to build a mapping from package names to the
source files that provide them.  The resolver is used to satisfy
``package require`` statements encountered during analysis.
"""

from __future__ import annotations

import logging
import os
import re
from dataclasses import dataclass
from pathlib import Path

log = logging.getLogger(__name__)


@dataclass
class PackageInfo:
    """Metadata for a discovered Tcl package."""

    name: str
    version: str
    source_files: list[str]  # absolute paths to implementation files
    pkg_index_path: str  # path to the pkgIndex.tcl that declared this


class PackageResolver:
    """Resolves ``package require`` to source files via pkgIndex.tcl scanning."""

    def __init__(self) -> None:
        self._packages: dict[str, list[PackageInfo]] = {}  # name -> versions
        self._search_paths: list[str] = []
        self._scanned: bool = False

    def configure(self, search_paths: list[str]) -> None:
        self._search_paths = list(search_paths)
        self._scanned = False

    def scan_packages(self) -> None:
        """Walk search paths looking for pkgIndex.tcl files."""
        self._packages.clear()
        for search_path in self._search_paths:
            expanded = os.path.expanduser(search_path)
            if not os.path.isdir(expanded):
                log.warning("PackageResolver: directory not found: %s", search_path)
                continue
            for root, _dirs, files in os.walk(expanded):
                if "pkgIndex.tcl" in files:
                    pkg_index_path = os.path.join(root, "pkgIndex.tcl")
                    self._parse_pkg_index(pkg_index_path, root)
        self._scanned = True
        log.info(
            "Package scan: found %d packages in %d search paths",
            len(self._packages),
            len(self._search_paths),
        )

    def resolve(self, package_name: str, version: str | None = None) -> list[str]:
        """Resolve a package name to its source file paths."""
        if not self._scanned:
            self.scan_packages()

        infos = self._packages.get(package_name, [])
        if not infos:
            return []

        if version:
            for info in infos:
                if info.version == version or info.version.startswith(version):
                    return list(info.source_files)
            # Specific version requested but not found
            return []

        # No version constraint -- return files from the first entry
        return list(infos[0].source_files)

    def all_package_names(self) -> list[str]:
        if not self._scanned:
            self.scan_packages()
        return list(self._packages.keys())

    # Pattern: package ifneeded <name> <version> <script>
    _PKG_IFNEEDED_RE = re.compile(
        r"package\s+ifneeded\s+"
        r"(\S+)\s+"  # package name
        r"([\d.]+(?:[ab]\d+)?)\s+"  # version
        r"(.*)",  # script (rest of line)
        re.MULTILINE,
    )

    # Patterns for extracting source file references from the script body.
    _SOURCE_JOIN_RE = re.compile(r"source\s+\[file\s+join\s+\$dir\s+([^\]]+)\]")
    _SOURCE_DIR_RE = re.compile(r"source\s+\$dir/(\S+)")

    def _parse_pkg_index(self, pkg_index_path: str, pkg_dir: str) -> None:
        """Parse a pkgIndex.tcl file."""
        try:
            content = Path(pkg_index_path).read_text(
                encoding="utf-8",
                errors="replace",
            )
        except Exception:
            log.debug(
                "PackageResolver: failed to read %s",
                pkg_index_path,
                exc_info=True,
            )
            return

        for match in self._PKG_IFNEEDED_RE.finditer(content):
            name = match.group(1)
            version = match.group(2)
            script = match.group(3).strip()

            source_files = self._extract_source_files(script, pkg_dir)

            if source_files:
                info = PackageInfo(
                    name=name,
                    version=version,
                    source_files=source_files,
                    pkg_index_path=pkg_index_path,
                )
                self._packages.setdefault(name, []).append(info)

    def _extract_source_files(self, script: str, pkg_dir: str) -> list[str]:
        """Extract source file paths from a pkgIndex.tcl script."""
        files: list[str] = []

        # source [file join $dir <filename>]
        for m in self._SOURCE_JOIN_RE.finditer(script):
            filename = m.group(1).strip().strip('"')
            full_path = os.path.join(pkg_dir, filename)
            if os.path.isfile(full_path):
                files.append(full_path)

        # source $dir/<filename>
        for m in self._SOURCE_DIR_RE.finditer(script):
            filename = m.group(1).strip().strip('"')
            full_path = os.path.join(pkg_dir, filename)
            if os.path.isfile(full_path):
                files.append(full_path)

        # Fallback: if no explicit source lines, look for .tcl files in the dir
        if not files:
            try:
                for f in os.listdir(pkg_dir):
                    if f.endswith(".tcl") and f != "pkgIndex.tcl":
                        files.append(os.path.join(pkg_dir, f))
            except OSError:
                pass

        return files
