"""Decoders for raw JSON-RPC LSP responses used by the e2e suite.

The harness speaks raw JSON, so every response is a plain ``dict``/``list``
rather than an ``lsprotocol`` model.  These helpers normalise the handful of
shapes the protocol allows (``Location`` vs ``LocationLink``, ``Hover``
content variants, hierarchical document symbols, the semantic-tokens delta
encoding) so the feature tests can assert on stable, decoded values.
"""

from __future__ import annotations

from typing import Any


def hover_text(hover: Any) -> str:
    """Flatten a Hover result's ``contents`` to plain text."""
    if not hover:
        return ""
    contents = hover.get("contents")
    if isinstance(contents, dict):
        return str(contents.get("value", ""))
    if isinstance(contents, list):
        parts = []
        for item in contents:
            if isinstance(item, dict):
                parts.append(str(item.get("value", "")))
            else:
                parts.append(str(item))
        return "\n".join(parts)
    return str(contents or "")


def locations(result: Any) -> list[dict]:
    """Normalise a definition/references result to a list of location dicts.

    Returns dicts with ``uri`` and ``range`` keys, collapsing the
    ``LocationLink`` form (``targetUri``/``targetRange``) onto the plain
    ``Location`` shape.
    """
    if not result:
        return []
    items = result if isinstance(result, list) else [result]
    out: list[dict] = []
    for item in items:
        if "targetUri" in item:
            out.append(
                {
                    "uri": item["targetUri"],
                    "range": item.get("targetSelectionRange") or item.get("targetRange"),
                }
            )
        else:
            out.append({"uri": item.get("uri"), "range": item.get("range")})
    return out


def starts(result: Any) -> set[tuple[int, int]]:
    """The ``(line, character)`` start of every location/highlight in a result."""
    out: set[tuple[int, int]] = set()
    items = result if isinstance(result, list) else ([result] if result else [])
    for item in items:
        rng = item.get("range") or item.get("targetSelectionRange") or item.get("targetRange")
        if rng:
            s = rng["start"]
            out.add((s["line"], s["character"]))
    return out


def start_lines(result: Any) -> set[int]:
    return {line for line, _ in starts(result)}


def completion_items(result: Any) -> list[dict]:
    if not result:
        return []
    if isinstance(result, dict):
        return list(result.get("items") or [])
    return list(result)


def completion_labels(result: Any) -> list[str]:
    return [str(item.get("label")) for item in completion_items(result)]


def flatten_symbols(symbols: Any) -> list[dict]:
    """Depth-first flatten of a hierarchical ``DocumentSymbol`` tree."""
    out: list[dict] = []

    def walk(items: Any) -> None:
        for sym in items or []:
            out.append(sym)
            walk(sym.get("children"))

    walk(symbols)
    return out


def symbol_names(symbols: Any) -> set[str]:
    return {str(s.get("name")) for s in flatten_symbols(symbols)}


def rename_edits(result: Any) -> dict[str, list[dict]]:
    """Return ``{uri: [{range, newText}, ...]}`` from a WorkspaceEdit result."""
    if not result:
        return {}
    if result.get("changes"):
        return result["changes"]
    out: dict[str, list[dict]] = {}
    for change in result.get("documentChanges") or []:
        uri = change.get("textDocument", {}).get("uri")
        if uri is not None:
            out.setdefault(uri, []).extend(change.get("edits") or [])
    return out


def decode_semantic_tokens(result: Any) -> list[dict]:
    """Decode the LSP semantic-tokens delta encoding into absolute tokens.

    Each token becomes ``{"line", "char", "length", "type", "modifiers"}``
    where ``type`` is the raw legend index.
    """
    data = (result or {}).get("data") or []
    out: list[dict] = []
    line = 0
    char = 0
    for i in range(0, len(data), 5):
        d_line, d_char, length, ttype, tmods = data[i : i + 5]
        if d_line:
            line += d_line
            char = d_char
        else:
            char += d_char
        out.append(
            {"line": line, "char": char, "length": length, "type": ttype, "modifiers": tmods}
        )
    return out
