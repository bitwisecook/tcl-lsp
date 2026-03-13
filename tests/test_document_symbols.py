"""Tests for the document symbol provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.document_symbols import get_document_symbols


def _range_contains(outer: types.Range, inner: types.Range) -> bool:
    if inner.start.line < outer.start.line:
        return False
    if inner.start.line == outer.start.line and inner.start.character < outer.start.character:
        return False
    if inner.end.line > outer.end.line:
        return False
    if inner.end.line == outer.end.line and inner.end.character > outer.end.character:
        return False
    return True


class TestDocumentSymbols:
    def test_single_proc(self):
        source = textwrap.dedent("""\
            proc greet {name} {
                puts "Hello $name"
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) == 1
        assert symbols[0].name == "greet"
        assert symbols[0].kind == types.SymbolKind.Function
        assert symbols[0].detail == "(name)"

    def test_proc_with_defaults(self):
        source = textwrap.dedent("""\
            proc greet {name {greeting Hello}} {
                puts "$greeting $name"
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) == 1
        assert symbols[0].detail == "(name {greeting Hello})"

    def test_multiple_procs(self):
        source = textwrap.dedent("""\
            proc foo {} { return 1 }
            proc bar {} { return 2 }
        """)
        symbols = get_document_symbols(source)
        names = {s.name for s in symbols}
        assert names == {"foo", "bar"}

    def test_namespace_with_proc(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} {
                    return 1
                }
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) == 1
        ns = symbols[0]
        assert ns.name == "myns"
        assert ns.kind == types.SymbolKind.Namespace
        assert ns.children is not None
        assert len(ns.children) == 1
        assert ns.children[0].name == "helper"
        assert ns.children[0].kind == types.SymbolKind.Function

    def test_global_variable(self):
        source = "set myvar 42\n"
        symbols = get_document_symbols(source)
        var_symbols = [s for s in symbols if s.kind == types.SymbolKind.Variable]
        assert len(var_symbols) >= 1
        assert any(s.name == "myvar" for s in var_symbols)

    def test_empty_file(self):
        symbols = get_document_symbols("")
        assert symbols == []

    def test_nested_namespace(self):
        source = textwrap.dedent("""\
            namespace eval outer {
                namespace eval inner {
                    proc deep {} { return }
                }
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) == 1
        outer = symbols[0]
        assert outer.name == "outer"
        assert outer.children is not None
        inner = [c for c in outer.children if c.name == "inner"]
        assert len(inner) == 1
        assert inner[0].children is not None
        assert inner[0].children[0].name == "deep"

    def test_proc_no_params(self):
        source = "proc nop {} { return }\n"
        symbols = get_document_symbols(source)
        assert symbols[0].detail == "()"

    def test_proc_symbol_range_contains_selection(self):
        source = textwrap.dedent("""\
            proc greet {name} {
                puts "Hello $name"
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) == 1
        proc_symbol = symbols[0]
        assert _range_contains(proc_symbol.range, proc_symbol.selection_range)
