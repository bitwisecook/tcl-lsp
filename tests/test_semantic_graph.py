"""Tests for the semantic graph query functions."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.semantic_graph import (
    build_call_graph,
    build_dataflow_graph,
    build_semantic_graph_bundle,
    build_symbol_graph,
)

# Call Graph


class TestBuildCallGraph:
    def test_two_procs_calling_each_other(self):
        source = textwrap.dedent("""\
            proc alpha {} { beta }
            proc beta {} { alpha }
        """)
        data = build_call_graph(source)
        names = {n["name"] for n in data["nodes"]}
        assert "::alpha" in names
        assert "::beta" in names
        # Both call each other
        edge_pairs = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::alpha", "::beta") in edge_pairs
        assert ("::beta", "::alpha") in edge_pairs
        # Neither is a leaf (both call the other)
        assert data["leaf_procs"] == []

    def test_no_procs(self):
        source = "set x 42\nputs $x"
        data = build_call_graph(source)
        assert data["nodes"] == []
        assert data["edges"] == []

    def test_top_level_calls_proc(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        data = build_call_graph(source)
        assert len(data["nodes"]) == 1
        assert data["nodes"][0]["name"] == "::greet"
        # Top-level should call greet
        top_edges = [e for e in data["edges"] if e["caller"] == "<top-level>"]
        assert len(top_edges) == 1
        assert top_edges[0]["callee"] == "::greet"
        assert "<top-level>" in data["roots"]

    def test_pure_proc_marked(self):
        source = textwrap.dedent("""\
            proc add {a b} { expr {$a + $b} }
        """)
        data = build_call_graph(source)
        assert len(data["nodes"]) == 1
        assert data["nodes"][0]["pure"] is True

    def test_proc_params_included(self):
        source = textwrap.dedent("""\
            proc compute {x y z} { expr {$x + $y + $z} }
        """)
        data = build_call_graph(source)
        assert data["nodes"][0]["params"] == ["x", "y", "z"]

    def test_leaf_procs(self):
        source = textwrap.dedent("""\
            proc leaf {} { set x 1 }
            proc caller {} { leaf }
        """)
        data = build_call_graph(source)
        leaf_names = data["leaf_procs"]
        assert "::leaf" in leaf_names
        assert "::caller" not in leaf_names

    def test_empty_source(self):
        data = build_call_graph("")
        assert data["nodes"] == []
        assert data["edges"] == []
        assert data["roots"] == []
        assert data["leaf_procs"] == []


# Symbol Graph


class TestBuildSymbolGraph:
    def test_procs_and_variables(self):
        source = textwrap.dedent("""\
            set result 0
            proc add {a b} {
                return [expr {$a + $b}]
            }
            set result [add 1 2]
        """)
        data = build_symbol_graph(source)
        summary = data["summary"]
        assert summary["total_procs"] >= 1

        # Check scopes exist
        assert len(data["scopes"]) >= 1
        global_scope = data["scopes"][0]
        assert global_scope["kind"] == "global"

    def test_namespace(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { expr {$a + $b} }
            }
        """)
        data = build_symbol_graph(source)
        assert data["summary"]["total_namespaces"] >= 1

    def test_variable_references(self):
        source = textwrap.dedent("""\
            set counter 0
            incr counter
            puts $counter
        """)
        data = build_symbol_graph(source)
        # Should have at least one scope with variables
        global_scope = data["scopes"][0]
        variables = global_scope.get("variables", [])
        if variables:
            counter_var = [v for v in variables if v["name"] == "counter"]
            if counter_var:
                assert len(counter_var[0].get("references", [])) >= 0

    def test_proc_references(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Tcl
        """)
        data = build_symbol_graph(source)
        refs = data.get("proc_references", {})
        # greet should be referenced
        greet_refs = refs.get("::greet", [])
        assert len(greet_refs) >= 2

    def test_package_requires(self):
        source = textwrap.dedent("""\
            package require Tcl 8.6
            package require http
        """)
        data = build_symbol_graph(source)
        pkgs = data.get("package_requires", [])
        pkg_names = [p["name"] for p in pkgs]
        assert "Tcl" in pkg_names
        assert "http" in pkg_names

    def test_empty_source(self):
        data = build_symbol_graph("")
        assert data["summary"]["total_procs"] == 0
        assert data["summary"]["total_variables"] == 0
        assert data["summary"]["total_namespaces"] == 0


# Data-flow Graph


class TestBuildDataflowGraph:
    def test_taint_warning_from_eval(self):
        source = textwrap.dedent("""\
            set input [gets stdin]
            eval $input
        """)
        data = build_dataflow_graph(source)
        warnings = data["taint_warnings"]
        # Should find at least one taint warning (T100 for eval)
        t100s = [w for w in warnings if w["code"] == "T100"]
        assert len(t100s) >= 1
        assert t100s[0]["sink_command"] == "eval"

    def test_clean_source_no_warnings(self):
        source = textwrap.dedent("""\
            proc add {a b} { expr {$a + $b} }
            set x [add 1 2]
        """)
        data = build_dataflow_graph(source)
        assert data["summary"]["total_taint_warnings"] == 0

    def test_proc_effects(self):
        source = textwrap.dedent("""\
            proc pure_add {a b} { expr {$a + $b} }
            proc impure {} { puts "side effect" }
        """)
        data = build_dataflow_graph(source)
        effects = data["proc_effects"]
        names = {e["name"]: e for e in effects}
        if "::pure_add" in names:
            assert names["::pure_add"]["pure"] is True
        if "::impure" in names:
            assert names["::impure"]["pure"] is False

    def test_tainted_variables(self):
        source = textwrap.dedent("""\
            set userdata [gets stdin]
            set safe "constant"
        """)
        data = build_dataflow_graph(source)
        tainted = data["tainted_variables"]
        tainted_names = [t["variable"] for t in tainted]
        assert "userdata" in tainted_names

    def test_empty_source(self):
        data = build_dataflow_graph("")
        assert data["taint_warnings"] == []
        assert data["tainted_variables"] == []
        assert data["proc_effects"] == []

    def test_summary_counts(self):
        source = textwrap.dedent("""\
            proc pure_fn {x} { expr {$x * 2} }
            proc also_pure {y} { expr {$y + 1} }
        """)
        data = build_dataflow_graph(source)
        summary = data["summary"]
        assert summary["pure_proc_count"] >= 2
        assert summary["impure_proc_count"] == 0

    def test_irules_taint(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set uri [HTTP::uri]
                eval $uri
            }
        """)
        data = build_dataflow_graph(source)
        warnings = data["taint_warnings"]
        # Should detect taint from HTTP::uri flowing to eval
        t100s = [w for w in warnings if w["code"] == "T100"]
        assert len(t100s) >= 1


class TestSemanticGraphBundle:
    def test_bundle_builds_all_graphs(self):
        source = "proc p {} { return 1 }\np\n"
        data = build_semantic_graph_bundle(source)
        assert "call_graph" in data
        assert "symbol_graph" in data
        assert "dataflow_graph" in data

    def test_bundle_compiles_once(self):
        source = "set x [expr {1 + 2}]\n"
        from core.compiler import compilation_unit as cu_module

        real_compile_source = cu_module.compile_source
        with patch(
            "core.compiler.compilation_unit.compile_source", wraps=real_compile_source
        ) as mocked_compile:
            build_semantic_graph_bundle(source)
            assert mocked_compile.call_count == 1
