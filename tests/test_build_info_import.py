"""Guard the build-info import path against build-system drift.

The Makefile generates the version stamp at exactly one location
(``BUILD_INFO := $(ROOT)shared/_build_info.py``).  Every module that
reports the version — the LSP server's ``initialize`` banner, the
zipapp/tooling entry points, the MCP server — must import ``FULL_VERSION``
from that same module (``shared._build_info``) or it silently falls back
to ``"dev"`` in any packaged / Makefile-built run.

A real regression slipped through exactly this gap: ``server/server.py``
imported ``from ._build_info`` (i.e. ``server/_build_info.py``), which the
build never generates, so the LanguageServer advertised ``v dev`` instead
of the release version.  These tests would have caught it.

Two complementary checks:

- ``test_canonical_build_info_module_matches_makefile`` pins the canonical
  module to whatever the Makefile actually generates, so this test follows
  a future relocation of the stamp instead of hard-coding a path.
- ``test_all_build_info_imports_use_canonical_module`` AST-scans every
  first-party source file and asserts that any ``_build_info`` import
  resolves to the canonical module — no relative ``._build_info`` pointing
  at a never-generated sibling, no stale package path.
"""

from __future__ import annotations

import ast
import re
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]

# Source trees that ship in zipapps / the packaged LSP and therefore must
# read the version from the generated module.  Tests and the zipapp-main
# shims are scanned too (the shims import build-info at runtime).
SCAN_DIRS = (
    "server",
    "shared",
    "tooling",
    "ai",
    "scripts/zipapp-main",
)


def _canonical_build_info_module() -> str:
    """Derive the build-info module from the Makefile's BUILD_INFO path.

    ``BUILD_INFO := $(ROOT)shared/_build_info.py`` -> ``shared._build_info``.
    """
    makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
    m = re.search(
        r"^BUILD_INFO\s*:?=\s*\$\(ROOT\)(\S+_build_info)\.py\s*$",
        makefile,
        re.MULTILINE,
    )
    assert m, "could not find 'BUILD_INFO := $(ROOT).../_build_info.py' in Makefile"
    return m.group(1).replace("/", ".")


CANONICAL = _canonical_build_info_module()


def _module_name(path: Path) -> str:
    """Dotted module name for a source file relative to the repo root."""
    rel = path.relative_to(ROOT).with_suffix("")
    return ".".join(rel.parts)


def _resolve(module_dotted: str, level: int, target: str | None) -> str:
    """Resolve a (possibly relative) import to its absolute module name.

    ``module_dotted`` is the importing module; ``level``/``target`` are the
    ``ImportFrom`` node's ``level`` and ``module``.  Mirrors Python's own
    relative-import resolution so a ``from ._build_info import X`` inside
    ``shared/`` correctly resolves to ``shared._build_info`` while the same
    statement inside ``server/`` resolves to the (wrong) ``server._build_info``.
    """
    if level == 0:
        return target or ""
    package_parts = module_dotted.split(".")[:-1]  # drop the module's own name
    base = package_parts[: len(package_parts) - (level - 1)]
    return ".".join([*base, target]) if target else ".".join(base)


def _tracked_py_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "--", *[f"{d}/*.py" for d in SCAN_DIRS]],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    files = [ROOT / line for line in out.splitlines() if line.strip()]
    # The generated module itself is gitignored, but guard anyway.
    return [f for f in files if f.name != "_build_info.py"]


def _build_info_imports(path: Path) -> list[tuple[int, str]]:
    """Return (lineno, resolved_module) for every ``_build_info`` import."""
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    module_dotted = _module_name(path)
    found: list[tuple[int, str]] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom):
            # from X._build_info import ...   /   from ._build_info import ...
            if node.module and node.module.split(".")[-1] == "_build_info":
                found.append((node.lineno, _resolve(module_dotted, node.level, node.module)))
            # from X import _build_info   /   from . import _build_info
            elif any(alias.name == "_build_info" for alias in node.names):
                base = _resolve(module_dotted, node.level, node.module)
                found.append((node.lineno, f"{base}._build_info" if base else "_build_info"))
        elif isinstance(node, ast.Import):
            # import X._build_info
            for alias in node.names:
                if alias.name.split(".")[-1] == "_build_info":
                    found.append((node.lineno, alias.name))
    return found


def test_canonical_build_info_module_matches_makefile():
    """The build system generates exactly the module the importers expect."""
    assert CANONICAL == "shared._build_info", (
        f"Makefile now generates build-info at {CANONICAL!r}; update the "
        "importers (and this expectation) to match."
    )


def test_all_build_info_imports_use_canonical_module():
    """No first-party module may import build-info from a non-generated path."""
    offenders: list[str] = []
    for path in _tracked_py_files():
        for lineno, resolved in _build_info_imports(path):
            if resolved != CANONICAL:
                rel = path.relative_to(ROOT)
                offenders.append(f"{rel}:{lineno} imports {resolved!r} (expected {CANONICAL!r})")
    assert not offenders, (
        "build-info imports must resolve to the Makefile-generated module "
        f"{CANONICAL!r}; otherwise the version banner falls back to 'dev':\n  "
        + "\n  ".join(offenders)
    )


def test_scan_actually_covers_the_known_importers():
    """Sanity: the scan finds the server/tooling importers it's meant to guard.

    Protects against the scan silently matching nothing (e.g. a future dir
    rename) and the suite passing vacuously.
    """
    importers = {
        path.relative_to(ROOT).as_posix()
        for path in _tracked_py_files()
        if _build_info_imports(path)
    }
    for expected in ("server/server.py", "server/workspace_init.py"):
        assert expected in importers, (
            f"{expected} no longer appears to import build-info — the guard "
            "in test_all_build_info_imports_use_canonical_module may be moot."
        )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-v"]))
