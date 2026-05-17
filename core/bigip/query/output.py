"""Render evaluator output for the ``f5 query`` verb.

Five flavours, picked by CLI flag:

- ``auto`` (default): SCF stanzas for streams of objects, one value
  per line for streams or lists of scalars, JSON for the mixed case.
  Lists of scalars are flattened so ``[.X[].name]`` renders the same
  one-per-line shape as the equivalent stream form.
- ``scf``: every object selection emits its stanza verbatim from the
  source; scalars get coerced through ``str``.
- ``raw``: one value per line, scalars only (jq's ``--raw-output``
  conventions — including ``null`` printed as the literal token).
- ``paths``: print the full-path of every object / path-ref produced.
- ``json``: a JSON array of values, with objects rendered as keyed
  field maps.
"""

from __future__ import annotations

import json
from typing import Any, Iterable

from ..types import BigipList
from .values import ObjectRef, PathRef, Stream, resolve_lazy_field


def _flat(values: Iterable[Any]) -> list[Any]:
    out: list[Any] = []
    for v in values:
        if isinstance(v, Stream):
            out.extend(v.items)
        else:
            out.append(v)
    return out


def render(values: list[Any], *, mode: str = "auto", **opts: Any) -> str:
    values = _flat(values)
    if mode == "auto":
        return _render_auto(values)
    if mode == "scf":
        return _render_scf(values)
    if mode == "raw":
        return _render_raw(values)
    if mode == "paths":
        return _render_paths(values)
    if mode == "json":
        return _render_json(values)
    if mode == "table":
        return _render_table(values, lineart=False)
    if mode == "table-lineart":
        return _render_table(values, lineart=True)
    # Fall through to the pluggable renderer registry.  Built-in
    # renderers (``mermaid``, ``gantt``, ``ascii-blocks``) are loaded
    # on first lookup; user-registered plugins are picked up after
    # whatever code path imported them ran.
    from .renderers import lookup as _lookup_renderer

    spec = _lookup_renderer(mode)
    if spec is None:
        raise ValueError(f"unknown output mode: {mode}")
    return spec.impl(values, **opts)


def _render_auto(values: list[Any]) -> str:
    if not values:
        return ""
    # All objects with a stanza_slot → SCF stanzas.
    if all(isinstance(v, ObjectRef) and v.stanza_slot is not None for v in values):
        return _render_scf(values)
    # All scalars (or lists of scalars) → flatten and render
    # one-per-line.  A ``[.X[].name]`` query produces a list of names;
    # treating it like a stream of names is what users want for grep
    # / xargs piping.
    flat = _flatten_scalar_lists(values)
    if flat is not None:
        return _render_raw(flat)
    return _render_json(values)


def _render_scf(values: list[Any]) -> str:
    parts: list[str] = []
    for v in values:
        if isinstance(v, ObjectRef) and v.stanza_slot is not None:
            text = v.stanza_slot.raw_text
            if not text.endswith("\n"):
                text += "\n"
            parts.append(text)
        else:
            parts.append(_scalar_str(v) + "\n")
    return "".join(parts)


def _render_raw(values: list[Any]) -> str:
    out: list[str] = []
    for v in _flatten_scalars(values):
        if not _is_scalar(v):
            # ``--raw`` is documented as "scalars only".  Falling
            # back to ``str(v)`` would emit a Python repr like
            # ``ObjectRef(kind='ltm pool', full_path='/Common/p')``
            # which is useless for scripting / piping.  Refuse and
            # tell the user to pick a different output mode.
            kind = type(v).__name__
            if isinstance(v, ObjectRef):
                kind = f"ObjectRef({v.kind!r})"
            raise ValueError(
                f"--raw cannot render {kind}; use --paths-only for object "
                f"identities or --json / --scf for full objects"
            )
        out.append(_scalar_str(v) + "\n")
    return "".join(out)


def _render_paths(values: list[Any]) -> str:
    lines: list[str] = []
    for v in _flatten_scalars(values):
        if isinstance(v, ObjectRef):
            lines.append(v.full_path)
        elif isinstance(v, PathRef):
            lines.append(v.full_path)
        else:
            lines.append(_scalar_str(v))
    return "".join(line + "\n" for line in lines)


def _render_json(values: list[Any]) -> str:
    return json.dumps([_to_json(v) for v in values], indent=2) + "\n"


def _is_scalar(v: Any) -> bool:
    return isinstance(v, (str, int, float, bool, PathRef)) or v is None


def _flatten_scalar_lists(values: list[Any]) -> list[Any] | None:
    """If every value is a scalar or a list of scalars, return the
    flat list; otherwise ``None``.

    Lets ``auto`` mode treat ``[.X[].name]`` (one list of names) the
    same as ``.X[].name`` (a stream of names) — both print
    one-per-line.  Mixed shapes (objects, nested lists) still drop
    through to JSON.
    """
    flat: list[Any] = []
    for v in values:
        if _is_scalar(v):
            flat.append(v)
            continue
        if isinstance(v, (list, BigipList)):
            for item in v:
                if not _is_scalar(item):
                    return None
                flat.append(item)
            continue
        return None
    return flat


