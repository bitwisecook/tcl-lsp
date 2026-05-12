"""Render evaluator output for the ``f5 query`` verb.

Four flavours, picked by CLI flag:

- ``auto`` (default): SCF stanzas for objects, one-value-per-line for
  scalars, JSON for the mixed case.
- ``scf``: every object selection emits its stanza verbatim from the
  source; scalars get coerced through ``str``.
- ``raw``: one value per line, scalars only.
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
    # All objects → SCF stanzas.
    if all(isinstance(v, ObjectRef) and v.stanza_slot is not None for v in values):
        return _render_scf(values)
    # All scalars (incl. PathRef) → one per line.
    if all(_is_scalar(v) for v in values):
        return _render_raw(values)
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
    return "".join(_scalar_str(v) + "\n" for v in values)


def _render_paths(values: list[Any]) -> str:
    lines: list[str] = []
    for v in values:
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


def _scalar_str(v: Any) -> str:
    if v is None:
        return ""
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
