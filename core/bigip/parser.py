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
    BigipApmEphemeralAuthSshSecurityConfig,
    BigipApmOauthDbInstance,
    BigipApmPolicyAccessPolicy,
    BigipApmPolicyAgent,
    BigipApmPolicyCustomizationSource,
    BigipApmPolicyItem,
    BigipApmReportDefaultReport,
    BigipCmCert,
    BigipCmDevice,
    BigipCmDeviceGroup,
    BigipCmKey,
    BigipCmTrafficGroup,
    BigipCmTrustDomain,
    BigipConfig,
    BigipDataGroup,
    BigipGenericObject,
    BigipGtmDatacenter,
    BigipGtmPool,
    BigipGtmProberPool,
    BigipGtmRegion,
    BigipGtmRule,
    BigipGtmServer,
    BigipGtmWideip,
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
    BigipPemForwardingEndpoint,
    BigipPemInterceptionEndpoint,
    BigipPemListener,
    BigipPemPolicy,
    BigipPemProfile,
    BigipPemRatingGroup,
    BigipPemRule,
    BigipPemServiceChainEndpoint,
    BigipPersistence,
    BigipPolicy,
    BigipPolicyAction,
    BigipPolicyCondition,
    BigipPolicyRule,
    BigipPool,
    BigipPoolMember,
    BigipProfile,
    BigipRule,
    BigipSecurityDeviceIdAttribute,
    BigipSecurityFirewallConfigEntityId,
    BigipSecurityFirewallPortList,
    BigipSecurityFirewallRuleList,
    BigipSecurityIpIntelligencePolicy,
    BigipSecurityProtocolInspectionComplianceMap,
    BigipSecurityProtocolInspectionComplianceObject,
    BigipSnatPool,
    BigipSysDns,
    BigipSysFileSslCert,
    BigipSysFileSslKey,
    BigipSysFolder,
    BigipSysGlobalSettings,
    BigipSysManagementRoute,
    BigipSysNtp,
    BigipSysProvision,
    BigipSysSnmp,
    BigipVirtualAddress,
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
        # sys.* — multi-word kinds.
        "file ssl-cert",
        "file ssl-key",
        # security.* — multi-word kinds.
        "firewall port-list",
        "firewall rule-list",
        "firewall config-entity-id",
        "ip-intelligence policy",
        "protocol-inspection compliance-map",
        "protocol-inspection compliance-objects",
        "device-id attribute",
        # apm.* — multi-word kinds.
        "ephemeral-auth ssh-security-config",
        "oauth db-instance",
        "policy access-policy",
        "policy customization-source",
        "policy policy-item",
        "report default-report",  # also a singleton (``apm report default-report {``)
        # gtm.* — record-type-tagged kinds.
        "pool a",
        "pool aaaa",
        "pool cname",
        "pool mx",
        "pool srv",
        "pool naptr",
        "wideip a",
        "wideip aaaa",
        "wideip cname",
        "wideip mx",
        "wideip srv",
        "wideip naptr",
        # pem.* — multi-word kinds.
        "profile diameter-endpoint",
        "profile radius-aaa",
        "profile spm",
        "profile subscriber-mgmt",
        "quota-mgmt rating-group",
    }
)


# Three-word stanza types (``apm policy agent <type>``, …).  Extracted
# the same way as two-word: matched against ``parts[1..3]`` and the
# identifier comes from ``parts[4]``.
# ``apm policy agent <type>`` covers ~50 sub-types — list each one so
# the header parser recognises the three-word kind and the dispatch
# below routes every variant into ``apm_policy_agents``.
_APM_POLICY_AGENT_TYPES = (
    "aaa-active-directory",
    "aaa-client-cert",
    "aaa-crldp",
    "aaa-http",
    "aaa-ldap",
    "aaa-oauth",
    "aaa-ocsp",
    "aaa-radius",
    "aaa-saml",
    "aaa-securid",
    "acct-radius",
    "acct-tacacsplus",
    "api-authentication",
    "api-server-selection",
    "decision-box",
    "dynamic-acl",
    "ending-allow",
    "ending-deny",
    "ending-redirect",
    "endpoint-check-machine-cert",
    "endpoint-check-software",
    "endpoint-linux-check-file",
    "endpoint-linux-check-process",
    "endpoint-mac-check-file",
    "endpoint-mac-check-process",
    "endpoint-machine-info",
    "endpoint-windows-browser-cache-cleaner",
    "endpoint-windows-check-file",
    "endpoint-windows-check-process",
    "endpoint-windows-check-registry",
    "endpoint-windows-group-policy",
    "endpoint-windows-info-os",
    "endpoint-windows-protected-workspace",
    "external-logon-page",
    "http-header-modify",
    "ip-geolocation-lookup",
    "ip-reputation-lookup",
    "irule-event",
    "kerberos",
    "l7-protocol-lookup",
    "logging",
    "logon-page",
    "message-box",
    "oam",
    "oauth-authz",
    "request-classification",
    "resource-assign",
    "response-selection",
    "route-domain-selection",
    "server-cert-response-control",
    "server-cert-status",
    "session-check",
    "ssl-check",
    "tacacsplus",
    "variable-assign",
)

_THREE_WORD_TYPES = frozenset(f"policy agent {t}" for t in _APM_POLICY_AGENT_TYPES)


def _parse_header(header: str) -> tuple[str, str, str] | None:
    """Parse a stanza header into ``(module, type, full_path)``.

    Returns ``None`` if the header doesn't match the expected format.
    """
    parts = header.split()
    if len(parts) < 3:
        return None
    module = parts[0]
    # Try three-word type first (apm policy agent <type> /Common/X).
    if len(parts) >= 5:
        three_word = f"{parts[1]} {parts[2]} {parts[3]}"
        if three_word in _THREE_WORD_TYPES:
            return (module, three_word, parts[4])
    # Two-word singleton (``apm report default-report {``) — 3 tokens
    # total and parts[1..2] is a known two-word kind.
    if len(parts) == 3:
        two_word = f"{parts[1]} {parts[2]}"
        if two_word in _TWO_WORD_TYPES:
            return (module, two_word, "")
    # Two-word type
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
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pool_member_body(braced_body: str) -> dict[str, str]:
    """Parse the per-member ``{ ... }`` body of a pool members entry."""
    inner = braced_body.strip()
    if inner.startswith("{"):
        inner = inner[1:]
    if inner.endswith("}"):
        inner = inner[:-1]
    return _parse_properties(inner)


