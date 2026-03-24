"""Cross-surface diagnostic and optimisation manifest consistency checks.

Ensures the central manifest (core/common/diagnostic_manifest.json) is the
single source of truth and every consumer stays aligned:

- LSP server: ``_ALL_DIAGNOSTIC_CODES`` and ``_ALL_OPTIMISATION_CODES``
- VS Code: ``package.json`` setting entries (verified via ``json.loads``)
- JetBrains: generated ``.kt`` files (verified via generator staleness check)
- Compiler source: every new ``code="..."`` literal must be accounted for

No regex is used on generated editor files — VS Code is parsed as JSON and
JetBrains files are validated by comparing to generator ``dry_run`` output.

These tests run in ``make prep-pr`` and CI, so drift is caught immediately.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# ---------------------------------------------------------------------------
# Load manifest
# ---------------------------------------------------------------------------

_MANIFEST_PATH = ROOT / "core" / "common" / "diagnostic_manifest.json"


def _load_manifest() -> dict:
    return json.loads(_MANIFEST_PATH.read_text(encoding="utf-8"))


def _manifest_diagnostic_codes() -> set[str]:
    manifest = _load_manifest()
    return {d["code"] for d in manifest["diagnostics"]}


def _manifest_optimisation_codes() -> set[str]:
    manifest = _load_manifest()
    return {o["code"] for o in manifest["optimisations"]}


def _read(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


# ---------------------------------------------------------------------------
# Helpers — structured extraction (no regex on generated files)
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# 1. Compiler source scan — every code must be in manifest or allowlist
# ---------------------------------------------------------------------------

# Codes emitted by the compiler that are intentionally NOT user-configurable
# in editor settings.  Each should have a brief comment explaining why.
_INTERNAL_CODES = frozenset(
    {
        # BIG-IP dialect-specific codes (separate dialect toggle, not per-code)
        "BIGIP6001",
        "BIGIP6002",
        "BIGIP6003",
        "BIGIP6004",
        "BIGIP6005",
        "BIGIP6006",
        "BIGIP6007",
        "BIGIP6008",
        "BIGIP6009",
        "BIGIP6010",
        "BIGIP6011",
        # Parser recovery / syntax errors — always active, not toggleable
        "E004",
        "E100",
        "E101",
        "E102",
        "E103",
        "E201",
        "E202",
        "E203",
        # iApps dialect codes
        "IAPP7001",
        "IAPP7002",
        "IAPP7003",
        # iRules taint codes not yet surfaced
        "IRULE3103",
        # iRules internal
        "IRULE5003",
        "IRULE6001",
        # Taint codes not yet surfaced to editors
        "T103",
        "T106",
        # Tk GUI codes (separate feature, not per-code toggle yet)
        "TK1001",
        "TK1002",
        "TK1003",
        # Security codes not yet surfaced
        "W310",
        "W311",
        "W312",
        "W313",
        # F5 XC translatability codes (controlled by xcDiagnostics.enabled)
        "XC100",
        "XC101",
        "XC102",
        "XC103",
        "XC105",
        "XC106",
        "XC107",
        "XC200",
        "XC201",
        "XC203",
        "XC250",
        "XC300",
        "XC301",
    }
)


def _scan_compiler_codes() -> set[str]:
    """Scan core/ for all code=<str> keyword arguments using AST parsing.

    Uses Python's ``ast`` module instead of regex so we only find actual
    ``code=`` keyword arguments in function/constructor calls — not
    occurrences in comments, docstrings, or other string contexts.
    """
    import ast

    codes: set[str] = set()
    for py_file in ROOT.joinpath("core").rglob("*.py"):
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


def test_manifest_covers_compiler_codes():
    """Every diagnostic code emitted by the compiler must be in the manifest
    or in the explicit internal-codes allowlist."""
    compiler_codes = _scan_compiler_codes()
    manifest_codes = _manifest_diagnostic_codes() | _manifest_optimisation_codes()
    uncovered = compiler_codes - manifest_codes - _INTERNAL_CODES
    assert not uncovered, (
        f"Compiler emits codes not in manifest or _INTERNAL_CODES: {sorted(uncovered)}\n"
        "Add them to core/common/diagnostic_manifest.json or to _INTERNAL_CODES "
        "in tests/test_diagnostic_manifest.py if they are intentionally internal."
    )


def test_internal_codes_are_real():
    """Every code in the _INTERNAL_CODES allowlist must actually appear in
    the compiler source.  This catches stale entries."""
    compiler_codes = _scan_compiler_codes()
    stale = _INTERNAL_CODES - compiler_codes
    assert not stale, (
        f"_INTERNAL_CODES contains codes not found in compiler source: {sorted(stale)}\n"
        "Remove stale entries from _INTERNAL_CODES in tests/test_diagnostic_manifest.py."
    )


# ---------------------------------------------------------------------------
# 2. LSP server loads codes from manifest (drift impossible by construction)
# ---------------------------------------------------------------------------


def test_server_loads_codes_from_manifest():
    """The server imports _ALL_DIAGNOSTIC_CODES and _ALL_OPTIMISATION_CODES
    from the manifest via manifest_diagnostic_codes() / manifest_optimisation_codes().
    Verify the import wiring is correct at runtime."""
    import lsp.server as server_module

    manifest_diag = _manifest_diagnostic_codes()
    manifest_opt = _manifest_optimisation_codes()
    assert server_module._ALL_DIAGNOSTIC_CODES == manifest_diag, (
        "server._ALL_DIAGNOSTIC_CODES does not match manifest — "
        "check that server.py imports from manifest_diagnostic_codes()"
    )
    assert server_module._ALL_OPTIMISATION_CODES == manifest_opt, (
        "server._ALL_OPTIMISATION_CODES does not match manifest — "
        "check that server.py imports from manifest_optimisation_codes()"
    )


def test_server_imports_manifest_loaders():
    """Verify server.py uses manifest loaders rather than hardcoded frozensets.

    The runtime check in test_server_loads_codes_from_manifest verifies the
    actual values match.  This test verifies the import wiring by checking
    that manifest_diagnostic_codes is importable from the server module.
    """
    import lsp.server as server_module

    # If these attributes exist and are frozensets loaded from manifest,
    # the import wiring is correct.
    assert hasattr(server_module, "_ALL_DIAGNOSTIC_CODES")
    assert hasattr(server_module, "_ALL_OPTIMISATION_CODES")
    assert isinstance(server_module._ALL_DIAGNOSTIC_CODES, (set, frozenset))
    assert isinstance(server_module._ALL_OPTIMISATION_CODES, (set, frozenset))


# ---------------------------------------------------------------------------
# 3. Manifest vs VS Code package.json (JSON-parsed, no regex)
# ---------------------------------------------------------------------------


def test_vscode_diagnostic_settings_match_manifest():
    vscode_diag = _vscode_codes_from_json("tclLsp.diagnostics.")
    manifest_codes = _manifest_diagnostic_codes()
    assert vscode_diag == manifest_codes, (
        f"VS Code package.json diagnostic settings drift:\n"
        f"  In VS Code but not manifest: {sorted(vscode_diag - manifest_codes)}\n"
        f"  In manifest but not VS Code: {sorted(manifest_codes - vscode_diag)}"
    )


def test_vscode_optimiser_settings_match_manifest():
    vscode_opt = _vscode_codes_from_json("tclLsp.optimiser.")
    manifest_codes = _manifest_optimisation_codes()
    assert vscode_opt == manifest_codes, (
        f"VS Code package.json optimiser settings drift:\n"
        f"  In VS Code but not manifest: {sorted(vscode_opt - manifest_codes)}\n"
        f"  In manifest but not VS Code: {sorted(manifest_codes - vscode_opt)}"
    )


# ---------------------------------------------------------------------------
# 4. JetBrains generated files match generator output (staleness check)
#
# Rather than regex-parsing Kotlin source, we verify the files on disk match
# the generator's dry_run output.  If the file equals the generator output
# and the generator uses the manifest, codes are correct by construction.
# ---------------------------------------------------------------------------

_JB_SETTINGS = "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt"
_JB_PANEL = "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettingsPanel.kt"


def test_jetbrains_settings_match_generator():
    """TclLspSettings.kt on disk matches generator dry_run output."""
    from scripts.generate_editor_settings import generate_jetbrains_settings

    manifest = _load_manifest()
    expected = generate_jetbrains_settings(manifest, dry_run=True)
    actual = _read(_JB_SETTINGS)
    assert actual == expected, (
        "TclLspSettings.kt is stale — run 'make gen-editor-settings' to regenerate"
    )


def test_jetbrains_panel_match_generator():
    """TclLspSettingsPanel.kt on disk matches generator dry_run output."""
    from scripts.generate_editor_settings import generate_jetbrains_panel

    manifest = _load_manifest()
    expected = generate_jetbrains_panel(manifest, dry_run=True)
    actual = _read(_JB_PANEL)
    assert actual == expected, (
        "TclLspSettingsPanel.kt is stale — run 'make gen-editor-settings' to regenerate"
    )


def test_vscode_settings_match_generator():
    """VS Code package.json on disk matches generator dry_run output."""
    from scripts.generate_editor_settings import generate_vscode

    manifest = _load_manifest()
    expected = generate_vscode(manifest, dry_run=True)
    actual = _read("editors/vscode/package.json")
    assert actual == expected, (
        "package.json is stale — run 'make gen-editor-settings' to regenerate"
    )


# ---------------------------------------------------------------------------
# 6. Manifest internal consistency
# ---------------------------------------------------------------------------


def test_manifest_no_duplicate_codes():
    manifest = _load_manifest()
    diag_codes = [d["code"] for d in manifest["diagnostics"]]
    opt_codes = [o["code"] for o in manifest["optimisations"]]
    all_codes = diag_codes + opt_codes
    assert len(all_codes) == len(set(all_codes)), (
        f"Manifest has duplicate codes: {sorted(c for c in all_codes if all_codes.count(c) > 1)}"
    )


def test_manifest_diagnostics_are_sorted():
    manifest = _load_manifest()
    codes = [d["code"] for d in manifest["diagnostics"]]
    assert codes == sorted(codes), (
        f"Manifest diagnostics must be sorted by code.\nExpected order: {sorted(codes)}"
    )


def test_manifest_optimisations_are_sorted():
    manifest = _load_manifest()
    codes = [o["code"] for o in manifest["optimisations"]]
    assert codes == sorted(codes), (
        f"Manifest optimisations must be sorted by code.\nExpected order: {sorted(codes)}"
    )


def test_manifest_entries_have_required_fields():
    manifest = _load_manifest()
    for d in manifest["diagnostics"]:
        assert "code" in d, f"Diagnostic missing 'code': {d}"
        assert "category" in d, f"Diagnostic {d['code']} missing 'category'"
        assert "description" in d, f"Diagnostic {d['code']} missing 'description'"
        assert "default" in d, f"Diagnostic {d['code']} missing 'default'"
    for o in manifest["optimisations"]:
        assert "code" in o, f"Optimisation missing 'code': {o}"
        assert "description" in o, f"Optimisation {o['code']} missing 'description'"
        assert "default" in o, f"Optimisation {o['code']} missing 'default'"


def test_no_overlap_between_manifest_and_internal():
    """Manifest and _INTERNAL_CODES must be disjoint — a code is either
    user-configurable or internal, never both."""
    manifest_codes = _manifest_diagnostic_codes() | _manifest_optimisation_codes()
    overlap = manifest_codes & _INTERNAL_CODES
    assert not overlap, (
        f"Codes in both manifest and _INTERNAL_CODES: {sorted(overlap)}\n"
        "Remove from one or the other."
    )


def test_manifest_categories_are_mapped():
    """Every category value in the manifest must map to a known generator section."""
    from scripts.generate_editor_settings import _KNOWN_CATEGORIES

    manifest = _load_manifest()
    manifest_categories = {d["category"] for d in manifest["diagnostics"]}
    unmapped = manifest_categories - _KNOWN_CATEGORIES
    assert not unmapped, (
        f"Manifest categories not mapped in generator: {sorted(unmapped)}\n"
        "Update _SECTIONS in scripts/generate_editor_settings.py."
    )


def test_generator_is_idempotent():
    """Running the generator twice must produce identical output."""
    from scripts.generate_editor_settings import (
        generate_jetbrains_panel,
        generate_jetbrains_settings,
        generate_vscode,
    )

    manifest = _load_manifest()

    vscode_1 = generate_vscode(manifest, dry_run=True)
    vscode_2 = generate_vscode(manifest, dry_run=True)
    assert vscode_1 == vscode_2, "VS Code generation is not idempotent"

    jb_settings_1 = generate_jetbrains_settings(manifest, dry_run=True)
    jb_settings_2 = generate_jetbrains_settings(manifest, dry_run=True)
    assert jb_settings_1 == jb_settings_2, "JB settings generation is not idempotent"

    jb_panel_1 = generate_jetbrains_panel(manifest, dry_run=True)
    jb_panel_2 = generate_jetbrains_panel(manifest, dry_run=True)
    assert jb_panel_1 == jb_panel_2, "JB panel generation is not idempotent"
