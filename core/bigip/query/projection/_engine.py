"""Lazy projection engine — turns BigipConfig into navigable Containers.

The engine is the only runtime code in the projection package;
:mod:`._data` is pure static dispatch configuration.
``root_container`` is the public entry point.
"""

from __future__ import annotations

import re
from typing import Any

from ...model import (
    BigipPolicyAction,
    BigipPolicyCondition,
    BigipPolicyRule,
    BigipPoolMember,
    BigipRule,
)
from ..errors import EvalError
from ..values import FieldSlot, ObjectRef, PathRef, Root
from ._classes import Container, FieldSpec
from ._data import (
    _KIND_FIELD_MAPS,
    _MODULE_KINDS,
    _OBJECT_KIND_ALIASES,
    LTM_KINDS,
    MODULE_KINDS,
)

# ---------------------------------------------------------------------------


def root_container(root: Root) -> Container:
    """Return the synthetic top-level container.

    Holds one child per known module (``ltm``, ``net``, …).  Each
    module container is lazy — its kind containers and per-object
    refs only materialise when navigated into.
    """
    return Container(kind="<root>", root=root, _entry_source="")


def _build_entries(container: Container) -> dict[str, Any]:
    root = container.root
    if container.kind == "<root>":
        return {
            module: Container(kind=module, root=root, _entry_source=module)
            for module in _MODULE_KINDS
        }

    # Module-level container (``.ltm``, ``.net``, …).
    if container.kind in _MODULE_KINDS:
        module = container.kind
        return {
            label: Container(
                kind=tmsh_kind,
                root=root,
                _entry_source=f"{module}.{label}",
            )
            for label, (_, tmsh_kind) in _MODULE_KINDS[module].items()
        }

    # Leaf kind container: project each object.
    label = container.kind
    if label in _OBJECT_KIND_ALIASES:
        cls_attr = _kind_to_attr(label)
        objects = getattr(root.config, cls_attr)
        entries: dict[str, Any] = {}
        for full_path, obj in objects.items():
            entries[full_path] = _build_object_ref(label, full_path, obj, root)
        return entries

    return {}


def _kind_to_attr(kind: str) -> str:
    for module in _MODULE_KINDS.values():
        for label, (attr, k) in module.items():
            if k == kind:
                return attr
    raise EvalError(f"unknown object kind: {kind!r}")


def _build_object_ref(
    kind: str,
    full_path: str,
    obj: object,
    root: Root,
) -> ObjectRef:
    # Key the cache by (kind, full_path).  BIG-IP allows different
    # object kinds to live under the same path string (a pool, a node,
    # and an iRule can all share ``/Common/shared``), so a single
    # ``full_path``-only key would let one kind's :class:`ObjectRef`
    # leak into a query that asks for another kind.
    cache_key = (kind, full_path)
    if cache_key in root._object_cache:
        return root._object_cache[cache_key]

    _, field_map = _KIND_FIELD_MAPS[kind]
    fields: dict[str, Any] = {}
    for tmsh_name, spec in field_map.items():
        value = _project_field(kind, obj, spec, root)
        fields[tmsh_name] = value
    field_slots = _collect_field_slots(kind, obj, field_map, root)

    stanza_slot = None
    rng = getattr(obj, "range", None)
    if rng is not None:
        # The dataclass range covers the brace block ``{ ... }``; extend
        # it backwards over the header so SCF stanza output includes
        # ``ltm virtual /Common/web_vs`` along with the body.
        header_start = _scan_back_to_line_start(root.source, rng.start.offset)
        stanza_slot = FieldSlot(
            source_uri=root.uri,
            start=header_start,
            end=rng.end.offset,
            raw_text=root.source[header_start : rng.end.offset],
        )

    ref = ObjectRef(
        kind=kind,
        full_path=full_path,
        fields=fields,
        field_slots=field_slots,
        stanza_slot=stanza_slot,
        config_uri=root.uri,
    )
    root._object_cache[cache_key] = ref
    return ref


