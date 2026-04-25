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
        self._auto_index: dict[str, list[str]] = {}  # proc_name -> [abs paths]
        self._search_paths: list[str] = []
        self._scanned_paths: set[str] = set()
        self._scanned: bool = False

    def configure(self, search_paths: list[str]) -> None:
        """Replace the search-path list and invalidate the scan cache.

        Workspace roots should appear first so that a workspace-local
        copy of a package wins over any copy provided by an installed
        library path — matching Tcl's own ``auto_path`` order, where the
        first ``pkgIndex.tcl`` that satisfies a ``package require`` is
        the one that takes effect.
        """
        self._search_paths = list(search_paths)
        self._scanned_paths.clear()
        self._scanned = False

    def add_search_paths(self, paths: list[str], *, prepend: bool = False) -> None:
        """Incorporate additional search paths without discarding the scan cache.

        Used to honour a document's own ``lappend auto_path`` declarations:
        the new paths are merged in (deduplicated) and scanned immediately
        for packages and tclIndex entries.  Existing ``_packages`` entries
        are preserved, so the first provider already discovered keeps
        priority.  Pass ``prepend=True`` to place the new entries at the
        front of the effective order (for workspace-wins semantics).
        """
        seen_new: set[str] = set()
        new_paths: list[str] = []
        for path in paths:
            if not path:
                continue
            expanded = os.path.expanduser(path)
            abs_path = os.path.abspath(expanded)
            if abs_path in self._scanned_paths or abs_path in seen_new:
                continue
            seen_new.add(abs_path)
            new_paths.append(abs_path)
        if not new_paths:
            return
        if prepend:
            self._search_paths = new_paths + [
                p
                for p in self._search_paths
                if os.path.abspath(os.path.expanduser(p)) not in seen_new
            ]
        else:
            existing = {os.path.abspath(os.path.expanduser(p)) for p in self._search_paths}
            self._search_paths.extend(p for p in new_paths if p not in existing)
        for path in new_paths:
            self._scan_single_path(path, prepend=prepend)

    def scan_packages(self) -> None:
        """Walk search paths looking for pkgIndex.tcl and tclIndex files.

        Paths are visited in ``_search_paths`` order and the first
        ``package ifneeded`` entry for a given name wins — later
        providers append to the version list but do not displace the
        head.  This mirrors Tcl's own ``auto_path`` semantics.
        """
        self._packages.clear()
        self._auto_index.clear()
        self._scanned_paths.clear()
        for search_path in self._search_paths:
            expanded = os.path.expanduser(search_path)
            if not os.path.isdir(expanded):
                log.warning("PackageResolver: directory not found: %s", search_path)
                continue
            self._scan_single_path(os.path.abspath(expanded))
        self._scanned = True
        log.info(
            "Package scan: found %d packages, %d auto-index procs in %d search paths",
            len(self._packages),
            len(self._auto_index),
            len(self._search_paths),
        )

    def _scan_single_path(self, abs_path: str, *, prepend: bool = False) -> None:
        """Walk *abs_path* recording pkgIndex.tcl and tclIndex entries.

        When *prepend* is true, every package entry produced from
        this scan is inserted at the head of the package's
        provider list so :meth:`resolve` returns the new path's
        provider before any previously-indexed one. Used by
        :meth:`add_search_paths` ``prepend=True`` to honour
        workspace-wins semantics.
        """
        if abs_path in self._scanned_paths:
            return
        # Don't mark a non-existent directory as scanned — a transient
        # mount or a path created later should still get picked up on
        # a subsequent call.
        if not os.path.isdir(abs_path):
            return
        self._scanned_paths.add(abs_path)
        for root, _dirs, files in os.walk(abs_path):
            if "pkgIndex.tcl" in files:
                pkg_index_path = os.path.join(root, "pkgIndex.tcl")
                self._parse_pkg_index(pkg_index_path, root, prepend=prepend)
            files_lower = {f.lower(): f for f in files}
            if "tclindex" in files_lower:
                tcl_index_path = os.path.join(root, files_lower["tclindex"])
                self._parse_auto_index(tcl_index_path)

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

    def _parse_pkg_index(
        self,
        pkg_index_path: str,
        pkg_dir: str,
        *,
        prepend: bool = False,
    ) -> None:
        """Parse a pkgIndex.tcl file.

        When *prepend* is true, parsed entries are inserted at the
        head of the package's provider list (so workspace-wins
        ordering is honoured by :meth:`resolve`'s "first match
        wins" logic). Otherwise entries are appended in scan order.
        """
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
                providers = self._packages.setdefault(name, [])
                if prepend:
                    providers.insert(0, info)
                else:
                    providers.append(info)

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

    def _parse_auto_index(self, tcl_index_path: str) -> None:
        """Parse a tclIndex file and register proc->file mappings."""
        from .auto_index import parse_tcl_index

        for entry in parse_tcl_index(tcl_index_path):
            self._auto_index.setdefault(entry.proc_name, []).append(entry.source_file)

    def resolve_auto_proc(self, proc_name: str) -> list[str]:
        """Resolve an auto-loaded proc name to its source file paths."""
        if not self._scanned:
            self.scan_packages()
        return list(self._auto_index.get(proc_name, []))

    def all_auto_proc_names(self) -> list[str]:
        """Return all known auto-loadable proc names from tclIndex files."""
        if not self._scanned:
            self.scan_packages()
        return list(self._auto_index.keys())
