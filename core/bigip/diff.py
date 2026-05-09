"""Object-aware diff between two BIG-IP configurations.

Compares two parsed :class:`BigipConfig` instances by full-path,
classifies each object as ``added`` / ``removed`` / ``modified``, and
ignores ordering and whitespace inside object bodies.  Whole-property
changes inside modified objects are reported field by field.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from .model import BigipConfig

_INTERESTING_FIELDS = (
    "destination",
    "pool",
    "rules",
    "profiles",
    "persist",
    "snatpool",
    "source_address_translation",
    "members",
    "monitor",
    "load_balancing_mode",
    "address",
    "kind",
    "value_type",
    "records",
    "monitor_type",
    "persistence_type",
    "profile_type",
    "source",  # iRule body
)


@dataclass(frozen=True, slots=True)
class FieldChange:
    field: str
    before: str
    after: str


@dataclass(frozen=True, slots=True)
class ObjectChange:
    full_path: str
    kind: str  # "virtual" | "pool" | ...
    status: str  # "added" | "removed" | "modified"
    fields: tuple[FieldChange, ...] = ()


@dataclass(frozen=True, slots=True)
class DiffReport:
    added: tuple[ObjectChange, ...] = ()
    removed: tuple[ObjectChange, ...] = ()
    modified: tuple[ObjectChange, ...] = ()
    text_report: str = ""

    @property
    def has_changes(self) -> bool:
        return bool(self.added or self.removed or self.modified)


# (module, object_type) pairs that have a specialised inventory in BigipConfig
# and therefore should be skipped when iterating cfg.generic_objects.
_SPECIALISED_GENERIC_TYPES: frozenset[tuple[str, str]] = frozenset(
    {
        ("ltm", "virtual"),
        ("ltm", "pool"),
        ("ltm", "node"),
        ("ltm", "monitor"),
        ("ltm", "snatpool"),
        ("ltm", "persistence"),
        ("ltm", "rule"),
        ("ltm", "data-group"),
        ("ltm", "profile"),
    }
)


def _kind_inventories(cfg: BigipConfig) -> dict[str, dict[str, object]]:
    """Return a mapping of ``kind -> {full_path: object}`` for every kind."""
    generic: dict[str, object] = {
        key: obj
        for key, obj in cfg.generic_objects.items()
        if (obj.module, obj.object_type) not in _SPECIALISED_GENERIC_TYPES
    }
    return {
        "virtual": dict(cfg.virtual_servers),
        "pool": dict(cfg.pools),
        "node": dict(cfg.nodes),
        "profile": dict(cfg.profiles),
        "monitor": dict(cfg.monitors),
        "data-group": dict(cfg.data_groups),
        "snatpool": dict(cfg.snat_pools),
        "persistence": dict(cfg.persistence),
        "rule": dict(cfg.rules),
        "generic": generic,
    }


def _normalise_irule_source(source: str) -> str:
    """Strip comments and collapse whitespace for body comparison."""
    no_comments = re.sub(r"#[^\n]*", "", source)
    return re.sub(r"\s+", " ", no_comments).strip()


def _field_value(obj: object, field_name: str) -> str:
    if not hasattr(obj, field_name):
        return ""
    val = getattr(obj, field_name)
    if val is None:
        return ""
    if field_name == "source":
        return _normalise_irule_source(str(val))
    if isinstance(val, tuple | list):
        return ",".join(sorted(str(v) for v in val))
    return str(val)


def _diff_object(kind: str, full_path: str, before: object, after: object) -> ObjectChange | None:
    changes: list[FieldChange] = []
    for f in _INTERESTING_FIELDS:
        b = _field_value(before, f)
        a = _field_value(after, f)
        if b != a:
            changes.append(FieldChange(field=f, before=b, after=a))
    if not changes:
        return None
    return ObjectChange(
        full_path=full_path,
        kind=kind,
        status="modified",
        fields=tuple(changes),
    )


def compute_diff(before: BigipConfig, after: BigipConfig) -> DiffReport:
    added: list[ObjectChange] = []
    removed: list[ObjectChange] = []
    modified: list[ObjectChange] = []

    a_inv = _kind_inventories(before)
    b_inv = _kind_inventories(after)

    for kind in sorted(set(a_inv) | set(b_inv)):
        a_kind = a_inv.get(kind, {})
        b_kind = b_inv.get(kind, {})
        for path in sorted(b_kind.keys() - a_kind.keys()):
            added.append(ObjectChange(full_path=path, kind=kind, status="added"))
        for path in sorted(a_kind.keys() - b_kind.keys()):
            removed.append(ObjectChange(full_path=path, kind=kind, status="removed"))
        for path in sorted(a_kind.keys() & b_kind.keys()):
            change = _diff_object(kind, path, a_kind[path], b_kind[path])
            if change is not None:
                modified.append(change)

    return DiffReport(
        added=tuple(added),
        removed=tuple(removed),
        modified=tuple(modified),
        text_report=_format_text(added, removed, modified),
    )


def _format_text(
    added: list[ObjectChange], removed: list[ObjectChange], modified: list[ObjectChange]
) -> str:
    lines: list[str] = [
        f"diff: +{len(added)} added, -{len(removed)} removed, ~{len(modified)} modified",
    ]
    for change in added:
        lines.append(f"+ [{change.kind}] {change.full_path}")
    for change in removed:
        lines.append(f"- [{change.kind}] {change.full_path}")
    for change in modified:
        lines.append(f"~ [{change.kind}] {change.full_path}")
        for fc in change.fields:
            lines.append(f"    {fc.field}: {fc.before!r} -> {fc.after!r}")
    return "\n".join(lines)


def report_to_dict(report: DiffReport) -> dict:
    def _ser(change: ObjectChange) -> dict:
        out: dict[str, object] = {
            "fullPath": change.full_path,
            "kind": change.kind,
            "status": change.status,
        }
        if change.fields:
            out["fields"] = [
                {"field": fc.field, "before": fc.before, "after": fc.after} for fc in change.fields
            ]
        return out

    return {
        "added": [_ser(c) for c in report.added],
        "removed": [_ser(c) for c in report.removed],
        "modified": [_ser(c) for c in report.modified],
        "summary": {
            "added": len(report.added),
            "removed": len(report.removed),
            "modified": len(report.modified),
        },
    }
