"""Lightweight contract tests for refactoring-related Claude skills."""

from __future__ import annotations

from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]


def _read(path: str) -> str:
    return (_ROOT / path).read_text(encoding="utf-8")


def test_tcl_refactor_skill_references_refactor_subcommand():
    text = _read("ai/claude/skills/tcl-refactor/SKILL.md")
    assert "tcl_ai.py refactor" in text
    assert "Extract variable" in text
    assert "switch → dict" in text


def test_irule_datagroup_skill_references_new_datagroup_tools():
    text = _read("ai/claude/skills/irule-datagroup/SKILL.md")
    assert "tcl_ai.py suggest-datagroups" in text
    assert "tcl_ai.py extract-datagroup" in text
