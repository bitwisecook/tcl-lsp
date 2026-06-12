"""Canonical, language-agnostic snapshots of the command and graph registries.

This module is the single source of truth for the *shape* of the
registries as seen from the front-ends.  It produces deterministic,
JSON-serialisable dictionaries describing:

- the full command registry per dialect (arities, switches, forms,
  subcommands, and every boolean trait property on ``CommandSpec``);
- the iRules event graph (per-event protocol properties, firing order,
  flow chains, and the commands valid in each event);
- the iRules / BIG-IP profile graph;
- the BIG-IP object graph (object kinds and their property reference
  edges).

The snapshots are consumed by:

- the ``tcl registry-dump`` and ``f5 registry-dump`` CLI verbs, which
  serialise them to ``--json``;
- ``scripts/codegen/registry_baselines.py``, which regenerates the
  golden fixtures under ``tests/baselines/registry/``;
- the front-end-behaviour contract tests under
  ``tests/registry_contract/``.

Because the snapshot is emitted *through the front-end CLIs*, a future
Rust front-end that re-implements the same verbs can be validated
against the very same golden fixtures: the contract is the JSON shape,
not any Python API.
"""

from __future__ import annotations

import dataclasses
import hashlib
from enum import Enum
from typing import Any

from compiler.registry import REGISTRY
from compiler.registry.info import lookup_command_info, lookup_event_info
from compiler.registry.signatures import Arity

# Dialects whose command registry we snapshot.  Ordered for stable output.
TCL_DIALECTS: tuple[str, ...] = ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0")
F5_DIALECTS: tuple[str, ...] = ("f5-irules", "f5-iapps")
ALL_DIALECTS: tuple[str, ...] = (*TCL_DIALECTS, *F5_DIALECTS)

# Non-boolean ``CommandSpec`` trait fields worth capturing in the
# per-command ``scalars`` block.  Boolean traits are captured
# automatically (see :func:`_command_bool_traits`) so new boolean flags
# need no edit here.
_SCALAR_TRAIT_FIELDS: tuple[str, ...] = (
    "body_kind",
    "body_arg_implicit_args",
    "assigns_variable_at",
    "format_string_type",
    "pattern_type",
    "required_package",
    "tcllib_package",
    "inferred_storage_type",
    "allow_unknown_subcommands",
)


def _jsonable(value: Any) -> Any:
    """Recursively convert *value* to a deterministic JSON-serialisable form.

    Sets and frozensets are sorted by their string form; dict keys are
    coerced to strings and sorted; dataclasses become field-ordered
    dicts; enums become their symbolic ``.name``.  Callables are dropped
    (they are never part of the wire shape).
    """
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, Enum):
        # Symbolic name is the stable, language-agnostic wire form — it is
        # independent of whether the enum uses ``auto()`` ints or strings,
        # so a Rust reimplementation can reproduce it without copying values.
        return value.name
    if isinstance(value, (set, frozenset)):
        return [_jsonable(v) for v in sorted(value, key=_sort_key)]
    if isinstance(value, (list, tuple)):
        return [_jsonable(v) for v in value]
    if isinstance(value, dict):
        return {
            str(k): _jsonable(v) for k, v in sorted(value.items(), key=lambda kv: _sort_key(kv[0]))
        }
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        out: dict[str, Any] = {}
        for f in dataclasses.fields(value):
            fv = getattr(value, f.name)
            if callable(fv) and not dataclasses.is_dataclass(fv):
                continue
            out[f.name] = _jsonable(fv)
        return out
    return str(value)


def _sort_key(value: Any) -> str:
    return value.value if isinstance(value, Enum) else str(value)


def digest_list(items: tuple[str, ...] | list[str]) -> str:
    """Stable content digest of a string list (order-insensitive).

    The two heaviest registry facts — the commands valid in each event,
    and the events each command is valid in — are full cross-products
    (thousands of entries).  Storing them verbatim in every fixture
    bloats the golden files.  Instead the fixtures carry a count plus
    this digest; the front-end-behaviour tests reconstruct the full list
    from the live ``command-info`` / ``event-info`` front-ends and check
    the digest matches.  So the contract still covers the exact content,
    not just the count.

    The list is sorted before hashing so the digest is independent of
    enumeration order (a Rust front-end need only emit the same *set*).
    """
    joined = "\n".join(sorted(items))
    return "sha256:" + hashlib.sha256(joined.encode("utf-8")).hexdigest()


