#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Inspect and materialise F5 oracle configurations without appliance access."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import ipaddress
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from functools import lru_cache

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "scripts/dev"))
from oracle_store import catalogue, retained_file

PLACEHOLDER = ipaddress.IPv4Network("192.168.1.0/24")
# These paths are interpreted by the F5 query engine, not a second SCF parser.
ADDRESS_FIELDS = (
    ".ltm.virtual[].destination",
    ".ltm.node[].address",
    ".ltm.pool[].members[].address",
)


def f5(arguments, accepted=(0,)):
    result = subprocess.run(["f5", *arguments], capture_output=True, check=False)
    if result.returncode not in accepted:
        raise ValueError(result.stderr.decode(errors="replace"))
    return result.stdout


def network(value):
    subnet = ipaddress.ip_network(value, strict=True)
    if subnet.version != 4 or subnet.prefixlen != 24:
        raise ValueError("the replay network must be an aligned IPv4 /24")
    return subnet


def host(value):
    if re.fullmatch(r"\.?[0-9]{1,3}", value):
        octet = int(value.lstrip("."))
        if 1 <= octet <= 254:
            return ipaddress.IPv4Address(int(PLACEHOLDER.network_address) + octet)
    address = ipaddress.IPv4Address(value)
    if address not in PLACEHOLDER or address in (
        PLACEHOLDER.network_address,
        PLACEHOLDER.broadcast_address,
    ):
        raise ValueError("source hosts must be usable addresses in 192.168.1.0/24")
    return address


def mapping(addresses, subnet, overrides):
    explicit = {}
    for override in overrides:
        source, target = override.split("=", 1)
        source = host(source)
        if re.fullmatch(r"\.?[0-9]{1,3}", target):
            target = ipaddress.IPv4Address(
                int(subnet.network_address) + int(target.lstrip("."))
            )
        else:
            target = ipaddress.IPv4Address(target)
        if target not in subnet or target in (
            subnet.network_address,
            subnet.broadcast_address,
        ):
            raise ValueError("mapped hosts must be usable addresses in the replay /24")
        if str(source) in explicit:
            raise ValueError(f"duplicate host mapping: {source}")
        explicit[str(source)] = str(target)
    absent = explicit.keys() - addresses
    if absent:
        raise ValueError(
            "mapped source address is absent: " + ", ".join(sorted(absent))
        )
    result = {}
    for value in sorted(addresses, key=ipaddress.IPv4Address):
        address = ipaddress.IPv4Address(value)
        if address in PLACEHOLDER:
            target = explicit.get(
                value,
                str(
                    ipaddress.IPv4Address(
                        int(subnet.network_address)
                        + int(address)
                        - int(PLACEHOLDER.network_address)
                    )
                ),
            )
        else:
            target = value
        result[value] = target
    if len(set(result.values())) != len(result):
        raise ValueError(
            "mapping aliases distinct source hosts; provide a non-conflicting mapping"
        )
    return result


def query_addresses(path):
    values = []
    rows = json.loads(f5(["query", ", ".join(ADDRESS_FIELDS), str(path), "--json"]))
    for row in rows:
        if isinstance(row, str):
            match = re.search(
                r"(?<![0-9.])(?:[0-9]{1,3}\.){3}[0-9]{1,3}(?![0-9.])", row
            )
            if match:
                values.append(str(ipaddress.IPv4Address(match[0])))
    return set(values)


def query_addresses_from_bytes(source):
    with tempfile.TemporaryDirectory(prefix="oracle-inventory-") as directory:
        path = Path(directory) / "input.scf"
        path.write_bytes(source)
        return query_addresses(path)


def remap(source, address_map):
    if all(source == target for source, target in address_map.items()):
        return source
    # Use the F5 address-map engine for one atomic transformation. Sequential
    # edits would incorrectly remap a host twice when users exchange addresses.
    with tempfile.TemporaryDirectory(prefix="oracle-map-") as directory:
        path = Path(directory) / "input.scf"
        path.write_bytes(source)
        inverse = {target: original for original, target in address_map.items()}
        map_path = Path(directory) / "inverse.toml"
        map_path.write_text(toml_map(inverse, PLACEHOLDER))
        return f5(["unredact", str(map_path), str(path)])


def toml_map(address_map, subnet):
    lines = [
        'schema = "f5-redact-map/v1"',
        'mode = "direct"',
        'seed = ""',
        "target_pool_v4 = [" + json.dumps(str(subnet)) + "]",
        "target_pool_v6 = []",
        "",
        "[ips]",
    ]
    lines.extend(
        json.dumps(source) + " = " + json.dumps(target)
        for source, target in address_map.items()
    )
    return "\n".join(lines) + "\n"


