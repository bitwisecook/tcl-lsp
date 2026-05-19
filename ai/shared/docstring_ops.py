"""Shared docstring operations for AI consumers (MCP server, tcl_ai CLI).

Provides high-level helpers that combine core analysis with docstring
utilities, so both the CLI and MCP entry points can share the same logic.
"""

from __future__ import annotations

from typing import Any

from analyser.semantic_model import AnalysisResult
from tooling.formatter.docstring import (
    DocstringTagStyle,
    generate_stub_for_proc,
    parse_docstring,
)


def collect_proc_docs(result: AnalysisResult) -> list[dict[str, Any]]:
    """Build a list of structured documentation dicts for all procs.

    Each entry contains ``name``, ``qualified_name``, ``params``,
    ``doc_raw``/``doc`` (parsed), and ``param_traits`` where available.
    """
    procs: list[dict[str, Any]] = []
    for _qname, proc_def in result.all_procs.items():
        entry: dict[str, Any] = {
            "name": proc_def.name,
            "qualified_name": proc_def.qualified_name,
            "params": [
                {"name": p.name, "default": p.default_value} if p.has_default else {"name": p.name}
                for p in proc_def.params
            ],
        }
        if proc_def.doc:
            entry["doc_raw"] = proc_def.doc
            parsed = parse_docstring(proc_def.doc)
            entry["doc"] = parsed.to_dict()
        else:
            entry["doc"] = None
        if proc_def.param_traits:
            entry["param_traits"] = {
                name: sorted(t.name for t in traits)
                for name, traits in proc_def.param_traits.items()
            }
        procs.append(entry)
    return procs


def insert_docstring_stubs(
    source: str,
    result: AnalysisResult,
    *,
    tag_style: DocstringTagStyle = DocstringTagStyle.DOXYGEN,
    decoration: bool = False,
) -> tuple[str, int]:
    """Insert docstring stubs for all undocumented procs.

    Returns ``(modified_source, documented_count)``.  Inserts from bottom
    to top so that line offsets are not shifted by earlier insertions.
    """
    procs_to_doc = [pd for pd in result.all_procs.values() if not pd.doc]
    procs_to_doc.sort(key=lambda p: p.name_range.start.line, reverse=True)

    lines = source.splitlines(keepends=True)
    for proc_def in procs_to_doc:
        stub = generate_stub_for_proc(proc_def, tag_style=tag_style, decoration=decoration)
        lines.insert(proc_def.name_range.start.line, stub + "\n")

    return "".join(lines), len(procs_to_doc)
