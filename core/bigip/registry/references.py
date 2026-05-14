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
from .value_specs import ListSpec, ParseContext, Reference, ReferenceContext


def references_via_spec(
    *,
    module: str,
    object_type: str,
    property_name: str,
    value: Any,
    owner_path: str = "",
    source_uri: str = "",
    base_offset: int = 0,
    source_text: str = "",
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

    Three input shapes are handled:

    - **Scalar typed value** (``Destination`` / ``MonitorExpression`` /
      ...): passed straight through to ``spec.value.references``.
    - **Raw string**: auto-coerced via ``spec.value.parse`` so a
      property still carrying its legacy ``str`` shape on the model
      flows into the spec uniformly.
    - **Tuple / list**: each element parses through the spec's
      ``ListSpec.item`` (when the spec is a list) and yields its own
      references.  Properties whose model still stores
      ``tuple[str, ...]`` (the legacy profile / persist / rules /
      vlans shape) migrate to ``ListSpec`` and get per-element refs
      without a model rewrite.
    """
    spec = pilot_property_spec_for(module, object_type, property_name)
    if spec is None:
        return None
    ctx = ReferenceContext(
        owner_kind=f"{module} {object_type}".strip(),
        owner_path=owner_path,
        source_uri=source_uri,
    )
    parse_ctx = ParseContext(
        module=module,
        object_type=object_type,
        object_path=owner_path,
        source_uri=source_uri,
        source_text=source_text,
        base_offset=base_offset,
    )
    if isinstance(spec.value, ListSpec):
        list_value = _materialise_bigip_list(
            spec.value, value, module, object_type, owner_path, parse_ctx
        )
        return tuple(spec.value.references(list_value, ctx))
    typed = _coerce_to_typed(value, spec.value, module, object_type, owner_path, parse_ctx)
    return tuple(spec.value.references(typed, ctx))


def _materialise_bigip_list(
    list_spec: ListSpec,
    value: Any,
    module: str,
    object_type: str,
    owner_path: str,
    parse_ctx: ParseContext | None = None,
) -> Any:
    """Coerce *value* into a :class:`BigipList` (or pass through one).

    - Pre-typed :class:`BigipList` values flow through unchanged.
    - Tuples / lists of typed item values wrap each element as a
      :class:`ListItem`; the ListSpec's ``references`` walker yields
      one Reference per item.
    - Raw strings parse through :meth:`ListSpec.parse`, which
      tokenises and feeds each token to the inner item spec.
    """
    from ..types import BigipList, ListItem

    if value is None or value == "":
        return BigipList(syntax=list_spec.syntax, raw="")
    if isinstance(value, BigipList):
        return value
    if isinstance(value, (tuple, list)):
        # Walk each element through the inner item spec so raw
        # strings stored on the legacy model parse into typed values
        # before reference dispatch.  Pre-typed items pass through
        # via _coerce_to_typed's no-op branch.
        items: list[ListItem] = []
        for v in value:
            if isinstance(v, ListItem):
                items.append(v)
                continue
            typed = _coerce_to_typed(v, list_spec.item, module, object_type, owner_path)
            items.append(ListItem(value=typed))
        return BigipList(items=tuple(items), syntax=list_spec.syntax)
    if isinstance(value, str):
        ctx_to_use = parse_ctx or ParseContext(
            module=module, object_type=object_type, object_path=owner_path
        )
        parsed = list_spec.parse(value, ctx_to_use)
        if isinstance(parsed.value, BigipList):
            return parsed.value
        return BigipList(syntax=list_spec.syntax, raw=value)
    return value


def _coerce_to_typed(
    value: Any,
    value_spec: Any,
    module: str,
    object_type: str,
    owner_path: str,
    parse_ctx: ParseContext | None = None,
) -> Any:
    """Run *value* through ``value_spec.parse`` when it's still raw text.

    The reference / edit / projection dispatch wants the structured
    typed value (``MonitorExpression`` / ``Destination`` / etc.) but
    a property still carrying its legacy ``str`` shape on the model
    arrives here as a string.  Parse first so the spec's methods see
    a uniform input.  Tuples / lists / pre-typed instances pass
    through unchanged.
    """
    if value is None or value == "":
        return value
    if isinstance(value, str):
        ctx_to_use = parse_ctx or ParseContext(
            module=module,
            object_type=object_type,
            object_path=owner_path,
        )
        parsed = value_spec.parse(value, ctx_to_use)
        return parsed.value
    return value


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
