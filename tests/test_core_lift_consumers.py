"""Architecture guards for lifted core modules.

When behaviour is lifted into ``core/``, consumers should import core
modules directly instead of compatibility shims.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

ROOT = Path(__file__).resolve().parent.parent

_BANNED_IMPORT_PATTERNS = (
    re.compile(r"^\s*from\s+lsp\.features\.semantic_graph\s+import\b", re.MULTILINE),
    re.compile(r"^\s*import\s+lsp\.features\.semantic_graph\b", re.MULTILINE),
    re.compile(r"^\s*from\s+lsp\.formatting(?:\.\w+)?\s+import\b", re.MULTILINE),
    re.compile(r"^\s*import\s+lsp\.formatting(?:\.\w+)?\b", re.MULTILINE),
    re.compile(r"^\s*from\s+lsp\.workspace\.package_resolver\s+import\b", re.MULTILINE),
    re.compile(r"^\s*import\s+lsp\.workspace\.package_resolver\b", re.MULTILINE),
    re.compile(r"^\s*from\s+\.workspace\.package_resolver\s+import\b", re.MULTILINE),
)


def _iter_python_files() -> list[Path]:
    roots = ("ai", "core", "explorer", "lsp", "scripts", "tests")
    files: list[Path] = []
    for root_name in roots:
        root = ROOT / root_name
        if not root.exists():
            continue
        files.extend(path for path in root.rglob("*.py") if "__pycache__" not in path.parts)
    return files


def test_consumers_import_lifted_core_modules_directly() -> None:
    violations: list[str] = []

    for path in _iter_python_files():
        text = path.read_text(encoding="utf-8")
        for pattern in _BANNED_IMPORT_PATTERNS:
            if pattern.search(text):
                rel = path.relative_to(ROOT).as_posix()
                violations.append(f"{rel}: matches {pattern.pattern}")

    assert not violations, "Lifted-core import violations:\n" + "\n".join(sorted(violations))