def _arity_json(arity: Arity | None) -> dict[str, int | None] | None:
    """Serialise an :class:`Arity` as ``{"min", "max"}`` (``max`` null = unbounded)."""
    if arity is None:
        return None
    return {"min": arity.min, "max": None if arity.is_unlimited else arity.max}


def _command_bool_traits(spec: Any) -> dict[str, bool]:
    """Every boolean field on a ``CommandSpec`` — captures all traits automatically."""
    traits: dict[str, bool] = {}
    for f in dataclasses.fields(spec):
        v = getattr(spec, f.name)
        if isinstance(v, bool):
            traits[f.name] = v
    return dict(sorted(traits.items()))


def _command_scalars(spec: Any) -> dict[str, Any]:
    scalars: dict[str, Any] = {}
    for name in _SCALAR_TRAIT_FIELDS:
        if not hasattr(spec, name):
            continue
        scalars[name] = _jsonable(getattr(spec, name))
    scalars["deprecated_replacement"] = spec.deprecated_replacement_name
    return scalars


def _subcommands_json(spec: Any, dialect: str) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for name in REGISTRY.subcommand_names(spec.name, dialect):
        sub = spec.subcommands[name]
        out.append(
            {
                "name": name,
                "arity": _arity_json(sub.arity),
                "pure": sub.pure,
                "mutator": sub.mutator,
                "destructive": sub.destructive,
                "deprecated": sub.deprecated_replacement_name is not None,
                "deprecatedReplacement": sub.deprecated_replacement_name,
                "options": sorted(opt.name for opt in sub.options),
                "returnsPath": sub.returns_path,
            }
        )
    return out


def _forms_json(spec: Any) -> list[dict[str, Any]]:
    return [
        {
            "kind": form.kind.value,
            "synopsis": form.synopsis,
            "arity": _arity_json(form.arity),
            "options": sorted(opt.name for opt in form.options),
            "pure": form.pure,
            "mutator": form.mutator,
        }
        for form in spec.forms
    ]


def command_entry(name: str, dialect: str) -> dict[str, Any]:
    """Full structured snapshot of a single command in *dialect*."""
    spec = REGISTRY.get(name, dialect)
    if spec is None:  # pragma: no cover - caller iterates command_names()
        raise KeyError(f"{name!r} not in dialect {dialect!r}")
    validation = REGISTRY.validation(name, dialect)
    info = lookup_command_info(name, dialect=dialect)
    return {
        "name": name,
        "dialects": _jsonable(spec.dialects),
        "arity": _arity_json(validation.arity if validation is not None else None),
        "switches": list(spec.switch_names(dialect)),
        "subcommands": _subcommands_json(spec, dialect),
        "forms": _forms_json(spec),
        "excludedEvents": sorted(spec.excluded_events),
        "traits": _command_bool_traits(spec),
        "scalars": _command_scalars(spec),
        # The shape the ``tcl command-info --json`` front-end emits.  The
        # full ``validEvents`` list is content-addressed (see
        # :func:`digest_list`) so the fixture stays small; the behaviour
        # test reconstructs and verifies the full list from the live verb.
        "info": {
            "summary": info.summary,
            "synopsis": list(info.synopsis),
            "switches": list(info.switches),
            "validEventCount": len(info.valid_events),
            "validEventsDigest": digest_list(info.valid_events),
            "validInAnyEvent": info.valid_in_any_event,
        },
    }


def command_registry_snapshot(dialect: str) -> dict[str, Any]:
    """Snapshot of every command available in *dialect*."""
    names = REGISTRY.command_names(dialect)
    return {
        "dialect": dialect,
        "commandCount": len(names),
        "commands": [command_entry(name, dialect) for name in names],
    }


