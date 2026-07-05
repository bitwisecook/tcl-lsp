# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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


def _looks_like_range(obj: Any) -> bool:
    """True for an LSP ``Range`` dict: ``{start:{line,character}, end:{...}}``."""
    if not isinstance(obj, dict):
        return False
    start, end = obj.get("start"), obj.get("end")
    return (
        isinstance(start, dict)
        and isinstance(end, dict)
        and "line" in start
        and "character" in start
        and "line" in end
        and "character" in end
    )


def iter_ranges(obj: Any):
    """Recursively yield every LSP ``Range`` dict nested anywhere in *obj*.

    Walks arbitrary response shapes (Location, LocationLink, DocumentSymbol
    trees, WorkspaceEdit, CodeAction, CompletionItem textEdits, …) so a single
    invariant pass can validate every span a provider returns.
    """
    if _looks_like_range(obj):
        yield obj
        # A Range's children are leaf positions; no need to recurse into them.
        return
    if isinstance(obj, dict):
        for value in obj.values():
            yield from iter_ranges(value)
    elif isinstance(obj, list):
        for item in obj:
            yield from iter_ranges(item)


def range_violations(
    result: Any,
    text: str,
    *,
    label: str = "",
    strict_end_column: bool = False,
) -> list[str]:
    """Return well-formedness violations for every ``Range`` in *result* (empty == ok).

    Universal across providers: a span a client acts on must have non-negative
    coordinates, ``start <= end``, sit on real lines, and a ``start`` column
    within its line (measured in **UTF-16** units, so a multibyte/emoji column
    slip surfaces).

    ``strict_end_column`` additionally requires the ``end`` column to be within
    its line.  It defaults off because Tcl statement / selection-range / symbol
    spans legitimately extend to the line terminator (``end`` = line length, or
    +1 for a CRLF ``\\r``), which clients clamp; semantic tokens, which must
    never over-run, are checked strictly via :func:`semantic_token_violations`.
    """
    out: list[str] = []
    line_u16 = _doc_lines_utf16(text)
    n_lines = len(line_u16)
    pre = f"{label}: " if label else ""
    for rng in iter_ranges(result):
        s, e = rng["start"], rng["end"]
        sl, sc, el, ec = s["line"], s["character"], e["line"], e["character"]
        if min(sl, sc, el, ec) < 0:
            out.append(f"{pre}negative coordinate in range {rng}")
            continue
        if (el, ec) < (sl, sc):
            out.append(f"{pre}end before start in range {rng}")
        # ``end`` may sit at (n_lines, 0) for a whole-document span; ``start``
        # must be on a real line.
        if sl >= n_lines:
            out.append(f"{pre}start line {sl} past end of document ({n_lines} lines)")
            continue
        if el > n_lines:
            out.append(f"{pre}end line {el} past end of document ({n_lines} lines)")
        if sc > line_u16[sl]:
            out.append(f"{pre}start col {sc} past line {sl} length {line_u16[sl]} (UTF-16)")
        if strict_end_column and el < n_lines and ec > line_u16[el]:
            out.append(f"{pre}end col {ec} past line {el} length {line_u16[el]} (UTF-16)")
    return out


def workspace_edit_violations(result: Any, *, label: str = "") -> list[str]:
    """Return overlap/duplicate violations in a ``WorkspaceEdit`` (empty == ok).

    A client applies the per-file ``TextEdit`` array as a batch; the LSP spec
    forbids overlapping edits (the result is undefined) and a duplicate edit at
    the same range is a sign of double-counting.  This pins both for rename and
    code-action results.
    """
    out: list[str] = []
    pre = f"{label}: " if label else ""
    for uri, edits in rename_edits(result).items():
        spans: list[tuple[tuple[int, int], tuple[int, int], str]] = []
        for ed in edits:
            rng = ed.get("range") or {}
            s, e = rng.get("start") or {}, rng.get("end") or {}
            spans.append(
                (
                    (s.get("line", 0), s.get("character", 0)),
                    (e.get("line", 0), e.get("character", 0)),
                    ed.get("newText", ""),
                )
            )
        spans.sort()
        for i in range(1, len(spans)):
            prev_s, prev_e, prev_t = spans[i - 1]
            cur_s, cur_e, cur_t = spans[i]
            if cur_s == prev_s and cur_e == prev_e and cur_t == prev_t:
                out.append(f"{pre}{uri}: duplicate edit at {cur_s}->{cur_e}")
            elif cur_s < prev_e:
                out.append(f"{pre}{uri}: overlapping edits {prev_s}->{prev_e} and {cur_s}->{cur_e}")
    return out


