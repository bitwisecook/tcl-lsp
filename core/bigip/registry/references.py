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
    if isinstance(spec.value, ListSpec):
        items = _materialise_list_items(spec.value, value)
        return _references_via_list_spec(spec.value, items, ctx, module, object_type, owner_path)
    typed = _coerce_to_typed(value, spec.value, module, object_type, owner_path)
    return tuple(spec.value.references(typed, ctx))


def _materialise_list_items(list_spec: ListSpec, value: Any) -> tuple:
    """Coerce *value* into the per-element view a ListSpec expects.

    Pre-typed tuples / lists pass through unchanged.  Raw strings —
    which the link-extract pass hands in when the source is a
    ``{ /Common/a /Common/b }`` style block — are tokenised through
    the parser helpers: space-separated list bodies via
    :func:`_parse_list_block`, keyed-block bodies via
    :func:`_parse_keyed_block_entries`.  The decision uses the inner
    spec's ``is_structured`` flag because keyed-block lists carry
    item bodies that need a structured parse (firewall rules, policy
    rules, cert-key-chain entries, ...) while plain list-of-refs
    lists just want the bare tokens.
    """
    if value is None or value == "":
        return ()
    if isinstance(value, (tuple, list)):
        return tuple(value)
    if not isinstance(value, str):
        return (value,)
    from ..parser._helpers import _parse_keyed_block_entries, _parse_list_block

    if list_spec.item is not None and list_spec.item.is_structured:
        entries = _parse_keyed_block_entries(value)
        return tuple(f"{name} {{ {body} }}" for name, body in entries)
    return tuple(_parse_list_block(value))


def _references_via_list_spec(
    list_spec: ListSpec,
    items: Any,
    ctx: ReferenceContext,
    module: str,
    object_type: str,
    owner_path: str,
) -> tuple[Reference, ...]:
    """Walk every element of a list-shaped value through the inner item
    spec and concatenate the resulting references."""
    if list_spec.item is None:
        return ()
    out: list[Reference] = []
    for item in items:
        typed = _coerce_to_typed(item, list_spec.item, module, object_type, owner_path)
        out.extend(list_spec.item.references(typed, ctx))
    return tuple(out)


def _coerce_to_typed(
    value: Any,
    value_spec: Any,
    module: str,
    object_type: str,
    owner_path: str,
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
        parsed = value_spec.parse(
            value,
            ParseContext(
                module=module,
                object_type=object_type,
                object_path=owner_path,
            ),
        )
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
