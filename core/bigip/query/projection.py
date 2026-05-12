"""Project a :class:`BigipConfig` into the tree shape the query DSL navigates.

The DSL exposes the parsed config as a nested mapping::

    .ltm.virtual["/Common/web_vs"].pool   -> PathRef("/Common/web_pool")
    .ltm.pool["/Common/web_pool"].members -> list[MemberObjectRef]
    .ltm.rule["/Common/r1"].body          -> string

This module builds those projections lazily.  Walking through containers
allocates lightweight :class:`Container` objects on demand; fully
projected :class:`.values.ObjectRef` instances are cached on the root.

The TMSH-spelt keys exposed here are the *user-visible* names.  They
map back to the underlying dataclass attribute names through
:data:`_KIND_FIELD_MAPS` for every supported object kind.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any

from ..model import (
    BigipDataGroup,
    BigipMonitor,
    BigipNode,
    BigipPersistence,
    BigipPolicy,
    BigipPool,
    BigipPoolMember,
    BigipProfile,
    BigipRule,
    BigipSnatPool,
    BigipVirtualServer,
)
from .errors import EvalError
from .values import FieldSlot, ObjectRef, PathRef, Root

# ---------------------------------------------------------------------------
# Container abstraction
# ---------------------------------------------------------------------------


@dataclass
class Container:
    """A navigable mapping projected from a :class:`BigipConfig`.

    Containers are returned for the namespace nodes ``.ltm``, ``.gtm``,
    and for kind nodes like ``.ltm.virtual``.  Leaf entries inside a
    kind container are :class:`ObjectRef` instances; entries inside a
    namespace container are themselves :class:`Container` instances.

    The ``kind`` field carries either the module name (``"ltm"``) or
    the full TMSH module+type (``"ltm virtual"``).  Builtins and
    error messages use it to describe the level being navigated.
    """

    kind: str
    root: Root
    # Lazily filled.  Keys are the user-visible identifiers — full-paths
    # for object kinds, plain TMSH type names for module namespaces.
    _entries: dict[str, Any] | None = None
    _entry_source: str = ""  # "ltm.virtual", "ltm", ""

    def entries(self) -> dict[str, Any]:
        if self._entries is None:
            self._entries = _build_entries(self)
        return self._entries

    def lookup(self, key: str) -> Any:
        ents = self.entries()
        if key in ents:
            return ents[key]
        # Partition shorthand: bare name resolves to ``/Common/<name>``
        # when that key exists and is unambiguous.  Any other matching
        # full-path with a different partition makes the lookup
        # ambiguous and we raise rather than guess.
        if self._is_object_kind() and not key.startswith("/"):
            full = f"/Common/{key}"
            matches = [k for k in ents if k.endswith(f"/{key}")]
            if full in ents:
                return ents[full]
            if len(matches) == 1:
                return ents[matches[0]]
            if len(matches) > 1:
                raise EvalError(
                    f"{self.kind}: name {key!r} is ambiguous "
                    f"({len(matches)} matches; use a full path)"
                )
        raise EvalError(f"{self.kind}: no entry {key!r}")

    def regex_keys(self, pattern: str) -> list[str]:
        try:
            rx = re.compile(pattern)
        except re.error as exc:
            raise EvalError(f"invalid regex subscript {pattern!r}: {exc}") from exc
        return [k for k in self.entries() if rx.search(k)]

    def _is_object_kind(self) -> bool:
        # Object kinds are the leaf containers (``ltm virtual`` etc.);
        # the bare module containers ("ltm", "gtm") hold sub-containers.
        return " " in self.kind or self.kind in _OBJECT_KIND_ALIASES


# ---------------------------------------------------------------------------
# Field maps — TMSH-spelt user names mapping to dataclass attribute names.
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class FieldSpec:
    """How a single TMSH-spelt field projects from a dataclass."""

    attr: str  # dataclass attribute
    # ``ref_kind`` non-empty signals "this is a PathRef into <kind>".
    # ``list_ref`` flags list-of-PathRef fields.
    ref_kind: str = ""
    list_ref: bool = False


_VS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "destination": FieldSpec("destination"),
    "pool": FieldSpec("pool", ref_kind="ltm pool"),
    "rules": FieldSpec("rules", ref_kind="ltm rule", list_ref=True),
    "profiles": FieldSpec("profiles", ref_kind="ltm profile", list_ref=True),
    "persist": FieldSpec("persist", ref_kind="ltm persistence", list_ref=True),
    "policies": FieldSpec("policies", ref_kind="ltm policy", list_ref=True),
    "snatpool": FieldSpec("snatpool", ref_kind="ltm snatpool"),
    "source-address-translation": FieldSpec("source_address_translation"),
}

_POOL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "monitor": FieldSpec("monitor", ref_kind="ltm monitor"),
    "load-balancing-mode": FieldSpec("load_balancing_mode"),
    "members": FieldSpec("members"),  # special-cased: list of member objects
}

_NODE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "address": FieldSpec("address"),
}

_RULE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "body": FieldSpec("source"),
    "refs": FieldSpec("__refs__"),  # synthesised — see ``_rule_refs_value``
}

_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("profile_type"),
}

_MONITOR_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("monitor_type"),
}

_PERSIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("persistence_type"),
}

_SNATPOOL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "members": FieldSpec("members"),
}

_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "strategy": FieldSpec("strategy"),
    "controls": FieldSpec("controls"),
    "requires": FieldSpec("requires"),
}

_DATAGROUP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("value_type"),
    "kind": FieldSpec("kind"),
    "records": FieldSpec("records"),
}


_KIND_FIELD_MAPS: dict[str, tuple[type, dict[str, FieldSpec]]] = {
    "ltm virtual": (BigipVirtualServer, _VS_FIELDS),
    "ltm pool": (BigipPool, _POOL_FIELDS),
    "ltm node": (BigipNode, _NODE_FIELDS),
    "ltm rule": (BigipRule, _RULE_FIELDS),
    "ltm profile": (BigipProfile, _PROFILE_FIELDS),
    "ltm monitor": (BigipMonitor, _MONITOR_FIELDS),
    "ltm persistence": (BigipPersistence, _PERSIST_FIELDS),
    "ltm snatpool": (BigipSnatPool, _SNATPOOL_FIELDS),
    "ltm policy": (BigipPolicy, _POLICY_FIELDS),
    "ltm data-group": (BigipDataGroup, _DATAGROUP_FIELDS),
}

# Map from "container key" (the name users write between dots) to the
# underlying BigipConfig dictionary attribute and the TMSH kind.
_LTM_KINDS: dict[str, tuple[str, str]] = {
    "virtual": ("virtual_servers", "ltm virtual"),
    "pool": ("pools", "ltm pool"),
    "node": ("nodes", "ltm node"),
    "rule": ("rules", "ltm rule"),
    "profile": ("profiles", "ltm profile"),
    "monitor": ("monitors", "ltm monitor"),
    "persistence": ("persistence", "ltm persistence"),
    "snatpool": ("snat_pools", "ltm snatpool"),
    "policy": ("policies", "ltm policy"),
    "data-group": ("data_groups", "ltm data-group"),
}

_OBJECT_KIND_ALIASES = frozenset(kind for _, kind in _LTM_KINDS.values())


# Public alias so ``builtins.rename_partition`` and other consumers can
# iterate the kinds without reaching into a single-underscore name.
LTM_KINDS = _LTM_KINDS


# ---------------------------------------------------------------------------
# Building containers and object refs
# ---------------------------------------------------------------------------


def root_container(root: Root) -> Container:
    """Return the synthetic top-level container.

    Holds a single ``ltm`` (and, when populated, ``gtm``) child.
    """
    return Container(kind="<root>", root=root, _entry_source="")


def _build_entries(container: Container) -> dict[str, Any]:
    root = container.root
    if container.kind == "<root>":
        out: dict[str, Any] = {
            "ltm": Container(kind="ltm", root=root, _entry_source="ltm"),
        }
        return out

    if container.kind == "ltm":
        out = {}
        for label in _LTM_KINDS:
            out[label] = Container(
                kind=_LTM_KINDS[label][1],
                root=root,
                _entry_source=f"ltm.{label}",
            )
        return out

    # Leaf kind containers: project each object.
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
    for label, (attr, k) in _LTM_KINDS.items():
        if k == kind:
            return attr
    raise EvalError(f"unknown object kind: {kind!r}")


def _build_object_ref(
    kind: str,
    full_path: str,
    obj: object,
    root: Root,
) -> ObjectRef:
    if full_path in root._object_cache:
        return root._object_cache[full_path]

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
    root._object_cache[full_path] = ref
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
        return [_member_object_ref(m) for m in raw or ()]
    if isinstance(raw, tuple):
        return list(raw)
    return raw


def _member_object_ref(member: BigipPoolMember) -> ObjectRef:
    return ObjectRef(
        kind="ltm pool-member",
        full_path=member.name,
        fields={
            "name": member.name,
            "address": member.address,
            "port": member.port,
            "monitor": member.monitor,
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
    from ..grep import compute_grep

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