def _utf16_len(s: str) -> int:
    """Length of *s* in UTF-16 code units — the unit LSP positions are measured in.

    A character outside the BMP (e.g. an emoji) is two UTF-16 units, so a token
    encoder that counts code points instead of UTF-16 units drifts here.
    """
    return len(s.encode("utf-16-le")) // 2


def _doc_lines_utf16(text: str) -> list[int]:
    """Per-line content length in UTF-16 units (trailing ``\\r`` excluded).

    Lines are split on ``\\n``; a trailing ``\\r`` (CRLF documents) is not part
    of the visible line content the encoder positions tokens within.
    """
    return [_utf16_len(line.rstrip("\r")) for line in text.split("\n")]


def semantic_token_violations(
    result: Any,
    text: str,
    *,
    token_types: list[str],
    token_modifiers: list[str],
) -> list[str]:
    """Return a list of semantic-token *invariant* violations (empty == valid).

    These are the properties a conforming server must satisfy for *any*
    document, independent of which tokens it chooses to emit — the things a
    client relies on and silently mis-renders (or logs "Overlapping semantic
    tokens detected") when they break:

    * the raw ``data`` is a flat run of 5-int groups;
    * every delta is non-negative (the encoding requires ascending order);
    * each ``tokenType`` index is within the advertised legend, and every
      ``tokenModifiers`` bit maps to a legend entry;
    * absolute tokens are strictly ordered and **non-overlapping** — on a line,
      the next token starts at or after the previous token's end;
    * every token lies within document bounds, measured in **UTF-16** units
      (so a multibyte/emoji column slip surfaces as an over-run).
    """
    out: list[str] = []
    data = (result or {}).get("data") if isinstance(result, dict) else None
    if data is None:
        return ["semanticTokens result has no `data` array"]
    if len(data) % 5 != 0:
        return [f"token data length {len(data)} is not a multiple of 5"]

    n_types = len(token_types)
    n_mods = len(token_modifiers)
    for i in range(0, len(data), 5):
        d_line, d_char, length, ttype, tmods = data[i : i + 5]
        if d_line < 0 or d_char < 0:
            out.append(f"token {i // 5}: negative delta (dline={d_line}, dchar={d_char})")
        if length <= 0:
            out.append(f"token {i // 5}: non-positive length {length}")
        if not (0 <= ttype < n_types):
            out.append(f"token {i // 5}: type index {ttype} outside legend (0..{n_types - 1})")
        if tmods < 0 or (n_mods < 32 and tmods >> n_mods):
            out.append(f"token {i // 5}: modifier bits {tmods:#b} outside legend ({n_mods} mods)")

    line_u16 = _doc_lines_utf16(text)
    n_lines = len(line_u16)
    prev: dict | None = None
    for idx, tok in enumerate(decode_semantic_tokens(result)):
        line, char, length = tok["line"], tok["char"], tok["length"]
        if line >= n_lines:
            out.append(f"token {idx}: line {line} past end of document ({n_lines} lines)")
        elif char + length > line_u16[line]:
            out.append(
                f"token {idx}: ends at utf16 col {char + length} but line {line} is "
                f"{line_u16[line]} units long (multibyte/UTF-16 column drift?)"
            )
        if prev is not None:
            if (line, char) < (prev["line"], prev["char"]):
                out.append(f"token {idx}: starts before previous token (not ascending)")
            elif line == prev["line"] and char < prev["char"] + prev["length"]:
                out.append(
                    f"token {idx}: overlaps previous token on line {line} "
                    f"(starts {char}, previous ends {prev['char'] + prev['length']})"
                )
        prev = tok
    return out
