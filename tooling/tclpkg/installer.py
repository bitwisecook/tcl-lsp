"""Install orchestration — source resolution, fetch, CAS store, materialise.

``tcl pkg install`` resolves the dependency graph (``resolver``), then for
each resolved package this module turns a ``name@version`` into a concrete
source, fetches it (``fetchers``), ingests it into the content-addressable
store (``cas``), and materialises it into the project ``lib/`` (or a venv
``lib/``).  The lockfile records the exact source, integrity hash, size,
``provides``/``license`` read back from the fetched package's own manifest.

Source resolution order (mirrors the architecture contract):

1. A root ``replace`` directive forces the version *and* the source URL.
2. A ``require``/``dev-require`` ``-source URL`` on the dependency.
3. The offline/online registry's first source entry for the package.

When no source can be determined — or a fetch fails / the registry is
unreachable — the caller keeps a placeholder lock entry (empty integrity)
and warns, rather than aborting the whole install.  This module is the
single source of truth shared by the Python CLI and mirrored byte-for-byte
by ``rust/tcl-pkg/src/installer.rs``.
"""

from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path

from .cas import ContentAddressableStore, tree_size
from .fetchers import fetch_git, fetch_path, fetch_tarball
from .lockfile import SourceSpec
from .registry import RegistryClient


@dataclass
class MaterialiseResult:
    """Outcome of fetching + storing + reading one package."""

    source: SourceSpec
    integrity: str
    size: int
    provides: list[str]
    license: str


def _classify(url: str, *, rev: str | None) -> SourceSpec:
    """Classify a source URL into a tarball / git / path :class:`SourceSpec`."""
    if Path(url).is_dir():
        return SourceSpec(type="path", url=url)
    if url.startswith("git+") or url.endswith(".git") or url.startswith("git@"):
        return SourceSpec(type="git", url=url, rev=rev)
    return SourceSpec(type="tarball", url=url)


def resolve_source(
    name: str,
    version: str,
    *,
    replaces: dict[str, str],
    manifest_sources: dict[str, str],
    registry: RegistryClient | None,
) -> SourceSpec | None:
    """Return the :class:`SourceSpec` for *name@version*, or ``None``.

    ``replaces``/``manifest_sources`` map package name → source URL. The
    registry is consulted last and any registry error (offline + no cache,
    network failure) is swallowed into a ``None`` result so the caller can
    degrade gracefully.
    """
    if name in replaces:
        return _classify(replaces[name], rev=version)
    if name in manifest_sources:
        return _classify(manifest_sources[name], rev=version)
    if registry is not None:
        try:
            entry = registry.lookup(name)
        except Exception:
            return None
        if entry and entry.sources:
            src = entry.sources[0]
            if src.method == "git" or src.url.endswith(".git"):
                return SourceSpec(type="git", url=src.url, rev=version)
            return SourceSpec(type="tarball", url=src.url)
    return None


def _read_pkg_meta(root: Path) -> tuple[list[str], str]:
    """Read ``provides`` / ``license`` from a fetched package's manifest."""
    manifest_path = root / "tclpkg.tcl"
    if not manifest_path.is_file():
        return [], ""
    try:
        from .manifest import load_manifest

        ast = load_manifest(manifest_path)
        return list(ast.provides), ast.license
    except Exception:
        return [], ""


def fetch_and_store(
    source: SourceSpec,
    name: str,
    version: str,
    cas: ContentAddressableStore,
    *,
    timeout: int = 60,
) -> MaterialiseResult:
    """Fetch *source* into a temp dir, ingest it into the CAS, read metadata.

    Returns a :class:`MaterialiseResult` with the (possibly rev-resolved)
    source, integrity hash, size, and package metadata.  Raises on fetch or
    store failure.
    """
    with tempfile.TemporaryDirectory(prefix="tclpkg-fetch-") as tmp:
        worktree = Path(tmp) / "src"
        resolved_source = source
        if source.type == "path":
            fetch_path(source.url, worktree)
        elif source.type == "git":
            sha = fetch_git(source.url, worktree, rev=source.rev, timeout=timeout)
            resolved_source = SourceSpec(
                type="git", url=source.url, subdir=source.subdir, rev=sha or source.rev
            )
        else:
            fetch_tarball(source.url, worktree, timeout=timeout)

        integrity = cas.store(worktree, name=name, version=version)
        size = tree_size(worktree)
        provides, license_ = _read_pkg_meta(worktree)

    return MaterialiseResult(
        source=resolved_source,
        integrity=integrity,
        size=size,
        provides=provides,
        license=license_,
    )


def materialise(
    cas: ContentAddressableStore,
    integrity: str,
    lib_dir: Path,
    name: str,
    version: str,
    *,
    symlink: bool = True,
) -> Path:
    """Materialise a CAS entry into ``lib_dir/<name>-<version>``."""
    dest = lib_dir / f"{name}-{version}"
    cas.materialise(integrity, dest, symlink=symlink)
    return dest


__all__ = [
    "MaterialiseResult",
    "fetch_and_store",
    "materialise",
    "resolve_source",
]
