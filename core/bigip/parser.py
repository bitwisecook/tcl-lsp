"""Parser for F5 BIG-IP configuration files (``bigip.conf`` / SCF format).

The BIG-IP configuration format is a nested brace-delimited structure::

    ltm virtual /Common/my_vs {
        destination /Common/10.0.0.1:443
        pool /Common/my_pool
        profiles {
            /Common/http { }
            /Common/clientssl {
                context clientside
            }
        }
        rules {
            /Common/my_irule
        }
    }

This parser extracts structured objects into :class:`BigipConfig`.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ..analysis.semantic_model import Range
from ..common.document_buffer import DocumentBuffer
from .model import (
    BigipConfig,
    BigipDataGroup,
    BigipGenericObject,
    BigipMonitor,
    BigipNetDnsResolver,
    BigipNetInterface,
    BigipNetPortList,
    BigipNetRoute,
    BigipNetRouteDomain,
    BigipNetSelf,
    BigipNetStp,
    BigipNetTunnel,
    BigipNetVlan,
    BigipNode,
    BigipPersistence,
    BigipPolicy,
    BigipPolicyAction,
    BigipPolicyCondition,
    BigipPolicyRule,
    BigipPool,
    BigipPoolMember,
    BigipProfile,
    BigipRule,
    BigipSnatPool,
    BigipVirtualServer,
    DataGroupType,
    ProfileType,
)

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
    relative to *body*. Sub-blocks (``key { ... }``) retain braced text.
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


def _parse_properties(body: str) -> dict[str, str]:
    """Parse simple ``key value`` properties from a block body."""
    return {key: prop.value for key, prop in _parse_properties_with_spans(body).items()}


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


def _range_from_offsets(source_map: DocumentBuffer, start: int, end: int) -> Range:
    return source_map.range_from_offsets(start, end)


# Profile type classification

_PROFILE_TYPE_MAP: dict[str, ProfileType] = {
    "http": ProfileType.HTTP,
    "http2": ProfileType.HTTP,
    "http-compression": ProfileType.HTTP,
    "http-proxy-connect": ProfileType.HTTP,
    "web-acceleration": ProfileType.HTTP,
    "tcp": ProfileType.TCP,
    "udp": ProfileType.UDP,
    "client-ssl": ProfileType.CLIENT_SSL,
    "server-ssl": ProfileType.SERVER_SSL,
    "ftp": ProfileType.FTP,
    "dns": ProfileType.DNS,
    "sip": ProfileType.SIP,
    "diameter": ProfileType.DIAMETER,
    "fix": ProfileType.FIX,
    "radius": ProfileType.RADIUS,
    "mqtt": ProfileType.MQTT,
    "websocket": ProfileType.WEBSOCKET,
    "stream": ProfileType.STREAM,
    "html": ProfileType.HTML,
    "rewrite": ProfileType.REWRITE,
    "fasthttp": ProfileType.FASTHTTP,
    "fastl4": ProfileType.FASTL4,
    "one-connect": ProfileType.ONE_CONNECT,
}


def _classify_profile(type_str: str) -> ProfileType:
    """Map a BIG-IP profile type string to a :class:`ProfileType`."""
    return _PROFILE_TYPE_MAP.get(type_str.lower(), ProfileType.OTHER)


# Object-specific parsers

# Regex to match ltm/gtm stanza headers
_HEADER_RE = re.compile(
    r"^(ltm|gtm|sys|net|auth|security)\s+"
    r"([\w-]+(?:\s+[\w-]+)?)\s+"  # type (possibly two words)
    r"(/[\w/.-]+)$"  # full path
)

_TWO_WORD_TYPES = frozenset(
    {
        "data-group internal",
        "data-group external",
        "profile http",
        "profile http2",
        "profile http-compression",
        "profile http-proxy-connect",
        "profile web-acceleration",
        "profile tcp",
        "profile udp",
        "profile client-ssl",
        "profile server-ssl",
        "profile ftp",
        "profile dns",
        "profile sip",
        "profile diameter",
        "profile fix",
        "profile radius",
        "profile mqtt",
        "profile websocket",
        "profile stream",
        "profile html",
        "profile rewrite",
        "profile fasthttp",
        "profile fastl4",
        "profile one-connect",
        "persistence cookie",
        "persistence dest-addr",
        "persistence hash",
        "persistence msrdp",
        "persistence sip",
        "persistence source-addr",
        "persistence ssl",
        "persistence universal",
        "monitor http",
        "monitor https",
        "monitor tcp",
        "monitor udp",
        "monitor icmp",
        "monitor gateway-icmp",
        "monitor inband",
        "monitor external",
        # net.* — multi-word kinds.
        "tunnels tunnel",
    }
)


def _parse_header(header: str) -> tuple[str, str, str] | None:
    """Parse a stanza header into ``(module, type, full_path)``.

    Returns ``None`` if the header doesn't match the expected format.
    """
    parts = header.split()
    if len(parts) < 3:
        return None
    module = parts[0]
    # Try two-word type first
    if len(parts) >= 4:
        two_word = f"{parts[1]} {parts[2]}"
        if two_word in _TWO_WORD_TYPES:
            return (module, two_word, parts[3])
    # Single-word type
    return (module, parts[1], parts[2])


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


def _parse_data_group(
    full_path: str,
    body: str,
    kind: DataGroupType,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipDataGroup:
    """Parse a ``ltm data-group internal|external`` block."""
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    value_type = props.get("type", "")
    records: list[str] = []
    records_block = props.get("records")
    if records_block:
        records = _parse_list_block(records_block)
    return BigipDataGroup(
        name=name,
        full_path=full_path,
        kind=kind,
        value_type=value_type,
        records=tuple(records),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pool(
    module: str,
    full_path: str,
    body: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipPool:
    """Parse a ``ltm pool`` block."""
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    monitor = props.get("monitor", "")
    lb_mode = props.get("load-balancing-mode", "")
    members: list[BigipPoolMember] = []
    members_block = props.get("members")
    if members_block:
        for member_name in _parse_list_block(members_block):
            members.append(BigipPoolMember(name=member_name))
    return BigipPool(
        name=name,
        full_path=full_path,
        module=module,
        members=tuple(members),
        monitor=monitor,
        load_balancing_mode=lb_mode,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_virtual(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipVirtualServer:
    """Parse a ``ltm virtual`` block."""
    props_with_spans = _parse_properties_with_spans(body)
    props = {key: prop.value for key, prop in props_with_spans.items()}
    name = full_path.rsplit("/", 1)[-1]

    pool = props.get("pool", "")
    destination = props.get("destination", "")
    snatpool = props.get("snatpool", "")
    pool_range: Range | None = None
    pool_prop = props_with_spans.get("pool")
    if (
        pool_prop is not None
        and pool_prop.value_start is not None
        and pool_prop.value_end is not None
    ):
        raw_value = body[pool_prop.value_start : pool_prop.value_end]
        token_span = _first_scalar_token_span(raw_value)
        if token_span is not None:
            body_base = block.start_offset + 1
            abs_start = body_base + pool_prop.value_start + token_span[0]
            abs_end = body_base + pool_prop.value_start + token_span[1]
            pool_range = _range_from_token_offsets(source_map, abs_start, abs_end)

    rules: list[str] = []
    rules_block = props.get("rules")
    if rules_block:
        rules = _parse_list_block(rules_block)

    profiles: list[str] = []
    profiles_block = props.get("profiles")
    if profiles_block:
        profiles = _parse_list_block(profiles_block)

    persist: list[str] = []
    persist_block = props.get("persist")
    if persist_block:
        persist = _parse_list_block(persist_block)

    policies: list[str] = []
    policies_block = props.get("policies")
    if policies_block:
        policies = _parse_list_block(policies_block)

    source_addr_translation = ""
    sat_block = props.get("source-address-translation")
    if sat_block:
        sat_props = _parse_properties(sat_block.strip("{}"))
        source_addr_translation = sat_props.get("type", "")
        if not snatpool:
            snatpool = sat_props.get("pool", "")

    return BigipVirtualServer(
        name=name,
        full_path=full_path,
        destination=destination,
        pool=pool,
        rules=tuple(rules),
        profiles=tuple(profiles),
        persist=tuple(persist),
        policies=tuple(policies),
        snatpool=snatpool,
        source_address_translation=source_addr_translation,
        pool_range=pool_range,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_node(full_path: str, body: str, source_map: DocumentBuffer, block: _Block) -> BigipNode:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipNode(
        name=name,
        full_path=full_path,
        address=props.get("address", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_profile(
    full_path: str, profile_type_str: str, source_map: DocumentBuffer, block: _Block
) -> BigipProfile:
    name = full_path.rsplit("/", 1)[-1]
    return BigipProfile(
        name=name,
        full_path=full_path,
        profile_type=_classify_profile(profile_type_str),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_monitor(
    full_path: str, monitor_type: str, source_map: DocumentBuffer, block: _Block
) -> BigipMonitor:
    name = full_path.rsplit("/", 1)[-1]
    return BigipMonitor(
        name=name,
        full_path=full_path,
        monitor_type=monitor_type,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_snatpool(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSnatPool:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    members: list[str] = []
    members_block = props.get("members")
    if members_block:
        members = _parse_list_block(members_block)
    return BigipSnatPool(
        name=name,
        full_path=full_path,
        members=tuple(members),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_persistence(
    full_path: str, persistence_type: str, source_map: DocumentBuffer, block: _Block
) -> BigipPersistence:
    name = full_path.rsplit("/", 1)[-1]
    return BigipPersistence(
        name=name,
        full_path=full_path,
        persistence_type=persistence_type,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_rule(full_path: str, body: str, source_map: DocumentBuffer, block: _Block) -> BigipRule:
    name = full_path.rsplit("/", 1)[-1]
    return BigipRule(
        name=name,
        full_path=full_path,
        source=body.strip(),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# LTM policy parsing
#
# F5 LTM policies have an unusual grammar: condition / action bodies
# carry positional bare-flag tokens (operand, selector, operator,
# modifiers) interleaved with key/value properties (``name``, ``pool``,
# ``location``) and a ``values { … }`` list block.  We classify each
# bare token by membership in a finite vocabulary rather than parsing
# positionally, since TMSH itself reorders bare tokens between save
# generations.


_POLICY_OPERANDS = frozenset(
    {
        "http-host",
        "http-uri",
        "http-method",
        "http-version",
        "http-status",
        "http-header",
        "http-cookie",
        "http-referer",
        "http-user-agent",
        "tcp",
        "ssl-extension",
        "ssl-cert",
        "geoip",
        # NOTE: ``client-accepted`` is intentionally NOT here even though
        # it appears in some operand documentation — it overlaps with
        # the event vocabulary in _POLICY_EVENTS, and the bare-token
        # classifier resolves operands before events.  Including it
        # would silently turn an event tag into a (broken) operand,
        # losing the event annotation.  If we ever need a real
        # ``client-accepted`` operand, give it a distinct name or add
        # disambiguation logic to the classifier first.
    }
)

_POLICY_SELECTORS = frozenset(
    {
        "host",
        "port",
        "path",
        "query",
        "all",
        "address",
        "version",
        "extension",
        "method",
        "scheme",
    }
)

_POLICY_OPERATORS = frozenset(
    {
        "equals",
        "starts-with",
        "ends-with",
        "contains",
        "matches",
        "exists",
        "missing",
        "less",
        "greater",
        "less-or-equal",
        "greater-or-equal",
    }
)

_POLICY_EVENTS = frozenset(
    {
        "request",
        "response",
        "client-accepted",
        "server-connected",
        "ssl-client-hello",
        "ssl-server-hello",
        "proxy-request",
        "proxy-response",
        "websocket-request",
        "websocket-response",
    }
)

_POLICY_ACTION_TARGETS = frozenset(
    {
        "forward",
        "http-reply",
        "http-uri",
        "http-host",
        "http-header",
        "http-cookie",
        "tcp",
        "log",
        "shutdown",
    }
)

_POLICY_ACTION_VERBS = frozenset(
    {
        "select",
        "redirect",
        "replace",
        "insert",
        "remove",
        "reset",
        "drop",
        "rewrite",
        "enable",
        "disable",
    }
)


def _strip_outer_braces(text: str) -> str:
    """Remove the surrounding ``{ … }`` from a sub-block value, if present."""
    s = text.strip()
    if s.startswith("{"):
        s = s[1:]
    if s.endswith("}"):
        s = s[:-1]
    return s


def _strip_quotes(text: str) -> str:
    s = text.strip()
    if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
        return s[1:-1]
    return s


def _parse_policy_values_block(braced: str) -> list[str]:
    """Quote-aware tokeniser for policy condition ``values { … }`` lists.

    ``_parse_list_block`` is whitespace-only and turns
    ``values { "Mozilla/5.0 (iPhone; CPU)" }`` into four tokens — that
    corrupts UA strings, regex literals, and any header value with
    spaces.  Policy values therefore need their own parser that
    respects double-quoted strings (with ``\\`` escapes), strips the
    surrounding quotes from each emitted value, and otherwise behaves
    like the list-block parser (whitespace-separated bare tokens).
    """
    inner = _strip_outer_braces(braced)
    out: list[str] = []
    pos = 0
    length = len(inner)
    while pos < length:
        ch = inner[pos]
        if ch in " \t\n\r":
            pos += 1
            continue
        if ch == '"':
            pos += 1
            buf: list[str] = []
            while pos < length and inner[pos] != '"':
                if inner[pos] == "\\" and pos + 1 < length:
                    buf.append(inner[pos + 1])
                    pos += 2
                    continue
                buf.append(inner[pos])
                pos += 1
            if pos < length and inner[pos] == '"':
                pos += 1  # consume closing quote
            out.append("".join(buf))
            continue
        # Bare token: read until whitespace.
        start = pos
        while pos < length and inner[pos] not in " \t\n\r":
            pos += 1
        out.append(inner[start:pos])
    return out


def _parse_policy_condition(index: int, body: str) -> BigipPolicyCondition:
    """Parse a single ``conditions { N { … } }`` body."""
    props = _parse_properties(body)
    operand = ""
    selector = ""
    operator = "equals"
    negate = False
    case_insensitive = False
    event = ""
    name = ""
    values: list[str] = []
    for key, value in props.items():
        if value:
            if key == "values":
                values = _parse_policy_values_block(value)
            elif key in ("name", "tm-name"):
                name = _strip_quotes(value)
            continue
        # Bare flag token: classify by vocabulary.
        if key in _POLICY_OPERANDS and not operand:
            operand = key
        elif key in _POLICY_SELECTORS and not selector:
            selector = key
        elif key in _POLICY_OPERATORS:
            operator = key
        elif key in _POLICY_EVENTS:
            event = key
        elif key == "not":
            negate = True
        elif key == "case-insensitive":
            case_insensitive = True
    return BigipPolicyCondition(
        index=index,
        operand=operand,
        selector=selector,
        operator=operator,
        values=tuple(values),
        name=name,
        negate=negate,
        case_insensitive=case_insensitive,
        event=event,
    )


def _parse_policy_action(index: int, body: str) -> BigipPolicyAction:
    """Parse a single ``actions { N { … } }`` body."""
    props = _parse_properties(body)
    target = ""
    verb = ""
    pool = ""
    location = ""
    name = ""
    value = ""
    path = ""
    query = ""
    host = ""
    event = ""
    for key, val in props.items():
        if val:
            if key == "pool":
                pool = val.strip()
            elif key == "location":
                location = _strip_quotes(val)
            elif key in ("name", "tm-name"):
                name = _strip_quotes(val)
            elif key == "value":
                value = _strip_quotes(val)
            elif key == "path":
                path = _strip_quotes(val)
            elif key == "query":
                query = _strip_quotes(val)
            elif key == "host":
                # ``http-uri replace host www.example.com`` — TMSH puts
                # the host component in the same key=value form as path
                # / query.  Without this branch the token would be
                # silently dropped from the parsed action.
                host = _strip_quotes(val)
            continue
        if key in _POLICY_ACTION_TARGETS and not target:
            target = key
        elif key in _POLICY_ACTION_VERBS and not verb:
            verb = key
        elif key in _POLICY_EVENTS:
            event = key
    return BigipPolicyAction(
        index=index,
        target=target,
        verb=verb,
        pool=pool,
        location=location,
        name=name,
        value=value,
        path=path,
        query=query,
        host=host,
        event=event,
    )


def _parse_policy_rule(name: str, body: str) -> BigipPolicyRule:
    props_with_spans = _parse_properties_with_spans(body)
    props = {key: prop.value for key, prop in props_with_spans.items()}
    try:
        ordinal = int(props.get("ordinal", "0").strip())
    except ValueError:
        ordinal = 0

    conditions: list[BigipPolicyCondition] = []
    cond_block = props.get("conditions", "")
    if cond_block:
        for cond_idx_str, cond_prop in _parse_properties_with_spans(
            _strip_outer_braces(cond_block)
        ).items():
            try:
                idx = int(cond_idx_str)
            except ValueError:
                continue
            conditions.append(_parse_policy_condition(idx, _strip_outer_braces(cond_prop.value)))

    actions: list[BigipPolicyAction] = []
    act_block = props.get("actions", "")
    if act_block:
        for act_idx_str, act_prop in _parse_properties_with_spans(
            _strip_outer_braces(act_block)
        ).items():
            try:
                idx = int(act_idx_str)
            except ValueError:
                continue
            actions.append(_parse_policy_action(idx, _strip_outer_braces(act_prop.value)))

    conditions.sort(key=lambda c: c.index)
    actions.sort(key=lambda a: a.index)
    return BigipPolicyRule(
        name=name,
        ordinal=ordinal,
        conditions=tuple(conditions),
        actions=tuple(actions),
    )


def _parse_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPolicy:
    """Parse a ``ltm policy`` block."""
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    strategy = (props.get("strategy", "") or "first-match").strip()
    # ``first-match`` is canonicalised — TMSH sometimes prepends the
    # path of a published strategy (``/Common/first-match``).
    strategy = strategy.rsplit("/", 1)[-1]

    requires: list[str] = []
    requires_block = props.get("requires", "")
    if requires_block:
        requires = _parse_list_block(requires_block)
    controls: list[str] = []
    controls_block = props.get("controls", "")
    if controls_block:
        controls = _parse_list_block(controls_block)

    rules: list[BigipPolicyRule] = []
    rules_block = props.get("rules", "")
    if rules_block:
        for rule_name, rule_prop in _parse_properties_with_spans(
            _strip_outer_braces(rules_block)
        ).items():
            rules.append(_parse_policy_rule(rule_name, _strip_outer_braces(rule_prop.value)))
    rules.sort(key=lambda r: (r.ordinal, r.name))

    return BigipPolicy(
        name=name,
        full_path=full_path,
        strategy=strategy,
        requires=tuple(requires),
        controls=tuple(controls),
        rules=tuple(rules),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# ── net.* parsers ────────────────────────────────────────────────────


def _parse_net_route(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetRoute:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipNetRoute(
        name=name,
        full_path=full_path,
        network=props.get("network", ""),
        gw=props.get("gw", ""),
        pool=props.get("pool", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_vlan(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetVlan:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    try:
        tag = int(props["tag"].value) if "tag" in props else 0
    except ValueError:
        tag = 0
    interfaces: tuple[str, ...] = ()
    if "interfaces" in props:
        interfaces = tuple(_parse_list_block(props["interfaces"].value))
    return BigipNetVlan(
        name=name,
        full_path=full_path,
        tag=tag,
        interfaces=interfaces,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_self(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetSelf:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    allow_service: tuple[str, ...] = ()
    if "allow-service" in props:
        value = props["allow-service"].value
        if value.startswith("{"):
            allow_service = tuple(_parse_list_block(value))
        elif value:
            # ``allow-service none`` / ``allow-service default`` (bare).
            allow_service = (value,)
    return BigipNetSelf(
        name=name,
        full_path=full_path,
        address=props["address"].value if "address" in props else "",
        vlan=props["vlan"].value if "vlan" in props else "",
        traffic_group=props["traffic-group"].value if "traffic-group" in props else "",
        allow_service=allow_service,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_route_domain(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetRouteDomain:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    try:
        id_val = int(props["id"].value) if "id" in props else 0
    except ValueError:
        id_val = 0
    vlans: tuple[str, ...] = ()
    if "vlans" in props:
        vlans = tuple(_parse_list_block(props["vlans"].value))
    return BigipNetRouteDomain(
        name=name,
        full_path=full_path,
        id=id_val,
        vlans=vlans,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_port_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetPortList:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    ports: tuple[str, ...] = ()
    if "ports" in props:
        ports = tuple(_parse_list_block(props["ports"].value))
    return BigipNetPortList(
        name=name,
        full_path=full_path,
        ports=ports,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_interface(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetInterface:
    props = _parse_properties(body)
    # ``net interface`` uses a bare slot/port name (``1.1``, ``mgmt``)
    # without a partition prefix.  Keep ``name`` and ``full_path`` as
    # the same bare token so downstream consumers don't have to
    # special-case the missing prefix.
    return BigipNetInterface(
        name=full_path,
        full_path=full_path,
        media_fixed=props.get("media-fixed", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_dns_resolver(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetDnsResolver:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    forward_zones: tuple[str, ...] = ()
    if "forward-zones" in props:
        # The forward-zones block is keyed by domain name.  Use the
        # list-block parser to extract the top-level keys; nested
        # ``nameservers { ... }`` sub-blocks are skipped.
        forward_zones = tuple(_parse_list_block(props["forward-zones"].value))
    return BigipNetDnsResolver(
        name=name,
        full_path=full_path,
        route_domain=props["route-domain"].value if "route-domain" in props else "",
        forward_zones=forward_zones,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_tunnel(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetTunnel:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    description = ""
    if "description" in props:
        desc = props["description"].value
        # Description values are usually quoted; strip the outermost
        # double quotes when present so users see the raw text.
        if desc.startswith('"') and desc.endswith('"'):
            description = desc[1:-1]
        else:
            description = desc
    return BigipNetTunnel(
        name=name,
        full_path=full_path,
        profile=props["profile"].value if "profile" in props else "",
        local_address=props["local-address"].value if "local-address" in props else "",
        remote_address=props["remote-address"].value if "remote-address" in props else "",
        description=description,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_stp(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetStp:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    interfaces: tuple[str, ...] = ()
    if "interfaces" in props:
        interfaces = tuple(_parse_list_block(props["interfaces"].value))
    return BigipNetStp(
        name=name,
        full_path=full_path,
        interfaces=interfaces,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Public API


def parse_bigip_conf(source: str) -> BigipConfig:
    """Parse a BIG-IP configuration file and return a :class:`BigipConfig`.

    Handles ``ltm`` and ``gtm`` stanzas.  Unknown stanza types are silently
    skipped.
    """
    config = BigipConfig()
    blocks = _extract_blocks(source)
    source_map = DocumentBuffer.from_source(source)

    for block in blocks:
        generic = _parse_generic_header(block.header)
        if generic is not None:
            module_g, obj_type_g, identifier_g = generic
            generic_key = f"{module_g}::{obj_type_g}::{identifier_g or '<singleton>'}"
            config.generic_objects[generic_key] = BigipGenericObject(
                module=module_g,
                object_type=obj_type_g,
                identifier=identifier_g,
                header=block.header,
                range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
            )

        parsed = _parse_header(block.header)
        if parsed is None:
            continue
        module, obj_type, full_path = parsed

        if module == "net":
            # ``net`` module has its own dispatch.  Unknown sub-types
            # fall through to ``generic_objects`` via the earlier
            # branch (which ran for every block above).
            if obj_type == "route":
                config.net_routes[full_path] = _parse_net_route(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "vlan":
                config.net_vlans[full_path] = _parse_net_vlan(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "self":
                config.net_selves[full_path] = _parse_net_self(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "route-domain":
                config.net_route_domains[full_path] = _parse_net_route_domain(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "port-list":
                config.net_port_lists[full_path] = _parse_net_port_list(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "interface":
                config.net_interfaces[full_path] = _parse_net_interface(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "dns-resolver":
                config.net_dns_resolvers[full_path] = _parse_net_dns_resolver(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "tunnels tunnel":
                config.net_tunnels[full_path] = _parse_net_tunnel(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "stp":
                config.net_stps[full_path] = _parse_net_stp(
                    full_path, block.body, source_map, block
                )
            continue

        if module not in ("ltm", "gtm"):
            continue

        match obj_type:
            case "data-group internal":
                dg = _parse_data_group(
                    full_path, block.body, DataGroupType.INTERNAL, source_map, block
                )
                config.data_groups[full_path] = dg
            case "data-group external":
                dg = _parse_data_group(
                    full_path, block.body, DataGroupType.EXTERNAL, source_map, block
                )
                config.data_groups[full_path] = dg
            case "pool":
                if module == "ltm":
                    pool = _parse_pool(module, full_path, block.body, source_map, block)
                    config.pools[full_path] = pool
            case "virtual":
                vs = _parse_virtual(full_path, block.body, source_map, block)
                config.virtual_servers[full_path] = vs
            case "node":
                node = _parse_node(full_path, block.body, source_map, block)
                config.nodes[full_path] = node
            case "snatpool":
                snatpool = _parse_snatpool(full_path, block.body, source_map, block)
                config.snat_pools[full_path] = snatpool
            case "rule":
                rule = _parse_rule(full_path, block.body, source_map, block)
                config.rules[full_path] = rule
            case "policy":
                if module == "ltm":
                    policy = _parse_policy(full_path, block.body, source_map, block)
                    config.policies[full_path] = policy
            case _ if obj_type.startswith("profile "):
                profile_type_str = obj_type.split(" ", 1)[1]
                profile = _parse_profile(full_path, profile_type_str, source_map, block)
                config.profiles[full_path] = profile
            case _ if obj_type.startswith("persistence "):
                persistence_type = obj_type.split(" ", 1)[1]
                persist = _parse_persistence(full_path, persistence_type, source_map, block)
                config.persistence[full_path] = persist
            case _ if obj_type.startswith("monitor "):
                monitor_type = obj_type.split(" ", 1)[1]
                monitor = _parse_monitor(full_path, monitor_type, source_map, block)
                config.monitors[full_path] = monitor

    return config
