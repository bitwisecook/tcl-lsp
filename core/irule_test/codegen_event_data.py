"""Generate _event_data.tcl from the authoritative Python event registry.

This script reads MASTER_ORDER, ONCE_PER_CONNECTION, PER_REQUEST, and
FLOW_CHAINS from ``core.commands.registry.namespace_data`` and emits
a Tcl file that ``orchestrator.tcl`` sources.  This ensures there is a
single source of truth for event ordering and flow chain definitions.

Usage::

    python -m core.irule_test.codegen_event_data

Or via make::

    make irule-event-data
"""

from __future__ import annotations

from pathlib import Path

from core.commands.registry.namespace_data import (
    FLOW_CHAINS,
    MASTER_ORDER,
    ONCE_PER_CONNECTION,
    PER_REQUEST,
)

_OUTPUT = Path(__file__).parent / "tcl" / "_event_data.tcl"


def _tcl_list(items: frozenset[str] | set[str]) -> str:
    """Format a Python set as a Tcl list ``{A B C}``."""
    return "{" + " ".join(sorted(items)) + "}"


def _generate() -> str:
    lines: list[str] = []

    lines.append(
        "# _event_data.tcl -- AUTO-GENERATED from Python event registry\n"
        "#\n"
        "# DO NOT EDIT.  Regenerate with:\n"
        "#   python -m core.irule_test.codegen_event_data\n"
        "#\n"
        "# Source: core/commands/registry/event_flow_chains.py\n"
        "#\n"
        "# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.\n"
    )

    lines.append("namespace eval ::orch {\n")

    # MASTER_ORDER

    lines.append("    # Master event ordering")
    lines.append("    #")
    lines.append("    # Each entry: {event_name profile_gates}")
    lines.append("    # Profile gates: empty = always relevant; non-empty = requires")
    lines.append("    # at least one of these profiles to be active.\n")
    lines.append("    variable MASTER_ORDER {")

    for event, gates in MASTER_ORDER:
        gate_str = _tcl_list(gates) if gates else "{}"
        lines.append(f"        {{{event:<40s} {gate_str}}}")

    lines.append("    }\n")

    # Index: event → position

    lines.append("    # Build index: event -> position in master ordering")
    lines.append("    variable _event_index")
    lines.append("    array set _event_index {}")
    lines.append("    variable _idx 0")
    lines.append("    foreach _entry $MASTER_ORDER {")
    lines.append("        set _event_index([lindex $_entry 0]) $_idx")
    lines.append("        incr _idx")
    lines.append("    }")
    lines.append("    unset _idx _entry\n")

    # ONCE_PER_CONNECTION

    lines.append("    # Events that fire at most once per connection")
    lines.append("    variable ONCE_PER_CONNECTION {")
    # Sort for stable output
    for event in sorted(ONCE_PER_CONNECTION):
        lines.append(f"        {event}")
    lines.append("    }\n")

    # PER_REQUEST

    lines.append("    # Events that fire once per HTTP transaction (repeatable on keep-alive)")
    lines.append("    variable PER_REQUEST {")
    for event in sorted(PER_REQUEST):
        lines.append(f"        {event}")
    lines.append("    }\n")

    # FLOW_CHAINS

    lines.append("    # Pre-built flow chains")
    lines.append("    #")
    lines.append("    # Each chain: profiles + ordered steps {event phase}")
    lines.append("")
    lines.append("    variable FLOW_CHAINS")
    lines.append("    array set FLOW_CHAINS {}\n")

    for chain_id in sorted(FLOW_CHAINS.keys()):
        chain = FLOW_CHAINS[chain_id]
        profiles_str = " ".join(sorted(chain.profiles))
        lines.append(f"    set FLOW_CHAINS({chain_id}) {{")
        lines.append(f"        profiles {{{profiles_str}}}")
        lines.append("        steps {")
        for step in chain.steps:
            lines.append(f"            {{{step.event} {step.phase}}}")
        lines.append("        }")
        lines.append("    }\n")

    lines.append("}")  # end namespace eval ::orch
    lines.append("")

    return "\n".join(lines)


def main() -> None:
    content = _generate()
    _OUTPUT.write_text(content)
    print(f"Generated {_OUTPUT}")


if __name__ == "__main__":
    main()
