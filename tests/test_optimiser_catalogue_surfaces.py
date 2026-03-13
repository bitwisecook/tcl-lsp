"""Cross-surface optimiser catalogue consistency checks.

Ensures optimisation codes are unique and complete across:
- LSP/server settings allowlist
- editor settings surfaces (VS Code, JetBrains)
- AI prompts and skills
- AI/MCP optimise tools (runtime-driven, no per-code filtering)
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALL_OPT_CODES = {f"O{i:03d}" for i in range(100, 127)}


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
    text = _read(rel_path)
    marker = "## Optimisation codes reference"
    start = text.find(marker)
    assert start != -1, f"{rel_path}: missing optimisation reference section"
    end = text.find("\n## ", start + len(marker))
    if end == -1:
        end = len(text)
    section = text[start:end]
    return re.findall(r"^- (O\d{3}):", section, flags=re.MULTILINE)


def test_lsp_server_allowlist_matches_catalogue() -> None:
    text = _read("lsp/server.py")
    match = re.search(
        r"_ALL_OPTIMISATION_CODES\s*=\s*frozenset\(\s*\{(.*?)\}\s*\)",
        text,
        flags=re.DOTALL,
    )
    assert match is not None, "lsp/server.py: missing _ALL_OPTIMISATION_CODES"
    codes = re.findall(r'"(O\d{3})"', match.group(1))
    _assert_complete_unique(codes, context="lsp/server.py _ALL_OPTIMISATION_CODES")


def test_vscode_settings_match_catalogue() -> None:
    text = _read("editors/vscode/package.json")
    codes = re.findall(r'"tclLsp\.optimiser\.(O\d{3})"\s*:', text)
    _assert_complete_unique(codes, context="editors/vscode/package.json")


def test_jetbrains_settings_match_catalogue() -> None:
    settings_text = _read(
        "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt"
    )
    panel_text = _read(
        "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettingsPanel.kt"
    )

    declared_codes = re.findall(r"var optimiser(O\d{3}): Boolean", settings_text)
    _assert_complete_unique(
        declared_codes,
        context="JetBrains settings declarations",
    )

    map_codes = re.findall(r'"(O\d{3})"\s+to\s+optimiserO\d{3}', settings_text)
    _assert_complete_unique(
        map_codes,
        context="JetBrains settings payload map",
    )

    checkbox_codes = re.findall(r'JBCheckBox\("(O\d{3})"\)', panel_text)
    _assert_complete_unique(
        checkbox_codes,
        context="JetBrains settings UI checkboxes",
    )


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
