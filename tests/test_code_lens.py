"""Tests for the code-lens provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from server.features.code_lens import get_code_lenses, resolve_code_lens

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
        # Thin client-side wrapper: it converts the JSON-RPC argument
        # shapes into the vscode.Uri/Position/Location instances that the
        # built-in ``editor.action.showReferences`` command requires.
        assert resolved.command.command == "tcl-lsp.showReferences"
        assert resolved.command.arguments is not None
        assert resolved.command.arguments[0] == TEST_URI
        # The second argument is the position to display in the peek header —
        # the start of the proc name range supplied by ``get_code_lenses``.
        assert resolved.command.arguments[1] == lenses[0].range.start
        # Third argument is the locations list. Without a reference-finder
        # passed in we still emit an empty list so the command is well-formed.
        assert resolved.command.arguments[2] == []

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

    def test_find_references_callback_populates_locations(self):
        from lsprotocol import types

        source = "proc greet {} { return hi }\n"
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        ws = _FakeWorkspaceIndex({"::greet": 2})

        want_loc = types.Location(
            uri=TEST_URI,
            range=types.Range(
                start=types.Position(line=5, character=0),
                end=types.Position(line=5, character=5),
            ),
        )

        def finder(uri: str, qname: str) -> list[types.Location]:
            assert uri == TEST_URI
            assert qname == "::greet"
            return [want_loc]

        resolved = resolve_code_lens(lenses[0], ws, finder)
        assert resolved.command is not None
        assert resolved.command.arguments is not None
        assert resolved.command.arguments[2] == [want_loc]

    def test_count_derives_from_resolved_locations(self):
        """When a resolver is supplied the title count equals ``len(locations)``.

        Regression for issue #637: the count must come from the same
        reference resolution that backs the peek, not the parallel
        ``proc_usage_counts`` aggregate — so it cannot disagree with the
        locations shown when the lens is clicked.
        """
        from lsprotocol import types

        source = "proc greet {} { return hi }\n"
        analysis = analyse(source)
        lenses = get_code_lenses(source, TEST_URI, analysis)
        # The cached aggregate deliberately disagrees with the resolver to
        # prove the title now follows the resolver, not the aggregate.
        ws = _FakeWorkspaceIndex({"::greet": 99})

        locs = [
            types.Location(
                uri=TEST_URI,
                range=types.Range(
                    start=types.Position(line=line, character=0),
                    end=types.Position(line=line, character=5),
                ),
            )
            for line in (3, 4)
        ]

        resolved = resolve_code_lens(lenses[0], ws, lambda _uri, _qname: locs)
        assert resolved.command is not None
        assert resolved.command.title == "2 references"
        assert resolved.command.arguments is not None
        peek = resolved.command.arguments[2]
        assert isinstance(peek, list)
        assert len(peek) == 2


def _title_count(title: str) -> int:
    return int(title.split(" ", 1)[0])


class TestCountMatchesReferences:
    """The CodeLens count must equal ``len(references)`` for the same proc.

    Regression for issue #637: ``resolve_code_lens`` reported "0 references"
    above a proc whose reference list (Find All References / the lens peek)
    clearly showed call sites. The two paths diverged because the count came
    from ``proc_usage_counts`` (keyed by the resolved qualified name, which is
    ``None`` for a forward or cross-file call) while the resolution matched by
    name. The cases below are the ones most likely to diverge.
    """

    CASES = {
        "forward_ref": "foo\nproc foo {} {}\n",
        "after_def": "proc foo {} {}\nfoo\n",
        "qualified_call": "proc foo {} {}\n::foo\n",
        "ns_qualified_call": "namespace eval ns { proc foo {} {} }\nns::foo\n",
        "call_inside_body": "proc foo {} {}\nproc bar {} { foo }\n",
        "cmd_substitution": "proc foo {} {}\nset x [foo]\n",
        "unused_proc": "proc foo {} {}\n",
    }

    @staticmethod
    def _finder(analysis):
        from server._lsp_conv import to_lsp_location
        from server.features.references import find_proc_call_sites

        def finder(uri: str, qname: str):
            proc_name = qname.rsplit("::", 1)[-1] or qname
            ranges = find_proc_call_sites(proc_name, qname, analysis)
            return [to_lsp_location(uri, r) for r in ranges]

        return finder

    def test_count_equals_reference_list(self):
        from server.features.references import get_references

        for name, source in self.CASES.items():
            analysis = analyse(source)
            ws = _FakeWorkspaceIndex(_resolved_counts(analysis))
            finder = self._finder(analysis)
            lenses = get_code_lenses(source, TEST_URI, analysis)
            for lens in lenses:
                resolved = resolve_code_lens(lens, ws, finder)
                assert resolved.command is not None
                count = _title_count(resolved.command.title)
                pos = lens.range.start
                # Find All References at the proc's name position, excluding
                # the declaration itself (LSP includeDeclaration semantics) —
                # the lens counts call sites, not the definition.
                refs = get_references(
                    source,
                    TEST_URI,
                    pos.line,
                    pos.character,
                    analysis,
                    include_declaration=False,
                )
                assert count == len(refs), f"{name}: count {count} != len(refs) {len(refs)}"
                # And the peek locations the lens carries match the count too.
                assert resolved.command.arguments is not None
                peek = resolved.command.arguments[2]
                assert isinstance(peek, list)
                assert count == len(peek)

    def test_forward_reference_is_not_zero(self):
        """The headline #637 symptom: a call before the definition.

        ``proc_usage_counts`` reports 0 for it, but the proc is plainly used.
        """
        source = self.CASES["forward_ref"]
        analysis = analyse(source)
        # The pre-fix count path would yield 0 here.
        assert _resolved_counts(analysis).get("::foo", 0) == 0
        ws = _FakeWorkspaceIndex(_resolved_counts(analysis))
        lenses = get_code_lenses(source, TEST_URI, analysis)
        resolved = resolve_code_lens(lenses[0], ws, self._finder(analysis))
        assert resolved.command is not None
        assert resolved.command.title == "1 reference"


def _resolved_counts(analysis) -> dict[str, int]:
    """Mirror ``WorkspaceIndex.proc_usage_counts`` for a single analysis."""
    counts: dict[str, int] = {}
    for invocation in analysis.command_invocations:
        qname = invocation.resolved_qualified_name
        if not qname:
            continue
        counts[qname] = counts.get(qname, 0) + 1
    return counts
