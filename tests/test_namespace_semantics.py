"""Namespace-aware semantics: lattice isolation, relative resolution, moves.

These exercise item-7 requirements that span namespaces:

- the constant-propagation lattice must stay isolated per scope, so a local
  in one namespace never leaks its value into a same-named local in another;
- relative command/variable references inside a ``namespace eval`` resolve to
  the namespace-qualified definition, not a global of the same name;
- same-named definitions in sibling namespaces do not cross-link in
  find-references / call hierarchy.
"""

from __future__ import annotations

import lsp.commands as commands
from lsp.features.call_hierarchy import incoming_calls, prepare_call_hierarchy
from lsp.features.references import get_references
from lsp.state import workspace_state

URI = "file:///ns.tcl"


def _optimise(source: str) -> str:
    workspace_state.open(URI, source)
    result = commands.on_optimise_document(URI)
    assert result is not None
    return result["source"]


def _ref_starts(source: str, line: int, char: int) -> set[tuple[int, int]]:
    return {
        (r.range.start.line, r.range.start.character)
        for r in get_references(source, URI, line, char)
    }


def _incoming(source: str, line: int, char: int) -> set[str]:
    items = prepare_call_hierarchy(source, URI, line, char)
    assert items, "expected a call hierarchy item"
    return {c.from_.name for c in incoming_calls(items[0], source, URI)}


class TestLatticeIsolationAcrossNamespaces:
    def test_same_named_locals_fold_independently(self):
        source = (
            "namespace eval a {\n  proc f {} {\n    set x 1\n    return [expr {$x + 1}]\n  }\n}\n"
            "namespace eval b {\n  proc f {} {\n    set x 99\n    return [expr {$x + 1}]\n  }\n}\n"
        )
        out = _optimise(source)
        # Each namespace's local x folds to its own value, no cross-talk.
        assert "return 2" in out
        assert "return 100" in out

    def test_global_var_not_constant_propagated(self):
        # A namespace/global variable may be mutated elsewhere, so the lattice
        # must NOT fold through it.
        out = _optimise("set ::g 5\nputs [expr {$::g + 1}]\n")
        assert "puts 6" not in out


class TestRelativeNamespaceResolution:
    def test_relative_call_resolves_within_namespace(self):
        source = (
            "namespace eval a {\n    proc helper {} {return 1}\n"
            "    proc caller {} {\n        helper\n    }\n}\n"
        )
        # Definition + the relative call site.
        assert _ref_starts(source, 1, 9) == {(1, 9), (3, 8)}
        assert _incoming(source, 1, 9) == {"caller"}

    def test_same_named_procs_in_sibling_namespaces_do_not_cross_link(self):
        source = (
            "namespace eval a {\n    proc helper {} {return 1}\n    proc f {} { helper }\n}\n"
            "namespace eval b {\n    proc helper {} {return 2}\n    proc g {} { helper }\n}\n"
        )
        # a::helper is only called from a::f, never from b::g.
        assert _incoming(source, 1, 9) == {"f"}
        # b::helper is only called from b::g.
        assert _incoming(source, 5, 9) == {"g"}


class TestNamespaceQualifiedVarMoves:
    def test_qualified_read_tracks_definition_across_forms(self):
        # Relative and absolute qualified reads resolve to the same definition.
        source = "namespace eval ns {\n    variable v 1\n}\nputs $ns::v\nputs $::ns::v\n"
        # Cursor on the $ns::v use; references must include the definition and
        # both qualified reads.
        starts = _ref_starts(source, 3, 7)
        assert (1, 13) in starts  # definition
        assert (3, 5) in starts  # $ns::v
        assert (4, 5) in starts  # $::ns::v
