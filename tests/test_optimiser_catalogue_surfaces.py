"""Cross-surface optimiser catalogue consistency checks.

Ensures optimisation codes are unique and complete across:
- Registry: all O-codes registered via ``opt()``
- LSP/server settings allowlist
- Editor settings surfaces (VS Code, JetBrains — via generated file staleness)
- AI prompts and skills

VS Code is validated via ``json.loads`` (structured parsing, no regex).
JetBrains and TypeScript are validated via generator staleness checks in
``test_diagnostic_manifest.py`` — no per-code parsing here.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

import server._codes_init  # noqa: F401
from shared.codes import optimisation_codes

ROOT = Path(__file__).resolve().parents[1]
ALL_OPT_CODES = optimisation_codes()


def _read(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


def _assert_complete_unique(codes: list[str], *, context: str) -> None:
    assert len(codes) == len(set(codes)), f"{context}: duplicate optimisation codes found"
    assert set(codes) == ALL_OPT_CODES, (
        f"{context}: expected {sorted(ALL_OPT_CODES)}, got {sorted(set(codes))}"
    )


def _extract_prompt_optimiser_codes(rel_path: str) -> list[str]:
    text = _read(rel_path)
    lines = [line for line in text.splitlines() if line.startswith("Optimiser:")]
    assert len(lines) == 1, f"{rel_path}: expected exactly one Optimiser catalogue line"
    return re.findall(r"O\d{3}", lines[0])


def _extract_skill_codes(rel_path: str) -> list[str]:
    """Extract O-codes from a skill file.

    Skills may either inline codes (``- O100: ...``) or reference the
    generated ``docs/generated/optimisation_codes.md``.  When a reference
    is found, codes are extracted from the generated file instead.
    """
    text = _read(rel_path)
    marker = "## Optimisation codes reference"
    start = text.find(marker)
    assert start != -1, f"{rel_path}: missing optimisation reference section"
    end = text.find("\n## ", start + len(marker))
    if end == -1:
        end = len(text)
    section = text[start:end]

    # Check for reference to generated file
    generated_ref = "docs/generated/optimisation_codes.md"
    if generated_ref in section:
        gen_text = _read(generated_ref)
        return re.findall(r"\| (O\d{3}) \|", gen_text)

    return re.findall(r"^- (O\d{3}):", section, flags=re.MULTILINE)


def test_lsp_server_allowlist_matches_catalogue() -> None:
    """server.settings._ALL_OPTIMISATION_CODES matches the registry."""
    from server.settings import _ALL_OPTIMISATION_CODES

    codes = sorted(_ALL_OPTIMISATION_CODES)
    _assert_complete_unique(codes, context="server/settings.py _ALL_OPTIMISATION_CODES")


def test_vscode_settings_match_catalogue() -> None:
    """VS Code package.json optimiser codes parsed from JSON structure."""
    data = json.loads(_read("editors/vscode/package.json"))
    prefix = "tclLsp.optimiser."
    codes = []
    for group in data["contributes"]["configuration"]:
        for key in group.get("properties", {}):
            if key.startswith(prefix):
                code = key[len(prefix) :]
                if code and code[0].isupper():
                    codes.append(code)
    _assert_complete_unique(codes, context="editors/vscode/package.json")


def test_jetbrains_generated_catalog_contains_all_python_codes() -> None:
    """DiagnosticCatalog.kt is now generated from the Rust `DiagCode` catalogue
    by `cargo xtask gen-jetbrains-catalog` (a superset of this Python registry),
    so it must carry a `DiagnosticDef` for every Python diagnostic code — extra
    Rust-only codes are fine. Byte-exact freshness is enforced Rust-side by
    `cargo xtask gen-jetbrains-catalog --check`."""
    import re

    from shared.codes import diagnostic_codes

    kt = _read(
        "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/generated/DiagnosticCatalog.kt"
    )
    kt_codes = set(re.findall(r'DiagnosticDef\("([^"]+)"', kt))
    missing = diagnostic_codes() - kt_codes
    assert not missing, f"DiagnosticCatalog.kt is missing diagnostics: {sorted(missing)}"


def test_ai_prompts_match_catalogue() -> None:
    tcl_codes = _extract_prompt_optimiser_codes("ai/prompts/tcl_system.md")
    _assert_complete_unique(tcl_codes, context="ai/prompts/tcl_system.md")

    irules_codes = _extract_prompt_optimiser_codes("ai/prompts/irules_system.md")
    _assert_complete_unique(irules_codes, context="ai/prompts/irules_system.md")


def test_ai_skills_match_catalogue() -> None:
    irule_codes = _extract_skill_codes("ai/claude/skills/irule-optimise/SKILL.md")
    _assert_complete_unique(
        irule_codes,
        context="ai/claude/skills/irule-optimise/SKILL.md",
    )

    tcl_codes = _extract_skill_codes("ai/claude/skills/tcl-optimise/SKILL.md")
    _assert_complete_unique(
        tcl_codes,
        context="ai/claude/skills/tcl-optimise/SKILL.md",
    )


def test_ai_tools_are_runtime_driven_not_code_allowlist_driven() -> None:
    tcl_ai = _read("ai/claude/tcl_ai.py")
    mcp = _read("ai/mcp/tcl_mcp_server.py")

    assert "find_optimisations(source)" in tcl_ai
    assert "find_optimisations(source)" in mcp

    # Tool adapters should not carry a full hardcoded O-code allowlist.
    assert not re.search(r"O100.*O125", tcl_ai, flags=re.DOTALL)
    assert not re.search(r"O100.*O125", mcp, flags=re.DOTALL)
