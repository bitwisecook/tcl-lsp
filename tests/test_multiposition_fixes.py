"""Multi-position quick-fix / optimisation application tests.

When a fix or optimisation applies at many positions in one file, applying
them in a single batch must be race-free (no edit shifts another's offsets,
none is dropped or duplicated), idempotent (re-running changes nothing), and
semantics-preserving (the command structure is unchanged apart from the
intended rewrite).
"""

from __future__ import annotations

import lsp.commands as commands
from core.parsing.command_segmenter import segment_commands
from lsp.features.diagnostics import get_diagnostics
from lsp.state import workspace_state


def _fix_all(uri: str) -> dict:
    result = commands.on_fix_all_safe_issues(uri)
    assert result is not None
    return result


def _command_signature(source: str) -> list[tuple[str, int]]:
    return [(c.texts[0], len(c.texts)) for c in segment_commands(source) if c.texts]


def _codes(source: str) -> set[str]:
    return {(d.code if isinstance(d.code, str) else str(d.code)) for d in get_diagnostics(source)}


class TestFixAllManyPositions:
    def test_w110_fixed_at_every_position(self):
        n = 12
        src = "".join(f'if {{$v{i} == "x{i}"}} {{set r{i} 1}}\n' for i in range(n))
        uri = "file:///w110_many.tcl"
        workspace_state.open(uri, src)

        result = commands.on_fix_all_safe_issues(uri)
        assert result is not None
        out = result["source"]

        # Every position fixed, none missed or duplicated.
        assert len([a for a in result["applied"] if a["code"] == "W110"]) == n
        assert out.count(" eq ") == n
        assert "==" not in out
        # No W110 remains — the fixes actually resolved the issue.
        assert "W110" not in _codes(out)

    def test_command_structure_preserved(self):
        n = 8
        src = "".join(f'if {{$v{i} == "x{i}"}} {{set r{i} 1}}\n' for i in range(n))
        uri = "file:///w110_struct.tcl"
        workspace_state.open(uri, src)
        out = _fix_all(uri)["source"]
        # Same commands, same arities — only the operator changed.
        assert _command_signature(out) == _command_signature(src)

    def test_idempotent(self):
        n = 6
        src = "".join(f'if {{$v{i} == "x{i}"}} {{set r{i} 1}}\n' for i in range(n))
        uri = "file:///w110_idem.tcl"
        workspace_state.open(uri, src)
        once = _fix_all(uri)["source"]

        workspace_state.open(uri, once)
        again = commands.on_fix_all_safe_issues(uri)
        # Nothing left to fix the second time.
        assert again is None or again["source"] == once

    def test_no_offset_drift_with_varying_lengths(self):
        # Mix short and long variable names so a missed offset adjustment in
        # batch application would corrupt later edits.
        names = ["a", "longishname", "x", "another_long_one", "z"]
        src = "".join(f'if {{${nm} == "{nm}"}} {{set ok 1}}\n' for nm in names)
        uri = "file:///w110_drift.tcl"
        workspace_state.open(uri, src)
        out = _fix_all(uri)["source"]
        for nm in names:
            assert f'if {{${nm} eq "{nm}"}}' in out, f"corrupted fix for {nm!r}:\n{out}"


class TestOptimiseManyPositions:
    def test_constant_fold_every_position(self):
        n = 10
        src = "".join(f"set c{i} [expr {{{i} + 1}}]\nputs $c{i}\n" for i in range(n))
        uri = "file:///opt_many.tcl"
        workspace_state.open(uri, src)
        result = commands.on_optimise_document(uri)
        assert result is not None
        out = result["source"]
        # Each folded constant appears; no expr command substitutions remain.
        for i in range(n):
            assert f"puts {i + 1}" in out
        assert "expr" not in out
