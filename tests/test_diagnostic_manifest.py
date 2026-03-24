"""Cross-surface diagnostic and optimisation manifest consistency checks.

Ensures the central manifest (core/common/diagnostic_manifest.json) is the
single source of truth and every consumer stays aligned:

- LSP server: ``_ALL_DIAGNOSTIC_CODES`` and ``_ALL_OPTIMISATION_CODES``
- VS Code: ``package.json`` setting entries
- JetBrains: ``TclLspSettings.kt`` vars + ``toServerSettings()`` map
- JetBrains: ``TclLspSettingsPanel.kt`` checkboxes
- Compiler source: every new ``code="..."`` literal must be accounted for

These tests run in ``make prep-pr`` and CI, so drift is caught immediately.
"""

from __future__ import annotations

import json
import re
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
# 1. Compiler source scan — every code must be in manifest or allowlist
# ---------------------------------------------------------------------------

# Codes emitted by the compiler that are intentionally NOT user-configurable
# in editor settings.  Each should have a brief comment explaining why.
_INTERNAL_CODES = frozenset(
    {
        # Parser recovery / syntax errors — always active, not toggleable
        "E004",
        "E100",
        "E101",
        "E102",
        "E103",
        "E201",
        "E202",
        "E203",
        # Newer taint codes not yet surfaced to editors
        "T103",
        "T106",
        # iRules taint codes not yet surfaced
        "IRULE3103",
        # iRules internal
        "IRULE5003",
        "IRULE6001",
        # Security codes not yet surfaced
        "W310",
        "W311",
        "W312",
        "W313",
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
        # iApps dialect codes
        "IAPP7001",
        "IAPP7002",
        "IAPP7003",
        # Tk GUI codes (separate feature, not per-code toggle yet)
        "TK1001",
        "TK1002",
        "TK1003",
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
    """Scan core/ for all code="..." string literals."""
    codes: set[str] = set()
    pattern = re.compile(r'code="([A-Z][A-Z0-9]+)"')
    for py_file in ROOT.joinpath("core").rglob("*.py"):
        for match in pattern.finditer(py_file.read_text(encoding="utf-8")):
            codes.add(match.group(1))
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
# 2. Manifest vs LSP server frozensets
# ---------------------------------------------------------------------------


def test_server_diagnostic_codes_match_manifest():
    text = _read("lsp/server.py")
    match = re.search(
        r"_ALL_DIAGNOSTIC_CODES\s*=\s*frozenset\(\s*\{(.*?)\}\s*\)",
        text,
        flags=re.DOTALL,
    )
    assert match is not None, "lsp/server.py: missing _ALL_DIAGNOSTIC_CODES"
    server_codes = set(re.findall(r'"([A-Z][A-Z0-9]+)"', match.group(1)))
    manifest_codes = _manifest_diagnostic_codes()
    assert server_codes == manifest_codes, (
        f"_ALL_DIAGNOSTIC_CODES drift:\n"
        f"  In server but not manifest: {sorted(server_codes - manifest_codes)}\n"
        f"  In manifest but not server: {sorted(manifest_codes - server_codes)}"
    )


def test_server_optimisation_codes_match_manifest():
    text = _read("lsp/server.py")
    match = re.search(
        r"_ALL_OPTIMISATION_CODES\s*=\s*frozenset\(\s*\{(.*?)\}\s*\)",
        text,
        flags=re.DOTALL,
    )
    assert match is not None, "lsp/server.py: missing _ALL_OPTIMISATION_CODES"
    server_codes = set(re.findall(r'"(O\d{3})"', match.group(1)))
    manifest_codes = _manifest_optimisation_codes()
    assert server_codes == manifest_codes, (
        f"_ALL_OPTIMISATION_CODES drift:\n"
        f"  In server but not manifest: {sorted(server_codes - manifest_codes)}\n"
        f"  In manifest but not server: {sorted(manifest_codes - server_codes)}"
    )


# ---------------------------------------------------------------------------
# 3. Manifest vs VS Code package.json
# ---------------------------------------------------------------------------


def test_vscode_diagnostic_settings_match_manifest():
    text = _read("editors/vscode/package.json")
    vscode_diag = set(re.findall(r'"tclLsp\.diagnostics\.([A-Z][A-Z0-9]+)"\s*:', text))
    manifest_codes = _manifest_diagnostic_codes()
    assert vscode_diag == manifest_codes, (
        f"VS Code package.json diagnostic settings drift:\n"
        f"  In VS Code but not manifest: {sorted(vscode_diag - manifest_codes)}\n"
        f"  In manifest but not VS Code: {sorted(manifest_codes - vscode_diag)}"
    )


def test_vscode_optimiser_settings_match_manifest():
    text = _read("editors/vscode/package.json")
    vscode_opt = set(re.findall(r'"tclLsp\.optimiser\.(O\d{3})"\s*:', text))
    manifest_codes = _manifest_optimisation_codes()
    assert vscode_opt == manifest_codes, (
        f"VS Code package.json optimiser settings drift:\n"
        f"  In VS Code but not manifest: {sorted(vscode_opt - manifest_codes)}\n"
        f"  In manifest but not VS Code: {sorted(manifest_codes - vscode_opt)}"
    )


# ---------------------------------------------------------------------------
# 4. Manifest vs JetBrains TclLspSettings.kt
# ---------------------------------------------------------------------------


_JB_SETTINGS = "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt"
_JB_PANEL = "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettingsPanel.kt"


def test_jetbrains_diagnostic_vars_match_manifest():
    text = _read(_JB_SETTINGS)
    jb_codes = set(re.findall(r"var diagnostic([A-Z][A-Z0-9]+): Boolean", text))
    manifest_codes = _manifest_diagnostic_codes()
    assert jb_codes == manifest_codes, (
        f"JetBrains TclLspSettings.kt diagnostic var drift:\n"
        f"  In JB but not manifest: {sorted(jb_codes - manifest_codes)}\n"
        f"  In manifest but not JB: {sorted(manifest_codes - jb_codes)}"
    )


def test_jetbrains_diagnostic_map_match_manifest():
    text = _read(_JB_SETTINGS)
    jb_map = set(re.findall(r'"([A-Z][A-Z0-9]+)"\s+to\s+diagnostic[A-Z]', text))
    manifest_codes = _manifest_diagnostic_codes()
    assert jb_map == manifest_codes, (
        f"JetBrains toServerSettings() diagnostics map drift:\n"
        f"  In JB map but not manifest: {sorted(jb_map - manifest_codes)}\n"
        f"  In manifest but not JB map: {sorted(manifest_codes - jb_map)}"
    )


def test_jetbrains_optimiser_vars_match_manifest():
    text = _read(_JB_SETTINGS)
    jb_codes = set(re.findall(r"var optimiser(O\d{3}): Boolean", text))
    manifest_codes = _manifest_optimisation_codes()
    assert jb_codes == manifest_codes, (
        f"JetBrains TclLspSettings.kt optimiser var drift:\n"
        f"  In JB but not manifest: {sorted(jb_codes - manifest_codes)}\n"
        f"  In manifest but not JB: {sorted(manifest_codes - jb_codes)}"
    )


def test_jetbrains_optimiser_map_match_manifest():
    text = _read(_JB_SETTINGS)
    jb_map = set(re.findall(r'"(O\d{3})"\s+to\s+optimiserO\d{3}', text))
    manifest_codes = _manifest_optimisation_codes()
    assert jb_map == manifest_codes, (
        f"JetBrains toServerSettings() optimiser map drift:\n"
        f"  In JB map but not manifest: {sorted(jb_map - manifest_codes)}\n"
        f"  In manifest but not JB map: {sorted(manifest_codes - jb_map)}"
    )


# ---------------------------------------------------------------------------
# 5. Manifest vs JetBrains TclLspSettingsPanel.kt
# ---------------------------------------------------------------------------


def test_jetbrains_panel_diagnostic_checkboxes_match_manifest():
    text = _read(_JB_PANEL)
    jb_panel = set(re.findall(r"private val diag([A-Z][A-Z0-9]+)\s*=\s*JBCheckBox", text))
    manifest_codes = _manifest_diagnostic_codes()
    assert jb_panel == manifest_codes, (
        f"JetBrains panel diagnostic checkbox drift:\n"
        f"  In panel but not manifest: {sorted(jb_panel - manifest_codes)}\n"
        f"  In manifest but not panel: {sorted(manifest_codes - jb_panel)}"
    )


def test_jetbrains_panel_optimiser_checkboxes_match_manifest():
    text = _read(_JB_PANEL)
    jb_panel = set(re.findall(r'JBCheckBox\("(O\d{3})(?:[^"]*)?"\)', text))
    manifest_codes = _manifest_optimisation_codes()
    assert jb_panel == manifest_codes, (
        f"JetBrains panel optimiser checkbox drift:\n"
        f"  In panel but not manifest: {sorted(jb_panel - manifest_codes)}\n"
        f"  In manifest but not panel: {sorted(manifest_codes - jb_panel)}"
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