def _flatten_scalars(values: list[Any]) -> list[Any]:
    """Same as :func:`_flatten_scalar_lists` but never refuses — used
    by ``--raw`` and ``--paths-only`` where the user has already
    opted into a flat output."""
    flat: list[Any] = []
    for v in values:
        if isinstance(v, (list, BigipList)):
            flat.extend(v)
        else:
            flat.append(v)
    return flat


def _scalar_str(v: Any) -> str:
    # Matches jq's ``--raw-output`` semantics: ``null`` renders as the
    # literal token, distinguishable from an empty string.
    if v is None:
        return "null"
    if isinstance(v, PathRef):
        return v.full_path
    if isinstance(v, bool):
        return "true" if v else "false"
    return str(v)


def _render_table(values: list[Any], *, lineart: bool) -> str:
    """Render *values* as an aligned table.

    Headers are inferred from the keys of the first dict-shaped
    row; every subsequent row is laid out under the same columns
    in the same order (missing keys render as empty cells).
    Mixed-shape input (some dicts, some bare scalars) falls back
    to one cell per row, no header.

    *lineart* picks the border style:

    - ``False`` → ASCII (``+---+`` / ``|`` separators), portable
      across every terminal and easily piped through other text
      tools.
    - ``True`` → Unicode box-drawing (``┌─┬─┐`` / ``│``); prettier
      for human viewing in modern terminals.
    """
    if not values:
        return ""
    # Discover headers from the first dict; if no value is a dict,
    # treat the whole list as a single anonymous column.
    headers: list[str] = []
    has_dict = False
    for v in values:
        if isinstance(v, dict):
            has_dict = True
            for k in v.keys():
                if k not in headers:
                    headers.append(k)
        elif isinstance(v, ObjectRef):
            has_dict = True
            for k in v.fields.keys():
                if k not in headers:
                    headers.append(k)
    if not has_dict:
        headers = ["value"]
        rows = [[_cell_str(v)] for v in values]
    else:
        rows = []
        for v in values:
            if isinstance(v, dict):
                rows.append([_cell_str(v.get(h, "")) for h in headers])
            elif isinstance(v, ObjectRef):
                rows.append(
                    [
                        _cell_str(resolve_lazy_field(v, h, v.fields[h]) if h in v.fields else "")
                        for h in headers
                    ]
                )
            else:
                # Scalar mixed with dicts — stretches the scalar across
                # the first column, leaves the rest empty.
                rows.append([_cell_str(v)] + [""] * (len(headers) - 1))
    widths = [max(len(h), max((len(r[i]) for r in rows), default=0)) for i, h in enumerate(headers)]
    if lineart:
        h = "─"
        v = "│"
        tl, tm, tr = "┌", "┬", "┐"
        ml, mm, mr = "├", "┼", "┤"
        bl, bm, br = "└", "┴", "┘"
    else:
        h = "-"
        v = "|"
        tl = tm = tr = ml = mm = mr = bl = bm = br = "+"
    # Borders.
    sep_top = tl + tm.join(h * (w + 2) for w in widths) + tr
    sep_mid = ml + mm.join(h * (w + 2) for w in widths) + mr
    sep_bot = bl + bm.join(h * (w + 2) for w in widths) + br
    header_row = v + v.join(f" {h_text:<{w}} " for h_text, w in zip(headers, widths)) + v
    lines: list[str] = [sep_top, header_row, sep_mid]
    for row in rows:
        lines.append(v + v.join(f" {cell:<{w}} " for cell, w in zip(row, widths)) + v)
    lines.append(sep_bot)
    return "\n".join(lines) + "\n"


def _cell_str(v: Any) -> str:
    """Coerce a single cell value to its display string.

    Mirrors the scalar / path-ref logic of :func:`_scalar_str` but
    also handles lists (joined with ``,``) and dicts (rendered as
    compact JSON so a nested object survives in one cell rather
    than blowing up the layout).
    """
    if v is None:
        return ""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, PathRef):
        return v.full_path
    if isinstance(v, ObjectRef):
        return v.full_path or v.kind
    if isinstance(v, (list, BigipList)):
        return ",".join(_cell_str(x) for x in v)
    if isinstance(v, dict):
        return json.dumps({k: _to_json(val) for k, val in v.items()}, separators=(",", ":"))
    return str(v)


def _to_json(v: Any) -> Any:
    if isinstance(v, ObjectRef):
        out = {
            "kind": v.kind,
            "full-path": v.full_path,
            "fields": {k: _to_json(resolve_lazy_field(v, k, val)) for k, val in v.fields.items()},
        }
        return out
    if isinstance(v, PathRef):
        return v.full_path
    if isinstance(v, Stream):
        return [_to_json(x) for x in v.items]
    if isinstance(v, (list, BigipList)):
        return [_to_json(x) for x in v]
    if isinstance(v, dict):
        # Object literals produce plain Python dicts; recurse so
        # nested PathRef / ObjectRef / Stream values render the same
        # way they would at the top level.
        return {k: _to_json(val) for k, val in v.items()}
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)
