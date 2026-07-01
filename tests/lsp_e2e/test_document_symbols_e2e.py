"""Document symbols, end-to-end against the packaged server.

Full-parity port of the Tcl/TclOO cases in ``tests/test_document_symbols.py``.
Symbol kinds come back as the raw LSP integer codes (``SymbolKind``); the named
constants here mirror that enum.

The BigIP outline (``get_bigip_document_symbols``) is an internal helper driven
by the BigIP dialect and stays unit-tested in ``tests/test_document_symbols.py``.
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

    def test_proc_with_defaults(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(
            uri, 'proc greet {name {greeting Hello}} {\n    puts "$greeting"\n}\n'
        )
        assert _top(lsp_server, uri)[0]["detail"] == "(name {greeting Hello})"

    def test_proc_no_params(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc nop {} { return }\n")
        assert _top(lsp_server, uri)[0]["detail"] == "()"

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

    def test_global_variable(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set myvar 42\n")
        var_syms = [s for s in _top(lsp_server, uri) if s["kind"] == VARIABLE]
        assert any(s["name"] == "myvar" for s in var_syms)

    def test_append_and_lappend_targets_are_variables(self, lsp_server, uri_factory):
        # `append` / `lappend` create their target variable, so it surfaces as a
        # VARIABLE symbol just like `set` (append/lappend var-definition fix).
        uri = uri_factory()
        lsp_server.open_ready(uri, "lappend safe 1\nappend note hi\n")
        names = {s["name"] for s in _top(lsp_server, uri) if s["kind"] == VARIABLE}
        assert {"safe", "note"} <= names, names

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
        syms = _top(lsp_server, uri)
        assert len(syms) == 1
        outer = syms[0]
        assert outer["name"] == "outer"
        inner = [c for c in outer["children"] if c["name"] == "inner"]
        assert len(inner) == 1
        assert inner[0]["children"][0]["name"] == "deep"

    def test_proc_symbol_range_contains_selection(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} {\n    puts "Hello $name"\n}\n')
        sym = _top(lsp_server, uri)[0]
        outer, inner = sym["range"], sym["selectionRange"]
        assert outer["start"]["line"] <= inner["start"]["line"]
        assert outer["end"]["line"] >= inner["end"]["line"]


class TestTclOOSymbols:
    def test_class_symbol_emitted(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(
            uri, 'oo::class create Dog {\n    method bark {} { return "woof" }\n}\n'
        )
        syms = _top(lsp_server, uri)
        assert len(syms) == 1
        assert syms[0]["kind"] == CLASS
        assert syms[0]["name"] == "Dog"

    def test_methods_nested_under_class(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} { return "woof" }
                method fetch {item} { return $item }
            }
        """)
        lsp_server.open_ready(uri, src)
        cls = _top(lsp_server, uri)[0]
        method_names = [c["name"] for c in cls["children"]]
        assert "bark" in method_names
        assert "fetch" in method_names
        assert all(c["kind"] == METHOD for c in cls["children"])

    def test_constructor_symbol(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(
            uri, "oo::class create Dog {\n    constructor {name} { set n $name }\n}\n"
        )
        ctor = _top(lsp_server, uri)[0]["children"][0]
        assert ctor["name"] == "constructor"
        assert ctor["kind"] == CONSTRUCTOR
        assert "(name)" in ctor["detail"]

    def test_property_symbol(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "oo::configurable create Point {\n    property x y\n}\n")
        cls = _top(lsp_server, uri)[0]
        props = {c["name"] for c in cls["children"] if c["kind"] == PROPERTY}
        assert {"x", "y"} <= props

    def test_class_detail_shows_superclass(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "oo::class create Dog {\n    superclass Animal\n}\n")
        assert ": Animal" in (_top(lsp_server, uri)[0].get("detail") or "")

    def test_class_detail_shows_metaclass(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "oo::abstract create Shape {\n    method area {} {}\n}\n")
        assert "oo::abstract" in (_top(lsp_server, uri)[0].get("detail") or "")

    def test_classmethod_detail(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(
            uri, "oo::class create Counter {\n    classmethod count {} { return 0 }\n}\n"
        )
        cls = _top(lsp_server, uri)[0]
        cm = [c for c in cls["children"] if c["name"] == "count"]
        assert len(cm) == 1
        assert "classmethod" in (cm[0].get("detail") or "")


class TestSymbolNamesNonEmpty:
    def test_all_symbols_have_non_empty_names(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "namespace eval a { proc b {} { return } }\noo::class create C { method m {} {} }\n"
        lsp_server.open_ready(uri, src)
        names = [s["name"] for s in flatten_symbols(_top(lsp_server, uri))]
        assert names and all(names)
