"""Regression checks for proc-lookup usage across LSP features."""

from __future__ import annotations

from core.analysis.analyser import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from lsp.features.call_hierarchy import prepare_call_hierarchy
from lsp.features.definition import get_definition
from lsp.features.references import get_references
from lsp.features.rename import get_rename_edits
from lsp.features.signature_help import get_signature_help

TEST_URI = "file:///proc-lookup.tcl"


def test_proc_lookup_consistent_across_lsp_features() -> None:
    source = (
        "namespace eval a {\n"
        "    proc foo {x} { return $x }\n"
        "}\n"
        "namespace eval b {\n"
        "    proc foo {y} { return $y }\n"
        "}\n"
        "foo 1\n"
    )
    analysis = analyse(source)

    proc_match = find_proc_by_reference(analysis, "foo")
    assert proc_match is not None
    expected_qname, expected_proc = proc_match
    assert expected_qname == "::a::foo"

    definition = get_definition(source, TEST_URI, 6, 1, analysis)
    assert len(definition) == 1
    assert definition[0].range.start.line == expected_proc.name_range.start.line

    references = get_references(
        source,
        TEST_URI,
        6,
        1,
        include_declaration=True,
        analysis=analysis,
    )
    assert any(loc.range.start.line == expected_proc.name_range.start.line for loc in references)

    rename_edits = get_rename_edits(source, TEST_URI, 6, 1, "foo_new", analysis)
    assert rename_edits is not None
    changes = rename_edits.changes
    assert changes is not None
    assert TEST_URI in changes
    assert any(
        edit.range.start.line == expected_proc.name_range.start.line for edit in changes[TEST_URI]
    )

    signature = get_signature_help(source, 6, 5, analysis)
    assert signature is not None
    assert signature.signatures[0].label.startswith("foo x")

    call_items = prepare_call_hierarchy(source, TEST_URI, 6, 1, analysis)
    assert len(call_items) == 1
    assert call_items[0].selection_range.start.line == expected_proc.name_range.start.line
