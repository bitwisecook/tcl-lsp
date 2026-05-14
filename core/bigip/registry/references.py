"""Phase 5 dispatch — references through :meth:`ValueSpec.references`.

Centralises the "what does this property point at?" question on the
registry so the query graph (``refs`` / ``referenced_by`` /
``references_to`` / rename preflights) and the LSP feature layer
(document links / definition / references / rename / semantic tokens)
share one source of truth instead of each rebuilding it via regex
seed-matching.

The dispatch is additive: a caller asks the registry whether a
property has been migrated and, when it has, consumes
``spec.value.references()`` to enumerate the outbound edges.
Unmigrated properties keep flowing through the existing
:func:`core.bigip.grep.compute_grep` path, so the migration is
incremental — Phase 6's compound specs (``MonitorExpressionSpec``,
``ProfileAttachmentSpec``, ...) will plug into this same dispatch as
they land.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import Any

from .pilot import pilot_property_spec_for
from .value_specs import Reference, ReferenceContext


def references_via_spec(
    *,
    module: str,
    object_type: str,
    property_name: str,
    value: Any,
    owner_path: str = "",
    source_uri: str = "",
) -> tuple[Reference, ...] | None:
    """Enumerate outbound references from a single property's value.

    Returns ``None`` when the property hasn't been migrated yet — the
    caller falls back to whatever it did before.  Returns a tuple
    of :class:`Reference` objects when the spec is registered,
    including the empty tuple for a migrated-but-empty value (so the
    caller can tell "no migration" apart from "migrated, no edges").

    The reference context carries the owning object's identity so
    cross-partition visibility checks downstream have the data they
    need without each caller threading it manually.
    """
    spec = pilot_property_spec_for(module, object_type, property_name)
    if spec is None:
        return None
    ctx = ReferenceContext(
        owner_kind=f"{module} {object_type}".strip(),
        owner_path=owner_path,
        source_uri=source_uri,
    )
    return tuple(spec.value.references(value, ctx))


def iter_object_references(
    *,
    module: str,
    object_type: str,
    properties: Iterable[tuple[str, Any]],
    owner_path: str = "",
    source_uri: str = "",
) -> Iterable[Reference]:
    """Walk every (property_name, value) pair on one object and yield
    every reference each migrated spec reports.

    Convenience layer over :func:`references_via_spec` for the case
    where a caller (the graph builder, an LSP link extractor) has the
    full property bag in hand.  Unmigrated properties are silently
    skipped — callers that need a complete picture should layer the
    legacy grep path on top.
    """
    for prop_name, value in properties:
        refs = references_via_spec(
            module=module,
            object_type=object_type,
            property_name=prop_name,
            value=value,
            owner_path=owner_path,
            source_uri=source_uri,
        )
        if refs is None:
            continue
        yield from refs


__all__ = [
    "iter_object_references",
    "references_via_spec",
]
