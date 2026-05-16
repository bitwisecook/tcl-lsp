"""BIG-IP document symbols — outline parsed objects for ``.conf`` / ``.scf`` files.

Tcl document symbols already cover ``ltm rule`` bodies via the
conf-wrapped path (``_conf_wrapped_symbols``).  This module is the
sibling for the *config* outline: every parsed BIG-IP object across
every kind becomes a ``DocumentSymbol`` so the editor's outline
view, breadcrumbs, and Cmd-O / Ctrl-Shift-O fuzzy navigator all
work on a 50K-line ``bigip.conf``.

The structure is module → kind → object, three levels deep:

    ltm
      virtual
        /Common/web_vs                  [Variable]
        /Common/api_vs                  [Variable]
      pool
        /Common/web_pool                [Variable]
      ...
    pem
      policy
        /Common/Default_Policy          [Variable]
    ...

Driven entirely by the parser — no regex tables, no per-kind dispatch.
:func:`BigipConfig.merge` -style introspection walks every dict-valued
field, so a kind added to the model is picked up here automatically.
"""

from __future__ import annotations

from dataclasses import fields
from typing import Any

from lsprotocol import types

from core.analysis.semantic_model import Range
from core.bigip.parser import parse_bigip_conf


def get_bigip_document_symbols(source: str) -> list[types.DocumentSymbol]:
    """Return a module → kind → object outline for *source*.

    *source* must be a parseable BIG-IP / SCF text.  Empty configs
    return an empty list.  Objects without a ``range`` (the parser
    didn't capture a byte span) fall through silently — they would
    have nowhere to point in the document.
    """
    try:
        config = parse_bigip_conf(source)
    except Exception:  # noqa: BLE001 — outline shouldn't fail the editor
        return []

    # Group objects by (module, kind).  Each dataclass field on
    # ``BigipConfig`` is a per-kind dict; the kind label lives on the
    # objects themselves (e.g. ``BigipMinimalObject.kind`` / ``BigipPool``
    # is implicitly "ltm pool").  We derive the (module, kind) tuple
    # from the field name when the dataclass doesn't carry it.
    by_module_kind: dict[str, dict[str, list[tuple[str, Range]]]] = {}
    for fld in fields(config):
        # ``generic_objects`` is the parser's "no typed dataclass for
        # this kind" fallback bucket — every typed kind also lands a
        # row there as ``<module>::<type>::<path>``.  Including it in
        # the outline would double-count every object and add a
        # synthetic "generic object" tree the user can't navigate.
        # Skip it; typed kinds already cover these objects.
        if fld.name == "generic_objects":
            continue
        kind_dict = getattr(config, fld.name)
        if not isinstance(kind_dict, dict) or not kind_dict:
            continue
        module, kind = _module_kind_for_field(fld.name, next(iter(kind_dict.values())))
        for full_path, obj in kind_dict.items():
            obj_range = getattr(obj, "range", None)
            if obj_range is None:
                continue
            by_module_kind.setdefault(module, {}).setdefault(kind, []).append(
                (full_path, obj_range)
            )

    # Build the three-level tree.  Module / kind ranges span their
    # children (min .. max) so collapsing the module folds away
    # everything underneath it.
    out: list[types.DocumentSymbol] = []
    for module in sorted(by_module_kind):
        kind_groups = by_module_kind[module]
        kind_symbols: list[types.DocumentSymbol] = []
        for kind in sorted(kind_groups):
            entries = sorted(kind_groups[kind], key=lambda e: e[1].start.line)
            object_symbols = [
                types.DocumentSymbol(
                    name=full_path,
                    kind=types.SymbolKind.Variable,
                    range=_to_lsp_range(rng),
                    selection_range=_to_lsp_range(rng),
                )
                for full_path, rng in entries
            ]
            kind_range = _span_of(entries[0][1], entries[-1][1])
            kind_symbols.append(
                types.DocumentSymbol(
                    name=kind,
                    kind=types.SymbolKind.Class,
                    range=_to_lsp_range(kind_range),
                    selection_range=_to_lsp_range(kind_range),
                    children=object_symbols,
                )
            )
        first_kind = sorted(kind_groups)[0]
        last_kind = sorted(kind_groups)[-1]
        first_range = sorted(kind_groups[first_kind], key=lambda e: e[1].start.line)[0][1]
        last_range = sorted(kind_groups[last_kind], key=lambda e: e[1].start.line)[-1][1]
        module_range = _span_of(first_range, last_range)
        out.append(
            types.DocumentSymbol(
                name=module,
                kind=types.SymbolKind.Module,
                range=_to_lsp_range(module_range),
                selection_range=_to_lsp_range(module_range),
                children=kind_symbols,
            )
        )
    return out


def _module_kind_for_field(field_name: str, sample_obj: Any) -> tuple[str, str]:
    """Return ``(module, kind-label)`` for a BigipConfig field.

    The query projection's :data:`MODULE_KINDS` table already maps
    ``module → kind label → (attr-name, tmsh-kind)``; reverse-index
    it so the outline labels match the canonical TMSH spelling
    everywhere else in the codebase (no second source of truth for
    field-name → kind translation).  Falls back to a heuristic
    ``BigipMinimalObject.kind`` lookup or the field name itself if
    the projection has no entry.
    """
    mapping = _module_kind_by_attr()
    if field_name in mapping:
        return mapping[field_name]

    # Minimal-object fallback: the dataclass carries its kind explicitly.
    explicit = getattr(sample_obj, "kind", None)
    if isinstance(explicit, str) and " " in explicit:
        module, _, rest = explicit.partition(" ")
        return module, rest

    # Last resort — derive from the field name.  Should rarely fire
    # now that the projection table covers every projected kind.
    parts = field_name.split("_")
    return ("?", " ".join(parts))


def _module_kind_by_attr() -> dict[str, tuple[str, str]]:
    """Reverse the projection's ``MODULE_KINDS`` so we can look up by
    BigipConfig attribute name.

    Cached on the function for cheap reuse — the table is static.
    """
    cache = getattr(_module_kind_by_attr, "_cache", None)
    if cache is not None:
        return cache
    from core.bigip.query.projection import MODULE_KINDS

    mapping: dict[str, tuple[str, str]] = {}
    for module, kind_table in MODULE_KINDS.items():
        for kind_label, (attr_name, _tmsh_kind) in kind_table.items():
            mapping[attr_name] = (module, kind_label)
    _module_kind_by_attr._cache = mapping  # type: ignore[attr-defined]
    return mapping


def _to_lsp_range(rng: Range) -> types.Range:
    return types.Range(
        start=types.Position(line=rng.start.line, character=rng.start.character),
        end=types.Position(line=rng.end.line, character=rng.end.character),
    )


def _span_of(first: Range, last: Range) -> Range:
    """Return a Range covering both *first* and *last* (used for parents)."""
    if (first.start.line, first.start.character) <= (last.start.line, last.start.character):
        start = first.start
    else:
        start = last.start
    if (first.end.line, first.end.character) >= (last.end.line, last.end.character):
        end = first.end
    else:
        end = last.end
    return Range(start=start, end=end)
