"""BIG-IP document links — clickable references in ``.conf`` / ``.scf`` files.

Tcl files already get ``source`` / ``package require`` links via
:mod:`document_links`.  This module is the BIG-IP sibling: every
object reference inside an ``ltm rule`` body becomes a clickable
:class:`types.DocumentLink` pointing at the target object's stanza
header in the same workspace.

Driven by :func:`extract_irules_object_references` — the same
iRule scanner ``f5 grep`` and the cleanup linter use — so the
link set always matches the rest of the BIG-IP-aware tooling.

Property-value links (``pool /Common/p``, ``defaults-from
/Common/parent`` on the property line itself) are a follow-up:
the parser's :class:`FieldSlot` only carries byte offsets, so
turning a slot into an LSP range needs a per-document
:class:`SourceMap` plumbed through.  For now the iRule-body
coverage already addresses the highest-value case (clicking
through every reference inside a Tcl rule body).
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis.semantic_model import Range, SourcePosition
from core.bigip.irules_refs import extract_irules_object_references
from core.bigip.model import BigipConfig
from core.bigip.object_registry import resolve_kind_in_configs
from core.bigip.parser import parse_bigip_conf
from core.common.lsp import to_lsp_range


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

    return _links_from_irule_bodies(config, uri, configs)


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
