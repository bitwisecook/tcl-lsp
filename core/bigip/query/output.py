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

from .values import ObjectRef, PathRef, Stream


def _flat(values: Iterable[Any]) -> list[Any]:
    out: list[Any] = []
    for v in values:
        if isinstance(v, Stream):
            out.extend(v.items)
        else:
            out.append(v)
    return out


def render(values: list[Any], *, mode: str = "auto") -> str:
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
    raise ValueError(f"unknown output mode: {mode}")


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
    return "".join(_scalar_str(v) + "\n" for v in _flatten_scalars(values))


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
        if isinstance(v, list):
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
        if isinstance(v, list):
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


def _to_json(v: Any) -> Any:
    if isinstance(v, ObjectRef):
        out = {
            "kind": v.kind,
            "full-path": v.full_path,
            "fields": {k: _to_json(val) for k, val in v.fields.items()},
        }
        return out
    if isinstance(v, PathRef):
        return v.full_path
    if isinstance(v, Stream):
        return [_to_json(x) for x in v.items]
    if isinstance(v, list):
        return [_to_json(x) for x in v]
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)
