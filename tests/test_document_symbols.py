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


class TestTclOOSymbols:
    """Tests for OO class/method symbol emission."""

    def test_class_symbol_emitted(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} { return "woof" }
            }
        """)
        symbols = get_document_symbols(source)
        assert len(symbols) == 1
        assert symbols[0].kind == types.SymbolKind.Class
        assert symbols[0].name == "Dog"

    def test_methods_nested_under_class(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} { return "woof" }
                method fetch {item} { return $item }
            }
        """)
        symbols = get_document_symbols(source)
        cls = symbols[0]
        assert cls.children is not None
        method_names = [c.name for c in cls.children]
        assert "bark" in method_names
        assert "fetch" in method_names
        for c in cls.children:
            assert c.kind == types.SymbolKind.Method

    def test_constructor_symbol(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                constructor {name} { set n $name }
            }
        """)
        symbols = get_document_symbols(source)
        cls = symbols[0]
        assert cls.children is not None
        ctor = cls.children[0]
        assert ctor.name == "constructor"
        assert ctor.kind == types.SymbolKind.Constructor
        assert ctor.detail is not None
        assert "(name)" in ctor.detail

    def test_property_symbol(self):
        source = textwrap.dedent("""\
            oo::configurable create Point {
                property x y
            }
        """)
        symbols = get_document_symbols(source)
        cls = symbols[0]
        assert cls.children is not None
        prop_names = [c.name for c in cls.children if c.kind == types.SymbolKind.Property]
        assert "x" in prop_names
        assert "y" in prop_names

    def test_class_detail_shows_superclass(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                superclass Animal
            }
        """)
        symbols = get_document_symbols(source)
        assert symbols[0].detail is not None
        assert ": Animal" in symbols[0].detail

    def test_class_detail_shows_metaclass(self):
        source = textwrap.dedent("""\
            oo::abstract create Shape {
                method area {} {}
            }
        """)
        symbols = get_document_symbols(source)
        assert symbols[0].detail is not None
        assert "oo::abstract" in symbols[0].detail

    def test_classmethod_detail(self):
        source = textwrap.dedent("""\
            oo::class create Counter {
                classmethod count {} { return 0 }
            }
        """)
        symbols = get_document_symbols(source)
        cls = symbols[0]
        assert cls.children is not None
        cm = [c for c in cls.children if c.name == "count"]
        assert len(cm) == 1
        assert cm[0].detail is not None
        assert "classmethod" in cm[0].detail


class TestBigipDocumentSymbols:
    def test_outline_groups_by_module_and_kind(self):
        from lsp.features._bigip_symbols import get_bigip_document_symbols

        source = (
            "ltm pool /Common/p1 { description x }\n"
            "ltm pool /Common/p2 { }\n"
            "ltm virtual /Common/vs { destination /Common/1.1.1.1:80 }\n"
            "pem policy /Common/pp { }\n"
            "net vlan /Common/v { tag 100 }\n"
        )
        out = get_bigip_document_symbols(source)
        names = sorted(s.name for s in out)
        assert names == ["ltm", "net", "pem"]
        ltm = next(s for s in out if s.name == "ltm")
        kinds = sorted(c.name for c in (ltm.children or []))
        assert "pool" in kinds and "virtual" in kinds
        pool_kind = next(c for c in (ltm.children or []) if c.name == "pool")
        pool_paths = sorted(o.name for o in (pool_kind.children or []))
        assert pool_paths == ["/Common/p1", "/Common/p2"]

    def test_outline_empty_for_non_bigip_input(self):
        from lsp.features._bigip_symbols import get_bigip_document_symbols

        # Garbage input parses to an empty BigipConfig (no outline).
        out = get_bigip_document_symbols("this is not a bigip config\n")
        assert out == []
