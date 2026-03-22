"""Tests for data-flow graph extraction and serialisation."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.dataflow_graph import (
    dataflow_graph_to_dict,
    dataflow_graph_to_mermaid,
    extract_dataflow_graph,
)


class TestDataFlowGraphExtraction:
    def test_simple_linear(self):
        graph = extract_dataflow_graph("set x 1\nset y [expr {$x + 1}]")
        assert len(graph.functions) >= 1
        assert graph.total_defs >= 2
        assert graph.total_uses >= 1

    def test_dead_definition_detected(self):
        graph = extract_dataflow_graph("set unused 42")
        top = graph.functions[0]
        dead_nodes = [n for n in top.nodes if n.is_dead]
        assert len(dead_nodes) >= 1

    def test_proc_extraction(self):
        source = "proc foo {x} { return $x }\nset r [foo 42]"
        graph = extract_dataflow_graph(source)
        assert len(graph.functions) >= 2  # top + ::foo

    def test_alias_detection(self):
        source = "proc bar {} { global gvar\n set gvar 42 }"
        graph = extract_dataflow_graph(source)
        # Find the bar function
        bar_fn = None
        for f in graph.functions:
            if "bar" in f.function_name:
                bar_fn = f
                break
        assert bar_fn is not None
        assert len(bar_fn.aliases) >= 1

    def test_phi_edges(self):
        source = "if {$cond} {set a 1} else {set a 2}\nset b $a"
        graph = extract_dataflow_graph(source)
        top = graph.functions[0]
        phi_edges = [e for e in top.edges if e.edge_kind == "phi"]
        assert len(phi_edges) >= 2  # Both branches feed into phi


class TestDataFlowGraphSerialisation:
    def test_to_dict(self):
        graph = extract_dataflow_graph("set x 1\nset y $x")
        d = dataflow_graph_to_dict(graph)
        assert "functions" in d
        assert "summary" in d
        assert d["summary"]["functionCount"] >= 1

        # Verify JSON serialisability
        json_str = json.dumps(d)
        assert len(json_str) > 0

    def test_to_dict_has_nodes_and_edges(self):
        graph = extract_dataflow_graph("set x 1\nset y [expr {$x + 1}]")
        d = dataflow_graph_to_dict(graph)
        func = d["functions"][0]
        assert len(func["nodes"]) >= 2
        assert len(func["edges"]) >= 1
        assert "name" in func["nodes"][0]
        assert "version" in func["nodes"][0]

    def test_to_mermaid(self):
        graph = extract_dataflow_graph("set x 1\nset y [expr {$x + 1}]")
        mermaid = dataflow_graph_to_mermaid(graph)
        assert "flowchart TD" in mermaid
        assert "subgraph" in mermaid

    def test_empty_source(self):
        graph = extract_dataflow_graph("")
        d = dataflow_graph_to_dict(graph)
        assert d["summary"]["functionCount"] >= 0


class TestDataFlowGraphExplorer:
    def test_pipeline_integration(self):
        """Verify the data-flow graph appears in explorer pipeline results."""
        from explorer.pipeline import run_pipeline
        from explorer.serialise import serialise_result

        result = run_pipeline("set x 1\nset y $x")
        assert result.dataflow_graph is not None

        d = serialise_result(result)
        assert "dataflow" in d
        assert d["dataflow"] is not None
        assert d["dataflow"]["summary"]["totalDefs"] >= 2

    def test_pipeline_stats(self):
        """Verify data-flow stats appear in pipeline stats."""
        from explorer.pipeline import compute_stats, run_pipeline

        result = run_pipeline("set x 1\nset y $x")
        stats = compute_stats(result)
        assert "dataflowDefs" in stats
        assert stats["dataflowDefs"] >= 2
