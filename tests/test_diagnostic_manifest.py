# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Cross-surface diagnostic and optimisation consistency checks.

Ensures the self-registering code registry (``shared.codes``) is the
single source of truth and every consumer stays aligned:

- Registry: all codes register via ``diag()``/``opt()`` at import time
- LSP server: ``_ALL_DIAGNOSTIC_CODES`` and ``_ALL_OPTIMISATION_CODES``
- VS Code: ``package.json`` setting entries (verified via ``json.loads``)
- JetBrains: generated ``DiagnosticCatalog.kt`` (staleness check)
- TypeScript: generated ``diagnosticCatalog.ts`` (staleness check)
- README: generated ``diagnostic_tables.md`` (staleness check)
- Compiler source: every ``code="..."`` literal must be in the registry

No regex is used on generated editor files — VS Code is parsed as JSON and
all generated files are validated by comparing to generator ``dry_run`` output.

These tests run in ``make prep-pr`` and CI, so drift is caught immediately.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest

import server._codes_init  # noqa: F401
from shared.codes import (
    SECTION_KEYS,
    all_codes,
    diagnostic_codes,
    internal_codes,
    optimisation_codes,
)

ROOT = Path(__file__).resolve().parents[1]


def _read(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


# Helpers — structured extraction (no regex on generated files)


def _vscode_codes_from_json(prefix: str) -> set[str]:
    """Extract setting codes from VS Code package.json by parsing JSON."""
    data = json.loads(_read("editors/vscode/package.json"))
    codes: set[str] = set()
    for group in data["contributes"]["configuration"]:
        for key in group.get("properties", {}):
            if key.startswith(prefix):
                code = key[len(prefix) :]
                # Skip non-code keys like genericVariablePatterns, enabled
                if code and code[0].isupper():
                    codes.add(code)
    return codes


# 1. Compiler source scan — every code must be in the registry


def _scan_compiler_codes() -> set[str]:
    """Scan the seven concern packages for all code=<str> keyword arguments.

    Uses Python's ``ast`` module instead of regex so we only find actual
    ``code=`` keyword arguments in function/constructor calls — not
    occurrences in comments, docstrings, or other string contexts.
    """
    import ast

    codes: set[str] = set()
    for pkg in ("compiler", "analyser", "dialects", "server", "tooling", "shared", "ai"):
        for py_file in ROOT.joinpath(pkg).rglob("*.py"):
            try:
                tree = ast.parse(py_file.read_text(encoding="utf-8"), filename=str(py_file))
            except SyntaxError:
                continue
            for node in ast.walk(tree):
                if not isinstance(node, ast.Call):
                    continue
                for kw in node.keywords:
                    if kw.arg in ("code", "diagnostic_code") and isinstance(kw.value, ast.Constant):
                        val = kw.value.value
                        if (
                            isinstance(val, str)
                            and val.isascii()
                            and val[:1].isupper()
                            and val.isalnum()
                        ):
                            codes.add(val)
    return codes


def test_registry_covers_compiler_codes():
    """Every diagnostic code emitted by the compiler must be in the registry."""
    compiler_codes = _scan_compiler_codes()
    registry_codes = set(all_codes())
    uncovered = compiler_codes - registry_codes
    assert not uncovered, (
        f"Compiler emits codes not in registry: {sorted(uncovered)}\n"
        "Register them via diag() or opt() in shared/codes_*.py."
    )


def test_internal_codes_are_real():
    """Every internal code in the registry must actually appear in
    the compiler source.  This catches stale entries."""
    compiler_codes = _scan_compiler_codes()
    stale = internal_codes() - compiler_codes
    assert not stale, (
        f"Internal codes not found in compiler source: {sorted(stale)}\n"
        "Remove stale entries from shared/codes_*.py."
    )


# 2. LSP server loads codes from registry


def test_server_diagnostic_codes_match_registry():
    """The server's _ALL_DIAGNOSTIC_CODES matches the registry."""
    from server.settings import _ALL_DIAGNOSTIC_CODES

    assert _ALL_DIAGNOSTIC_CODES == diagnostic_codes(), (
        "server.settings._ALL_DIAGNOSTIC_CODES does not match registry"
    )


def test_server_optimisation_codes_match_registry():
    """The server's _ALL_OPTIMISATION_CODES matches the registry."""
    from server.settings import _ALL_OPTIMISATION_CODES

    assert _ALL_OPTIMISATION_CODES == optimisation_codes(), (
        "server.settings._ALL_OPTIMISATION_CODES does not match registry"
    )


# 3. VS Code package.json matches registry (JSON-parsed, no regex)


def test_vscode_diagnostic_settings_match_registry():
    vscode_diag = _vscode_codes_from_json("tclLsp.diagnostics.")
    assert vscode_diag == diagnostic_codes(), (
        f"VS Code package.json diagnostic settings drift:\n"
        f"  In VS Code but not registry: {sorted(vscode_diag - diagnostic_codes())}\n"
        f"  In registry but not VS Code: {sorted(diagnostic_codes() - vscode_diag)}"
    )


def test_vscode_optimiser_settings_match_registry():
    vscode_opt = _vscode_codes_from_json("tclLsp.optimiser.")
    assert vscode_opt == optimisation_codes(), (
        f"VS Code package.json optimiser settings drift:\n"
        f"  In VS Code but not registry: {sorted(vscode_opt - optimisation_codes())}\n"
        f"  In registry but not VS Code: {sorted(optimisation_codes() - vscode_opt)}"
    )


# 4. All generated files match generator dry_run output (staleness checks)


def test_all_generated_files_are_fresh():
    """Every generated file on disk matches the generator's dry_run output."""
    from scripts.codegen.editor_settings import render_all

    stale = []
    for path, expected in render_all(dry_run=True):
        if not path.exists():
            stale.append(f"{path.relative_to(ROOT)} (missing)")
            continue
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            stale.append(str(path.relative_to(ROOT)))
    assert not stale, (
        f"Generated files are stale: {', '.join(stale)}\n"
        "Run 'make gen-editor-settings' to regenerate."
    )


# 5. Registry internal consistency


def test_registry_no_duplicate_codes():
    """Registry rejects duplicates at registration time, but verify the
    final sets are also consistent."""
    diag = diagnostic_codes()
    opt = optimisation_codes()
    internal = internal_codes()
    # No overlap between public diagnostics and optimisations
    overlap = diag & opt
    assert not overlap, f"Codes in both diagnostics and optimisations: {sorted(overlap)}"
    # Internal codes should not appear in public sets
    assert not (diag & internal), (
        f"Codes in both diagnostics and internal: {sorted(diag & internal)}"
    )
    assert not (opt & internal), (
        f"Codes in both optimisations and internal: {sorted(opt & internal)}"
    )


def test_registry_codes_are_sorted_within_sections():
    """Registry diagnostics are sorted by code within each section."""
    from shared.codes import diagnostics_sorted, optimisations_sorted

    # Diagnostics: sorted within each section, sections in SECTIONS order
    current_section = None
    section_codes: list[str] = []
    for d in diagnostics_sorted():
        if d.section != current_section:
            if section_codes:
                assert section_codes == sorted(section_codes), (
                    f"Section {current_section!r} not sorted: {section_codes}"
                )
            current_section = d.section
            section_codes = [d.code]
        else:
            section_codes.append(d.code)
    if section_codes:
        assert section_codes == sorted(section_codes), (
            f"Section {current_section!r} not sorted: {section_codes}"
        )

    # Optimisations: globally sorted (single group)
    opt_codes = [o.code for o in optimisations_sorted()]
    assert opt_codes == sorted(opt_codes), (
        f"Registry optimisations not sorted.\nExpected: {sorted(opt_codes)}"
    )


def test_registry_sections_are_valid():
    """Every code's section is in the SECTION_KEYS list."""
    from shared.codes import _registry

    for code, info in _registry.items():
        if hasattr(info, "section") and info.section:
            assert info.section in SECTION_KEYS, f"Code {code} has unknown section {info.section!r}"


def test_no_overlap_between_public_and_internal():
    """Public codes and internal codes must be disjoint."""
    public = diagnostic_codes() | optimisation_codes()
    overlap = public & internal_codes()
    assert not overlap, (
        f"Codes in both public and internal: {sorted(overlap)}\n"
        "A code should be either user-configurable or internal, not both."
    )


def test_generator_is_idempotent():
    """Running the generator twice must produce identical output."""
    from scripts.codegen.editor_settings import render_all

    results_1 = render_all(dry_run=True)
    results_2 = render_all(dry_run=True)
    for (path1, content1), (path2, content2) in zip(results_1, results_2):
        assert path1 == path2
        assert content1 == content2, f"Generation not idempotent for {path1}"


# 7. Native language toolchain validation (skipped if toolchain not present)

_EXT_DIR = ROOT / "editors" / "vscode"
_PROJECT_TSC = _EXT_DIR / "node_modules" / ".bin" / "tsc"


def _resolve_tsc() -> str | None:
    """Locate a usable ``tsc`` so these tests RUN whenever a TypeScript
    toolchain is present, rather than only when the VS Code workspace has
    been ``npm install``-ed.

    Order: the project-local binary (``editors/vscode/node_modules/.bin/tsc``)
    first — it pins the version the extension actually builds with — then any
    ``tsc`` on ``PATH`` (a global install).  Returns ``None`` only when no
    compiler can be found, which is the one case that legitimately skips.
    """
    if _PROJECT_TSC.exists():
        return str(_PROJECT_TSC)
    return shutil.which("tsc")


_TSC = _resolve_tsc()
_HAS_PROJECT_TSC = _TSC is not None


@pytest.mark.skipif(not _HAS_PROJECT_TSC, reason="no tsc found (project npm install or global tsc)")
def test_typescript_catalog_compiles():
    """Generated diagnosticCatalog.ts compiles with tsc (if available)."""
    assert _TSC is not None  # guaranteed by the skipif above
    ts_path = ROOT / "editors" / "vscode" / "src" / "generated" / "diagnosticCatalog.ts"
    assert ts_path.exists(), f"Missing {ts_path}"

    with tempfile.TemporaryDirectory() as tmp:
        result = subprocess.run(
            [
                _TSC,
                "--noEmit",
                "--strict",
                "--target",
                "ES2020",
                "--moduleResolution",
                "node16",
                "--module",
                "node16",
                str(ts_path),
            ],
            capture_output=True,
            text=True,
            cwd=tmp,
            timeout=30,
        )
        assert result.returncode == 0, f"tsc failed on diagnosticCatalog.ts:\n{result.stderr}"


@pytest.mark.skipif(
    not shutil.which("node") or not _HAS_PROJECT_TSC,
    reason="node or project tsc not found",
)
def test_typescript_catalog_importable_by_node():
    """Generated diagnosticCatalog.ts can be transpiled and loaded by Node."""
    assert _TSC is not None  # guaranteed by the skipif above
    ts_path = ROOT / "editors" / "vscode" / "src" / "generated" / "diagnosticCatalog.ts"
    assert ts_path.exists(), f"Missing {ts_path}"

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        # Transpile to JS for Node ``require()`` consumption.  Mirrors
        # editors/vscode/tsconfig.json's ``module``/``moduleResolution``
        # (Node16): with no ``package.json`` in ``outDir`` to declare
        # ``"type": "module"``, tsc's Node16 emit defaults the output to
        # a plain CJS module, same as the project's own build.
        result = subprocess.run(
            [
                _TSC,
                "--outDir",
                tmp,
                "--target",
                "ES2020",
                "--module",
                "node16",
                "--moduleResolution",
                "node16",
                str(ts_path),
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert result.returncode == 0, f"tsc transpile failed:\n{result.stderr}"

        js_file = tmp_path / "diagnosticCatalog.js"
        assert js_file.exists(), "Transpiled JS file not found"

        # Validate via Node: import and check arrays are non-empty
        check_script = tmp_path / "check.js"
        check_script.write_text(
            'const c = require("./diagnosticCatalog.js");\n'
            "if (!Array.isArray(c.DIAGNOSTICS) || c.DIAGNOSTICS.length === 0) {\n"
            "  process.exit(1);\n"
            "}\n"
            "if (!Array.isArray(c.OPTIMISATIONS) || c.OPTIMISATIONS.length === 0) {\n"
            "  process.exit(1);\n"
            "}\n"
            f"if (c.DIAGNOSTICS.length !== {len(diagnostic_codes())}) {{\n"
            '  console.error("DIAGNOSTICS count mismatch:", c.DIAGNOSTICS.length);\n'
            "  process.exit(1);\n"
            "}\n"
            f"if (c.OPTIMISATIONS.length !== {len(optimisation_codes())}) {{\n"
            '  console.error("OPTIMISATIONS count mismatch:", c.OPTIMISATIONS.length);\n'
            "  process.exit(1);\n"
            "}\n"
            'console.log("OK:", c.DIAGNOSTICS.length, "diagnostics,",\n'
            '  c.OPTIMISATIONS.length, "optimisations");\n',
        )
        result = subprocess.run(
            ["node", str(check_script)],
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert result.returncode == 0, f"Node validation failed:\n{result.stderr}\n{result.stdout}"


@pytest.mark.skipif(not shutil.which("kotlinc"), reason="kotlinc not found")
def test_kotlin_catalog_compiles():
    """Generated DiagnosticCatalog.kt compiles with kotlinc (if available)."""
    kt_path = (
        ROOT
        / "editors"
        / "jetbrains"
        / "src"
        / "main"
        / "kotlin"
        / "com"
        / "tcllsp"
        / "jetbrains"
        / "settings"
        / "generated"
        / "DiagnosticCatalog.kt"
    )
    assert kt_path.exists(), f"Missing {kt_path}"

    with tempfile.TemporaryDirectory() as tmp:
        result = subprocess.run(
            ["kotlinc", str(kt_path), "-d", tmp],
            capture_output=True,
            text=True,
            timeout=120,
        )
        assert result.returncode == 0, f"kotlinc failed on DiagnosticCatalog.kt:\n{result.stderr}"
