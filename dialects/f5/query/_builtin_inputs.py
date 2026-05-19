"""Built-in input formats — JSON, JSONL, CSV, and F5 syslog.

Each format is a tiny adapter over the parsers in :mod:`._inputs`
(which the in-query ``json_load`` / ``jsonl_load`` / ``csv_load`` /
``f5log_load`` builtins also call), wired into the
:mod:`.inputs` plugin registry so the runner's structured-root
dispatch goes through the same lookup as user plugins.

Keeping these tucked away in their own module lets
:mod:`.inputs` stay free of import-time side-effects — callers can
``import dialects.f5.query.inputs`` for the public registry helpers
without dragging the builtin parsers in unless they actually use
them.
"""

from __future__ import annotations

import json as _json
from typing import Any

from ._inputs import parse_csv, parse_f5log, parse_jsonl
from .errors import QueryError
from .inputs import input_format


@input_format(
    "json",
    summary="JSON side-input — parsed as a single value (dict, list, scalar).",
)
def _parse_json(source: str, *, uri: str, options: tuple[tuple[str, Any], ...] = ()) -> Any:  # noqa: ARG001
    try:
        return _json.loads(source)
    except _json.JSONDecodeError as exc:
        raise QueryError(f"{uri}: invalid JSON ({exc.msg} at line {exc.lineno})") from exc


@input_format(
    "jsonl",
    summary="JSON Lines (NDJSON) — one JSON value per non-blank line.",
)
def _parse_jsonl(source: str, *, uri: str, options: tuple[tuple[str, Any], ...] = ()) -> Any:  # noqa: ARG001
    return parse_jsonl(source, source=uri)


@input_format(
    "csv",
    summary="CSV — list of row-dicts; first row names columns unless 'headers' option is set.",
    details=(
        "Option ``headers`` (tuple of column names) skips header-row "
        "discovery so every line of *source* is treated as data — "
        "useful for CSVs without a leading header row."
    ),
)
def _parse_csv(source: str, *, uri: str, options: tuple[tuple[str, Any], ...] = ()) -> Any:
    headers = None
    for k, v in options:
        if k == "headers":
            headers = list(v) if v else None
            break
    return parse_csv(source, headers=headers, source=uri)


@input_format(
    "f5log",
    summary="F5 BIG-IP syslog — structured event dicts (timestamp / module / message / ...).",
)
def _parse_f5log(source: str, *, uri: str, options: tuple[tuple[str, Any], ...] = ()) -> Any:  # noqa: ARG001
    return parse_f5log(source, source=uri)
