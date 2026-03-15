#!/usr/bin/env python3
"""Audit tcl-lsp iRules metadata against BIG-IP extract artifacts.

This script compares registry metadata with:
  - irule-schema-split (commands/events/namespaces)
  - man-*/ltm_rule_command_*.3 VALID DURING sections

Usage:
  uv run python scripts/audit_irules_extract_alignment.py \
      --extract-root ~/src/bigip-extract
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

from core.commands.registry import REGISTRY
from core.commands.registry.namespace_data import (
    EVENT_PROPS,
    PROFILE_SPECS,
    PROTOCOL_NAMESPACE_SPECS,
)
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY


def _load_schema_split(split_dir: Path) -> tuple[set[str], set[str], set[str], set[str]]:
    commands: set[str] = set()
    for path in [split_dir / "global_commands.json", *sorted(split_dir.glob("namespace_*.json"))]:
        if not path.exists():
            continue
        for entry in json.loads(path.read_text(encoding="utf-8")):
            name = entry.get("commandName")
            if isinstance(name, str) and name:
                commands.add(name)

    events_json = json.loads((split_dir / "events.json").read_text(encoding="utf-8"))
    events: set[str] = set()
    profiles: set[str] = set()
    for entry in events_json:
        event_name = entry.get("eventName")
        if isinstance(event_name, str) and event_name:
            events.add(event_name)
        for profile in entry.get("profiles:", []) or ():
            if isinstance(profile, str) and profile:
                profiles.add(profile)

    namespaces = set(json.loads((split_dir / "namespaces.json").read_text(encoding="utf-8")))
    return commands, events, profiles, namespaces


def _event_scoped_commands_with_no_legal_events() -> list[str]:
    legality = REGISTRY.command_legality("f5-irules")
    out: list[str] = []
    for name in REGISTRY.command_names("f5-irules"):
        spec = REGISTRY.get(name, "f5-irules")
        if spec is None:
            continue
        if spec.event_requires is None and not spec.excluded_events:
            continue
        if not legality.events_for_command(name):
            out.append(name)
    return sorted(out)


def _man_valid_during_mismatches(man_dir: Path) -> tuple[int, list[tuple[str, int]]]:
    section_re = re.compile(r'^\.SH\s+"?([^"]+)"?')
    clean_steps = (
        (re.compile(r"\\s[+-]?\d+"), ""),
        (re.compile(r"\\&"), ""),
        (re.compile(r"\\\*\([A-Za-z0-9]{2}"), ""),
        (re.compile(r"\\[-]"), "-"),
        (re.compile(r"\\"), ""),
    )
    non_events = frozenset(
        {
            "CLASS_FUNCTION",
            "GLOBAL_GTM",
            "INFORMATIONAL_COMMAND",
            "STRING_FUNCTION",
            "UTILITY_COMMAND",
        }
    )
    known_events = set(EVENT_PROPS)
    legality = REGISTRY.command_legality("f5-irules")
    parsed = 0
    mismatches: list[tuple[str, int]] = []

    for path in sorted(man_dir.glob("ltm_rule_command_*.3")):
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        sections: dict[str, list[str]] = {}
        current: str | None = None
        for line in lines:
            match = section_re.match(line.strip())
            if match:
                current = match.group(1).strip()
                sections[current] = []
                continue
            if current is not None:
                sections[current].append(line)

        command = next((name for name in sections if "::" in name), None)
        if command is None:
            continue
        if command not in REGISTRY.command_names("f5-irules"):
            continue
        valid_section = sections.get("VALID DURING")
        if not valid_section:
            continue
        parsed += 1

        raw_parts: list[str] = []
        for line in valid_section:
            text = line.strip()
            if not text or text.startswith(".") or text.startswith("'"):
                continue
            for regex, replacement in clean_steps:
                text = regex.sub(replacement, text)
            text = " ".join(text.split())
            if text:
                raw_parts.append(text)
        if not raw_parts:
            continue

        text = " ".join(raw_parts)
        excluded: set[str] = set()
        if " except " in text:
            text, _, tail = text.partition(" except ")
            excluded = {
                token.strip() for token in re.split(r"[,\s]+", tail.strip()) if token.strip()
            }

        tokens = [token.strip() for token in re.split(r"[,\s]+", text) if token.strip()]
        tokens = [token for token in tokens if token not in non_events]
        any_event = "ANY_EVENT" in tokens or "ANY_EVENTS" in tokens
        tokens = [token for token in tokens if token not in {"ANY_EVENT", "ANY_EVENTS"}]

        expected = {token for token in tokens if token in known_events}
        if any_event:
            expected = set(known_events)
        expected -= excluded
        if not expected:
            continue

        legal = set(legality.events_for_command(command))
        missing = expected - legal
        if missing:
            mismatches.append((command, len(missing)))

    mismatches.sort(key=lambda item: (-item[1], item[0]))
    return parsed, mismatches


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--extract-root",
        type=Path,
        required=True,
        help="Path to BIG-IP extract root (contains irule-schema-split and man-* dirs)",
    )
    parser.add_argument(
        "--top",
        type=int,
        default=10,
        help="Show top N VALID DURING mismatch commands",
    )
    args = parser.parse_args()

    extract_root = args.extract_root.expanduser().resolve()
    split_dir = extract_root / "irule-schema-split"
    man_dirs = sorted(extract_root.glob("man-*"))
    if not split_dir.exists():
        raise SystemExit(f"Missing schema split directory: {split_dir}")
    if not man_dirs:
        raise SystemExit(f"Missing man-* directory under: {extract_root}")
    man_dir = man_dirs[0]

    src_commands, src_events, src_profiles, src_namespaces = _load_schema_split(split_dir)

    reg_commands = set(REGISTRY.command_names("f5-irules"))
    reg_events = set(EVENT_PROPS)
    reg_profiles = set(PROFILE_SPECS)
    reg_namespaces = set(PROTOCOL_NAMESPACE_SPECS)
    reg_command_namespaces = {name.split("::", 1)[0] for name in reg_commands if "::" in name}

    print("Schema Split Coverage")
    print(f"- Source commands: {len(src_commands)}")
    print(f"- Registry commands: {len(reg_commands)}")
    print(f"- Missing source commands in registry: {len(src_commands - reg_commands)}")
    print(f"- Source events: {len(src_events)}")
    print(f"- Registry events: {len(reg_events)}")
    print(f"- Missing source events in registry: {len(src_events - reg_events)}")
    print(f"- Source namespaces: {len(src_namespaces)}")
    print(
        "- Missing source namespaces from command prefixes: "
        f"{len(src_namespaces - reg_command_namespaces)}"
    )
    print(
        "- Missing source namespaces from protocol namespace specs: "
        f"{len(src_namespaces - reg_namespaces)}"
    )
    unresolved_namespace_lookups = sorted(
        name for name in src_namespaces if NAMESPACE_REGISTRY.get_protocol_namespace(name) is None
    )
    print(
        "- Source namespaces unresolved by NamespaceRegistry lookup: "
        f"{len(unresolved_namespace_lookups)}"
    )
    missing_profiles = sorted(src_profiles - reg_profiles)
    print(f"- Missing source profile tokens from PROFILE_SPECS: {len(missing_profiles)}")
    if missing_profiles:
        print(f"  {missing_profiles}")

    zero_legal = _event_scoped_commands_with_no_legal_events()
    print("\nEvent Legality Sanity")
    print(f"- Event-scoped commands with zero legal events: {len(zero_legal)}")
    if zero_legal:
        print(f"  {zero_legal}")

    parsed, mismatches = _man_valid_during_mismatches(man_dir)
    print("\nMan VALID DURING Parity (event names only)")
    print(f"- Parsed command man pages with VALID DURING: {parsed}")
    print(f"- Commands missing one or more man-listed events: {len(mismatches)}")
    if mismatches:
        print("- Top mismatches:")
        for command, missing_count in mismatches[: args.top]:
            print(f"  {command}: {missing_count} missing events")


if __name__ == "__main__":
    main()
