#!/usr/bin/env python3
"""Generate event flow visualizations from parsed HSL log data.

Supports three output formats:

- **ascii**: Vertical flow diagram with timing and match indicators.
- **mermaid**: Mermaid sequence diagram (Client → BIG-IP → Server).
- **dot**: Graphviz DOT directed graph with timing labels.

Usage::

    python graph_gen.py --file hsl_events.log --format ascii
    python graph_gen.py --file hsl_events.log --format mermaid > events.md
    python graph_gen.py --file hsl_events.log --format dot | dot -Tpng -o events.png
"""

from __future__ import annotations

from string import Template
from textwrap import dedent

from .parse_logs import EventRecord

# Client-side / server-side classification for Mermaid diagrams

_CLIENT_SIDE_PREFIXES = ("CLIENT", "CLIENTSSL")
_SERVER_SIDE_PREFIXES = ("SERVER", "SERVERSSL")


def _is_client_event(event: str) -> bool:
    return any(event.startswith(p) for p in _CLIENT_SIDE_PREFIXES)


def _is_server_event(event: str) -> bool:
    return any(event.startswith(p) for p in _SERVER_SIDE_PREFIXES)


# ASCII flow diagram


def generate_ascii(
    records: list[EventRecord],
    expected: list[str] | None = None,
) -> str:
    """Generate an ASCII vertical flow diagram.

    Example::

        OK  CLIENT_ACCEPTED       ─── 0ms
            │
        OK  CLIENTSSL_HANDSHAKE   ─── 5ms
            │
        !!  HTTP_REQUEST          ─── 8ms   (expected: LB_SELECTED)
    """
    if not records:
        return "(no events)"

    max_len = max(len(r.event) for r in records)
    lines: list[str] = []

    for i, rec in enumerate(records):
        marker = "  "
        note = ""
        if expected is not None:
            if i < len(expected) and rec.event == expected[i]:
                marker = "OK"
            else:
                marker = "!!"
                if i < len(expected):
                    note = f"   (expected: {expected[i]})"

        lines.append(f"  {marker}  {rec.event:<{max_len}}  ─── {rec.time_ms}ms{note}")
        if i < len(records) - 1:
            pad = 6 + max_len // 2
            lines.append(f"{'│':>{pad}}")

    return "\n".join(lines)


# Mermaid sequence diagram


def generate_mermaid(
    records: list[EventRecord],
    expected: list[str] | None = None,
) -> str:
    """Generate a Mermaid sequence diagram."""
    lines = [
        "sequenceDiagram",
        "    participant C as Client",
        "    participant B as BIG-IP",
        "    participant S as Server",
    ]

    for rec in records:
        event = rec.event
        t = f"{rec.time_ms}ms"
        if _is_client_event(event):
            lines.append(f"    C->>B: {event} ({t})")
        elif _is_server_event(event):
            lines.append(f"    B->>S: {event} ({t})")
        else:
            lines.append(f"    Note over B: {event} ({t})")

    return "\n".join(lines)


# Graphviz DOT

_DOT_HEADER = dedent("""\
    digraph event_flow {
        rankdir=TB;
        node [shape=box, style=filled, fillcolor="#e8f0fe", fontname="monospace"];
        edge [fontname="monospace", fontsize=10];""")

_DOT_NODE = Template('    n$i [label="$event\\n${time}ms", fillcolor="$color"];')
_DOT_EDGE = Template('    n$i -> n$j [label="+${delta}ms"];')


def generate_dot(
    records: list[EventRecord],
    expected: list[str] | None = None,
) -> str:
    """Generate a Graphviz DOT directed graph."""
    lines = [_DOT_HEADER]

    for i, rec in enumerate(records):
        color = "#e8f0fe"  # default blue
        if expected is not None:
            if i < len(expected) and rec.event != expected[i]:
                color = "#fce8e6"  # red for mismatch
        lines.append(_DOT_NODE.substitute(i=i, event=rec.event, time=rec.time_ms, color=color))

    for i in range(len(records) - 1):
        delta = records[i + 1].time_ms - records[i].time_ms
        lines.append(_DOT_EDGE.substitute(i=i, j=i + 1, delta=delta))

    lines.append("}")
    return "\n".join(lines)


# Public dispatch

FORMATS = {
    "ascii": generate_ascii,
    "mermaid": generate_mermaid,
    "dot": generate_dot,
}


def generate_graph(
    fmt: str,
    records: list[EventRecord],
    expected: list[str] | None = None,
) -> str:
    """Generate an event flow graph in the given format.

    Raises ``ValueError`` for unknown formats.
    """
    func = FORMATS.get(fmt)
    if func is None:
        raise ValueError(f"Unknown format {fmt!r}; choose from {sorted(FORMATS)}")
    return func(records, expected)
