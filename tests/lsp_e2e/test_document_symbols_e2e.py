"""Document symbols, end-to-end against the packaged server.

Ported from ``tests/test_document_symbols.py``.  Symbol kinds come back as the
raw LSP integer codes (``SymbolKind``); the named constants used here mirror
that enum.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import flatten_symbols, symbol_names

# LSP SymbolKind integer codes.
NAMESPACE = 3
CLASS = 5
METHOD = 6
PROPERTY = 7
CONSTRUCTOR = 9
FUNCTION = 12
VARIABLE = 13


def _top(lsp_server, uri):
    return lsp_server.document_symbols(uri) or []


class TestDocumentSymbols:
    def test_single_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} {\n    puts "Hello $name"\n}\n')
        syms = _top(lsp_server, uri)
        assert len(syms) == 1
        assert syms[0]["name"] == "greet"
        assert syms[0]["kind"] == FUNCTION
        assert syms[0]["detail"] == "(name)"

    def test_multiple_procs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc foo {} { return 1 }\nproc bar {} { return 2 }\n")
        assert {"foo", "bar"} <= symbol_names(_top(lsp_server, uri))

    def test_namespace_with_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} {
                    return 1
                }
            }
        """)
        lsp_server.open_ready(uri, src)
        syms = _top(lsp_server, uri)
        assert len(syms) == 1
        ns = syms[0]
        assert ns["name"] == "myns"
        assert ns["kind"] == NAMESPACE
        assert [c["name"] for c in ns["children"]] == ["helper"]
        assert ns["children"][0]["kind"] == FUNCTION

    def test_empty_file(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "")
        assert (lsp_server.document_symbols(uri) or []) == []

    def test_nested_namespace(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval outer {
                namespace eval inner {
                    proc deep {} { return }
                }
            }
        """)
        lsp_server.open_ready(uri, src)
        assert {"outer", "inner", "deep"} <= symbol_names(_top(lsp_server, uri))


class TestTclOOSymbols:
    def test_class_with_methods_and_ctor(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Dog {
                constructor {name} { set n $name }
                method bark {} { return "woof" }
                method fetch {item} { return $item }
            }
        """)
        lsp_server.open_ready(uri, src)
        syms = _top(lsp_server, uri)
        assert len(syms) == 1
        cls = syms[0]
        assert cls["kind"] == CLASS
        assert cls["name"] == "Dog"
        kinds = {c["name"]: c["kind"] for c in cls["children"]}
        assert kinds["bark"] == METHOD
        assert kinds["fetch"] == METHOD
        assert kinds["constructor"] == CONSTRUCTOR

    def test_class_detail_shows_superclass(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "oo::class create Dog {\n    superclass Animal\n}\n")
        syms = _top(lsp_server, uri)
        assert ": Animal" in (syms[0].get("detail") or "")

    def test_property_symbols(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "oo::configurable create Point {\n    property x y\n}\n")
        cls = _top(lsp_server, uri)[0]
        props = {c["name"] for c in cls["children"] if c["kind"] == PROPERTY}
        assert {"x", "y"} <= props


class TestSelectionRangeContainment:
    def test_proc_symbol_range_contains_selection(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} {\n    puts "Hello $name"\n}\n')
        sym = _top(lsp_server, uri)[0]
        outer, inner = sym["range"], sym["selectionRange"]
        assert outer["start"]["line"] <= inner["start"]["line"]
        assert outer["end"]["line"] >= inner["end"]["line"]

    def test_all_symbols_have_non_empty_names(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "namespace eval a { proc b {} { return } }\noo::class create C { method m {} {} }\n"
        lsp_server.open_ready(uri, src)
        names = [s["name"] for s in flatten_symbols(_top(lsp_server, uri))]
        assert names and all(names)
