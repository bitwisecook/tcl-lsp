"""Shared iRule analysis utilities for AI consumers.

This module provides a single import point for common iRule analysis
operations used by ``ai.claude.tcl_ai``, ``ai.mcp.tcl_mcp_server``,
editor AI features, and MCP skills.  It wraps the underlying compiler,
registry, and diagram APIs behind simple, typed functions.

Design goals:
  - Eliminate duplicated event-order / multiplicity / taint / diagram
    logic across the AI layer.
  - Lazy imports only — no heavy module-level work so that importing
    this file is cheap.
  - Pure data out — returns dicts / dataclasses, never prints.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

# Event ordering & multiplicity


@dataclass(frozen=True, slots=True)
class EventInfo:
    """An event in canonical firing order with metadata."""

    index: int
    name: str
    multiplicity: str  # "init" | "once_per_connection" | "per_request" | "unknown"


def event_multiplicity(event_name: str) -> str:
    """Return the multiplicity category for a single event name.

    Returns one of ``"init"``, ``"once_per_connection"``,
    ``"per_request"``, or ``"unknown"``.

    Delegates to the Rust ``tcl_lsp_py.event_multiplicity`` facade.
    """
    from ai.shared.rust_bridge import require_rust

    return require_rust().event_multiplicity(event_name)


def ordered_events(source: str) -> list[EventInfo]:
    """Scan ``when`` blocks in *source* and return events in firing order.

    Each entry includes its 1-based index and multiplicity.
    """
    from ai.shared.rust_bridge import require_rust

    rust = require_rust()
    names = rust.order_events_for_file(source)
    return [
        EventInfo(index=i, name=name, multiplicity=rust.event_multiplicity(name))
        for i, name in enumerate(names, 1)
    ]


def ordered_events_as_dicts(source: str) -> list[dict[str, Any]]:
    """Same as :func:`ordered_events` but returns plain dicts.

    Convenient for JSON serialisation in MCP tools.
    """
    return [
        {"index": e.index, "name": e.name, "multiplicity": e.multiplicity}
        for e in ordered_events(source)
    ]


# Taint analysis


def taint_warnings(source: str) -> list[dict[str, Any]]:
    """Run taint analysis on *source* and return warnings as dicts.

    Returns an empty list if the taint module is unavailable or the
    source cannot be compiled.  Each dict has at minimum ``code``,
    ``message`` keys, plus optional ``range``, ``variable``, and
    ``sink_command``.
    """
    try:
        from ai.shared.rust_bridge import require_rust

        # The Rust dataflow graph carries the taint warnings (0-based `line`,
        # not a full range) plus the tainted variable and sink command.
        graph = require_rust().dataflow_graph(source, dialect="f5-irules")
        return list(graph.get("taint_warnings", []))
    except Exception:
        return []


# Diagram / CFG extraction


def diagram_data(source: str) -> dict[str, Any]:
    """Extract control flow diagram data from compiler IR.

    Returns the raw dict from ``extract_diagram_data``, or an empty
    dict with an ``"error"`` key on failure.
    """
    try:
        from ai.shared.rust_bridge import require_rust

        return require_rust().diagram_data(source, dialect="f5-irules")
    except Exception as exc:
        return {"error": str(exc), "events": {}}


# Diagnostic conversion


def diagnostic_to_dict(d: Any) -> dict[str, Any]:
    """Convert a ``semantic_model.Diagnostic`` to a plain dict.

    Includes category from the shared categorisation table.
    """
    from ai.shared.diagnostics import categorise

    result: dict[str, Any] = {
        "code": d.code or "",
        "severity": d.severity.name.lower(),
        "message": d.message,
        "range": range_to_dict(d.range),
        "category": categorise(d.code or ""),
    }
    if d.fixes:
        result["fixes"] = [
            {
                "range": range_to_dict(f.range),
                "new_text": f.new_text,
                "description": f.description,
            }
            for f in d.fixes
        ]
    return result


def range_to_dict(r: Any) -> dict[str, Any]:
    """Convert a Range to ``{start: {line, character}, end: ...}``.

    Works with both ``semantic_model.Range`` and ``lsprotocol.types.Range``.
    """
    return {
        "start": {"line": r.start.line, "character": r.start.character},
        "end": {"line": r.end.line, "character": r.end.character},
    }
