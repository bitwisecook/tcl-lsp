"""Lazy projection engine — turns BigipConfig into navigable Containers.

The engine is the only runtime code in the projection package;
:mod:`._data` is pure static dispatch configuration.
``root_container`` is the public entry point.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import fields, is_dataclass
from typing import Any

from ...model import (
    BigipPolicyAction,
    BigipPolicyCondition,
    BigipPolicyRule,
    BigipPoolMember,
    BigipRule,
)
from ..errors import EvalError
from ..values import FieldSlot, LazyField, ObjectRef, PathRef, Root
from ._classes import Container, FieldSpec
from ._data import (
    _KIND_FIELD_MAPS,
    _MODULE_KINDS,
    _OBJECT_KIND_ALIASES,
    LTM_KINDS,
    MODULE_KINDS,
)

# ---------------------------------------------------------------------------


def root_container(root: Root) -> "Container | object":
    """Return the synthetic top-level container or external JSON.

    For BIG-IP roots: a synthetic ``<root>`` container that holds
    one child per known module (``ltm``, ``net``, …).  Each
    module container is lazy — its kind containers and per-object
    refs only materialise when navigated into.

    For external-JSON roots (``Root.json_value`` set): returns the
    parsed JSON value directly so ``.`` / ``.foo.bar`` use native
    dict/list semantics — matching jq on plain JSON.
    """
    if root.is_json:
        return root.json_value
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
        objects = _read_model_attr(root.config, cls_attr)
        if not isinstance(objects, dict):
            return {}
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
        if spec.attr.startswith("__"):
            # Synthesised field — defer the computation until something
            # actually reads ``.refs`` etc.  See ``LazyField``: building
            # every rule's ``refs`` eagerly turned ``[.ltm.rule[]] |
            # count`` into a full-config grep per rule.
            fields[tmsh_name] = LazyField(
                # Default args bind kind/obj/spec/tmsh_name at thunk-
                # creation time so the loop variables aren't captured
                # by reference.
                lambda kind=kind, obj=obj, spec=spec, tmsh_name=tmsh_name: _project_field(
                    kind, obj, spec, root, tmsh_name=tmsh_name
                )
            )
        else:
            fields[tmsh_name] = _project_field(kind, obj, spec, root, tmsh_name=tmsh_name)
    field_slots = _collect_field_slots(kind, obj, field_map, root)

    stanza_slot = None
    rng = _read_model_attr(obj, "range", None)
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
    *,
    tmsh_name: str = "",
) -> Any:
    # Synthesised fields first.
    if spec.attr == "__refs__":
        if kind == "ltm rule" and isinstance(obj, BigipRule):
            return _rule_refs_value(obj, root)
        return None

    raw = _read_model_attr(obj, spec.attr)
    if raw is _MISSING:
        return None

    # Phase 2 of the registry rearchitecture: when this property has
    # been migrated to a :class:`PropertySpec` in the new shape, run
    # the projection through the spec's :meth:`ValueSpec.project`
    # method.  Unmigrated properties keep flowing through the
    # legacy branches below — the new shape is purely additive
    # during the migration.  ``DestinationSpec.project`` returns the
    # canonical string just like the legacy ``typed=True`` branch
    # does, so observable behaviour stays identical for the pilot.
    #
    # The pilot lookup keys by TMSH name (``"source-address-
    # translation"``, ``"profiles"``) which can differ from the
    # model attribute spelling (``"source_address_translation"``,
    # ``"profile_attachments"``).  Pass both: lookup uses
    # *tmsh_name* when supplied; the engine falls back to
    # *spec.attr* otherwise.  When the pilot has its own ``attr``
    # field different from the FieldSpec's, fetch from THAT model
    # attribute so the spec sees the typed value the parser wrote
    # there (rather than the legacy back-compat tuple stored under
    # the FieldSpec attr).
    lookup_name = tmsh_name or spec.attr
    pilot_raw, used_pilot_attr = _resolve_pilot_value(kind, lookup_name, obj, raw)
    pilot_value = _project_via_pilot_spec(kind, lookup_name, pilot_raw, root)
    if pilot_value is not _PILOT_MISS:
        return pilot_value

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


# Sentinel returned when the pilot lookup didn't find a migrated
# spec.  Using a sentinel (rather than ``None``) lets the legacy
# branches keep ``None`` as a valid projection value for a missing
# typed field — only the explicit miss falls through to the legacy
# dispatch.
_PILOT_MISS = object()


def _resolve_pilot_value(
    kind: str, tmsh_name: str, obj: object, fallback_raw: object
) -> tuple[object, str | None]:
    """Resolve the model value the pilot wants for *tmsh_name*.

    The pilot for one TMSH property may target a different model
    attribute than the FieldSpec (e.g. ``"profiles"`` →
    ``profile_attachments`` typed :class:`BigipList`).  This helper
    looks up the pilot for *tmsh_name*, prefers its ``attr`` when
    that attribute exists on *obj*, and falls back to the value the
    engine already fetched via the FieldSpec.  Returns ``(value,
    used_attr_name)`` so callers can record which model attribute
    they actually consumed.
    """
    if " " not in kind:
        return fallback_raw, None
    module, _, object_type = kind.partition(" ")
    from ...registry.pilot import pilot_property_spec_for

    spec = pilot_property_spec_for(module, object_type, tmsh_name)
    if spec is None or not spec.attr:
        return fallback_raw, None
    candidate = _read_model_attr(obj, spec.attr)
    if candidate is not _MISSING:
        if candidate is not None and candidate != "":
            return candidate, spec.attr
    return fallback_raw, None


def _project_via_pilot_spec(kind: str, attr: str, raw: object, root: Root) -> object:
    """Look up the migrated :class:`PropertySpec` for one property and run
    its :meth:`ValueSpec.project` if present; otherwise return the
    sentinel so the caller falls back to the legacy branches.

    *kind* is the TMSH module+type pair (``"ltm virtual"``); the
    pilot table is keyed by the tuple split into module and type
    independently so we mirror the registry's existing index.
    """
    if " " not in kind:
        return _PILOT_MISS
    module, _, object_type = kind.partition(" ")
    from ...registry.pilot import pilot_property_spec_for
    from ...registry.value_specs import ProjectionContext

    spec = pilot_property_spec_for(module, object_type, attr)
    if spec is None:
        return _PILOT_MISS
    if spec.project_via_legacy:
        # The migration owns parse / edit / references for this
        # property but wants the projection to stay on the legacy
        # ``FieldSpec`` branches — typically because the legacy
        # projection wraps the value in a ``PathRef`` whose
        # back-compat shape the value spec can't reproduce without
        # a ``Root`` reference.
        return _PILOT_MISS
    if raw is None:
        # The legacy ``typed=True`` branch returned ``""`` for
        # ``None`` typed values so falsey-truthiness stayed
        # consistent with empty strings; keep that here.
        return ""
    ctx = ProjectionContext(root_uri=root.uri, owner_kind=kind)
    return spec.value.project(raw, ctx)


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
    rng = _read_model_attr(obj, "range", None)
    if rng is None:
        return {}
    body_start = rng.start.offset
    body_end = rng.end.offset
    body_text = root.source[body_start:body_end]
    slots: dict[str, FieldSlot] = {}
    for key, value_start, value_end, value_text in _iter_top_level_scalar_slots(body_text):
        # Map TMSH key back to our field-map name.  Keys with spaces or
        # nested braces are not handled here (those are sub-blocks).
        if key not in field_map:
            continue
        slots[key] = FieldSlot(
            source_uri=root.uri,
            start=body_start + value_start,
            end=body_start + value_end,
            raw_text=value_text,
        )
    return slots


class _Missing:
    __slots__ = ()


_MISSING = _Missing()


def _read_model_attr(obj: object, name: str, default: Any = _MISSING) -> Any:
    """Read a declared dataclass field from a BIG-IP model object."""
    if not is_dataclass(obj) or isinstance(obj, type):
        return default
    for model_field in fields(obj):
        if model_field.name == name:
            return getattr(obj, name)
    return default


def _iter_top_level_scalar_slots(body: str) -> Iterable[tuple[str, int, int, str]]:
    """Yield ``(key, value_start, value_end, value_text)`` for scalar lines.

    The scanner is deliberately brace-depth aware.  It only reports
    top-level ``key value`` lines and skips sub-block bodies so a nested
    ``pool`` / ``rules`` / ``profiles`` key cannot masquerade as an
    editable property on the owning object.
    """
    target_depth = 1 if body.lstrip().startswith("{") else 0
    depth = 0
    line_start = 0
    for line in body.splitlines(keepends=True):
        line_end = line_start + len(line)
        stripped = line.strip()
        if stripped and not stripped.startswith("#") and depth == target_depth:
            parsed = _parse_scalar_slot_line(line, line_start)
            if parsed is not None:
                yield parsed
        depth = _brace_depth_after_line(line, depth)
        line_start = line_end


def _parse_scalar_slot_line(line: str, line_start: int) -> tuple[str, int, int, str] | None:
    """Parse one top-level scalar property line without regular expressions."""
    content = line[:-1] if line.endswith("\n") else line
    pos = 0
    while pos < len(content) and content[pos] in " \t":
        pos += 1
    if pos >= len(content) or content[pos] in "{}#":
        return None
    key_start = pos
    while pos < len(content) and content[pos] not in " \t{}":
        pos += 1
    key = content[key_start:pos]
    if not key:
        return None
    while pos < len(content) and content[pos] in " \t":
        pos += 1
    if pos >= len(content):
        return None
    value = content[pos:].rstrip(" \t")
    if not value or "{" in value or "}" in value:
        return None
    value_start = line_start + pos
    value_end = value_start + len(value)
    return key, value_start, value_end, value


def _brace_depth_after_line(line: str, depth: int) -> int:
    """Update brace depth for one line, ignoring quoted strings."""
    in_quote = False
    escaped = False
    for ch in line:
        if escaped:
            escaped = False
            continue
        if ch == "\\" and in_quote:
            escaped = True
            continue
        if ch == '"':
            in_quote = not in_quote
            continue
        if in_quote:
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth = max(0, depth - 1)
    return depth


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
