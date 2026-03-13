#!/usr/bin/env python3
"""Parse HSL or syslog output to extract and display event ordering.

Reads log lines (from stdin or a file) containing ``EVTORD`` markers
produced by the generated event-ordering iRules.  Groups events by
session ID, sorts by sequence number, and prints a table.

Optionally compares against the expected ordering from the flow chain
metadata and generates event flow graphs.

Usage::

    # Pipe from HSL receiver output
    python hsl_receiver.py --output events.log --timeout 30
    python parse_logs.py --file events.log

    # From BIG-IP syslog (legacy, also works)
    ssh admin@bigip 'cat /var/log/ltm' | python parse_logs.py

    # Compare + graph
    python parse_logs.py --file events.log --compare --graph ascii
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass


@dataclass
class EventRecord:
    scenario: str
    sid: str
    seq: int
    time_ms: int
    event: str


# Pattern matches:  EVTORD scenario=X sid=Y seq=N t=M event=E
_EVTORD_RE = re.compile(
    r"EVTORD\s+"
    r"scenario=(\S+)\s+"
    r"sid=(\S+)\s+"
    r"seq=(\d+)\s+"
    r"t=(-?\d+)\s+"
    r"event=(\S+)"
)

# RULE_INIT has no session — match:  EVTORD scenario=X event=RULE_INIT
_EVTORD_INIT_RE = re.compile(
    r"EVTORD\s+"
    r"scenario=(\S+)\s+"
    r"event=(RULE_INIT)"
)


def parse_lines(lines: list[str]) -> list[EventRecord]:
    """Extract EventRecords from log lines."""
    records: list[EventRecord] = []
    for line in lines:
        m = _EVTORD_RE.search(line)
        if m:
            records.append(
                EventRecord(
                    scenario=m.group(1),
                    sid=m.group(2),
                    seq=int(m.group(3)),
                    time_ms=int(m.group(4)),
                    event=m.group(5),
                )
            )
    return records


def group_by_session(records: list[EventRecord]) -> dict[str, list[EventRecord]]:
    """Group records by session ID, sorted by sequence number."""
    groups: dict[str, list[EventRecord]] = {}
    for rec in records:
        groups.setdefault(rec.sid, []).append(rec)
    for recs in groups.values():
        recs.sort(key=lambda r: r.seq)
    return groups


def print_table(groups: dict[str, list[EventRecord]]) -> None:
    """Print a human-readable table of event ordering per session."""
    for sid, records in sorted(groups.items()):
        scenario = records[0].scenario if records else "?"
        print(f"\n=== Session {sid} (scenario={scenario}) ===")
        print(f"{'seq':>4}  {'time_ms':>8}  event")
        print(f"{'---':>4}  {'-------':>8}  -----")
        for rec in records:
            print(f"{rec.seq:>4}  {rec.time_ms:>8}  {rec.event}")


def compare_with_expected(
    groups: dict[str, list[EventRecord]],
) -> None:
    """Compare observed ordering against flow chain metadata."""
    # Late import to avoid requiring the full server package for
    # standalone log parsing
    try:
        from core.commands.registry.namespace_data import FLOW_CHAINS
    except ImportError:
        print(
            "\n[warn] Cannot import flow chains for comparison (run from tcl-lsp/ directory)",
            file=sys.stderr,
        )
        return

    for sid, records in sorted(groups.items()):
        scenario_name = records[0].scenario
        chain = FLOW_CHAINS.get(scenario_name)
        if chain is None:
            print(f"\n[warn] No flow chain for scenario '{scenario_name}'")
            continue

        observed = [r.event for r in records]
        expected = [step.event for step in chain.steps if not step.conditional]

        # Filter expected to only events that were observed
        expected_filtered = [e for e in expected if e in set(observed)]

        print(f"\n=== Comparison for session {sid} (scenario={scenario_name}) ===")
        if observed == expected_filtered:
            print("  MATCH: observed order matches expected")
        else:
            print("  MISMATCH:")
            print(f"    Expected: {' -> '.join(expected_filtered)}")
            print(f"    Observed: {' -> '.join(observed)}")
            # Show specific differences
            for i, (exp, obs) in enumerate(zip(expected_filtered, observed)):
                if exp != obs:
                    print(f"    First diff at position {i}: expected {exp}, got {obs}")
                    break


def main() -> None:
    parser = argparse.ArgumentParser(description="Parse BigIP HSL/syslog for event ordering")
    parser.add_argument("--file", "-f", help="Log file (default: stdin)")
    parser.add_argument(
        "--compare", "-c", action="store_true", help="Compare against expected flow chain ordering"
    )
    parser.add_argument(
        "--graph",
        choices=["ascii", "mermaid", "dot"],
        help="Generate event flow graph in given format",
    )
    args = parser.parse_args()

    if args.file:
        with open(args.file) as f:
            lines = f.readlines()
    else:
        lines = sys.stdin.readlines()

    records = parse_lines(lines)
    if not records:
        print("No EVTORD records found in input.", file=sys.stderr)
        sys.exit(1)

    groups = group_by_session(records)
    print_table(groups)

    if args.compare:
        compare_with_expected(groups)

    if args.graph:
        from .graph_gen import generate_graph

        for sid, session_records in sorted(groups.items()):
            scenario = session_records[0].scenario
            print(f"\n=== Graph for session {sid} (scenario={scenario}) ===")
            print(generate_graph(args.graph, session_records))


if __name__ == "__main__":
    main()
