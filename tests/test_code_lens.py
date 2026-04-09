"""Tests for the code-lens provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from lsp.features.code_lens import get_code_lenses, resolve_code_lens

TEST_URI = "file:///test.tcl"


class _FakeWorkspaceIndex:
    def __init__(self, counts: dict[str, int]) -> None:
        self._counts = counts

    def proc_usage_counts(self) -> dict[str, int]:
        return dict(self._counts)


class TestGetCodeLenses:
    def test_lens_per_proc(self):
        source = textwrap.dedent("""\
            proc greet {} { return hi }
            proc shout {} { return HI }
        """)
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        assert len(lenses) == 2
        # Unresolved — no command, but data payload present.
        for lens in lenses:
            assert lens.command is None
            assert isinstance(lens.data, dict)
            assert lens.data["kind"] == "proc_ref_count"
            assert lens.data["uri"] == TEST_URI

    def test_empty_when_no_procs(self):
        source = "set x 1\n"
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        assert lenses == []

    def test_none_analysis_runs_inline_analysis(self):
        """Passing analysis=None triggers a throwaway analyse() inline.

        This happens on every codeLens request that arrives before the
        fire-and-forget did_open analysis completes, so it must return
        the same result shape as the pre-analysed path.
        """
        lenses = get_code_lenses("proc f {} {}\nproc g {} {}\n", TEST_URI, None)
        assert len(lenses) == 2
        for lens in lenses:
            assert lens.data is not None


class TestResolveCodeLens:
    def test_populates_title_and_command(self):
        source = "proc greet {} { return hi }\n"
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        assert len(lenses) == 1
        ws = _FakeWorkspaceIndex({"::greet": 3})
        resolved = resolve_code_lens(lenses[0], ws)
        assert resolved.command is not None
        assert resolved.command.title == "3 references"
        assert resolved.command.command == "tcl-lsp.findReferences"
        assert resolved.command.arguments == [TEST_URI, "::greet"]

    def test_singular_title(self):
        source = "proc greet {} { return hi }\n"
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        ws = _FakeWorkspaceIndex({"::greet": 1})
        resolved = resolve_code_lens(lenses[0], ws)
        assert resolved.command is not None
        assert resolved.command.title == "1 reference"

    def test_zero_references(self):
        source = "proc greet {} { return hi }\n"
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        ws = _FakeWorkspaceIndex({})
        resolved = resolve_code_lens(lenses[0], ws)
        assert resolved.command is not None
        assert resolved.command.title == "0 references"
