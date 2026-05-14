"""Low-level block / property / header extraction helpers.

The brace-balanced block scanner (:class:`_Block`,
:func:`_extract_blocks`), the property-with-spans parser
(:func:`_parse_properties_with_spans`), the list-block parser
(:func:`_parse_list_block`), and :func:`_parse_generic_header`
live here.  These are the building blocks every per-kind
``_parse_*`` function in :mod:`._parsers` is built on; nothing in
:mod:`._parsers` is referenced from here, so external consumers
(``link_extract``, ``emit``, ``pcap_enrich``, ``wireshark_profile``)
can import these helpers without dragging in the full parser
machinery.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ...analysis.semantic_model import Range
from ...common.document_buffer import DocumentBuffer

# Low-level brace-balanced block extraction


@dataclass(frozen=True, slots=True)
class _Block:
    """A parsed top-level config stanza."""

    header: str  # e.g. "ltm virtual /Common/my_vs"
    body: str  # text between the outermost { }
    start_offset: int
    end_offset: int


@dataclass(frozen=True, slots=True)
class _Property:
    """A top-level key/value property parsed from a BIG-IP block body."""

    key: str
    value: str
    value_start: int | None = None  # local offset within body
    value_end: int | None = None  # local offset within body (exclusive)


def _extract_blocks(source: str) -> list[_Block]:
    """Extract all top-level ``keyword ... { ... }`` blocks from *source*.

    Handles nested braces and respects quoted strings.
    """
    blocks: list[_Block] = []
    pos = 0
    length = len(source)

    while pos < length:
        # Skip whitespace and comments
        while pos < length and source[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break
        if source[pos] == "#":
            # Skip to end of line
            while pos < length and source[pos] != "\n":
                pos += 1
            continue

        # Read header (everything up to the opening brace)
        header_start = pos
        while pos < length and source[pos] != "{":
            if source[pos] == "\n":
                # If we hit a newline without finding a brace, this is not
                # a block header — skip the line.
                pos += 1
                break
            pos += 1
        else:
            if pos < length and source[pos] == "{":
                header = source[header_start:pos].strip()
                brace_start = pos
                pos += 1  # skip opening brace
                depth = 1
                body_start = pos
                while pos < length and depth > 0:
                    ch = source[pos]
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                    elif ch == "\\" and pos + 1 < length:
                        # Backslash-escaped token outside a string —
                        # tmsh emits ``\"key\"`` for record keys with
                        # special characters.  Treat the backslash and
                        # the following character as opaque so the
                        # subsequent quote doesn't start a string scan
                        # that would gobble braces.
                        pos += 1
                    elif ch == '"':
                        # Skip quoted string
                        pos += 1
                        while pos < length and source[pos] != '"':
                            if source[pos] == "\\" and pos + 1 < length:
                                pos += 1  # skip escaped char
                            pos += 1
                    pos += 1
                body = source[body_start : pos - 1]
                blocks.append(
                    _Block(
                        header=header,
                        body=body,
                        start_offset=brace_start,
                        end_offset=pos,
                    )
                )
            continue

    return blocks


def _parse_properties_with_spans(body: str) -> dict[str, _Property]:
    """Parse top-level ``key value`` properties from a block body.

    Returns ``{key: _Property}`` where each property includes its value span
    relative to *body*.  Sub-blocks (``key { ... }``) retain braced text.

    Each value extends from the key's whitespace separator to the next
    newline.  That's correct for the multi-line shape every well-known
    TMSH property uses (one property per line; multi-token values like
    legacy ``network ADDR MASK`` or ``last-resort-pool TYPE PATH`` are
    kept together as one value).  When the body is a compact one-line
    stanza (``{ key1 v1 key2 v2 }``) the read-to-newline rule swallows
    every property after the first; typed parsers that need to support
    compact bodies should re-split the captured value via
    :func:`_split_inline_keys` once they know the sibling key set.
    """
    props: dict[str, _Property] = {}
    pos = 0
    length = len(body)

    while pos < length:
        # Skip whitespace
        while pos < length and body[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break

        # Read key
        key_start = pos
        while pos < length and body[pos] not in " \t\n\r{":
            pos += 1
        key = body[key_start:pos].strip()
        if not key:
            pos += 1
            continue

        # Skip whitespace
        while pos < length and body[pos] in " \t":
            pos += 1

        if pos >= length or body[pos] == "\n":
            # Key with no value (flag-style)
            props[key] = _Property(key=key, value="")
            continue

        if body[pos] == "{":
            # Sub-block value
            val_start = pos
            pos += 1
            depth = 1
            while pos < length and depth > 0:
                ch = body[pos]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                elif ch == "\\" and pos + 1 < length:
                    # See `_extract_blocks` for the same handler — tmsh
                    # writes ``\"...\"`` for keys with special chars,
                    # and the backslash must protect the following
                    # quote from being interpreted as a string start.
                    pos += 1
                elif ch == '"':
                    pos += 1
                    while pos < length and body[pos] != '"':
                        if body[pos] == "\\" and pos + 1 < length:
                            pos += 1
                        pos += 1
                pos += 1
            val_end = pos
            props[key] = _Property(
                key=key,
                value=body[val_start:val_end].strip(),
                value_start=val_start,
                value_end=val_end,
            )
        else:
            # Simple value — read to end of line
            val_start = pos
            while pos < length and body[pos] != "\n":
                pos += 1
            val_end = pos
            props[key] = _Property(
                key=key,
                value=body[val_start:val_end].strip(),
                value_start=val_start,
                value_end=val_end,
            )

    return props


def _split_inline_keys(value: str, *, known_keys: tuple[str, ...]) -> dict[str, str]:
    """Re-split a read-to-EOL value containing inline sibling properties.

    Compact one-line TMSH stanzas like

        net route /Common/r1 { network 10.0.0.0/8 gw 192.168.1.1 }

    leave ``_parse_properties_with_spans`` with ``network`` carrying the
    rest of the line — ``10.0.0.0/8 gw 192.168.1.1`` — because the
    line-based reader has no schema to know that ``gw`` is the next
    key.  Typed parsers that DO know their sibling-key set call this
    helper to pull the inline pairs back apart.

    Returns a flat ``{key: value}`` map.  The original key gets its
    real first-token value; every recognised inline ``<known-key>
    <token>`` pair lands as an additional entry.  Tokens that aren't
    one of *known_keys* stay attached to the preceding value (so
    legacy ``network 10.0.0.0 255.255.255.0`` keeps the dotted-quad
    netmask glued onto ``network``).  Returns an empty dict when the
    value has no embedded sibling — caller can fall back to the
    original single value.
    """
    out: dict[str, str] = {}
    key_set = set(known_keys)
    tokens = value.split()
    if not tokens:
        return out
    current_key: str | None = None
    current_value: list[str] = [tokens[0]]
    brace_depth = 0
    for tok in tokens[1:]:
        # Track brace depth so braced sub-block contents (keyed-list
        # bodies like ``profiles { /Common/clientssl { context
        # clientside } }``) don't have their inner tokens mistaken
        # for sibling keys at the outer level.
        if brace_depth > 0:
            current_value.append(tok)
            brace_depth += tok.count("{") - tok.count("}")
            continue
        if tok in key_set:
            # Capture the accumulated value under the *previous* key
            # and start collecting the next one.
            if current_key is None:
                out["__first__"] = " ".join(current_value)
            else:
                out[current_key] = " ".join(current_value)
            current_key = tok
            current_value = []
            continue
        current_value.append(tok)
        brace_depth += tok.count("{") - tok.count("}")
    if current_key is None:
        return out
    out[current_key] = " ".join(current_value)
    # Preserve the head value the caller already has under its
    # original key; surface only the additional inline pairs.
    head = out.pop("__first__", None)
    if head is not None:
        out["__head__"] = head
    return out


def _parse_properties(body: str) -> dict[str, str]:
    """Parse simple ``key value`` properties from a block body."""
    return {key: prop.value for key, prop in _parse_properties_with_spans(body).items()}


def _unquote(value: str) -> str:
    """Strip a single layer of surrounding double quotes, when present."""
    if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
        return value[1:-1]
    return value


def _state_flag(props: dict[str, str]) -> str:
    """Return ``"enabled"`` / ``"disabled"`` for bare state flags in *props*.

    BIG-IP emits ``enabled`` / ``disabled`` as bare tokens (no value),
    which the property parser surfaces as a key with an empty value.
    """
    if "enabled" in props:
        return "enabled"
    if "disabled" in props:
        return "disabled"
    return ""


def _description(props: dict[str, str]) -> str:
    """Return the unquoted ``description`` value from *props*, if any."""
    return _unquote(props.get("description", ""))


def _range_from_token_offsets(source_map: DocumentBuffer, start: int, end_exclusive: int) -> Range:
    """Create an inclusive range from token offsets."""
    end = max(start, end_exclusive - 1)
    return source_map.range_from_offsets(start, end)


def _first_scalar_token_span(value_text: str) -> tuple[int, int] | None:
    """Return ``(start, end)`` (exclusive) for the first scalar token."""
    match = re.search(r"[^\s#{}]+", value_text)
    if not match:
        return None
    return (match.start(), match.end())


def _parse_list_block(braced: str) -> list[str]:
    """Extract top-level item names from a braced block.

    Handles both simple lists (``{ /Common/a /Common/b }``) and nested
    entries (``/Common/web1:80 { address 10.0.1.10 }``), skipping the
    contents of nested sub-blocks.
    """
    inner = braced.strip()
    if inner.startswith("{"):
        inner = inner[1:]
    if inner.endswith("}"):
        inner = inner[:-1]

    items: list[str] = []
    pos = 0
    length = len(inner)

    while pos < length:
        loop_start = pos
        # Skip whitespace
        while pos < length and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break

        # Read the item name (up to whitespace or brace).  Backslash
        # escapes (``\"key\"``) are folded in as opaque pairs so a
        # tmsh-emitted escaped quote inside the name doesn't terminate
        # the token at the trailing ``"``.
        name_start = pos
        while pos < length and inner[pos] not in " \t\n\r{}":
            if inner[pos] == "\\" and pos + 1 < length:
                pos += 2
                continue
            pos += 1
        name = inner[name_start:pos].strip()

        # Skip whitespace after name
        while pos < length and inner[pos] in " \t":
            pos += 1

        # If followed by a brace, skip the sub-block.  Honour the
        # ``\``-escape inside the sub-block too — same reason as
        # above.
        if pos < length and inner[pos] == "{":
            pos += 1
            depth = 1
            while pos < length and depth > 0:
                ch = inner[pos]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                elif ch == "\\" and pos + 1 < length:
                    pos += 1
                pos += 1

        if name and name != "{" and name != "}":
            items.append(name)

        # No-progress guard.  An unmatched closing brace at the top
        # level used to leave ``pos`` unchanged and spin the loop
        # forever (regression from a tmsh-emitted ``records`` block
        # whose escaped keys threw the upstream property-extractor
        # off, leaking a stray ``}`` into the list value).  Treat
        # that as the end of the list rather than hang.
        if pos == loop_start:
            break

    return items


def _parse_keyed_block_entries(braced: str) -> list[tuple[str, str]]:
    """Extract ``(name, body)`` pairs from a keyed-block list.

    Sister to :func:`_parse_list_block` but returns each entry's
    name AND its braced body content (without the surrounding
    ``{`` / ``}``) so callers can hand the body to a typed-value
    parser without re-tokenising the outer list.  Used by the
    security firewall rule-list parser to promote bodies into
    typed :class:`FirewallRule` values.

    Mirrors the no-progress guard from :func:`_parse_list_block`
    so a malformed input can't hang the parser.
    """
    inner = braced.strip()
    if inner.startswith("{"):
        inner = inner[1:]
    if inner.endswith("}"):
        inner = inner[:-1]

    entries: list[tuple[str, str]] = []
    pos = 0
    length = len(inner)

    while pos < length:
        loop_start = pos
        while pos < length and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break

        name_start = pos
        while pos < length and inner[pos] not in " \t\n\r{}":
            if inner[pos] == "\\" and pos + 1 < length:
                pos += 2
                continue
            pos += 1
        name = inner[name_start:pos].strip()

        while pos < length and inner[pos] in " \t":
            pos += 1

        body = ""
        if pos < length and inner[pos] == "{":
            body_start = pos + 1
            pos += 1
            depth = 1
            while pos < length and depth > 0:
                ch = inner[pos]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        break
                elif ch == "\\" and pos + 1 < length:
                    pos += 1
                pos += 1
            body = inner[body_start:pos]
            if pos < length:
                pos += 1  # consume closing ``}``

        if name and name != "{" and name != "}":
            entries.append((name, body))

        if pos == loop_start:
            break

    return entries


def _range_from_offsets(source_map: DocumentBuffer, start: int, end: int) -> Range:
    return source_map.range_from_offsets(start, end)


def _parse_generic_header(header: str) -> tuple[str, str, str] | None:
    """Parse a stanza header into ``(module, type, identifier)``.

    Works for both named and singleton stanzas, including non-LTM modules.
    """
    parts = header.split()
    if len(parts) < 2:
        return None
    module = parts[0]
    if len(parts) == 2:
        return (module, parts[1], "")
    identifier = parts[-1]
    object_type = " ".join(parts[1:-1]).strip()
    if not object_type:
        object_type = parts[1]
        identifier = parts[2] if len(parts) >= 3 else ""
    return (module, object_type, identifier)
