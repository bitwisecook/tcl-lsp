"""BIG-IP document links — clickable references in ``.conf`` / ``.scf`` files.

Tcl files already get ``source`` / ``package require`` links via
:mod:`document_links`.  This module is the BIG-IP sibling: every
object reference inside an ``ltm rule`` body, every TMSH property
value that resolves to another object (``pool /Common/p``,
``defaults-from /Common/parent``, ``profiles { /Common/clientssl
{ ... } }``), and every nested reference inside a structured value
(firewall rule destination address-lists, cert-key-chain cert
refs, ...) becomes a clickable :class:`types.DocumentLink` pointing
at the target object's stanza in the same workspace.

Driven by two engines:

- :func:`extract_irules_object_references` — the iRule scanner
  ``f5 grep`` and the cleanup linter share — covers Tcl-body
  references inside an ``ltm rule``.
- :func:`core.bigip.registry.references_via_spec` — the registry's
  value-spec dispatch — covers property-value references on every
  migrated property, with byte-accurate :class:`SourceRange`
  spans so each link points at the exact reference token (not the
  surrounding property line).
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis.semantic_model import Range, SourcePosition
from core.bigip.irules_refs import extract_irules_object_references
from core.bigip.model import BigipConfig
from core.bigip.object_registry import resolve_kind_in_configs
from core.bigip.parser import parse_bigip_conf
from core.bigip.parser._helpers import (
    _extract_blocks,
    _parse_generic_header,
    _parse_properties_with_spans,
)
from core.bigip.registry import (
    candidate_registry_kinds_for_display,
    references_via_spec,
)
from core.bigip.registry.pilot import pilot_property_spec_for
from shared.document_buffer import DocumentBuffer
from shared.lsp import to_lsp_range


def get_bigip_document_links(
    source: str,
    *,
    uri: str,
    workspace_configs: dict[str, BigipConfig] | None = None,
) -> list[types.DocumentLink]:
    """Return document links for every object reference in *source*.

    *workspace_configs* maps URI → parsed :class:`BigipConfig` for
    every BIG-IP file the scanner has indexed; cross-file references
    resolve through it.  Pass an empty dict for single-file mode —
    only same-file references get a target then.
    """
    try:
        config = parse_bigip_conf(source)
    except Exception:  # noqa: BLE001
        return []

    configs = dict(workspace_configs or {})
    configs.setdefault(uri, config)

    out: list[types.DocumentLink] = []
    out.extend(_links_from_irule_bodies(config, uri, configs))
    out.extend(_links_from_registry_refs(source, uri, configs))
    return out


def _links_from_registry_refs(
    source: str,
    self_uri: str,
    configs: dict[str, BigipConfig],
) -> list[types.DocumentLink]:
    """Emit links from every migrated property's references.

    Walks the source's top-level stanzas, tokenises each into
    ``(key, value)`` properties, and asks the registry's pilot
    dispatch which references each carries.  Each yielded
    :class:`Reference` arrives with an absolute :class:`SourceRange`
    so the link points at the exact reference token — no per-line
    approximation."""
    out: list[types.DocumentLink] = []
    buffer = DocumentBuffer.from_source(source)
    for block in _extract_blocks(source):
        generic = _parse_generic_header(block.header)
        if generic is None:
            continue
        module, object_type, identifier = generic
        prop_map = _parse_properties_with_spans(block.body)
        # Absolute offset of the body's first byte in source.
        body_base = block.start_offset + 1
        for key, prop in prop_map.items():
            if pilot_property_spec_for(module, object_type, key) is None:
                continue
            if prop.value_start is None:
                continue
            base = body_base + prop.value_start
            refs = references_via_spec(
                module=module,
                object_type=object_type,
                property_name=key,
                value=prop.value,
                owner_path=identifier,
                source_uri=self_uri,
                base_offset=base,
                source_text=source,
            )
            for ref in refs or ():
                target = _resolve_via_kinds(ref.target_kind, ref.target_path, configs)
                if target is None:
                    continue
                if ref.range is None:
                    continue
                # ``ref.range`` is a half-open ``[start, end)`` span;
                # ``range_from_offsets`` takes an inclusive end so
                # subtract one byte at the boundary.  Otherwise the
                # link would extend one character past the reference
                # token and bleed into the trailing brace / space.
                rng = buffer.range_from_offsets(ref.range.start, ref.range.end - 1)
                out.append(
                    types.DocumentLink(
                        range=to_lsp_range(rng),
                        tooltip=f"Go to {ref.target_path}",
                        target=target,
                    )
                )
    return out


def _resolve_via_kinds(
    display_kind: str,
    target_path: str,
    configs: dict[str, BigipConfig],
) -> str | None:
    """Resolve *display_kind* + *target_path* into a ``uri#L<line>``.

    Display kinds (``"ltm pool"``, ``"ltm monitor"``,
    ``"security firewall address-list"``) fan out via
    :func:`candidate_registry_kinds_for_display` to one or more
    registry keys — the resolver tries each until one finds the
    object."""
    for kind in candidate_registry_kinds_for_display(display_kind):
        resolved = resolve_kind_in_configs(kind, target_path, configs, preferred_module="")
        if resolved is not None:
            target_uri, target_range = resolved
            return f"{target_uri}#L{target_range.start.line + 1}"
    return None


def _links_from_irule_bodies(
    config: BigipConfig,
    self_uri: str,
    configs: dict[str, BigipConfig],
) -> list[types.DocumentLink]:
    """Emit links for every object reference inside an ``ltm rule`` body."""
    out: list[types.DocumentLink] = []
    for rule in config.rules.values():
        rule_range = getattr(rule, "range", None)
        if rule_range is None:
            continue
        body_start_line = rule_range.start.line + 1
        for ref in extract_irules_object_references(rule.source):
            absolute_range = _offset_range(ref.range, body_start_line)
            target = _resolve_first_kind(ref.name, ref.kinds, configs)
            tooltip = f"Go to {ref.name}" if target else f"{ref.name} (no definition found)"
            out.append(
                types.DocumentLink(
                    range=to_lsp_range(absolute_range),
                    tooltip=tooltip,
                    target=target,
                )
            )
    return out


def _offset_range(rng: Range, line_offset: int) -> Range:
    """Shift *rng* by *line_offset* lines, leaving columns intact."""
    return Range(
        start=SourcePosition(
            line=rng.start.line + line_offset,
            character=rng.start.character,
            offset=rng.start.offset,
        ),
        end=SourcePosition(
            line=rng.end.line + line_offset,
            character=rng.end.character,
            offset=rng.end.offset,
        ),
    )


def _resolve_first_kind(
    name: str,
    kinds: tuple[str, ...],
    configs: dict[str, BigipConfig],
) -> str | None:
    """Return ``uri#L<line>`` for the first kind that resolves *name*."""
    for kind in kinds:
        resolved = resolve_kind_in_configs(kind, name, configs, preferred_module="")
        if resolved is not None:
            target_uri, target_range = resolved
            return f"{target_uri}#L{target_range.start.line + 1}"
    return None
