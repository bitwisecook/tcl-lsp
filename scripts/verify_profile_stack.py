#!/usr/bin/env python3
"""Verify the profile/protocol stack model against the BIG-IP schema.

Checks:
  1. All schema profiles are in PROFILE_SPECS
  2. Event implied_profiles contain all schema profiles (profile stacking)
  3. Schema categories match event transport/side properties
  4. PROTOCOL_NAMESPACE_SPECS coverage
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from core.commands.registry import REGISTRY
from core.commands.registry.namespace_data import (
    EVENT_PROPS,
    FLOW_CHAINS,
    MODIFICATION_SPECS,
    PROFILE_SPECS,
    PROTOCOL_NAMESPACE_SPECS,
    event_satisfies,
)
from core.commands.registry.runtime import configure_signatures

SCHEMA_DIR = Path.home() / "src" / "bigip-extract" / "irule-schema-split"


def load_schema_events() -> list[dict]:
    with open(SCHEMA_DIR / "events.json") as f:
        return json.load(f)


def load_schema_namespace_commands(ns: str) -> list[dict]:
    path = SCHEMA_DIR / f"namespace_{ns}.json"
    if not path.exists():
        return []
    with open(path) as f:
        return json.load(f)


def main():
    configure_signatures(dialect="f5-irules")
    schema_events = load_schema_events()

    # Build schema event -> profiles/categories maps
    schema_by_name: dict[str, dict] = {}
    for e in schema_events:
        schema_by_name[e["eventName"]] = e

    # ---- 1. PROFILE_SPECS COVERAGE ----
    print("=" * 70)
    print("SECTION 1: PROFILE_SPECS COVERAGE")
    print("=" * 70)

    # Collect all unique profiles from schema
    schema_profiles: set[str] = set()
    for e in schema_events:
        for p in e.get("profiles:", []):
            schema_profiles.add(p)

    registry_profiles = set(PROFILE_SPECS.keys())

    # Known name mappings (schema name -> registry name)
    PROFILE_NAME_MAP = {
        "IPS": "PROTOCOL_INSPECTION",
        "MSSQL": "TDS",
        "RADIUS_AAA": "RADIUS",
        "DIAMETERSESSION": "DIAMETER",
        "DIAMETER_ENDPOINT": "DIAMETER",
        "SIPSESSION": "SIP",
        "SIPROUTER": "SIP",
        "PERSIST": "SSL_PERSISTENCE",
        "HTTP_PROXY_CONNECT": "HTTP",
    }

    missing_profiles = set()
    for p in sorted(schema_profiles):
        mapped = PROFILE_NAME_MAP.get(p, p)
        if mapped not in registry_profiles:
            missing_profiles.add(p)

    if missing_profiles:
        print(f"\nSchema profiles NOT in PROFILE_SPECS ({len(missing_profiles)}):")
        for p in sorted(missing_profiles):
            # Find events that use this profile
            evts = [e["eventName"] for e in schema_events if p in e.get("profiles:", [])]
            print(f"  {p}: used by {', '.join(evts[:5])}")
    else:
        print("\n  All schema profiles are in PROFILE_SPECS (with name mappings).")

    # ---- 2. EVENT PROFILE STACKING ----
    print("\n" + "=" * 70)
    print("SECTION 2: EVENT PROFILE STACKING")
    print("=" * 70)

    mismatches = []
    for event_name in sorted(set(schema_by_name.keys()) & set(EVENT_PROPS.keys())):
        schema_entry = schema_by_name[event_name]
        props = EVENT_PROPS[event_name]

        schema_profs = set(schema_entry.get("profiles:", []))
        if not schema_profs:
            continue  # No profile requirement in schema

        registry_profs = set(props.implied_profiles)

        # Map schema profile names to registry names
        mapped_schema = set()
        for p in schema_profs:
            mapped = PROFILE_NAME_MAP.get(p, p)
            mapped_schema.add(mapped)

        # Check if all mapped schema profiles are in registry
        missing = mapped_schema - registry_profs
        if missing:
            # Check if ANY of the schema profiles are present
            present = mapped_schema & registry_profs
            mismatches.append((event_name, schema_profs, registry_profs, missing, present))

    if mismatches:
        print(f"\nEvents with missing implied profiles ({len(mismatches)}):")
        for event_name, schema_profs, registry_profs, missing, present in mismatches:
            print(f"\n  {event_name}:")
            print(f"    schema profiles: {sorted(schema_profs)}")
            print(f"    registry:        {sorted(registry_profs)}")
            print(f"    MISSING:         {sorted(missing)}")
    else:
        print("\n  All event profiles match schema (with name mappings).")

    # ---- 3. SCHEMA CATEGORIES vs EVENT PROPERTIES ----
    print("\n" + "=" * 70)
    print("SECTION 3: SCHEMA CATEGORIES vs EVENT PROPERTIES")
    print("=" * 70)

    cat_mismatches = []
    for event_name in sorted(set(schema_by_name.keys()) & set(EVENT_PROPS.keys())):
        schema_entry = schema_by_name[event_name]
        props = EVENT_PROPS[event_name]
        cats = set(schema_entry.get("categories:", []))
        issues = []

        # Normalize transport to a set for comparison
        t = props.transport
        t_set = set(t) if isinstance(t, tuple) else ({t} if t else set())

        # TCP category -> transport should include tcp
        if "TCP" in cats and "tcp" not in t_set:
            issues.append(f"schema has TCP category but transport={props.transport}")

        # UDP category -> transport should include udp
        if "UDP" in cats and "udp" not in t_set:
            issues.append(f"schema has UDP category but transport={props.transport}")

        # SCTP category
        if "SCTP" in cats and not (t_set & {"tcp", "sctp"} or not t_set):
            issues.append(f"schema has SCTP category but transport={props.transport}")

        # CLIENTSIDE category -> client_side should be True
        if "CLIENTSIDE" in cats and not props.client_side:
            issues.append(f"schema has CLIENTSIDE but client_side=False")

        # SERVERSIDE category -> server_side should be True
        if "SERVERSIDE" in cats and not props.server_side:
            issues.append(f"schema has SERVERSIDE but server_side=False")

        if issues:
            cat_mismatches.append((event_name, issues))

    if cat_mismatches:
        print(f"\nCategory/property mismatches ({len(cat_mismatches)}):")
        for event_name, issues in cat_mismatches:
            for issue in issues:
                print(f"  {event_name}: {issue}")
    else:
        print("\n  All category/property mappings are correct.")

    # ---- 4. PROTOCOL_NAMESPACE_SPECS COVERAGE ----
    print("\n" + "=" * 70)
    print("SECTION 4: PROTOCOL_NAMESPACE_SPECS COVERAGE")
    print("=" * 70)

    # Collect all namespaces from schema
    ns_json = json.load(open(SCHEMA_DIR / "namespaces.json"))
    schema_namespaces = set(ns_json)

    pns_namespaces = set(PROTOCOL_NAMESPACE_SPECS.keys())

    missing_pns = sorted(schema_namespaces - pns_namespaces)
    if missing_pns:
        print(f"\nSchema namespaces NOT in PROTOCOL_NAMESPACE_SPECS ({len(missing_pns)}):")
        for ns in missing_pns:
            cmds = load_schema_namespace_commands(ns)
            # Check if any of these commands have event_requires with profiles
            print(f"  {ns} ({len(cmds)} cmds)")
    else:
        print("\n  All schema namespaces are in PROTOCOL_NAMESPACE_SPECS.")

    # ---- 5. PROFILE LAYER ASSIGNMENTS ----
    print("\n" + "=" * 70)
    print("SECTION 5: PROFILE LAYER/REQUIRES VALIDATION")
    print("=" * 70)

    # Check that profile dependencies are correct
    # E.g., ASM requires HTTP, so ASM events should include HTTP in profiles
    issues = []
    for pname, pspec in sorted(PROFILE_SPECS.items()):
        if pspec.requires:
            # Check that events with this profile also include the required profiles
            for event_name, props in EVENT_PROPS.items():
                if pname in props.implied_profiles:
                    for req in pspec.requires:
                        if req not in props.implied_profiles:
                            issues.append(
                                f"  {event_name}: has profile {pname} which requires {req}, "
                                f"but {req} not in implied_profiles {props.implied_profiles}"
                            )

    if issues:
        print(f"\nProfile dependency violations ({len(issues)}):")
        for issue in issues:
            print(issue)
    else:
        print("\n  All profile dependencies satisfied in event definitions.")

    # ---- 6. COMMAND-EVENT CROSS CHECK ----
    print("\n" + "=" * 70)
    print("SECTION 6: COMMAND-EVENT CROSS CHECK (schema examples)")
    print("=" * 70)

    # Parse schema command examples to extract which events they're used in
    # Then verify those commands are legal in those events
    import re
    legality = REGISTRY.command_legality("f5-irules")

    violations = []
    checked = 0
    for ns in schema_namespaces:
        for cmd_entry in load_schema_namespace_commands(ns):
            cmd_name = cmd_entry["commandName"]
            examples = cmd_entry.get("examples", "")
            if not examples:
                continue

            # Extract events from "when EVENT_NAME {" patterns
            example_events = re.findall(r'when\s+(\w+)\s*\{', examples)
            for event in set(example_events):
                if event not in EVENT_PROPS:
                    continue
                spec = REGISTRY.get(cmd_name, "f5-irules")
                if spec is None:
                    continue
                checked += 1
                if not legality.is_legal(event, cmd_name):
                    violations.append((cmd_name, event))

    # Also check global commands
    global_cmds = json.load(open(SCHEMA_DIR / "global_commands.json"))
    for cmd_entry in global_cmds:
        cmd_name = cmd_entry["commandName"]
        examples = cmd_entry.get("examples", "")
        if not examples:
            continue
        example_events = re.findall(r'when\s+(\w+)\s*\{', examples)
        for event in set(example_events):
            if event not in EVENT_PROPS:
                continue
            spec = REGISTRY.get(cmd_name, "f5-irules")
            if spec is None:
                continue
            checked += 1
            if not legality.is_legal(event, cmd_name):
                violations.append((cmd_name, event))

    print(f"\n  Checked {checked} command-in-event pairs from schema examples.")
    if violations:
        print(f"\n  Commands flagged illegal but used in schema examples ({len(violations)}):")
        for cmd, event in sorted(set(violations)):
            spec = REGISTRY.get(cmd, "f5-irules")
            er = spec.event_requires if spec else None
            print(f"    {cmd} in {event}  (requires: {er})")
    else:
        print("  All command-event pairs from schema examples are legal.")

    # ---- 7. FLOW CHAIN VALIDATION ----
    print("\n" + "=" * 70)
    print("SECTION 7: FLOW CHAIN VALIDATION")
    print("=" * 70)

    for chain in FLOW_CHAINS:
        chain_events = [step.event for step in chain.steps]
        missing_events = [e for e in chain_events if e not in EVENT_PROPS]
        if missing_events:
            print(f"  {chain.chain_id}: events not in EVENT_PROPS: {missing_events}")
        else:
            print(f"  {chain.chain_id}: OK ({len(chain_events)} events)")

    print()


if __name__ == "__main__":
    main()
