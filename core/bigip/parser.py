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
                    elif ch == '"':
                        # Skip quoted string
                        pos += 1
                        while pos < length and source[pos] != '"':
                            if source[pos] == "\\":
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
                elif ch == '"':
                    pos += 1
                    while pos < length and body[pos] != '"':
                        if body[pos] == "\\":
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
        # Skip whitespace
        while pos < length and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break

        # Read the item name (up to whitespace or brace)
        name_start = pos
        while pos < length and inner[pos] not in " \t\n\r{}":
            pos += 1
        name = inner[name_start:pos].strip()

        # Skip whitespace after name
        while pos < length and inner[pos] in " \t":
            pos += 1

        # If followed by a brace, skip the sub-block
        if pos < length and inner[pos] == "{":
            pos += 1
            depth = 1
            while pos < length and depth > 0:
                if inner[pos] == "{":
                    depth += 1
                elif inner[pos] == "}":
                    depth -= 1
                pos += 1

        if name and name != "{" and name != "}":
            items.append(name)

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
        "client-accepted",
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
                values = _parse_list_block(value)
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
            conditions.append(
                _parse_policy_condition(idx, _strip_outer_braces(cond_prop.value))
            )

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
            rules.append(
                _parse_policy_rule(rule_name, _strip_outer_braces(rule_prop.value))
            )
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