def _project_field(
    kind: str,
    obj: object,
    spec: FieldSpec,
    root: Root,
) -> Any:
    # Synthesised fields first.
    if spec.attr == "__refs__":
        if kind == "ltm rule" and isinstance(obj, BigipRule):
            return _rule_refs_value(obj, root)
        return None

    if not hasattr(obj, spec.attr):
        return None
    raw = getattr(obj, spec.attr)

    if spec.ref_kind and spec.list_ref:
        # Tuple/list of full-path strings.
        return [PathRef(full_path=p, root=root, expected_kind=spec.ref_kind) for p in raw or ()]
    if spec.ref_kind:
        return PathRef(full_path=raw or "", root=root, expected_kind=spec.ref_kind)
    if kind == "ltm pool" and spec.attr == "members":
        return [_member_object_ref(m, root) for m in raw or ()]
    if kind == "ltm policy" and spec.attr == "rules":
        return [_policy_rule_object_ref(r, root) for r in raw or ()]
    if spec.typed:
        # Typed value (``Network`` / ``IPAddress`` / ``Destination`` / …)
        # — render to its canonical string spelling so DSL users keep
        # seeing strings.  ``None`` becomes ``""`` so falsey-truthiness
        # matches the prior string-field behaviour for empty values.
        return str(raw) if raw is not None else ""
    if isinstance(raw, tuple):
        return list(raw)
    return raw


def _member_object_ref(member: BigipPoolMember, root: Root) -> ObjectRef:
    # ``member.address`` is now a typed :class:`Address` or ``None``;
    # render it to the canonical text spelling for the DSL surface so
    # queries against ``.pool.members[].address`` keep seeing strings.
    field_slots: dict[str, FieldSlot] = {}
    if member.field_offsets:
        for key, (start, end) in member.field_offsets.items():
            field_slots[key] = FieldSlot(
                source_uri=root.uri,
                start=start,
                end=end,
                raw_text=root.source[start:end],
            )
    return ObjectRef(
        kind="ltm pool-member",
        full_path=member.name,
        fields={
            "name": member.name,
            "address": str(member.address) if member.address is not None else "",
            "port": member.port,
            "monitor": PathRef(full_path=member.monitor, root=root, expected_kind="ltm monitor")
            if member.monitor
            else "",
            "description": member.description,
            "state": member.state,
            "ratio": member.ratio,
            "priority-group": member.priority_group,
            "connection-limit": member.connection_limit,
            "rate-limit": member.rate_limit,
        },
        field_slots=field_slots,
        config_uri=root.uri,
    )


def _policy_condition_object_ref(cond: BigipPolicyCondition) -> ObjectRef:
    return ObjectRef(
        kind="ltm policy-condition",
        full_path=str(cond.index),
        fields={
            "index": cond.index,
            "operand": cond.operand,
            "selector": cond.selector,
            "operator": cond.operator,
            "values": list(cond.values),
            "name": cond.name,
            "negate": cond.negate,
            "case-insensitive": cond.case_insensitive,
            "event": cond.event,
        },
    )


def _policy_action_object_ref(action: BigipPolicyAction, root: Root) -> ObjectRef:
    return ObjectRef(
        kind="ltm policy-action",
        full_path=str(action.index),
        fields={
            "index": action.index,
            "target": action.target,
            "verb": action.verb,
            # ``pool`` is a PathRef into ``ltm pool`` — chaining
            # ``.pool.members`` from a policy action walks into the
            # target pool.
            "pool": PathRef(full_path=action.pool, root=root, expected_kind="ltm pool"),
            "location": action.location,
            "name": action.name,
            "value": action.value,
            "path": action.path,
            "query": action.query,
            "host": action.host,
            "event": action.event,
        },
    )


def _policy_rule_object_ref(rule: BigipPolicyRule, root: Root) -> ObjectRef:
    return ObjectRef(
        kind="ltm policy-rule",
        full_path=rule.name,
        fields={
            "name": rule.name,
            "ordinal": rule.ordinal,
            "conditions": [_policy_condition_object_ref(c) for c in rule.conditions],
            "actions": [_policy_action_object_ref(a, root) for a in rule.actions],
        },
    )