def configuration(group, case):
    if not group:
        if case:
            raise ValueError("--case requires --group")
        return None, None
    directory = HERE / "groups" / group
    descriptor = json.loads((directory / "group.json").read_text())
    if not case:
        return directory / "group.scf", (directory / "group.scf").read_bytes()
    recipes = json.loads(gzip.decompress((directory / "cases.json.gz").read_bytes()))
    recipe = recipes[case]
    baseline = (directory / descriptor["reconstruction_template"]).read_bytes()
    source = retained_file(ROOT, recipe["historical_configuration"]).read_bytes()
    # Source slices preserve exact historical bytes. Projection is a replay
    # transformation and never changes a historical receipt or its digest.
    target = normalise(source)
    rebuilt = b"".join(
        (baseline if part[0] == "group" else target)[part[1] : part[1] + part[2]]
        for part in recipe["delta"]
    )
    if hashlib.sha256(rebuilt).hexdigest() != recipe["replay_sha256"]:
        raise ValueError("replay delta does not reconstruct the selected configuration")
    return directory / "group.scf", rebuilt


@lru_cache(maxsize=64)
def normalise(source):
    with tempfile.TemporaryDirectory(prefix="oracle-normalise-") as directory:
        path = Path(directory) / "input.scf"
        path.write_bytes(source)
        addresses = query_addresses(path)
    projection = json.loads((HERE / "base/base.json").read_text())[
        "address_projection_by_sha256"
    ]
    address_map = {
        address: projection[hashlib.sha256(address.encode()).hexdigest()]
        for address in addresses
        if hashlib.sha256(address.encode()).hexdigest() in projection
    }
    undeclared = {
        address
        for address in addresses
        if ipaddress.IPv4Address(address) not in PLACEHOLDER
        and address not in address_map
        and not ipaddress.IPv4Address(address).is_loopback
        and not ipaddress.IPv4Address(address).is_unspecified
    }
    if undeclared:
        raise ValueError(
            "configuration contains undeclared endpoint projections; add verified bindings to base.json"
        )
    return remap(source, address_map) if address_map else source


def trigger_source(source, nonce):
    if not re.fullmatch(r"[0-9]+", nonce):
        raise ValueError("the comment nonce must contain decimal digits only")
    return source.rstrip(b"\n") + b"\n# oracle-trigger " + nonce.encode() + b"\n"