def _parse_pool_members(braced: str) -> list[BigipPoolMember]:
    """Extract pool members from a ``members { ... }`` block.

    Each member entry may carry an inline body — ``/Common/n:80 { address ... }``
    — and the per-member properties are surfaced on
    :class:`BigipPoolMember` so projections (and the query DSL) can
    interrogate state, ratio, and monitor without reaching back into
    the source.
    """
    members: list[BigipPoolMember] = []
    inner = braced.strip()
    if inner.startswith("{"):
        inner = inner[1:]
    if inner.endswith("}"):
        inner = inner[:-1]

    pos = 0
    length = len(inner)
    while pos < length:
        while pos < length and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break
        name_start = pos
        while pos < length and inner[pos] not in " \t\n\r{":
            pos += 1
        name = inner[name_start:pos]
        if not name:
            pos += 1
            continue
        while pos < length and inner[pos] in " \t":
            pos += 1
        body_text = ""
        if pos < length and inner[pos] == "{":
            body_start = pos
            pos += 1
            depth = 1
            while pos < length and depth > 0:
                ch = inner[pos]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                pos += 1
            body_text = inner[body_start:pos]
        props = _parse_pool_member_body(body_text) if body_text else {}
        addr = props.get("address", "")
        port = 0
        if ":" in name and not addr:
            tail = name.rsplit(":", 1)[-1]
            try:
                port = int(tail)
            except ValueError:
                port = 0
        if not port:
            try:
                port = int(props.get("port", "0"))
            except ValueError:
                port = 0
        members.append(
            BigipPoolMember(
                name=name,
                address=addr,
                port=port,
                monitor=props.get("monitor", ""),
                description=_unquote(props.get("description", "")),
                state=_state_flag(props),
                ratio=props.get("ratio", ""),
                priority_group=props.get("priority-group", ""),
                connection_limit=props.get("connection-limit", ""),
                rate_limit=props.get("rate-limit", ""),
            )
        )
    return members


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
        members = _parse_pool_members(members_block)
    profiles: tuple[str, ...] = ()
    profiles_block = props.get("profiles")
    if profiles_block:
        profiles = tuple(_parse_list_block(profiles_block))
    return BigipPool(
        name=name,
        full_path=full_path,
        module=module,
        members=tuple(members),
        monitor=monitor,
        load_balancing_mode=lb_mode,
        description=_description(props),
        min_active_members=props.get("min-active-members", ""),
        min_up_members=props.get("min-up-members", ""),
        service_down_action=props.get("service-down-action", ""),
        slow_ramp_time=props.get("slow-ramp-time", ""),
        allow_snat=props.get("allow-snat", ""),
        allow_nat=props.get("allow-nat", ""),
        reselect_tries=props.get("reselect-tries", ""),
        queue_depth_limit=props.get("queue-depth-limit", ""),
        queue_time_limit=props.get("queue-time-limit", ""),
        connection_limit=props.get("connection-limit", ""),
        rate_limit=props.get("rate-limit", ""),
        ratio=props.get("ratio", ""),
        down_interval=props.get("down-interval", ""),
        interval=props.get("interval", ""),
        min_up_members_action=props.get("min-up-members-action", ""),
        min_up_members_checking=props.get("min-up-members-checking", ""),
        ip_tos_to_client=props.get("ip-tos-to-client", ""),
        ip_tos_to_server=props.get("ip-tos-to-server", ""),
        link_qos_to_client=props.get("link-qos-to-client", ""),
        link_qos_to_server=props.get("link-qos-to-server", ""),
        gateway_failsafe_device=props.get("gateway-failsafe-device", ""),
        ignore_persisted_weight=props.get("ignore-persisted-weight", ""),
        inherit_profile=props.get("inherit-profile", ""),
        queue_on_connection_limit=props.get("queue-on-connection-limit", ""),
        address_family=props.get("address-family", ""),
        autopopulate=props.get("autopopulate", ""),
        profiles=profiles,
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

    vlans: tuple[str, ...] = ()
    vlans_block = props.get("vlans")
    if vlans_block:
        vlans = tuple(_parse_list_block(vlans_block))

    auth_profiles: tuple[str, ...] = ()
    auth_block = props.get("auth")
    if auth_block:
        auth_profiles = tuple(_parse_list_block(auth_block))

    traffic_classes: tuple[str, ...] = ()
    tc_block = props.get("traffic-classes")
    if tc_block:
        traffic_classes = tuple(_parse_list_block(tc_block))

    clone_pools: tuple[str, ...] = ()
    cp_block = props.get("clone-pools")
    if cp_block:
        clone_pools = tuple(_parse_list_block(cp_block))

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
        description=_description(props),
        mask=props.get("mask", ""),
        source=props.get("source", ""),
        ip_protocol=props.get("ip-protocol", ""),
        connection_limit=props.get("connection-limit", ""),
        rate_limit=props.get("rate-limit", ""),
        rate_limit_mode=props.get("rate-limit-mode", ""),
        auto_lasthop=props.get("auto-lasthop", ""),
        translate_address=props.get("translate-address", ""),
        translate_port=props.get("translate-port", ""),
        state=_state_flag(props),
        address_status=props.get("address-status", ""),
        auto_discovery=props.get("auto-discovery", ""),
        cmp_enabled=props.get("cmp-enabled", ""),
        eviction_protected=props.get("eviction-protected", ""),
        dhcp_relay="dhcp-relay" in props,
        internal="internal" in props,
        ip_forward="ip-forward" in props,
        l2_forward="l2-forward" in props,
        reject="reject" in props,
        nat64=props.get("nat64", ""),
        gtm_score=props.get("gtm-score", ""),
        mirror=props.get("mirror", ""),
        service_down_immediate_action=props.get("service-down-immediate-action", ""),
        source_port=props.get("source-port", ""),
        serverssl_use_sni=props.get("serverssl-use-sni", ""),
        rate_limit_dst_mask=props.get("rate-limit-dst-mask", ""),
        rate_limit_src_mask=props.get("rate-limit-src-mask", ""),
        rate_class=props.get("rate-class", ""),
        per_flow_request_access_policy=props.get("per-flow-request-access-policy", ""),
        transparent_nexthop=props.get("transparent-nexthop", ""),
        vlans=vlans,
        vlans_disabled="vlans-disabled" in props,
        vlans_enabled="vlans-enabled" in props,
        fallback_persistence=props.get("fallback-persistence", ""),
        last_hop_pool=props.get("last-hop-pool", ""),
        fw_enforced_policy=props.get("fw-enforced-policy", ""),
        fw_staged_policy=props.get("fw-staged-policy", ""),
        flow_eviction_policy=props.get("flow-eviction-policy", ""),
        service_policy=props.get("service-policy", ""),
        auth_profiles=auth_profiles,
        traffic_classes=traffic_classes,
        clone_pools=clone_pools,
        pool_range=pool_range,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_node(full_path: str, body: str, source_map: DocumentBuffer, block: _Block) -> BigipNode:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    fqdn = ""
    fqdn_block = props.get("fqdn", "")
    if fqdn_block.startswith("{"):
        fqdn_props = _parse_properties(fqdn_block.strip("{}"))
        fqdn = fqdn_props.get("name", "")
    return BigipNode(
        name=name,
        full_path=full_path,
        address=props.get("address", ""),
        description=_description(props),
        monitor=props.get("monitor", ""),
        state=_state_flag(props),
        connection_limit=props.get("connection-limit", ""),
        rate_limit=props.get("rate-limit", ""),
        ratio=props.get("ratio", ""),
        fqdn=fqdn,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_virtual_address(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipVirtualAddress:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipVirtualAddress(
        name=name,
        full_path=full_path,
        address=props.get("address", ""),
        mask=props.get("mask", ""),
        arp=props.get("arp", ""),
        icmp_echo=props.get("icmp-echo", ""),
        auto_delete=props.get("auto-delete", ""),
        connection_limit=props.get("connection-limit", ""),
        traffic_group=props.get("traffic-group", ""),
        inherited_traffic_group=props.get("inherited-traffic-group", ""),
        route_advertisement=props.get("route-advertisement", ""),
        server_scope=props.get("server-scope", ""),
        spanning=props.get("spanning", ""),
        unit=props.get("unit", ""),
        description=_description(props),
        state=_state_flag(props),
        floating=props.get("floating", ""),
        traffic_group_restored=props.get("traffic-group-restored", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_profile(
    full_path: str,
    profile_type_str: str,
    body: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipProfile(
        name=name,
        full_path=full_path,
        profile_type=_classify_profile(profile_type_str),
        defaults_from=props.get("defaults-from", ""),
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_monitor(
    full_path: str,
    monitor_type: str,
    body: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipMonitor:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipMonitor(
        name=name,
        full_path=full_path,
        monitor_type=monitor_type,
        defaults_from=props.get("defaults-from", ""),
        description=_description(props),
        interval=props.get("interval", ""),
        timeout=props.get("timeout", ""),
        destination=props.get("destination", ""),
        send=_unquote(props.get("send", "")),
        recv=_unquote(props.get("recv", "")),
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
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_persistence(
    full_path: str,
    persistence_type: str,
    body: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipPersistence:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipPersistence(
        name=name,
        full_path=full_path,
        persistence_type=persistence_type,
        defaults_from=props.get("defaults-from", ""),
        description=_description(props),
        timeout=props.get("timeout", ""),
        match_across_pools=props.get("match-across-pools", ""),
        match_across_services=props.get("match-across-services", ""),
        match_across_virtuals=props.get("match-across-virtuals", ""),
        mirror=props.get("mirror", ""),
        override_connection_limit=props.get("override-connection-limit", ""),
        always_send=props.get("always-send", ""),
        cookie_name=_unquote(props.get("cookie-name", "")),
        cookie_encryption=props.get("cookie-encryption", ""),
        cookie_encryption_passphrase=props.get("cookie-encryption-passphrase", ""),
        httponly=props.get("httponly", ""),
        secure=props.get("secure", ""),
        expiration=props.get("expiration", ""),
        method=props.get("method", ""),
        hash_length=props.get("hash-length", ""),
        hash_offset=props.get("hash-offset", ""),
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
        description=_description(props),
        status=props.get("status", ""),
        last_modified=_unquote(props.get("last-modified", "")),
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
        description=_description(props),
        mtu=props.get("mtu", ""),
        blackhole="blackhole" in props,
        interface=props.get("interface", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_vlan(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetVlan:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
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
        description=_description(plain),
        mtu=plain.get("mtu", ""),
        cmp_hash=plain.get("cmp-hash", ""),
        failsafe=plain.get("failsafe", ""),
        failsafe_action=plain.get("failsafe-action", ""),
        failsafe_timeout=plain.get("failsafe-timeout", ""),
        fwd_mode=plain.get("fwd-mode", ""),
        hardware_syncookie=plain.get("hardware-syncookie", ""),
        learning=plain.get("learning", ""),
        tag_mode=plain.get("tag-mode", ""),
        virtual_wire=plain.get("virtual-wire", ""),
        auto_lasthop=plain.get("auto-lasthop", ""),
        source_check=plain.get("source-check", ""),
        source_checking=plain.get("source-checking", ""),
        syn_flood_rate_limit=plain.get("syn-flood-rate-limit", ""),
        syncache_threshold=plain.get("syncache-threshold", ""),
        service_policy=plain.get("service-policy", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_self(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetSelf:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
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
        address=plain.get("address", ""),
        vlan=plain.get("vlan", ""),
        traffic_group=plain.get("traffic-group", ""),
        allow_service=allow_service,
        description=_description(plain),
        floating=plain.get("floating", ""),
        unit=plain.get("unit", ""),
        service_policy=plain.get("service-policy", ""),
        fw_enforced_policy=plain.get("fw-enforced-policy", ""),
        fw_staged_policy=plain.get("fw-staged-policy", ""),
        inherited_traffic_group=plain.get("inherited-traffic-group", ""),
        address_source=plain.get("address-source", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_route_domain(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetRouteDomain:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    try:
        id_val = int(props["id"].value) if "id" in props else 0
    except ValueError:
        id_val = 0
    vlans: tuple[str, ...] = ()
    if "vlans" in props:
        vlans = tuple(_parse_list_block(props["vlans"].value))
    routing_protocol: tuple[str, ...] = ()
    if "routing-protocol" in props:
        routing_protocol = tuple(_parse_list_block(props["routing-protocol"].value))
    return BigipNetRouteDomain(
        name=name,
        full_path=full_path,
        id=id_val,
        vlans=vlans,
        description=_description(plain),
        parent=plain.get("parent", ""),
        strict=plain.get("strict", ""),
        fw_enforced_policy=plain.get("fw-enforced-policy", ""),
        fw_staged_policy=plain.get("fw-staged-policy", ""),
        bwc_policy=plain.get("bwc-policy", ""),
        connection_limit=plain.get("connection-limit", ""),
        flow_eviction_policy=plain.get("flow-eviction-policy", ""),
        routing_protocol=routing_protocol,
        security_nat_policy=plain.get("security-nat-policy", ""),
        service_policy=plain.get("service-policy", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_port_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetPortList:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    ports: tuple[str, ...] = ()
    if "ports" in props:
        ports = tuple(_parse_list_block(props["ports"].value))
    return BigipNetPortList(
        name=name,
        full_path=full_path,
        ports=ports,
        description=_description(plain),
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
    sflow_poll_interval = ""
    sflow_poll_interval_global = ""
    sflow_block = props.get("sflow")
    if sflow_block and sflow_block.startswith("{"):
        sflow_inner = _parse_properties(sflow_block.strip("{}"))
        sflow_poll_interval = sflow_inner.get("poll-interval", "")
        sflow_poll_interval_global = sflow_inner.get("poll-interval-global", "")
    return BigipNetInterface(
        name=full_path,
        full_path=full_path,
        media_fixed=props.get("media-fixed", ""),
        description=_description(props),
        enabled="enabled" in props,
        disabled="disabled" in props,
        bundle=props.get("bundle", ""),
        bundle_speed=props.get("bundle-speed", ""),
        lldp_admin=props.get("lldp-admin", ""),
        mtu=props.get("mtu", ""),
        flow_control=props.get("flow-control", ""),
        mac_address=props.get("mac-address", ""),
        media_active=props.get("media-active", ""),
        media_max=props.get("media-max", ""),
        media_sfp=props.get("media-sfp", ""),
        port_fwd_mode=props.get("port-fwd-mode", ""),
        qinq_ethertype=props.get("qinq-ethertype", ""),
        stp=props.get("stp", ""),
        stp_edge_port=props.get("stp-edge-port", ""),
        stp_link_type=props.get("stp-link-type", ""),
        stp_auto_edge_port=props.get("stp-auto-edge-port", ""),
        stp_reset=props.get("stp-reset", ""),
        sflow_poll_interval=sflow_poll_interval,
        sflow_poll_interval_global=sflow_poll_interval_global,
        vendor=_unquote(props.get("vendor", "")),
        vendor_oui=props.get("vendor-oui", ""),
        vendor_partnum=_unquote(props.get("vendor-partnum", "")),
        vendor_revision=props.get("vendor-revision", ""),
        virtual_wire=props.get("virtual-wire", ""),
        transmitter_technology=props.get("transmitter-technology", ""),
        lacp_port_priority=props.get("lacp-port-priority", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_dns_resolver(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetDnsResolver:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    forward_zones: tuple[str, ...] = ()
    if "forward-zones" in props:
        # The forward-zones block is keyed by domain name.  Use the
        # list-block parser to extract the top-level keys; nested
        # ``nameservers { ... }`` sub-blocks are skipped.
        forward_zones = tuple(_parse_list_block(props["forward-zones"].value))
    nameservers: tuple[str, ...] = ()
    if "nameservers" in props:
        nameservers = tuple(_parse_list_block(props["nameservers"].value))
    return BigipNetDnsResolver(
        name=name,
        full_path=full_path,
        route_domain=plain.get("route-domain", ""),
        forward_zones=forward_zones,
        description=_description(plain),
        cache_size=plain.get("cache-size", ""),
        randomize_query_name_case=plain.get("randomize-query-name-case", ""),
        use_ipv4=plain.get("use-ipv4", ""),
        use_ipv6=plain.get("use-ipv6", ""),
        use_tcp=plain.get("use-tcp", ""),
        use_udp=plain.get("use-udp", ""),
        nameservers=nameservers,
        answer_default_zones=plain.get("answer-default-zones", ""),
        prefetch=plain.get("prefetch", ""),
        nameserver_min_rtt=plain.get("nameserver-min-rtt", ""),
        nameserver_ttl=plain.get("nameserver-ttl", ""),
        outbound_msg_retry=plain.get("outbound-msg-retry", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_tunnel(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetTunnel:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    description = _strip_quotes(props["description"].value) if "description" in props else ""
    return BigipNetTunnel(
        name=name,
        full_path=full_path,
        profile=props["profile"].value if "profile" in props else "",
        local_address=props["local-address"].value if "local-address" in props else "",
        remote_address=props["remote-address"].value if "remote-address" in props else "",
        description=description,
        mtu=props["mtu"].value if "mtu" in props else "",
        mode=props["mode"].value if "mode" in props else "",
        idle_timeout=props["idle-timeout"].value if "idle-timeout" in props else "",
        auto_lasthop=props["auto-lasthop"].value if "auto-lasthop" in props else "",
        secondary_address=(
            props["secondary-address"].value if "secondary-address" in props else ""
        ),
        traffic_group=props["traffic-group"].value if "traffic-group" in props else "",
        transparent=props["transparent"].value if "transparent" in props else "",
        key=props["key"].value if "key" in props else "",
        use_pmtu=props["use-pmtu"].value if "use-pmtu" in props else "",
        tos=props["tos"].value if "tos" in props else "",
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_net_stp(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipNetStp:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    interfaces: tuple[str, ...] = ()
    if "interfaces" in props:
        interfaces = tuple(_parse_list_block(props["interfaces"].value))
    vlans: tuple[str, ...] = ()
    if "vlans" in props:
        vlans = tuple(_parse_list_block(props["vlans"].value))
    return BigipNetStp(
        name=name,
        full_path=full_path,
        interfaces=interfaces,
        description=_description(plain),
        mode=plain.get("mode", ""),
        priority=plain.get("priority", ""),
        external_path_cost=plain.get("external-path-cost", ""),
        internal_path_cost=plain.get("internal-path-cost", ""),
        vlans=vlans,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# sys.* parsers — singletons (``sys dns``, ``sys ntp``, ``sys snmp``,
# ``sys global-settings``) have an empty full-path; everything else
# uses the identifier from the header.


def _parse_sys_dns(body: str, source_map: DocumentBuffer, block: _Block) -> BigipSysDns:
    props = _parse_properties_with_spans(body)
    name_servers: tuple[str, ...] = ()
    if "name-servers" in props:
        name_servers = tuple(_parse_list_block(props["name-servers"].value))
    search: tuple[str, ...] = ()
    if "search" in props:
        search = tuple(_parse_list_block(props["search"].value))
    return BigipSysDns(
        name_servers=name_servers,
        search=search,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_ntp(body: str, source_map: DocumentBuffer, block: _Block) -> BigipSysNtp:
    props = _parse_properties_with_spans(body)
    servers: tuple[str, ...] = ()
    if "servers" in props:
        servers = tuple(_parse_list_block(props["servers"].value))
    return BigipSysNtp(
        servers=servers,
        timezone=props["timezone"].value if "timezone" in props else "",
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_snmp(body: str, source_map: DocumentBuffer, block: _Block) -> BigipSysSnmp:
    props = _parse_properties_with_spans(body)
    agent_addresses: tuple[str, ...] = ()
    if "agent-addresses" in props:
        agent_addresses = tuple(_parse_list_block(props["agent-addresses"].value))
    communities: tuple[str, ...] = ()
    if "communities" in props:
        # ``communities`` is a block of named sub-objects; the top-level
        # keys are the community full-paths.
        communities = tuple(_parse_list_block(props["communities"].value))
    return BigipSysSnmp(
        agent_addresses=agent_addresses,
        communities=communities,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_global_settings(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSysGlobalSettings:
    props = _parse_properties_with_spans(body)
    return BigipSysGlobalSettings(
        hostname=props["hostname"].value if "hostname" in props else "",
        gui_setup=props["gui-setup"].value if "gui-setup" in props else "",
        mgmt_dhcp=props["mgmt-dhcp"].value if "mgmt-dhcp" in props else "",
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_provision(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSysProvision:
    # ``sys provision <module>`` uses the bare module name as identifier
    # (e.g. ``ltm``, ``sslo``).  No partition prefix.
    props = _parse_properties(body)
    return BigipSysProvision(
        name=full_path,
        full_path=full_path,
        level=props.get("level", ""),
        cpu_ratio=props.get("cpu-ratio", ""),
        memory_ratio=props.get("memory-ratio", ""),
        disk_ratio=props.get("disk-ratio", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_folder(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSysFolder:
    props = _parse_properties(body)
    # ``sys folder /`` uses ``/`` as the identifier; rsplit would leave
    # ``name`` empty, so fall back to the full-path itself.
    name = full_path.rsplit("/", 1)[-1] or full_path
    return BigipSysFolder(
        name=name,
        full_path=full_path,
        device_group=props.get("device-group", ""),
        traffic_group=props.get("traffic-group", ""),
        hidden=props.get("hidden", ""),
        description=_description(props),
        inherited_device_group=props.get("inherited-devicegroup", "")
        or props.get("inherited-device-group", ""),
        inherited_traffic_group=props.get("inherited-traffic-group", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_file_ssl_cert(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSysFileSslCert:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    bundle_certificates: tuple[str, ...] = ()
    if "bundle-certificates" in props:
        bundle_certificates = tuple(_parse_list_block(props["bundle-certificates"]))
    cert_validation_options: tuple[str, ...] = ()
    if "cert-validation-options" in props:
        raw = props["cert-validation-options"]
        cert_validation_options = tuple(_parse_list_block(raw)) if raw.startswith("{") else (raw,)
    cert_validators: tuple[str, ...] = ()
    if "cert-validators" in props:
        cert_validators = tuple(_parse_list_block(props["cert-validators"]))
    return BigipSysFileSslCert(
        name=name,
        full_path=full_path,
        source_path=props.get("source-path", ""),
        cache_path=props.get("cache-path", ""),
        revision=props.get("revision", ""),
        description=_description(props),
        issuer=_unquote(props.get("issuer", "")),
        subject=_unquote(props.get("subject", "")),
        expiration_string=_unquote(props.get("expiration-string", "")),
        expiration_date=props.get("expiration-date", ""),
        fingerprint=props.get("fingerprint", ""),
        key_size=props.get("key-size", ""),
        key_type=props.get("key-type", ""),
        is_bundle=props.get("is-bundle", ""),
        certificate_key_size=props.get("certificate-key-size", ""),
        issuer_cert=props.get("issuer-cert", ""),
        serial_number=props.get("serial-number", ""),
        version=props.get("version", ""),
        subject_alternative_name=_unquote(props.get("subject-alternative-name", "")),
        bundle_certificates=bundle_certificates,
        cert_validation_options=cert_validation_options,
        cert_validators=cert_validators,
        checksum=_unquote(props.get("checksum", "")),
        mode=props.get("mode", ""),
        size=props.get("size", ""),
        create_time=_unquote(props.get("create-time", "")),
        created_by=props.get("created-by", ""),
        last_update_time=_unquote(props.get("last-update-time", "")),
        updated_by=props.get("updated-by", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_file_ssl_key(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSysFileSslKey:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSysFileSslKey(
        name=name,
        full_path=full_path,
        source_path=props.get("source-path", ""),
        cache_path=props.get("cache-path", ""),
        revision=props.get("revision", ""),
        passphrase=props.get("passphrase", ""),
        description=_description(props),
        key_size=props.get("key-size", ""),
        key_type=props.get("key-type", ""),
        security_type=props.get("security-type", ""),
        checksum=_unquote(props.get("checksum", "")),
        mode=props.get("mode", ""),
        size=props.get("size", ""),
        create_time=_unquote(props.get("create-time", "")),
        created_by=props.get("created-by", ""),
        last_update_time=_unquote(props.get("last-update-time", "")),
        updated_by=props.get("updated-by", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_sys_management_route(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSysManagementRoute:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    description = _strip_quotes(props["description"].value) if "description" in props else ""
    return BigipSysManagementRoute(
        name=name,
        full_path=full_path,
        gateway=props["gateway"].value if "gateway" in props else "",
        network=props["network"].value if "network" in props else "",
        mtu=props["mtu"].value if "mtu" in props else "",
        description=description,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# security.* parsers


def _parse_security_firewall_port_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallPortList:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    ports: tuple[str, ...] = ()
    if "ports" in props:
        ports = tuple(_parse_list_block(props["ports"].value))
    return BigipSecurityFirewallPortList(
        name=name,
        full_path=full_path,
        ports=ports,
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_rule_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallRuleList:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    rules: tuple[str, ...] = ()
    if "rules" in props:
        rules = tuple(_parse_list_block(props["rules"].value))
    return BigipSecurityFirewallRuleList(
        name=name,
        full_path=full_path,
        rules=rules,
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_config_entity_id(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallConfigEntityId:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityFirewallConfigEntityId(
        name=name,
        full_path=full_path,
        entity_id=plain.get("entity-id", ""),
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_ip_intelligence_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityIpIntelligencePolicy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityIpIntelligencePolicy(
        name=name,
        full_path=full_path,
        description=_description(props),
        default_action=props.get("default-action", ""),
        default_log_blacklist_hit_only=props.get("default-log-blacklist-hit-only", ""),
        default_log_blacklist_category=props.get("default-log-blacklist-category", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_pi_compliance_map(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityProtocolInspectionComplianceMap:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityProtocolInspectionComplianceMap(
        name=name,
        full_path=full_path,
        insp_id=plain.get("insp-id", ""),
        key_type=plain.get("key-type", ""),
        value_type=plain.get("value-type", ""),
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_pi_compliance_object(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityProtocolInspectionComplianceObject:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityProtocolInspectionComplianceObject(
        name=name,
        full_path=full_path,
        insp_id=plain.get("insp-id", ""),
        type_=plain.get("type", ""),
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_device_id_attribute(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityDeviceIdAttribute:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityDeviceIdAttribute(
        name=name,
        full_path=full_path,
        id_=props["id"].value if "id" in props else "",
        description=_description({k: v.value for k, v in props.items()}),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# apm.* parsers


def _collect_named_property_from_subblocks(braced: str, prop_name: str) -> tuple[str, ...]:
    """For a block like ``{ 1 { cipher-name aes256-ctr } 2 { ... } }``,
    return the ``prop_name`` value from every direct sub-block, in
    document order.
    """
    inner = _strip_outer_braces(braced)
    out: list[str] = []
    for prop in _parse_properties_with_spans(inner).values():
        if not prop.value.startswith("{"):
            continue
        sub_inner = _strip_outer_braces(prop.value)
        sub_props = _parse_properties_with_spans(sub_inner)
        if prop_name in sub_props:
            out.append(sub_props[prop_name].value)
    return tuple(out)


def _collect_named_property_from_anon_subblocks(braced: str, prop_name: str) -> tuple[str, ...]:
    """For a block of anonymous sub-blocks like ``{ { ip 10.0.0.1 } { ip 10.0.0.2 } }``,
    extract *prop_name* from every direct sub-block in document order.

    Distinct from :func:`_collect_named_property_from_subblocks` which
    expects each sub-block to carry an identifier key (``0 { ... }``).
    """
    inner = _strip_outer_braces(braced)
    out: list[str] = []
    pos = 0
    length = len(inner)
    while pos < length:
        while pos < length and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= length or inner[pos] != "{":
            break
        block_start = pos
        pos += 1
        depth = 1
        while pos < length and depth > 0:
            ch = inner[pos]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            pos += 1
        block_text = inner[block_start:pos]
        sub_inner = _strip_outer_braces(block_text)
        sub_props = _parse_properties_with_spans(sub_inner)
        if prop_name in sub_props:
            out.append(sub_props[prop_name].value)
    return tuple(out)


def _parse_apm_ssh_security_config(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipApmEphemeralAuthSshSecurityConfig:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipApmEphemeralAuthSshSecurityConfig(
        name=name,
        full_path=full_path,
        ciphers=_collect_named_property_from_subblocks(props["ciphers"].value, "cipher-name")
        if "ciphers" in props
        else (),
        hmacs=_collect_named_property_from_subblocks(props["hmacs"].value, "hmac-name")
        if "hmacs" in props
        else (),
        kex_methods=_collect_named_property_from_subblocks(
            props["kex-methods"].value, "kex-method-name"
        )
        if "kex-methods" in props
        else (),
        compressions=_collect_named_property_from_subblocks(
            props["compressions"].value, "compression-name"
        )
        if "compressions" in props
        else (),
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_apm_oauth_db_instance(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipApmOauthDbInstance:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipApmOauthDbInstance(
        name=name,
        full_path=full_path,
        description=_strip_quotes(plain.get("description", "")),
        db_name=_strip_quotes(plain.get("db-name", "")),
        purge_frequency=plain.get("purge-frequency", ""),
        purge_time=_strip_quotes(plain.get("purge-time", "")),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_apm_policy_access_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipApmPolicyAccessPolicy:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    items: tuple[str, ...] = ()
    if "items" in props:
        items = tuple(_parse_list_block(props["items"].value))
    return BigipApmPolicyAccessPolicy(
        name=name,
        full_path=full_path,
        start_item=plain.get("start-item", ""),
        default_ending=plain.get("default-ending", ""),
        items=items,
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_apm_policy_customization_source(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipApmPolicyCustomizationSource:
    props = _parse_properties(body) if body else {}
    name = full_path.rsplit("/", 1)[-1]
    return BigipApmPolicyCustomizationSource(
        name=name,
        full_path=full_path,
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_apm_policy_item(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipApmPolicyItem:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    agents: tuple[str, ...] = ()
    if "agents" in props:
        agents = tuple(_parse_list_block(props["agents"].value))
    return BigipApmPolicyItem(
        name=name,
        full_path=full_path,
        caption=_strip_quotes(plain.get("caption", "")),
        color=plain.get("color", ""),
        item_type=plain.get("item-type", ""),
        agents=agents,
        description=_description(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_apm_policy_agent(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block, agent_type: str
) -> BigipApmPolicyAgent:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipApmPolicyAgent(
        name=name,
        full_path=full_path,
        agent_type=agent_type,
        customization_group=plain.get("customization-group", ""),
        auth=plain.get("auth", ""),
        max_logon_attempt=plain.get("max-logon-attempt", ""),
        auth_max_logon_attempt=plain.get("auth-max-logon-attempt", ""),
        fetch_nested_groups=plain.get("fetch-nested-groups", ""),
        fetch_primary_groups=plain.get("fetch-primary-groups", ""),
        password_source=plain.get("password-source", ""),
        query=_strip_quotes(plain.get("query", "")),
        query_attrname=plain.get("query-attrname", ""),
        query_filter=_strip_quotes(plain.get("query-filter", "")),
        server=plain.get("server", ""),
        show_extended_error=plain.get("show-extended-error", ""),
        upn=plain.get("upn", ""),
        username_source=plain.get("username-source", ""),
        attribute_consuming_service=plain.get("attribute-consuming-service", ""),
        attr_consuming_service_session_var=plain.get("attr-consuming-service-session-var", ""),
        hints=_strip_quotes(plain.get("hints", "")),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_apm_report_default_report(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipApmReportDefaultReport:
    props = _parse_properties_with_spans(body)
    return BigipApmReportDefaultReport(
        report_name=props["report-name"].value if "report-name" in props else "",
        user=props["user"].value if "user" in props else "",
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# cm.* parsers


def _parse_cm_cert(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipCmCert:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipCmCert(
        name=name,
        full_path=full_path,
        cache_path=_unquote(props.get("cache-path", "")),
        checksum=_unquote(props.get("checksum", "")),
        revision=props.get("revision", ""),
        issuer=_unquote(props.get("issuer", "")),
        subject=_unquote(props.get("subject", "")),
        subject_alternative_name=_unquote(props.get("subject-alternative-name", "")),
        expiration_date=props.get("expiration-date", ""),
        expiration_string=_unquote(props.get("expiration-string", "")),
        fingerprint=_unquote(props.get("fingerprint", "")),
        serial_number=props.get("serial-number", ""),
        version=props.get("version", ""),
        key_type=props.get("key-type", ""),
        certificate_key_size=props.get("certificate-key-size", ""),
        is_bundle=props.get("is-bundle", ""),
        email=_unquote(props.get("email", "")),
        source_path=_unquote(props.get("source-path", "")),
        system_path=_unquote(props.get("system-path", "")),
        size=props.get("size", ""),
        mode=props.get("mode", ""),
        create_time=_unquote(props.get("create-time", "")),
        created_by=props.get("created-by", ""),
        last_update_time=_unquote(props.get("last-update-time", "")),
        updated_by=props.get("updated-by", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_cm_key(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipCmKey:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipCmKey(
        name=name,
        full_path=full_path,
        cache_path=_unquote(props.get("cache-path", "")),
        checksum=_unquote(props.get("checksum", "")),
        revision=props.get("revision", ""),
        key_size=props.get("key-size", ""),
        key_type=props.get("key-type", ""),
        security_type=props.get("security-type", ""),
        source_path=_unquote(props.get("source-path", "")),
        system_path=_unquote(props.get("system-path", "")),
        size=props.get("size", ""),
        mode=props.get("mode", ""),
        create_time=_unquote(props.get("create-time", "")),
        created_by=props.get("created-by", ""),
        last_update_time=_unquote(props.get("last-update-time", "")),
        updated_by=props.get("updated-by", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_cm_device(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipCmDevice:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]

    def get(key: str) -> str:
        return props[key].value if key in props else ""

    unicast_address: tuple[str, ...] = ()
    if "unicast-address" in props:
        # Either anonymous sub-blocks (``{ { ip ... } { ip ... } }``)
        # or numerically-keyed sub-blocks; try both shapes so we surface
        # the bare ``ip`` values regardless of which form was emitted.
        ua_value = props["unicast-address"].value
        unicast_address = _collect_named_property_from_anon_subblocks(
            ua_value, "ip"
        ) or _collect_named_property_from_subblocks(ua_value, "ip")

    return BigipCmDevice(
        name=name,
        full_path=full_path,
        hostname=get("hostname"),
        management_ip=get("management-ip"),
        base_mac=get("base-mac"),
        build=get("build"),
        edition=get("edition"),
        version=get("version"),
        product=get("product"),
        platform_id=get("platform-id"),
        chassis_id=get("chassis-id"),
        marketing_name=_strip_quotes(get("marketing-name")),
        self_device=get("self-device"),
        time_zone=get("time-zone"),
        cert=get("cert"),
        key=get("key"),
        description=_strip_quotes(get("description")),
        comment=_strip_quotes(get("comment")),
        contact=_strip_quotes(get("contact")),
        location=_strip_quotes(get("location")),
        mirror_ip=get("mirror-ip"),
        mirror_secondary_ip=get("mirror-secondary-ip"),
        multicast_interface=get("multicast-interface"),
        multicast_ip=get("multicast-ip"),
        multicast_port=get("multicast-port"),
        unicast_address=unicast_address,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_cm_device_group(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipCmDeviceGroup:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    devices: tuple[str, ...] = ()
    if "devices" in props:
        devices = tuple(_parse_list_block(props["devices"].value))
    return BigipCmDeviceGroup(
        name=name,
        full_path=full_path,
        auto_sync=plain.get("auto-sync", ""),
        network_failover=plain.get("network-failover", ""),
        hidden=plain.get("hidden", ""),
        devices=devices,
        description=_description(plain),
        type_=plain.get("type", ""),
        save_on_auto_sync=plain.get("save-on-auto-sync", ""),
        full_load_on_sync=plain.get("full-load-on-sync", ""),
        asm_sync=plain.get("asm-sync", ""),
        incremental_config_sync_size_max=plain.get("incremental-config-sync-size-max", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_cm_traffic_group(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipCmTrafficGroup:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    ha_order: tuple[str, ...] = ()
    if "ha-order" in props:
        ha_order = tuple(_parse_list_block(props["ha-order"].value))
    return BigipCmTrafficGroup(
        name=name,
        full_path=full_path,
        unit_id=plain.get("unit-id", ""),
        description=_description(plain),
        default_device=plain.get("default-device", ""),
        ha_load_factor=plain.get("ha-load-factor", ""),
        ha_order=ha_order,
        auto_failback_enabled=plain.get("auto-failback-enabled", ""),
        auto_failback_time=plain.get("auto-failback-time", ""),
        mac=plain.get("mac", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_cm_trust_domain(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipCmTrustDomain:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    ca_devices: tuple[str, ...] = ()
    if "ca-devices" in props:
        ca_devices = tuple(_parse_list_block(props["ca-devices"].value))
    return BigipCmTrustDomain(
        name=name,
        full_path=full_path,
        ca_cert=props["ca-cert"].value if "ca-cert" in props else "",
        ca_cert_bundle=(props["ca-cert-bundle"].value if "ca-cert-bundle" in props else ""),
        ca_key=props["ca-key"].value if "ca-key" in props else "",
        ca_devices=ca_devices,
        guid=props["guid"].value if "guid" in props else "",
        status=props["status"].value if "status" in props else "",
        trust_group=props["trust-group"].value if "trust-group" in props else "",
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# gtm.* parsers


def _parse_gtm_datacenter(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmDatacenter:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmDatacenter(
        name=name,
        full_path=full_path,
        contact=_strip_quotes(plain.get("contact", "")),
        location=_strip_quotes(plain.get("location", "")),
        description=_description(plain),
        prober_pool=plain.get("prober-pool", ""),
        prober_preference=plain.get("prober-preference", ""),
        prober_fallback=plain.get("prober-fallback", ""),
        state=_state_flag(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_server(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmServer:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    # ``devices { 0 { addresses { 10.2.3.7 { } } } 1 { ... } }`` —
    # flatten every address across every numbered device sub-block.
    addresses: list[str] = []
    if "devices" in props:
        inner = _strip_outer_braces(props["devices"].value)
        for dev_prop in _parse_properties_with_spans(inner).values():
            if not dev_prop.value.startswith("{"):
                continue
            dev_inner = _strip_outer_braces(dev_prop.value)
            dev_props = _parse_properties_with_spans(dev_inner)
            if "addresses" in dev_props:
                addresses.extend(_parse_list_block(dev_props["addresses"].value))
    # ``virtual-servers { 0 { destination 10.2.3.8:5050 } 1 { ... } }`` —
    # surface the destination of each numbered entry.
    virtual_servers: list[str] = []
    if "virtual-servers" in props:
        inner = _strip_outer_braces(props["virtual-servers"].value)
        for vs_prop in _parse_properties_with_spans(inner).values():
            if not vs_prop.value.startswith("{"):
                continue
            vs_inner = _strip_outer_braces(vs_prop.value)
            vs_props = _parse_properties_with_spans(vs_inner)
            if "destination" in vs_props:
                virtual_servers.append(vs_props["destination"].value)
    return BigipGtmServer(
        name=name,
        full_path=full_path,
        datacenter=plain.get("datacenter", ""),
        monitor=plain.get("monitor", ""),
        product=plain.get("product", ""),
        addresses=tuple(addresses),
        virtual_servers=tuple(virtual_servers),
        description=_description(plain),
        state=_state_flag(plain),
        prober_pool=plain.get("prober-pool", ""),
        prober_preference=plain.get("prober-preference", ""),
        prober_fallback=plain.get("prober-fallback", ""),
        virtual_server_discovery=plain.get("virtual-server-discovery", ""),
        expose_route_domains=plain.get("expose-route-domains", ""),
        iq_allow_path=plain.get("iq-allow-path", ""),
        iq_allow_service_check=plain.get("iq-allow-service-check", ""),
        iq_allow_snmp=plain.get("iq-allow-snmp", ""),
        limit_max_bps=plain.get("limit-max-bps", ""),
        limit_max_connections=plain.get("limit-max-connections", ""),
        limit_max_pps=plain.get("limit-max-pps", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_pool(
    full_path: str,
    body: str,
    record_type: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipGtmPool:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    members: tuple[str, ...] = ()
    if "members" in props:
        members = tuple(_parse_list_block(props["members"].value))
    return BigipGtmPool(
        name=name,
        full_path=full_path,
        record_type=record_type,
        members=members,
        monitor=plain.get("monitor", ""),
        alternate_mode=plain.get("alternate-mode", ""),
        fallback_mode=plain.get("fallback-mode", ""),
        load_balancing_mode=plain.get("load-balancing-mode", ""),
        ttl=plain.get("ttl", ""),
        description=_description(plain),
        state=_state_flag(plain),
        verify_member_availability=plain.get("verify-member-availability", ""),
        fallback_ip=plain.get("fallback-ip", ""),
        max_answers_returned=plain.get("max-answers-returned", ""),
        qos_hit_ratio=plain.get("qos-hit-ratio", ""),
        qos_hops=plain.get("qos-hops", ""),
        qos_kbps=plain.get("qos-kilobytes-second", "") or plain.get("qos-kbps", ""),
        qos_lcs=plain.get("qos-lcs", ""),
        qos_packet_rate=plain.get("qos-packet-rate", ""),
        qos_rtt=plain.get("qos-rtt", ""),
        qos_topology=plain.get("qos-topology", ""),
        qos_vs_capacity=plain.get("qos-vs-capacity", ""),
        qos_vs_score=plain.get("qos-vs-score", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_wideip(
    full_path: str,
    body: str,
    record_type: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipGtmWideip:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    pools: tuple[str, ...] = ()
    if "pools" in props:
        pools = tuple(_parse_list_block(props["pools"].value))
    aliases: tuple[str, ...] = ()
    if "aliases" in props:
        aliases = tuple(_parse_list_block(props["aliases"].value))
    # ``last-resort-pool`` is emitted with the record-type prefix,
    # e.g. ``last-resort-pool mx /AS3_Tenant/.../pool2``.  Strip the
    # leading ``<record-type> `` so the field holds a clean path.
    last_resort = ""
    if "last-resort-pool" in props:
        raw = props["last-resort-pool"].value
        parts = raw.split(None, 1)
        last_resort = parts[1] if len(parts) == 2 else raw
    return BigipGtmWideip(
        name=name,
        full_path=full_path,
        record_type=record_type,
        pools=pools,
        aliases=aliases,
        pool_lb_mode=plain.get("pool-lb-mode", ""),
        last_resort_pool=last_resort,
        description=_description(plain),
        state=_state_flag(plain),
        failure_rcode=plain.get("failure-rcode", ""),
        failure_rcode_response=plain.get("failure-rcode-response", ""),
        failure_rcode_ttl=plain.get("failure-rcode-ttl", ""),
        minimal_response=plain.get("minimal-response", ""),
        persistence=plain.get("persistence", ""),
        persist_cidr_ipv4=plain.get("persist-cidr-ipv4", ""),
        persist_cidr_ipv6=plain.get("persist-cidr-ipv6", ""),
        topology_prefer_edns0_client_subnet=plain.get("topology-prefer-edns0-client-subnet", ""),
        ttl_persistence=plain.get("ttl-persistence", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_prober_pool(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmProberPool:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    members: tuple[str, ...] = ()
    if "members" in props:
        members = tuple(_parse_list_block(props["members"].value))
    return BigipGtmProberPool(
        name=name,
        full_path=full_path,
        description=_description(plain),
        load_balancing_mode=plain.get("load-balancing-mode", ""),
        members=members,
        state=_state_flag(plain),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_multitoken_keyed_entries(braced: str) -> list[str]:
    """For a block like ``{ continent SA { } subnet 192.0.2.0/24 { } }``,
    return each entry's full pre-brace key as a normalised string —
    ``["continent SA", "subnet 192.0.2.0/24"]``.

    Unlike ``_parse_list_block``, this keeps multi-token names
    together by scanning to the next ``{`` to capture the entry key.
    """
    inner = _strip_outer_braces(braced)
    entries: list[str] = []
    pos = 0
    length = len(inner)
    while pos < length:
        # Skip whitespace.
        while pos < length and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= length:
            break
        # Accumulate the key tokens up to the next ``{``.
        key_start = pos
        while pos < length and inner[pos] != "{":
            pos += 1
        key = " ".join(inner[key_start:pos].split())
        if not key:
            break
        entries.append(key)
        if pos >= length:
            break
        # Skip the sub-block, respecting nested braces.
        depth = 1
        pos += 1
        while pos < length and depth > 0:
            ch = inner[pos]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            pos += 1
    return entries


def _parse_gtm_region(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmRegion:
    props = _parse_properties_with_spans(body)
    name = full_path.rsplit("/", 1)[-1]
    region_members: tuple[str, ...] = ()
    if "region-members" in props:
        # Region-member keys are multi-token (``continent SA``,
        # ``not country DE``, ``subnet 192.0.2.0/24``) — the plain
        # list-block parser would split them on whitespace.
        region_members = tuple(_parse_multitoken_keyed_entries(props["region-members"].value))
    return BigipGtmRegion(
        name=name,
        full_path=full_path,
        description=_strip_quotes(props["description"].value) if "description" in props else "",
        region_members=region_members,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_rule(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmRule:
    # The body of a GTM iRule is Tcl, not properties — store verbatim.
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmRule(
        name=name,
        full_path=full_path,
        source=body.strip(),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# pem.* parsers


def _parse_pem_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemPolicy:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    rules: tuple[str, ...] = ()
    if "rules" in props:
        # Each rule is a named sub-block — surface the top-level keys
        # (rule names) so consumers know what is defined without
        # modelling the full action/condition grammar in v1.
        rule_props = _parse_properties_with_spans(_strip_outer_braces(props["rules"].value))
        rules = tuple(rule_props.keys())
    return BigipPemPolicy(
        name=name,
        full_path=full_path,
        description=_description(plain),
        rules=rules,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_irule(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemRule:
    # PEM iRule bodies are Tcl — store verbatim, surface description
    # only when emitted as a top-level property (rare).
    name = full_path.rsplit("/", 1)[-1]
    return BigipPemRule(
        name=name,
        full_path=full_path,
        source=body.strip(),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_listener(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemListener:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    virtual_servers: tuple[str, ...] = ()
    if "virtual-servers" in props:
        virtual_servers = tuple(_parse_list_block(props["virtual-servers"].value))
    return BigipPemListener(
        name=name,
        full_path=full_path,
        description=_description(plain),
        profile_spm=plain.get("profile-spm", ""),
        profile_subscriber_mgmt=plain.get("profile-subscriber-mgmt", ""),
        virtual_servers=virtual_servers,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_forwarding_endpoint(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemForwardingEndpoint:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipPemForwardingEndpoint(
        name=name,
        full_path=full_path,
        description=_description(props),
        pool=props.get("pool", ""),
        snat_pool=props.get("snat-pool", ""),
        source_ip=props.get("source-ip", ""),
        destination_ip=props.get("destination-ip", ""),
        type_=props.get("type", ""),
        persistence=props.get("persistence", ""),
        translate_address=props.get("translate-address", ""),
        translate_service=props.get("translate-service", ""),
        fallback=props.get("fallback", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_interception_endpoint(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemInterceptionEndpoint:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipPemInterceptionEndpoint(
        name=name,
        full_path=full_path,
        description=_description(props),
        pool=props.get("pool", ""),
        persistence=props.get("persistence", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_service_chain_endpoint(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemServiceChainEndpoint:
    props = _parse_properties_with_spans(body)
    plain = {key: prop.value for key, prop in props.items()}
    name = full_path.rsplit("/", 1)[-1]
    service_endpoints: tuple[str, ...] = ()
    if "service-endpoints" in props:
        # Service-endpoints is a block of named sub-blocks keyed by
        # endpoint name; surface the top-level keys.
        sub = _parse_properties_with_spans(_strip_outer_braces(props["service-endpoints"].value))
        service_endpoints = tuple(sub.keys())
    return BigipPemServiceChainEndpoint(
        name=name,
        full_path=full_path,
        description=_description(plain),
        service_endpoints=service_endpoints,
        steering_policy=plain.get("steering-policy", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_profile(
    full_path: str,
    profile_type: str,
    body: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipPemProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipPemProfile(
        name=name,
        full_path=full_path,
        profile_type=profile_type,
        defaults_from=props.get("defaults-from", ""),
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_pem_rating_group(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipPemRatingGroup:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipPemRatingGroup(
        name=name,
        full_path=full_path,
        description=_description(props),
        rating_group_id=props.get("rating-group-id", ""),
        default_quota=props.get("default-quota", ""),
        default_quota_holding_time=props.get("default-quota-holding-time", ""),
        default_validity_time=props.get("default-validity-time", ""),
        default_threshold=props.get("default-threshold", ""),
        total_octets=props.get("total-octets", ""),
        input_octets=props.get("input-octets", ""),
        output_octets=props.get("output-octets", ""),
        time=props.get("time", ""),
        consumption_time=props.get("consumption-time", ""),
        usage_time=props.get("usage-time", ""),
        volume=props.get("volume", ""),
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
            # Singleton stanzas (``sys dns {``, ``sys ntp {``,
            # ``sys snmp {``, ``sys global-settings {``) have a
            # two-token header that the standard header parser
            # rejects.  Dispatch them off ``generic`` and continue.
            if generic is not None:
                module_g, obj_type_g, identifier_g = generic
                if module_g == "sys" and identifier_g == "":
                    if obj_type_g == "dns":
                        config.sys_dns[""] = _parse_sys_dns(block.body, source_map, block)
                    elif obj_type_g == "ntp":
                        config.sys_ntp[""] = _parse_sys_ntp(block.body, source_map, block)
                    elif obj_type_g == "snmp":
                        config.sys_snmp[""] = _parse_sys_snmp(block.body, source_map, block)
                    elif obj_type_g == "global-settings":
                        config.sys_global_settings[""] = _parse_sys_global_settings(
                            block.body, source_map, block
                        )
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

        if module == "sys":
            # ``sys`` module dispatch.  Singletons are handled above
            # via the ``parsed is None`` branch.
            if obj_type == "provision":
                config.sys_provisions[full_path] = _parse_sys_provision(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "folder":
                config.sys_folders[full_path] = _parse_sys_folder(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "file ssl-cert":
                config.sys_file_ssl_certs[full_path] = _parse_sys_file_ssl_cert(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "file ssl-key":
                config.sys_file_ssl_keys[full_path] = _parse_sys_file_ssl_key(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "management-route":
                config.sys_management_routes[full_path] = _parse_sys_management_route(
                    full_path, block.body, source_map, block
                )
            continue

        if module == "security":
            if obj_type == "firewall port-list":
                config.security_firewall_port_lists[full_path] = _parse_security_firewall_port_list(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "firewall rule-list":
                config.security_firewall_rule_lists[full_path] = _parse_security_firewall_rule_list(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "firewall config-entity-id":
                config.security_firewall_config_entity_ids[full_path] = (
                    _parse_security_firewall_config_entity_id(
                        full_path, block.body, source_map, block
                    )
                )
            elif obj_type == "ip-intelligence policy":
                config.security_ip_intelligence_policies[full_path] = (
                    _parse_security_ip_intelligence_policy(full_path, block.body, source_map, block)
                )
            elif obj_type == "protocol-inspection compliance-map":
                config.security_pi_compliance_maps[full_path] = _parse_security_pi_compliance_map(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "protocol-inspection compliance-objects":
                config.security_pi_compliance_objects[full_path] = (
                    _parse_security_pi_compliance_object(full_path, block.body, source_map, block)
                )
            elif obj_type == "device-id attribute":
                config.security_device_id_attributes[full_path] = (
                    _parse_security_device_id_attribute(full_path, block.body, source_map, block)
                )
            continue

        if module == "apm":
            if obj_type == "ephemeral-auth ssh-security-config":
                config.apm_ephemeral_auth_ssh_security_configs[full_path] = (
                    _parse_apm_ssh_security_config(full_path, block.body, source_map, block)
                )
            elif obj_type == "oauth db-instance":
                config.apm_oauth_db_instances[full_path] = _parse_apm_oauth_db_instance(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "policy access-policy":
                config.apm_policy_access_policies[full_path] = _parse_apm_policy_access_policy(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "policy customization-source":
                config.apm_policy_customization_sources[full_path] = (
                    _parse_apm_policy_customization_source(full_path, block.body, source_map, block)
                )
            elif obj_type == "policy policy-item":
                config.apm_policy_items[full_path] = _parse_apm_policy_item(
                    full_path, block.body, source_map, block
                )
            elif obj_type.startswith("policy agent "):
                # ``policy agent <type>`` — surface every variant in the
                # same container, classified by ``agent_type``.
                agent_type = obj_type.rsplit(" ", 1)[-1]
                config.apm_policy_agents[full_path] = _parse_apm_policy_agent(
                    full_path, block.body, source_map, block, agent_type
                )
            elif obj_type == "report default-report":
                # Singleton — stored under the empty-string key.
                config.apm_report_default_report[""] = _parse_apm_report_default_report(
                    block.body, source_map, block
                )
            continue

        if module == "cm":
            if obj_type == "cert":
                config.cm_certs[full_path] = _parse_cm_cert(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "key":
                config.cm_keys[full_path] = _parse_cm_key(full_path, block.body, source_map, block)
            elif obj_type == "device":
                config.cm_devices[full_path] = _parse_cm_device(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "device-group":
                config.cm_device_groups[full_path] = _parse_cm_device_group(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "traffic-group":
                config.cm_traffic_groups[full_path] = _parse_cm_traffic_group(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "trust-domain":
                config.cm_trust_domains[full_path] = _parse_cm_trust_domain(
                    full_path, block.body, source_map, block
                )
            continue

        if module == "gtm":
            # gtm.* dispatch.  ``pool a|aaaa|cname|mx|srv|naptr`` and
            # ``wideip <record-type>`` are two-word kinds; the record
            # type tags the dataclass so all six DNS variants share
            # one container.  Fall through to the shared ``ltm/gtm``
            # match below for unmodelled gtm kinds (``monitor``,
            # ``rule``) so existing behaviour is preserved.
            if obj_type == "datacenter":
                config.gtm_datacenters[full_path] = _parse_gtm_datacenter(
                    full_path, block.body, source_map, block
                )
                continue
            if obj_type == "server":
                config.gtm_servers[full_path] = _parse_gtm_server(
                    full_path, block.body, source_map, block
                )
                continue
            if obj_type.startswith("pool "):
                record_type = obj_type.split(" ", 1)[1]
                config.gtm_pools[full_path] = _parse_gtm_pool(
                    full_path, block.body, record_type, source_map, block
                )
                continue
            if obj_type.startswith("wideip "):
                record_type = obj_type.split(" ", 1)[1]
                config.gtm_wideips[full_path] = _parse_gtm_wideip(
                    full_path, block.body, record_type, source_map, block
                )
                continue
            if obj_type == "prober-pool":
                config.gtm_prober_pools[full_path] = _parse_gtm_prober_pool(
                    full_path, block.body, source_map, block
                )
                continue
            if obj_type == "region":
                config.gtm_regions[full_path] = _parse_gtm_region(
                    full_path, block.body, source_map, block
                )
                continue
            if obj_type == "rule":
                config.gtm_rules[full_path] = _parse_gtm_rule(
                    full_path, block.body, source_map, block
                )
                continue

        if module == "pem":
            if obj_type == "policy":
                config.pem_policies[full_path] = _parse_pem_policy(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "irule":
                config.pem_rules[full_path] = _parse_pem_irule(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "listener":
                config.pem_listeners[full_path] = _parse_pem_listener(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "forwarding-endpoint":
                config.pem_forwarding_endpoints[full_path] = _parse_pem_forwarding_endpoint(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "interception-endpoint":
                config.pem_interception_endpoints[full_path] = _parse_pem_interception_endpoint(
                    full_path, block.body, source_map, block
                )
            elif obj_type == "service-chain-endpoint":
                config.pem_service_chain_endpoints[full_path] = _parse_pem_service_chain_endpoint(
                    full_path, block.body, source_map, block
                )
            elif obj_type in (
                "profile diameter-endpoint",
                "profile radius-aaa",
                "profile spm",
                "profile subscriber-mgmt",
            ):
                profile_type = obj_type.split(" ", 1)[1]
                config.pem_profiles[full_path] = _parse_pem_profile(
                    full_path, profile_type, block.body, source_map, block
                )
            elif obj_type == "quota-mgmt rating-group":
                config.pem_rating_groups[full_path] = _parse_pem_rating_group(
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
            case "virtual-address":
                if module == "ltm":
                    va = _parse_virtual_address(full_path, block.body, source_map, block)
                    config.virtual_addresses[full_path] = va
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
                profile = _parse_profile(full_path, profile_type_str, block.body, source_map, block)
                config.profiles[full_path] = profile
            case _ if obj_type.startswith("persistence "):
                persistence_type = obj_type.split(" ", 1)[1]
                persist = _parse_persistence(
                    full_path, persistence_type, block.body, source_map, block
                )
                config.persistence[full_path] = persist
            case _ if obj_type.startswith("monitor "):
                monitor_type = obj_type.split(" ", 1)[1]
                monitor = _parse_monitor(full_path, monitor_type, block.body, source_map, block)
                config.monitors[full_path] = monitor

    return config