def _rule_refs_value(obj: BigipRule, root: Root) -> ObjectRef:
    """Build the synthesised ``.ltm.rule[].refs`` object.

    Each ref slot is a list of :class:`PathRef`s drawn from the same
    reference graph :mod:`core.bigip.grep` walks, so the query DSL and
    the grep verb always agree on what an iRule "uses".
    """
    pools, persists, data_groups = _extract_rule_refs(obj, root)
    return ObjectRef(
        kind="ltm rule-refs",
        full_path=obj.full_path,
        fields={
            "pools": [PathRef(p, root=root, expected_kind="ltm pool") for p in pools],
            "persists": [PathRef(p, root=root, expected_kind="ltm persistence") for p in persists],
            "data-groups": [
                PathRef(p, root=root, expected_kind="ltm data-group") for p in data_groups
            ],
        },
    )


def _extract_rule_refs(obj: BigipRule, root: Root) -> tuple[list[str], list[str], list[str]]:
    from ...grep import compute_grep

    report = compute_grep(
        sources={root.uri: root.source},
        configs={root.uri: root.config},
        pattern=obj.full_path,
        use_regex=False,
        use_cidr=False,
        direction="forward",
        max_depth=1,
        max_nodes=1024,
        include_body=False,
        recurse=True,
    )
    pools: set[str] = set()
    persists: set[str] = set()
    data_groups: set[str] = set()
    for node in report.related:
        if node.full_path == obj.full_path:
            continue
        if node.module == "ltm" and node.object_type == "pool":
            pools.add(node.full_path)
        elif node.module == "ltm" and node.object_type.startswith("persistence"):
            persists.add(node.full_path)
        elif node.object_type.startswith("data-group"):
            data_groups.add(node.full_path)
    return sorted(pools), sorted(persists), sorted(data_groups)


# ---------------------------------------------------------------------------
# Field-slot byte-range discovery
# ---------------------------------------------------------------------------


_KEY_LINE_RE = re.compile(
    r"^(?P<indent>[ \t]*)(?P<key>[A-Za-z0-9_./\-]+)[ \t]+(?P<value>[^\n{]+?)[ \t]*$",
    re.MULTILINE,
)


def _scan_back_to_line_start(source: str, offset: int) -> int:
    """Return the offset of the start of the line containing *offset*.

    Used to extend a stanza range backwards over its header so SCF
    output includes ``ltm virtual /Common/foo`` along with the body.
    """
    i = offset
    while i > 0 and source[i - 1] != "\n":
        i -= 1
    return i


def _collect_field_slots(
    kind: str,
    obj: object,
    field_map: dict[str, FieldSpec],
    root: Root,
) -> dict[str, FieldSlot]:
    """Locate each top-level property's value span inside its stanza.

    Returns a mapping of TMSH-spelt field name → :class:`FieldSlot` for
    every field that appears as a single-line property in the source.
    Fields with no slot (compound list/sub-block values, identity
    fields whose location is the header) are simply absent — the edit
    planner falls back to its own strategy for those.
    """
    rng = getattr(obj, "range", None)
    if rng is None:
        return {}
    body_start = rng.start.offset
    body_end = rng.end.offset
    body_text = root.source[body_start:body_end]
    slots: dict[str, FieldSlot] = {}
    for match in _KEY_LINE_RE.finditer(body_text):
        key = match.group("key")
        # Map TMSH key back to our field-map name.  Keys with spaces or
        # nested braces are not handled here (those are sub-blocks).
        if key not in field_map:
            continue
        value_text = match.group("value")
        value_start = match.start("value") + body_start
        value_end = value_start + len(value_text)
        slots[key] = FieldSlot(
            source_uri=root.uri,
            start=value_start,
            end=value_end,
            raw_text=value_text,
        )
    return slots


# Public API surface.  ``MODULE_KINDS`` / ``LTM_KINDS`` enumerate the
# navigable module/kind tree exposed to the DSL; ``root_container``
# materialises the top-level entry into projection state.  Container
# / FieldSpec are part of the public surface so consumers can build
# their own typed objects.  Everything else (per-kind field maps,
# per-kind projector helpers, internal dispatch tables) is
# implementation detail.
__all__ = [
    "Container",
    "FieldSpec",
    "LTM_KINDS",
    "MODULE_KINDS",
    "root_container",
]