def trigger_plan(args):
    if not args.input or not args.rule or not args.output:
        raise ValueError("trigger requires --input, --rule and --output")
    if args.output.exists():
        raise ValueError("output directory already exists")
    if not re.fullmatch(r"/[A-Za-z0-9_./-]+", args.rule):
        raise ValueError("use an absolute, literal iRule object name")
    original = f5(
        [
            "query",
            ".ltm.rule[" + json.dumps(args.rule) + "].body",
            str(args.input),
            "--raw",
        ]
    )
    if not original.strip():
        raise ValueError("the selected rule has no source")
    if args.trigger_kind == "comment" and args.nonce is None:
        raise ValueError("a comment trigger requires --nonce")
    changed = (
        trigger_source(original, args.nonce) if args.trigger_kind == "comment" else None
    )
    args.output.mkdir(parents=True)
    result = {
        "kind": args.trigger_kind,
        "rule": args.rule,
        "input": str(args.input),
        "source_query": ".ltm.rule[" + json.dumps(args.rule) + "].body",
        "configuration_status": "local plan; not appliance-measured",
        "state_reset_proven": False,
        "required_receipts": [
            "appliance identity",
            "worker identifiers",
            "RULE_INIT invocation marker",
            "completion and result",
            "before/after state preconditions",
        ],
    }
    if args.trigger_kind == "comment":
        staging = getattr(args, "appliance_directory", "/var/tmp/f5-oracle").rstrip("/")
        output = b"ltm rule " + args.rule.encode() + b" {\n" + changed + b"}\n"
        (args.output / "trigger.scf").write_bytes(output)
        result.update(
            {
                "nonce": args.nonce,
                "commands": [
                    [
                        "tmsh",
                        "load",
                        "sys",
                        "config",
                        "merge",
                        "file=" + staging + "/trigger.scf",
                        "verify",
                    ],
                    [
                        "tmsh",
                        "load",
                        "sys",
                        "config",
                        "merge",
                        "file=" + staging + "/trigger.scf",
                    ],
                ],
                "scope": "Changes the selected iRule source. The comment is a new execution input and is visible to source introspection. Observe execution; do not infer variable or native catalogue reset.",
                "staging_directory": staging,
                "capture_correlation": "Use a fresh capture/run identity and before/after log cursors. Probe log markers can stay unchanged after a comment edit; do not mingle repeated invocations.",
            }
        )
    else:
        result.update(
            {
                "commands": [["tmsh", "load", "sys", "config"]],
                "scope": "Loads the appliance's saved configuration, including other rules. Unsaved probe changes may be replaced. Check saved/live configuration and intended rule identity before running; record observed RULE_INIT receipts.",
            }
        )
    (args.output / "trigger.json").write_text(json.dumps(result, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "action",
        choices=[
            "addresses",
            "cases",
            "materialise",
            "read",
            "verify",
            "trigger",
            "extract",
        ],
    )
    parser.add_argument("--group")
    parser.add_argument(
        "--all", action="store_true", help="list addresses from every replay case"
    )
    parser.add_argument("--case")
    parser.add_argument("--network", default=str(PLACEHOLDER))
    parser.add_argument("--map", action="append", default=[], metavar="SOURCE=TARGET")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--path", help="historical logical artefact path")
    parser.add_argument("--json-pointer")
    parser.add_argument("--input", type=Path)
    parser.add_argument("--rule")
    parser.add_argument("--nonce")
    parser.add_argument(
        "--trigger-kind", choices=["comment", "reload"], default="comment"
    )
    parser.add_argument(
        "--appliance-directory",
        default="/var/tmp/f5-oracle",
        help="owned appliance staging directory for generated commands",
    )
    args = parser.parse_args()
    if args.group and not re.fullmatch(r"[a-z0-9-]+", args.group):
        parser.error("group must be a group directory name")
    if args.action == "trigger":
        trigger_plan(args)
        return
    if args.action == "extract":
        if not args.path or not args.output:
            parser.error("extract requires --path PREFIX and --output NEW_DIRECTORY")
        prefix = Path(args.path)
        if prefix.is_absolute() or ".." in prefix.parts or not prefix.parts:
            parser.error("extract requires a repository-relative prefix")
        if args.output.exists():
            parser.error("extraction output already exists")
        names = [
            name
            for name in catalogue(ROOT)["files"]
            if name == args.path or name.startswith(args.path.rstrip("/") + "/")
        ]
        if not names:
            parser.error("no retained files match this prefix")
        args.output.mkdir(parents=True)
        for name in names:
            data = retained_file(ROOT, name).read_bytes()
            target = args.output / name
            if not target.resolve().is_relative_to(args.output.resolve()):
                raise ValueError("extraction path escapes output")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        print(f"Extracted {len(names)} verified logical files")
        return
    if args.action == "cases":
        if not args.group:
            parser.error("cases requires --group")
        values = json.loads(
            gzip.decompress(
                (HERE / "groups" / args.group / "cases.json.gz").read_bytes()
            )
        )
        print(
            json.dumps(
                [
                    {
                        "case": key,
                        "historical_configuration": row["historical_configuration"],
                        "addresses": row["addresses"],
                    }
                    for key, row in values.items()
                ],
                indent=2,
            )
        )
        return
    if args.action == "read":
        if not args.path:
            parser.error("read requires --path")
        data = retained_file(ROOT, args.path).read_bytes()
        if args.json_pointer is not None:
            value = json.loads(data)
            for part in (
                args.json_pointer.lstrip("/").split("/") if args.json_pointer else []
            ):
                part = part.replace("~1", "/").replace("~0", "~")
                value = value[int(part)] if isinstance(value, list) else value[part]
            data = json.dumps(value, ensure_ascii=True, indent=2).encode() + b"\n"
        sys.stdout.buffer.write(data)
        return
    if args.action == "verify":
        for name, row in catalogue(ROOT)["files"].items():
            if (
                hashlib.sha256(retained_file(ROOT, name).read_bytes()).hexdigest()
                != row["sha256"]
            ):
                raise ValueError(f"retained bytes changed: {name}")
        print("All retained logical artefacts verify")
        return
    subnet = network(args.network)
    before, selected = configuration(args.group, args.case)
    sources = [(HERE / "base/base.scf").read_bytes()]
    if selected is not None:
        sources.append(selected)
    with tempfile.TemporaryDirectory(prefix="oracle-addresses-") as directory:
        path = Path(directory) / "selected.scf"
        path.write_bytes(b"\n".join(sources))
        configured_addresses = query_addresses(path)
        addresses = set(configured_addresses)
    template_addresses = (
        query_addresses(HERE / "groups" / args.group / "template.scf")
        if args.group
        else set()
    )
    addresses.update(template_addresses)
    if args.all:
        for case_file in sorted((HERE / "groups").glob("*/cases.json.gz")):
            values = json.loads(gzip.decompress(case_file.read_bytes()))
            for row in values.values():
                configured_addresses.update(row["addresses"])
        addresses.update(configured_addresses)
    topology = json.loads((HERE / "base/base.json").read_text())
    roles = topology["addresses"]
    addresses.update(roles)
    address_map = mapping(addresses, subnet, args.map)
    rows = [
        {
            "source": source,
            "target": target,
            "role": roles.get(source),
            "configuration": source in configured_addresses,
            "template_configuration": source in template_addresses,
        }
        for source, target in address_map.items()
    ]
    if args.action == "addresses":
        print(json.dumps(rows, indent=2))
        return
    if not args.output:
        parser.error("materialise requires --output")
    if args.output.exists():
        raise ValueError("output directory already exists")
    args.output.mkdir(parents=True)
    (args.output / "base.scf").write_bytes(remap(sources[0], address_map))
    if before is not None:
        (args.output / "group.scf").write_bytes(remap(before.read_bytes(), address_map))
    if selected is not None:
        (args.output / "test.scf").write_bytes(remap(selected, address_map))
    if args.group:
        (args.output / "template.scf").write_bytes(
            remap(
                (HERE / "groups" / args.group / "template.scf").read_bytes(),
                address_map,
            )
        )
    (args.output / "addresses.json").write_text(json.dumps(rows, indent=2) + "\n")
    (args.output / "addresses.toml").write_text(toml_map(address_map, subnet))
    (args.output / "address-query.f5").write_text(", ".join(ADDRESS_FIELDS) + "\n")
    (args.output / "inverse-addresses.toml").write_text(
        toml_map(
            {target: source for source, target in address_map.items()}, PLACEHOLDER
        )
    )
    (args.output / "bindings.json").write_text(
        json.dumps(
            {role: address_map[address] for address, role in roles.items()}, indent=2
        )
        + "\n"
    )
    topology_inputs = [str(args.output / "base.scf")] + (
        [str(args.output / "test.scf")] if selected is not None else []
    )
    (args.output / "topology.json").write_text(
        json.dumps(
            {
                "external_roles": {
                    role: address_map[address] for address, role in roles.items()
                },
                "configured_objects": json.loads(
                    f5(
                        [
                            "query",
                            ".ltm.virtual[], .ltm.node[], .ltm.pool[]",
                            *topology_inputs,
                            "--merge",
                            "--json",
                        ]
                    )
                ),
                "scope": "Configured endpoint ports, route domains and pool relationships. External roles require the selected client/server programs and recorded lab prerequisites.",
            },
            indent=2,
        )
        + "\n"
    )
    if selected is not None:
        diff = f5(
            [
                "diff",
                str(args.output / "template.scf"),
                str(args.output / "test.scf"),
                "--json",
            ],
            accepted=(0, 1),
        )
        (args.output / "object-delta.json").write_bytes(diff)
    command_files = (
        ["base.scf"]
        + (["group.scf"] if before is not None else [])
        + (["test.scf"] if selected is not None else [])
    )
    (args.output / "commands.json").write_text(
        json.dumps(
            {
                "configuration_commands": [
                    {
                        "verify": [
                            "tmsh",
                            "load",
                            "sys",
                            "config",
                            "merge",
                            "file=" + args.appliance_directory.rstrip("/") + "/" + name,
                            "verify",
                        ],
                        "load": [
                            "tmsh",
                            "load",
                            "sys",
                            "config",
                            "merge",
                            "file=" + args.appliance_directory.rstrip("/") + "/" + name,
                        ],
                    }
                    for name in command_files
                ],
                "scope": "Local command plan only. Use the selected historical driver for ownership absence checks, invocation, stateful phase order and teardown. A load may immediately execute RULE_INIT; it is not passive setup.",
                "client_and_server_bindings": "bindings.json",
                "configuration_saved": False,
                "staging_directory": args.appliance_directory,
                "staging_precondition": "Check ownership and absence, then copy the generated SCFs to this appliance directory before executing the plan.",
                "group_template": "template.scf is an initial test programme, not passive setup. The object comparison shows the selected test's minimal object changes from this model. Follow its historical stateful driver before reusing an active model.",
                "address_inventory_command": [
                    "f5",
                    "query",
                    ", ".join(ADDRESS_FIELDS),
                    "test.scf" if selected is not None else "base.scf",
                    "--json",
                ],
                "address_rewrite_command": [
                    "f5",
                    "unredact",
                    "inverse-addresses.toml",
                    "PLACEHOLDER_INPUT.scf",
                ],
            },
            indent=2,
        )
        + "\n"
    )
    (args.output / "manifest.json").write_text(
        json.dumps(
            {
                "configuration_status": "reconstructed; not appliance-measured",
                "remapping_method": "atomic F5 address-map engine; known endpoint literals, including literal references in scripts; encoded test payloads unchanged",
                "f5_version": f5(["--version"]).decode().strip(),
                "group": args.group,
                "case": args.case,
                "network": str(subnet),
                "files": [p.name for p in sorted(args.output.iterdir())],
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        raise SystemExit(str(error)) from error