def command_registry_snapshots(dialects: tuple[str, ...] = ALL_DIALECTS) -> dict[str, Any]:
    return {
        "schema": "tcl-lsp/registry/commands/v1",
        "dialects": {dialect: command_registry_snapshot(dialect) for dialect in dialects},
    }


def event_graph_snapshot(dialect: str = "f5-irules") -> dict[str, Any]:
    """Snapshot of the iRules event graph: props, firing order, and valid commands."""
    from compiler.registry import namespace_data as nd
    from compiler.registry.namespace_registry import NAMESPACE_REGISTRY

    event_names = sorted(NAMESPACE_REGISTRY.all_event_names())
    events: list[dict[str, Any]] = []
    for name in event_names:
        info = lookup_event_info(name, dialect=dialect)
        props = nd.get_event_props(name)
        events.append(
            {
                "event": name,
                "props": _jsonable(props) if props is not None else None,
                "orderIndex": NAMESPACE_REGISTRY.event_index(name),
                "known": info.known,
                "deprecated": info.deprecated,
                "multiplicity": info.multiplicity,
                "side": info.side,
                "transport": info.transport,
                "impliedProfiles": list(info.implied_profiles),
                "validCommandCount": info.valid_command_count,
                "validCommandsDigest": digest_list(info.valid_commands),
            }
        )
    master_order = [
        {"event": event, "profiles": sorted(profiles)} for event, profiles in nd.MASTER_ORDER
    ]
    return {
        "schema": "tcl-lsp/registry/events/v1",
        "dialect": dialect,
        "eventCount": len(events),
        "events": events,
        "masterOrder": master_order,
        "flowChains": _jsonable(nd.FLOW_CHAINS),
    }


def profile_graph_snapshot() -> dict[str, Any]:
    """Snapshot of the protocol profile graph and protocol-namespace map."""
    from compiler.registry import namespace_data as nd

    return {
        "schema": "tcl-lsp/registry/profiles/v1",
        "profiles": _jsonable(nd.PROFILE_SPECS),
        "protocolNamespaces": _jsonable(nd.PROTOCOL_NAMESPACE_SPECS),
    }


def object_graph_snapshot() -> dict[str, Any]:
    """Snapshot of the BIG-IP object graph: object kinds and reference edges."""
    from dialects.f5.bigip.registry.data import (
        HEADER_KIND_MAP,
        OBJECT_KIND_SPECS,
        PROPERTY_REFERENCE_SPECS,
    )

    kinds = [_jsonable(spec) for _, spec in sorted(OBJECT_KIND_SPECS.items())]
    header_map = [
        {"module": module, "objectType": object_type, "kind": kind}
        for (module, object_type), kind in sorted(HEADER_KIND_MAP.items())
    ]
    properties: list[dict[str, Any]] = []
    for (module, object_type), section_map in sorted(PROPERTY_REFERENCE_SPECS.items()):
        for section, specs in sorted(section_map.items()):
            for spec in specs:
                properties.append(
                    {
                        "module": module,
                        "objectType": object_type,
                        "section": section,
                        "property": spec.name,
                        "valueType": spec.value_type,
                        "required": spec.required,
                        "references": sorted(spec.references),
                        "enumValues": sorted(spec.enum_values),
                        "usageFlags": sorted(spec.usage_flags),
                    }
                )
    return {
        "schema": "tcl-lsp/registry/objects/v1",
        "objectKindCount": len(kinds),
        "objectKinds": kinds,
        "headerKindMap": header_map,
        "propertyReferences": properties,
    }


# Maps a logical snapshot section to its builder.  Used by the CLI verbs
# and the baseline generator so both stay in lockstep.
def all_snapshots() -> dict[str, Any]:
    """Every snapshot section, keyed by fixture name."""
    snapshots: dict[str, Any] = {}
    for dialect in ALL_DIALECTS:
        snapshots[f"commands-{dialect}"] = command_registry_snapshot(dialect)
    snapshots["events"] = event_graph_snapshot()
    snapshots["profiles"] = profile_graph_snapshot()
    snapshots["objects"] = object_graph_snapshot()
    return snapshots
