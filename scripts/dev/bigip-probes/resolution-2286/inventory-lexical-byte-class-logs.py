#!/usr/bin/env python3
"""Inventory R2286LBC chunk streams without accepting incomplete transcripts."""

import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("input", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()

pattern = re.compile(
    rb"R2286LBC\|run=(?P<run>[^|]+)\|ctx=(?P<context>[^|]+)\|"
    rb"group_rc=(?P<group_rc>[^|]+)\|group=(?P<group>[^|]+)\|"
    rb"unit_rc=(?P<unit_rc>[^|]+)\|unit=(?P<unit>[^|]+)\|"
    rb"session=(?P<session>[^|]+)\|seq=(?P<seq>[0-9]+)/(?P<total>[0-9]+)\|"
    rb"hex=(?P<hex>[0-9a-f]*)"
)
groups: dict[tuple[bytes, ...], list[tuple[int, int, bytes]]] = defaultdict(list)
for line in args.input.read_bytes().splitlines():
    match = pattern.search(line)
    if match is None:
        continue
    key = tuple(
        match.group(name)
        for name in (
            "run",
            "context",
            "group_rc",
            "group",
            "unit_rc",
            "unit",
            "session",
        )
    )
    groups[key].append(
        (int(match.group("seq")), int(match.group("total")), match.group("hex"))
    )

inventory = []
for key, chunks in sorted(groups.items()):
    totals = sorted({total for _, total, _ in chunks})
    by_sequence: dict[int, list[bytes]] = defaultdict(list)
    for sequence, _, value in chunks:
        by_sequence[sequence].append(value)
    duplicate_sequences = sorted(
        sequence for sequence, values in by_sequence.items() if len(values) > 1
    )
    conflicting_sequences = sorted(
        sequence for sequence, values in by_sequence.items() if len(set(values)) > 1
    )
    complete = (
        len(totals) == 1
        and not conflicting_sequences
        and set(by_sequence) == set(range(totals[0]))
    )
    assembled = b""
    if complete:
        assembled_hex = b"".join(by_sequence[index][0] for index in range(totals[0]))
        assembled = bytes.fromhex(assembled_hex.decode("ascii"))
    expected = totals[0] if len(totals) == 1 else None
    missing = (
        [] if expected is None else sorted(set(range(expected)) - set(by_sequence))
    )
    inventory.append(
        {
            "run": key[0].decode("ascii"),
            "context": key[1].decode("ascii"),
            "group_rc": key[2].decode("ascii"),
            "group": key[3].decode("ascii"),
            "unit_rc": key[4].decode("ascii"),
            "unit": key[5].decode("ascii"),
            "session": key[6].decode("ascii"),
            "declared_totals": totals,
            "received_lines": len(chunks),
            "received_unique_sequences": len(by_sequence),
            "first_sequence": min(by_sequence) if by_sequence else None,
            "last_sequence": max(by_sequence) if by_sequence else None,
            "duplicate_sequences": duplicate_sequences,
            "conflicting_sequences": conflicting_sequences,
            "missing_count": len(missing),
            "missing_sequences": missing,
            "complete": complete,
            "assembled_bytes": len(assembled),
            "assembled_sha256": hashlib.sha256(assembled).hexdigest()
            if complete
            else None,
        }
    )

args.output.write_text(
    json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="ascii"
)
