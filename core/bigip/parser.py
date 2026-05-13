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
from collections.abc import Callable
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
    BigipAuthApmAuth,
    BigipAuthCertLdap,
    BigipAuthLdap,
    BigipAuthLoginFailures,
    BigipAuthPartition,
    BigipAuthPassword,
    BigipAuthPasswordPolicy,
    BigipAuthRadius,
    BigipAuthRadiusServer,
    BigipAuthRemoteRole,
    BigipAuthRemoteUser,
    BigipAuthSource,
    BigipAuthTacacs,
    BigipAuthUser,
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
    BigipGtmDistributedApp,
    BigipGtmGlobalSettingsGeneral,
    BigipGtmGlobalSettingsLoadBalancing,
    BigipGtmGlobalSettingsMetrics,
    BigipGtmGlobalSettingsMetricsExclusions,
    BigipGtmLink,
    BigipGtmListener,
    BigipGtmListenerDohProxy,
    BigipGtmListenerDohServer,
    BigipGtmPool,
    BigipGtmProberPool,
    BigipGtmRegion,
    BigipGtmRule,
    BigipGtmServer,
    BigipGtmTopology,
    BigipGtmWideip,
    BigipLtmAuthObject,
    BigipLtmCipherGroup,
    BigipLtmCipherRule,
    BigipLtmDnsAnalyticsGlobalSettings,
    BigipLtmDnsCacheGlobalSettings,
    BigipLtmDnsCacheRecord,
    BigipLtmDnsCacheResolver,
    BigipLtmDnsCacheTransparent,
    BigipLtmDnsCacheValidatingResolver,
    BigipLtmDnsDnssecKey,
    BigipLtmDnsDnssecZone,
    BigipLtmDnsHpkeKey,
    BigipLtmDnsHpkeProfile,
    BigipLtmDnsNameserver,
    BigipLtmDnsTsigKey,
    BigipLtmDnsZone,
    BigipLtmEvictionPolicy,
    BigipLtmIfile,
    BigipLtmMessageRoutingObject,
    BigipLtmNat,
    BigipLtmPolicyStrategy,
    BigipLtmSnat,
    BigipLtmSnatTranslation,
    BigipLtmTrafficClass,
    BigipLtmTrafficMatchingCriteria,
    BigipMinimalObject,
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
    BigipSecurityBotDefenseProfile,
    BigipSecurityDeviceIdAttribute,
    BigipSecurityDosProfile,
    BigipSecurityFirewallAddressList,
    BigipSecurityFirewallConfigChangeLog,
    BigipSecurityFirewallConfigEntityId,
    BigipSecurityFirewallGlobalFqdnPolicy,
    BigipSecurityFirewallGlobalRules,
    BigipSecurityFirewallManagementIpRules,
    BigipSecurityFirewallOnDemandCompilation,
    BigipSecurityFirewallOnDemandRuleDeploy,
    BigipSecurityFirewallPolicy,
    BigipSecurityFirewallPortList,
    BigipSecurityFirewallPortMisusePolicy,
    BigipSecurityFirewallRuleList,
    BigipSecurityFirewallSchedule,
    BigipSecurityFirewallUserDomain,
    BigipSecurityFirewallUserList,
    BigipSecurityFirewallUuidDefaultAutogenerate,
    BigipSecurityHttpProfile,
    BigipSecurityIpIntelligenceFeedList,
    BigipSecurityIpIntelligenceGlobalPolicy,
    BigipSecurityIpIntelligencePolicy,
    BigipSecurityLogProfile,
    BigipSecurityNatDestinationTranslation,
    BigipSecurityNatPolicy,
    BigipSecurityNatSourceTranslation,
    BigipSecurityPacketFilterDefaultRules,
    BigipSecurityPacketFilterPolicy,
    BigipSecurityProtectedZone,
    BigipSecurityProtocolInspectionComplianceMap,
    BigipSecurityProtocolInspectionComplianceObject,
    BigipSecuritySshProfile,
    BigipSecurityZone,
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
        # ltm monitor.* additional subtypes (audited from real configs).
        "monitor diameter",
        "monitor dns",
        "monitor module-score",
        "monitor mqtt",
        "monitor rpc",
        "monitor sasp",
        "monitor smb",
        "monitor snmp-dca",
        "monitor snmp-dca-base",
        "monitor tcp-echo",
        "monitor virtual-location",
        # ltm profile.* additional subtypes (audited from real configs).
        "profile analytics",
        "profile classification",
        "profile ipother",
        "profile request-log",
        "profile tcp-analytics",
        # gtm monitor protocol variants (bundle 11) — 25 new types
        # beyond the ltm-shared http/https/tcp/udp/gateway-icmp/external.
        "monitor bigip",
        "monitor bigip-link",
        "monitor firepass",
        "monitor ftp",
        "monitor gtp",
        "monitor imap",
        "monitor ldap",
        "monitor mssql",
        "monitor mysql",
        "monitor nntp",
        "monitor oracle",
        "monitor pop3",
        "monitor postgresql",
        "monitor radius",
        "monitor radius-accounting",
        "monitor real-server",
        "monitor scripted",
        "monitor sip",
        "monitor smtp",
        "monitor snmp",
        "monitor snmp-link",
        "monitor soap",
        "monitor tcp-half-open",
        "monitor wap",
        "monitor wmi",
        # gtm bundle 12 — global-settings 4-singleton family.
        "global-settings general",
        "global-settings load-balancing",
        "global-settings metrics",
        "global-settings metrics-exclusions",
        # ltm bundle 13 — cipher group / cipher rule are two-word kinds.
        "cipher group",
        "cipher rule",
        # ltm bundle 14 — DNS Express two-word kinds.
        "dns nameserver",
        "dns tsig-key",
        "dns zone",
        # ltm bundle 16 — auth profiles.
        "auth profile",
        "auth ldap",
        "auth radius",
        "auth radius-server",
        "auth tacacs",
        "auth crldp-server",
        "auth ocsp-responder",
        "auth kerberos-delegation",
        "auth ssl-cc-ldap",
        "auth ssl-crldp",
        "auth ssl-ocsp",
        # ltm bundle 18 — global-settings singletons (the ``general``
        # one is shared with gtm bundle 12).
        "global-settings connection",
        "global-settings rule",
        "global-settings traffic-control",
        # ltm bundle 19 — classification + clientssl.
        "classification application",
        "classification category",
        "classification ce",
        "classification signature-update-schedule",
        "classification url-cat-policy",
        "classification url-category",
        "classification urldb-feed-list",
        "classification urldb-file",
        "clientssl ocsp-stapling-responses",
        "clientssl-proxy cached-certs",
        # ltm bundle 20 — tacdb.
        "tacdb customdb",
        "tacdb customdb-file",
        "tacdb licenseddb",
        # Bundle 21 — net routing.
        "routing access-list",
        "routing bfd",
        "routing bgp",
        "routing community-list",
        "routing extcommunity-list",
        "routing prefix-list",
        "routing route-map",
        "routing debug",
        # Bundle 22 — net tunnels family (each protocol two-word).
        "tunnels endpoint",
        "tunnels etherip",
        "tunnels fec",
        "tunnels geneve",
        "tunnels gre",
        "tunnels ipip",
        "tunnels ipsec",
        "tunnels lw4o6",
        "tunnels map",
        "tunnels ppp",
        "tunnels tcp-forward",
        "tunnels v6rd",
        "tunnels vxlan",
        "tunnels wccp",
        # Bundle 23 — net ipsec.
        "ipsec ike-daemon",
        "ipsec ike-peer",
        "ipsec ipsec-policy",
        "ipsec manual-security-association",
        "ipsec traffic-selector",
        # Bundle 24 — net BWC / cos / rate-shaping.
        "bwc policy",
        "bwc priority-group",
        "bwc traffic-group",
        "cos global-settings",
        "cos map-8021p",
        "cos map-dscp",
        "cos traffic-priority",
        "rate-shaping class",
        "rate-shaping color-policer",
        "rate-shaping drop-policy",
        "rate-shaping queue",
        "rate-shaping shaping-policy",
        # Bundle 25 — net L2 / misc two-word.
        "fdb tunnel",
        "fdb vlan",
        # Bundle 26 — net sfc.
        "sfc chain",
        "sfc sf",
        # Bundle 27 — apm aaa.* (24 kinds).
        "aaa active-directory",
        "aaa active-directory-trusted-domains",
        "aaa crldp",
        "aaa endpoint-management-system",
        "aaa f5-mfa-configuration",
        "aaa f5-service-connector",
        "aaa http",
        "aaa http-connector-request",
        "aaa http-connector-transport",
        "aaa kerberos",
        "aaa kerberos-keytab-file",
        "aaa ldap",
        "aaa oam",
        "aaa oauth-provider",
        "aaa oauth-request",
        "aaa oauth-server",
        "aaa ocsp",
        "aaa okta-connector",
        "aaa radius",
        "aaa saml",
        "aaa saml-idp-automation",
        "aaa saml-idp-connector",
        "aaa securid",
        "aaa tacacsplus",
        # Bundle 28 — apm profile.* + apm sso.* (16 kinds).
        "profile access",
        "profile connectivity",
        "profile exchange",
        "profile oauth",
        "profile vdi",
        "sso basic",
        "sso form-based",
        "sso form-basedv2",
        "sso kerberos",
        "sso ntlmv1",
        "sso ntlmv2",
        "sso oauth-bearer",
        "sso saml",
        "sso saml-resource",
        "sso saml-sp-automation",
        "sso saml-sp-connector",
        # Bundle 29 — apm resource.* (two-word forms).
        "resource address-space",
        "resource app-tunnel",
        "resource client-rate-class",
        "resource client-traffic-classifier",
        "resource ipv6-leasepool",
        "resource leasepool",
        "resource network-access",
        "resource portal-access",
        "resource sandbox",
        "resource webtop",
        "resource webtop-link",
        # Bundle 30 — apm oauth.* (7 kinds).
        "oauth jwk-config",
        "oauth jwt-config",
        "oauth jwt-provider-list",
        "oauth oauth-claim",
        "oauth oauth-client-app",
        "oauth oauth-resource-server",
        "oauth oauth-scope",
        # Bundle 31 — apm saml/ntlm/configuration/etc.
        "saml artifact-resolution-service",
        "saml attribute-consuming-service",
        "saml auth-context-class-list",
        "ntlm machine-account",
        "ntlm ntlm-auth",
        "client image",
        "configuration captcha",
        "epsec epsec-package",
        "report custom-report-field",
        "policy customization-group",
        "policy customization-languages",
        "policy image-file",
        "policy windows-group-policy-file",
        # Bundle 32 — pem.* globals + protocol.
        "global-settings analytics",
        "global-settings gx",
        "global-settings hsl-flow",
        "global-settings hsl-report",
        "global-settings insert-content",
        "global-settings policy",
        "global-settings quota-mgmt",
        "global-settings session-mgmt-attributes",
        "global-settings subscriber-activity-log",
        "protocol diameter-avp",
        "protocol radius-avp",
        "reporting format-script",
        # Bundle 33 — sys core configuration two-word kinds.
        "application service",
        "application template",
        "application apl-script",
        "application custom-stat",
        "url-db download-schedule",
        "url-db url-category",
        # Bundle 34 — sys file.* two-word kinds.
        "file data-group",
        "file external-monitor",
        "file ifile",
        "file rewrite-rule",
        "file apache-ssl-cert",
        "file ssl-crl",
        "file lwtunneltbl",
        "file browser-capabilities-db",
        "file device-capabilities-db",
        # Bundle 35 — sys log-config two-word kinds.
        "log-config filter",
        "log-config publisher",
        # Bundle 36 — sys daemon-log-settings two-word kinds.
        "daemon-log-settings clusterd",
        "daemon-log-settings csyncd",
        "daemon-log-settings icr-eventd",
        "daemon-log-settings icrd",
        "daemon-log-settings lind",
        "daemon-log-settings mcpd",
        "daemon-log-settings tmm",
        # Bundle 37 — sys crypto two-word kinds.
        "crypto cert",
        "crypto key",
        "crypto crl",
        "crypto csr",
        "crypto master-key",
        "crypto cert-order-manager",
        "crypto ca-bundle-manager",
        "crypto client",
        "crypto server",
        "crypto acceleration-strategy",
        # Bundle 38 — sys ipfix + icall.
        "ipfix destination",
        "ipfix element",
        "ipfix irules",
        "icall script",
        "icall istats-trigger",
        # Bundle 39 — sys sflow two-word kind.
        "sflow receiver",
        # Bundle 40 — sys software two-word kinds.
        "software hotfix",
        "software image",
        "software signature",
        "software volume",
        # Bundle 41 — sys runtime two-word kinds.
        "alert lcd",
        "appiq config",
        "turboflex profile-config",
        "fpga firmware-config",
        # Bundle 44 — cli alias.
        "alias private",
        "alias shared",
        # Bundle 45 — api-protection profile.
        "profile apiprotection",
        # Audit follow-ups — kinds found in real BIG-IP configs that
        # the projection doc didn't enumerate.
        "html-rule comment-raise-event",
        "html-rule comment-remove",
        "html-rule tag-append-html",
        "html-rule tag-prepend-html",
        "html-rule tag-raise-event",
        "html-rule tag-remove",
        "html-rule tag-remove-attribute",
        "shared-objects port-list",
        "shared-objects address-list",
        "ecm cloud-provider",
        "software update",
        "dynad settings",
        "dos ipv6-ext-hdr",
        "diags ihealth",
        # Sibling-completeness follow-ups discovered by the wider
        # corpus scan (HOL-2571 + BigIPReport + sslo .scf).
        "routing as-path",
        "dos profile-signatures",
        "aaa localdb",
        # net.* — multi-word kinds.
        "tunnels tunnel",
        # sys.* — multi-word kinds.
        "file ssl-cert",
        "file ssl-key",
        # security.* — multi-word kinds.
        "firewall port-list",
        "firewall rule-list",
        "firewall config-entity-id",
        # security firewall.* bundle 9 — policies, address-lists,
        # singletons, schedules, user-list / user-domain, and the
        # afm meta-singletons.
        "firewall policy",
        "firewall address-list",
        "firewall global-rules",
        "firewall management-ip-rules",
        "firewall schedule",
        "firewall user-list",
        "firewall user-domain",
        "firewall global-fqdn-policy",
        "firewall port-misuse-policy",
        "firewall on-demand-compilation",
        "firewall on-demand-rule-deploy",
        "firewall uuid-default-autogenerate",
        "firewall config-change-log",
        # bundle 10a — high-value security.* outside firewall.*
        "nat policy",
        "nat source-translation",
        "nat destination-translation",
        "log profile",
        "dos profile",
        "ip-intelligence feed-list",
        "ip-intelligence global-policy",
        "protected zone",
        "packet-filter policy",
        "packet-filter default-rules",
        "ssh profile",
        "http profile",
        "bot-defense profile",
        # bundle 10b — minimal security.* projections (37 kinds).
        "analytics settings",
        "anti-fraud profile",
        "anti-fraud signatures-update",
        "blacklist-publisher category",
        "blacklist-publisher profile",
        "bot-defense signature",
        "bot-defense signature-category",
        "cloud-services connector",
        "datasync background-tasks",
        "datasync global-profile",
        "datasync local-profile",
        "debug drop-redirect-stats",
        "debug matcher",
        "debug register",
        "device device-context",
        "dos autodos-file-object",
        "dos behavioral-signature",
        "dos bot-signature",
        "dos bot-signature-category",
        "dos device-config",
        "dos dns-nxdomain-stat",
        "dos dos-signature",
        "dos dynamic-signatures",
        "dos ip-uncommon-protolist",
        "dos l4bdos-file-object",
        "dos network-whitelist",
        "dos stress-stats",
        "dos udp-portlist",
        "dos virtual",
        "flowspec-route-injector profile",
        "ip-intelligence blacklist-category",
        "protocol-inspection common-config",
        "protocol-inspection learning-stats",
        "protocol-inspection profile",
        "protocol-inspection signature",
        "scrubber profile",
        "ssh ciphers",
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

_THREE_WORD_TYPES = frozenset(
    [
        *(f"policy agent {t}" for t in _APM_POLICY_AGENT_TYPES),
        # ltm dns bundle 14 — three-word kinds.
        "dns dnssec key",
        "dns dnssec zone",
        "dns cache resolver",
        "dns cache transparent",
        "dns cache validating-resolver",
        "dns cache global-settings",  # singleton — header has no identifier
        "dns analytics global-settings",  # singleton
        "dns hpke key",
        "dns hpke profile",
        # ltm bundle 19 — three-word.
        "classification auto-update settings",
        # net bundle 21 — three-word.
        "routing profile bgp",
        # apm bundle 29 — resource remote-desktop sub-kinds.
        "resource remote-desktop citrix",
        "resource remote-desktop citrix-client-bundle",
        "resource remote-desktop citrix-client-package-file",
        "resource remote-desktop quest",
        "resource remote-desktop rdp",
        "resource remote-desktop vmware-view",
        # pem bundle 32 — protocol profile sub-kinds.
        "protocol profile gx",
        "protocol profile radius",
        # sys bundle 35 — log-config destination three-word kinds.
        "log-config destination alertd",
        "log-config destination arcsight",
        "log-config destination ipfix",
        "log-config destination local-database",
        "log-config destination local-syslog",
        "log-config destination management-port",
        "log-config destination remote-high-speed-log",
        "log-config destination remote-syslog",
        "log-config destination splunk",
        # sys bundle 37 — crypto three-word kinds.
        "crypto cert-validator crl",
        "crypto cert-validator ocsp",
        "crypto cert-validation-response ocsp",
        "crypto fips key",
        "crypto fips external-hsm",
        # sys bundle 38 — icall handler three-word kinds.
        "icall handler periodic",
        "icall handler perpetual",
        "icall handler triggered",
        # sys bundle 39 — sflow global-settings three-word kinds.
        "sflow global-settings http",
        "sflow global-settings interface",
        "sflow global-settings system",
        "sflow global-settings vlan",
        # ltm message-routing bundle 15 — three-word kinds.
        "message-routing diameter peer",
        "message-routing diameter route",
        "message-routing diameter transport-config",
        "message-routing sip peer",
        "message-routing sip route",
        "message-routing sip transport-config",
        "message-routing mqtt peer",
        "message-routing mqtt route",
        "message-routing mqtt transport-config",
        "message-routing generic peer",
        "message-routing generic protocol",
        "message-routing generic route",
        "message-routing generic router",
        "message-routing generic transport-config",
    ]
)

# Four-word kinds (header has 6 tokens: module + 4 kind tokens +
# identifier).  Currently only ``ltm dns cache records *``.
_FOUR_WORD_TYPES = frozenset(
    [
        "dns cache records all",
        "dns cache records key",
        "dns cache records msg",
        "dns cache records nameserver",
        "dns cache records rrset",
        # ltm message-routing bundle 15 — four-word ``profile *`` kinds.
        "message-routing diameter profile router",
        "message-routing diameter profile session",
        "message-routing sip profile router",
        "message-routing sip profile session",
        "message-routing mqtt profile router",
        "message-routing mqtt profile session",
    ]
)


def _parse_header(header: str) -> tuple[str, str, str] | None:
    """Parse a stanza header into ``(module, type, full_path)``.

    Returns ``None`` if the header doesn't match the expected format.
    """
    parts = header.split()
    if len(parts) < 3:
        return None
    module = parts[0]
    # Four-word path (6 tokens: module + 4-word kind + identifier).
    if len(parts) >= 6:
        four_word = " ".join(parts[1:5])
        if four_word in _FOUR_WORD_TYPES:
            return (module, four_word, parts[5])
    # Three-word path (5 tokens: module + 3-word kind + identifier).
    if len(parts) >= 5:
        three_word = " ".join(parts[1:4])
        if three_word in _THREE_WORD_TYPES:
            return (module, three_word, parts[4])
    # Three-word singleton (4 tokens: module + 3-word kind, no identifier).
    if len(parts) == 4:
        three_word = " ".join(parts[1:4])
        if three_word in _THREE_WORD_TYPES:
            return (module, three_word, "")
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


# Bundle 13 parsers — ltm.* cross-cutting infra.


def _parse_ltm_cipher_group(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmCipherGroup:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmCipherGroup(
        name=name,
        full_path=full_path,
        description=_description(props),
        allow=_list_field(props, "allow"),
        require=_list_field(props, "require"),
        exclude=_list_field(props, "exclude"),
        ordering=props.get("ordering", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_cipher_rule(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmCipherRule:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmCipherRule(
        name=name,
        full_path=full_path,
        description=_description(props),
        cipher=props.get("cipher", ""),
        dh_groups=props.get("dh-groups", ""),
        signature_algorithms=props.get("signature-algorithms", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_nat(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmNat:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmNat(
        name=name,
        full_path=full_path,
        description=_description(props),
        translation_address=props.get("translation-address", ""),
        originating_address=props.get("originating-address", ""),
        traffic_group=props.get("traffic-group", ""),
        vlans=_list_field(props, "vlans"),
        vlans_disabled="vlans-disabled" in props,
        vlans_enabled="vlans-enabled" in props,
        mirror=props.get("mirror", ""),
        arp=props.get("arp", ""),
        state=_state_flag(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_snat(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmSnat:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmSnat(
        name=name,
        full_path=full_path,
        description=_description(props),
        origins=_list_field(props, "origins"),
        translation=props.get("translation", ""),
        snatpool=props.get("snatpool", ""),
        vlans=_list_field(props, "vlans"),
        vlans_disabled="vlans-disabled" in props,
        vlans_enabled="vlans-enabled" in props,
        automap="automap" in props,
        mirror=props.get("mirror", ""),
        state=_state_flag(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_snat_translation(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmSnatTranslation:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmSnatTranslation(
        name=name,
        full_path=full_path,
        description=_description(props),
        address=props.get("address", ""),
        inherited_traffic_group=props.get("inherited-traffic-group", ""),
        traffic_group=props.get("traffic-group", ""),
        connection_limit=props.get("connection-limit", ""),
        ip_idle_timeout=props.get("ip-idle-timeout", ""),
        tcp_idle_timeout=props.get("tcp-idle-timeout", ""),
        udp_idle_timeout=props.get("udp-idle-timeout", ""),
        state=_state_flag(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_policy_strategy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmPolicyStrategy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    # ``operands { 0 { ... } 1 { ... } ... }`` — surface the indexed
    # operand keys; per-operand bodies are out of scope in v1.
    operands = ()
    if "operands" in props:
        raw = props["operands"]
        if raw.startswith("{"):
            inner = _strip_outer_braces(raw)
            operands = tuple(_parse_properties_with_spans(inner).keys())
    return BigipLtmPolicyStrategy(
        name=name,
        full_path=full_path,
        description=_description(props),
        strategy=props.get("strategy", ""),
        operands=operands,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_traffic_class(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmTrafficClass:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmTrafficClass(
        name=name,
        full_path=full_path,
        description=_description(props),
        classification=props.get("classification", ""),
        match_method=props.get("match-method", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_traffic_matching_criteria(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmTrafficMatchingCriteria:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmTrafficMatchingCriteria(
        name=name,
        full_path=full_path,
        description=_description(props),
        destination_address_list=props.get("destination-address-list", ""),
        destination_address_inline=props.get("destination-address-inline", ""),
        destination_port_list=props.get("destination-port-list", ""),
        destination_port_inline=props.get("destination-port-inline", ""),
        source_address_list=props.get("source-address-list", ""),
        source_address_inline=props.get("source-address-inline", ""),
        source_port_list=props.get("source-port-list", ""),
        source_port_inline=props.get("source-port-inline", ""),
        protocol=props.get("protocol", ""),
        route_domain=props.get("route-domain", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_ifile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmIfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmIfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        file_name=props.get("file-name", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_eviction_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmEvictionPolicy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmEvictionPolicy(
        name=name,
        full_path=full_path,
        description=_description(props),
        high_water_mark=props.get("high-water-mark", ""),
        low_water_mark=props.get("low-water-mark", ""),
        slow_flow_throttle=props.get("slow-flow-throttle", ""),
        slow_flow_monitoring=props.get("slow-flow-monitoring", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Bundle 14 parsers — ltm dns.* (DNS Express).


def _parse_ltm_dns_nameserver(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsNameserver:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsNameserver(
        name=name,
        full_path=full_path,
        description=_description(props),
        address=props.get("address", ""),
        port=props.get("port", ""),
        tsig_key=props.get("tsig-key", ""),
        route_domain=props.get("route-domain", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_tsig_key(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsTsigKey:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsTsigKey(
        name=name,
        full_path=full_path,
        description=_description(props),
        algorithm=props.get("algorithm", ""),
        secret=props.get("secret", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_zone(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsZone:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsZone(
        name=name,
        full_path=full_path,
        description=_description(props),
        dns_express_server=props.get("dns-express-server", ""),
        dns_express_allow_notify=_list_field(props, "dns-express-allow-notify"),
        dns_express_enabled=props.get("dns-express-enabled", ""),
        response_policy=props.get("response-policy", ""),
        transfer_clients=_list_field(props, "transfer-clients"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_dnssec_key(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsDnssecKey:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsDnssecKey(
        name=name,
        full_path=full_path,
        description=_description(props),
        type_=props.get("type", ""),
        algorithm=props.get("algorithm", ""),
        bit_width=props.get("bit-width", ""),
        rollover_period=props.get("rollover-period", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_dnssec_zone(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsDnssecZone:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsDnssecZone(
        name=name,
        full_path=full_path,
        description=_description(props),
        keys=_list_field(props, "keys"),
        enable=props.get("enable", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_cache_resolver(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsCacheResolver:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    forward_zones = ()
    if "forward-zones" in props:
        raw = props["forward-zones"]
        if raw.startswith("{"):
            forward_zones = tuple(_parse_properties_with_spans(_strip_outer_braces(raw)).keys())
    return BigipLtmDnsCacheResolver(
        name=name,
        full_path=full_path,
        description=_description(props),
        message_cache_size=props.get("message-cache-size", ""),
        resolver_cache_size=props.get("resolver-cache-size", ""),
        answer_default_zones=props.get("answer-default-zones", ""),
        forward_zones=forward_zones,
        route_domain=props.get("route-domain", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_cache_transparent(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsCacheTransparent:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsCacheTransparent(
        name=name,
        full_path=full_path,
        description=_description(props),
        message_cache_size=props.get("message-cache-size", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_cache_validating_resolver(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsCacheValidatingResolver:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsCacheValidatingResolver(
        name=name,
        full_path=full_path,
        description=_description(props),
        message_cache_size=props.get("message-cache-size", ""),
        resolver_cache_size=props.get("resolver-cache-size", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_cache_global_settings(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsCacheGlobalSettings:
    props = _parse_properties(body)
    return BigipLtmDnsCacheGlobalSettings(
        description=_description(props),
        expiry_time=props.get("expiry-time", ""),
        nameserver_ttl=props.get("nameserver-ttl", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_cache_record(
    full_path: str,
    body: str,
    record_kind: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipLtmDnsCacheRecord:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1] if full_path else ""
    return BigipLtmDnsCacheRecord(
        name=name,
        full_path=full_path,
        record_kind=record_kind,
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_hpke_key(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsHpkeKey:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsHpkeKey(
        name=name,
        full_path=full_path,
        description=_description(props),
        algorithm=props.get("algorithm", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_hpke_profile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsHpkeProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipLtmDnsHpkeProfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        defaults_from=props.get("defaults-from", ""),
        keys=_list_field(props, "keys"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_dns_analytics_global_settings(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipLtmDnsAnalyticsGlobalSettings:
    props = _parse_properties(body)
    return BigipLtmDnsAnalyticsGlobalSettings(
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Bundle 15 parsers — ltm message-routing.*  shared parser routing
# off a single dispatch table keyed by the in-config obj_type.


# Bundles 17-20 — generic ltm.* minimal parser.  Shared by the
# long-tail kinds where the v1 projection only needs identity +
# description.

_LTM_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 17 — ltm CGNAT / LSN.
    "lsn-pool": "ltm_lsn_pools",
    "lsn-log-profile": "ltm_lsn_log_profiles",
    "alg-log-profile": "ltm_alg_log_profiles",
    # Bundle 18 — ltm global-settings + misc singletons.
    "default-node-monitor": "ltm_default_node_monitor",
    "global-settings connection": "ltm_global_settings_connection",
    "global-settings general": "ltm_global_settings_general",
    "global-settings rule": "ltm_global_settings_rule",
    "global-settings traffic-control": "ltm_global_settings_traffic_control",
    "rule-profiler": "ltm_rule_profiler",
    # Bundle 19 — ltm classification + clientssl.
    "classification application": "ltm_classification_application",
    "classification auto-update settings": "ltm_classification_auto_update_settings",
    "classification category": "ltm_classification_category",
    "classification ce": "ltm_classification_ce",
    "classification signature-update-schedule": "ltm_classification_signature_update_schedule",
    "classification url-cat-policy": "ltm_classification_url_cat_policy",
    "classification url-category": "ltm_classification_url_category",
    "classification urldb-feed-list": "ltm_classification_urldb_feed_list",
    "classification urldb-file": "ltm_classification_urldb_file",
    "clientssl ocsp-stapling-responses": "ltm_clientssl_ocsp_stapling_responses",
    "clientssl-proxy cached-certs": "ltm_clientssl_proxy_cached_certs",
    # Bundle 20 — ltm tacdb.
    "tacdb customdb": "ltm_tacdb_customdb",
    "tacdb customdb-file": "ltm_tacdb_customdb_file",
    "tacdb licenseddb": "ltm_tacdb_licenseddb",
    # Audit follow-up — ltm html-rule subtypes.
    "html-rule comment-raise-event": "ltm_html_rule_comment_raise_event",
    "html-rule comment-remove": "ltm_html_rule_comment_remove",
    "html-rule tag-append-html": "ltm_html_rule_tag_append_html",
    "html-rule tag-prepend-html": "ltm_html_rule_tag_prepend_html",
    "html-rule tag-raise-event": "ltm_html_rule_tag_raise_event",
    "html-rule tag-remove": "ltm_html_rule_tag_remove",
    "html-rule tag-remove-attribute": "ltm_html_rule_tag_remove_attribute",
}


def _parse_minimal(
    full_path: str,
    body: str,
    kind_label: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipMinimalObject:
    """Build a :class:`BigipMinimalObject` for any minimal-shape kind.

    Every "minimal" projection (the 300+ kinds whose typed surface is
    just ``name`` / ``full_path`` / ``kind`` / ``description``) uses
    this single parser regardless of module.  The per-module
    ``_parse_<module>_minimal`` aliases below exist only so call
    sites stay grep-able by module.
    """
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1] if full_path else ""
    return BigipMinimalObject(
        name=name,
        full_path=full_path,
        kind=kind_label,
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Module-scoped aliases — every one resolves to the shared
# :func:`_parse_minimal` above.
_parse_ltm_minimal = _parse_minimal


_NET_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 21 — net routing (10 kinds).
    "routing access-list": "net_routing_access_lists",
    "routing bfd": "net_routing_bfd",
    "routing bgp": "net_routing_bgp",
    "routing community-list": "net_routing_community_lists",
    "routing extcommunity-list": "net_routing_extcommunity_lists",
    "routing prefix-list": "net_routing_prefix_lists",
    "routing profile bgp": "net_routing_profile_bgp",
    "routing route-map": "net_routing_route_maps",
    "routing debug": "net_routing_debug",
    "router-advertisement": "net_router_advertisements",
    # Bundle 22 — net tunnels family (14 kinds).
    "tunnels endpoint": "net_tunnels_endpoints",
    "tunnels etherip": "net_tunnels_etherip",
    "tunnels fec": "net_tunnels_fec",
    "tunnels geneve": "net_tunnels_geneve",
    "tunnels gre": "net_tunnels_gre",
    "tunnels ipip": "net_tunnels_ipip",
    "tunnels ipsec": "net_tunnels_ipsec",
    "tunnels lw4o6": "net_tunnels_lw4o6",
    "tunnels map": "net_tunnels_map",
    "tunnels ppp": "net_tunnels_ppp",
    "tunnels tcp-forward": "net_tunnels_tcp_forward",
    "tunnels v6rd": "net_tunnels_v6rd",
    "tunnels vxlan": "net_tunnels_vxlan",
    "tunnels wccp": "net_tunnels_wccp",
    # Bundle 23 — net ipsec (5 kinds).
    "ipsec ike-daemon": "net_ipsec_ike_daemon",
    "ipsec ike-peer": "net_ipsec_ike_peers",
    "ipsec ipsec-policy": "net_ipsec_ipsec_policies",
    "ipsec manual-security-association": "net_ipsec_manual_security_associations",
    "ipsec traffic-selector": "net_ipsec_traffic_selectors",
    # Bundle 24 — net BWC / rate-shaping / cos (12 kinds).
    "bwc policy": "net_bwc_policies",
    "bwc priority-group": "net_bwc_priority_groups",
    "bwc traffic-group": "net_bwc_traffic_groups",
    "cos global-settings": "net_cos_global_settings",
    "cos map-8021p": "net_cos_map_8021p",
    "cos map-dscp": "net_cos_map_dscp",
    "cos traffic-priority": "net_cos_traffic_priority",
    "rate-shaping class": "net_rate_shaping_class",
    "rate-shaping color-policer": "net_rate_shaping_color_policer",
    "rate-shaping drop-policy": "net_rate_shaping_drop_policy",
    "rate-shaping queue": "net_rate_shaping_queue",
    "rate-shaping shaping-policy": "net_rate_shaping_shaping_policy",
    # Bundle 25 — net L2 / misc (22 kinds; mix of single- and two-word).
    "address-list": "net_address_lists",
    "arp": "net_arp",
    "dag-globals": "net_dag_globals",
    "fdb tunnel": "net_fdb_tunnel",
    "fdb vlan": "net_fdb_vlan",
    "interface-cos": "net_interface_cos",
    "ipv6-subscriber-prefix-length": "net_ipv6_subscriber_prefix_length",
    "lacp-globals": "net_lacp_globals",
    "lldp-globals": "net_lldp_globals",
    "multicast-globals": "net_multicast_globals",
    "ndp": "net_ndp",
    "packet-filter": "net_packet_filter",
    "packet-filter-trusted": "net_packet_filter_trusted",
    "port-mirror": "net_port_mirror",
    "rst-cause": "net_rst_cause",
    "self-allow": "net_self_allow",
    "service-policy": "net_service_policy",
    "stp-globals": "net_stp_globals",
    "timer-policy": "net_timer_policy",
    "trunk": "net_trunk",
    "vlan-group": "net_vlan_group",
    "wccp": "net_wccp",
    # Bundle 26 — net sfc (2 kinds).
    "sfc chain": "net_sfc_chain",
    "sfc sf": "net_sfc_sf",
    # Sibling-completeness follow-up.
    "routing as-path": "net_routing_as_paths",
}


_parse_net_minimal = _parse_minimal


_APM_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 27 — apm aaa.* (24 kinds).
    "aaa active-directory": "apm_aaa_active_directory",
    "aaa active-directory-trusted-domains": "apm_aaa_active_directory_trusted_domains",
    "aaa crldp": "apm_aaa_crldp",
    "aaa endpoint-management-system": "apm_aaa_endpoint_management_system",
    "aaa f5-mfa-configuration": "apm_aaa_f5_mfa_configuration",
    "aaa f5-service-connector": "apm_aaa_f5_service_connector",
    "aaa http": "apm_aaa_http",
    "aaa http-connector-request": "apm_aaa_http_connector_request",
    "aaa http-connector-transport": "apm_aaa_http_connector_transport",
    "aaa kerberos": "apm_aaa_kerberos",
    "aaa kerberos-keytab-file": "apm_aaa_kerberos_keytab_file",
    "aaa ldap": "apm_aaa_ldap",
    "aaa oam": "apm_aaa_oam",
    "aaa oauth-provider": "apm_aaa_oauth_provider",
    "aaa oauth-request": "apm_aaa_oauth_request",
    "aaa oauth-server": "apm_aaa_oauth_server",
    "aaa ocsp": "apm_aaa_ocsp",
    "aaa okta-connector": "apm_aaa_okta_connector",
    "aaa radius": "apm_aaa_radius",
    "aaa saml": "apm_aaa_saml",
    "aaa saml-idp-automation": "apm_aaa_saml_idp_automation",
    "aaa saml-idp-connector": "apm_aaa_saml_idp_connector",
    "aaa securid": "apm_aaa_securid",
    "aaa tacacsplus": "apm_aaa_tacacsplus",
    # Bundle 28 — apm profile.* + apm sso.* (16 kinds).
    "profile access": "apm_profile_access",
    "profile connectivity": "apm_profile_connectivity",
    "profile exchange": "apm_profile_exchange",
    "profile oauth": "apm_profile_oauth",
    "profile vdi": "apm_profile_vdi",
    "sso basic": "apm_sso_basic",
    "sso form-based": "apm_sso_form_based",
    "sso form-basedv2": "apm_sso_form_basedv2",
    "sso kerberos": "apm_sso_kerberos",
    "sso ntlmv1": "apm_sso_ntlmv1",
    "sso ntlmv2": "apm_sso_ntlmv2",
    "sso oauth-bearer": "apm_sso_oauth_bearer",
    "sso saml": "apm_sso_saml",
    "sso saml-resource": "apm_sso_saml_resource",
    "sso saml-sp-automation": "apm_sso_saml_sp_automation",
    "sso saml-sp-connector": "apm_sso_saml_sp_connector",
    # Bundle 29 — apm resource.* (17 kinds, 6 three-word).
    "resource address-space": "apm_resource_address_space",
    "resource app-tunnel": "apm_resource_app_tunnel",
    "resource client-rate-class": "apm_resource_client_rate_class",
    "resource client-traffic-classifier": "apm_resource_client_traffic_classifier",
    "resource ipv6-leasepool": "apm_resource_ipv6_leasepool",
    "resource leasepool": "apm_resource_leasepool",
    "resource network-access": "apm_resource_network_access",
    "resource portal-access": "apm_resource_portal_access",
    "resource remote-desktop citrix": "apm_resource_remote_desktop_citrix",
    "resource remote-desktop citrix-client-bundle": "apm_resource_remote_desktop_citrix_client_bundle",
    "resource remote-desktop citrix-client-package-file": "apm_resource_remote_desktop_citrix_client_package_file",
    "resource remote-desktop quest": "apm_resource_remote_desktop_quest",
    "resource remote-desktop rdp": "apm_resource_remote_desktop_rdp",
    "resource remote-desktop vmware-view": "apm_resource_remote_desktop_vmware_view",
    "resource sandbox": "apm_resource_sandbox",
    "resource webtop": "apm_resource_webtop",
    "resource webtop-link": "apm_resource_webtop_link",
    # Bundle 30 — apm oauth.* (7 kinds, beyond db-instance from bundle 6).
    "oauth jwk-config": "apm_oauth_jwk_config",
    "oauth jwt-config": "apm_oauth_jwt_config",
    "oauth jwt-provider-list": "apm_oauth_jwt_provider_list",
    "oauth oauth-claim": "apm_oauth_oauth_claim",
    "oauth oauth-client-app": "apm_oauth_oauth_client_app",
    "oauth oauth-resource-server": "apm_oauth_oauth_resource_server",
    "oauth oauth-scope": "apm_oauth_oauth_scope",
    # Bundle 31 — apm saml/ntlm/acl/configuration/etc (18 kinds).
    "saml artifact-resolution-service": "apm_saml_artifact_resolution_service",
    "saml attribute-consuming-service": "apm_saml_attribute_consuming_service",
    "saml auth-context-class-list": "apm_saml_auth_context_class_list",
    "ntlm machine-account": "apm_ntlm_machine_account",
    "ntlm ntlm-auth": "apm_ntlm_ntlm_auth",
    "acl": "apm_acl",
    "log-setting": "apm_log_setting",
    "url-filter": "apm_url_filter",
    "swg-scheme": "apm_swg_scheme",
    "client image": "apm_client_image",
    "configuration captcha": "apm_configuration_captcha",
    "epsec epsec-package": "apm_epsec_epsec_package",
    "apm-avr-config": "apm_apm_avr_config",
    "report custom-report-field": "apm_report_custom_report_field",
    "policy customization-group": "apm_policy_customization_group",
    "policy customization-languages": "apm_policy_customization_languages",
    "policy image-file": "apm_policy_image_file",
    "policy windows-group-policy-file": "apm_policy_windows_group_policy_file",
    # Audit follow-up — apm.* found in real BIG-IP configs.
    "client-packaging": "apm_client_packaging",
    # Sibling-completeness follow-up (sslo swg_profile.scf).
    "aaa localdb": "apm_aaa_localdb",
}


_parse_apm_minimal = _parse_minimal


_PEM_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 32 — pem.* globals + protocol (16 kinds).
    "global-settings analytics": "pem_gs_analytics",
    "global-settings gx": "pem_gs_gx",
    "global-settings hsl-flow": "pem_gs_hsl_flow",
    "global-settings hsl-report": "pem_gs_hsl_report",
    "global-settings insert-content": "pem_gs_insert_content",
    "global-settings policy": "pem_gs_policy",
    "global-settings quota-mgmt": "pem_gs_quota_mgmt",
    "global-settings session-mgmt-attributes": "pem_gs_session_mgmt_attributes",
    "global-settings subscriber-activity-log": "pem_gs_subscriber_activity_log",
    "protocol diameter-avp": "pem_protocol_diameter_avp",
    "protocol radius-avp": "pem_protocol_radius_avp",
    "protocol profile gx": "pem_protocol_profile_gx",
    "protocol profile radius": "pem_protocol_profile_radius",
    "reporting format-script": "pem_reporting_format_script",
    "subscriber": "pem_subscriber",
    "subscriber-attribute": "pem_subscriber_attribute",
}


_parse_pem_minimal = _parse_minimal


_SYS_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 33 — sys core configuration kinds (18).
    "ha-group": "sys_ha_group",
    "application service": "sys_application_service",
    "application template": "sys_application_template",
    "application apl-script": "sys_application_apl_script",
    "application custom-stat": "sys_application_custom_stat",
    "autoscale-group": "sys_autoscale_group",
    "db": "sys_db",
    "httpd": "sys_httpd",
    "sshd": "sys_sshd",
    "syslog": "sys_syslog",
    "outbound-smtp": "sys_outbound_smtp",
    "smtp-server": "sys_smtp_server",
    "feature-module": "sys_feature_module",
    "console": "sys_console",
    "log-rotate": "sys_log_rotate",
    "ucs": "sys_ucs",
    "url-db download-schedule": "sys_url_db_download_schedule",
    "url-db url-category": "sys_url_db_url_category",
    # Bundle 34 — sys file.* (9).
    "file data-group": "sys_file_data_group",
    "file external-monitor": "sys_file_external_monitor",
    "file ifile": "sys_file_ifile",
    "file rewrite-rule": "sys_file_rewrite_rule",
    "file apache-ssl-cert": "sys_file_apache_ssl_cert",
    "file ssl-crl": "sys_file_ssl_crl",
    "file lwtunneltbl": "sys_file_lwtunneltbl",
    "file browser-capabilities-db": "sys_file_browser_capabilities_db",
    "file device-capabilities-db": "sys_file_device_capabilities_db",
    # Bundle 35 — sys log-config (11).
    "log-config destination alertd": "sys_log_config_destination_alertd",
    "log-config destination arcsight": "sys_log_config_destination_arcsight",
    "log-config destination ipfix": "sys_log_config_destination_ipfix",
    "log-config destination local-database": "sys_log_config_destination_local_database",
    "log-config destination local-syslog": "sys_log_config_destination_local_syslog",
    "log-config destination management-port": "sys_log_config_destination_management_port",
    "log-config destination remote-high-speed-log": "sys_log_config_destination_remote_high_speed_log",
    "log-config destination remote-syslog": "sys_log_config_destination_remote_syslog",
    "log-config destination splunk": "sys_log_config_destination_splunk",
    "log-config filter": "sys_log_config_filter",
    "log-config publisher": "sys_log_config_publisher",
    # Bundle 36 — sys daemon-log-settings (7).
    "daemon-log-settings clusterd": "sys_daemon_log_settings_clusterd",
    "daemon-log-settings csyncd": "sys_daemon_log_settings_csyncd",
    "daemon-log-settings icr-eventd": "sys_daemon_log_settings_icr_eventd",
    "daemon-log-settings icrd": "sys_daemon_log_settings_icrd",
    "daemon-log-settings lind": "sys_daemon_log_settings_lind",
    "daemon-log-settings mcpd": "sys_daemon_log_settings_mcpd",
    "daemon-log-settings tmm": "sys_daemon_log_settings_tmm",
    # Bundle 37 — sys crypto (15).
    "crypto cert": "sys_crypto_cert",
    "crypto key": "sys_crypto_key",
    "crypto crl": "sys_crypto_crl",
    "crypto csr": "sys_crypto_csr",
    "crypto master-key": "sys_crypto_master_key",
    "crypto cert-order-manager": "sys_crypto_cert_order_manager",
    "crypto ca-bundle-manager": "sys_crypto_ca_bundle_manager",
    "crypto cert-validator crl": "sys_crypto_cert_validator_crl",
    "crypto cert-validator ocsp": "sys_crypto_cert_validator_ocsp",
    "crypto cert-validation-response ocsp": "sys_crypto_cert_validation_response_ocsp",
    "crypto client": "sys_crypto_client",
    "crypto server": "sys_crypto_server",
    "crypto acceleration-strategy": "sys_crypto_acceleration_strategy",
    "crypto fips key": "sys_crypto_fips_key",
    "crypto fips external-hsm": "sys_crypto_fips_external_hsm",
    # Bundle 38 — sys ipfix + icall (8).
    "ipfix destination": "sys_ipfix_destination",
    "ipfix element": "sys_ipfix_element",
    "ipfix irules": "sys_ipfix_irules",
    "icall handler periodic": "sys_icall_handler_periodic",
    "icall handler perpetual": "sys_icall_handler_perpetual",
    "icall handler triggered": "sys_icall_handler_triggered",
    "icall script": "sys_icall_script",
    "icall istats-trigger": "sys_icall_istats_trigger",
    # Bundle 39 — sys management/state-mirroring/sflow (11).
    "management-dhcp": "sys_management_dhcp",
    "management-ip": "sys_management_ip",
    "management-ovsdb": "sys_management_ovsdb",
    "management-proxy-config": "sys_management_proxy_config",
    "state-mirroring": "sys_state_mirroring",
    "datastor": "sys_datastor",
    "sflow receiver": "sys_sflow_receiver",
    "sflow global-settings http": "sys_sflow_global_settings_http",
    "sflow global-settings interface": "sys_sflow_global_settings_interface",
    "sflow global-settings system": "sys_sflow_global_settings_system",
    "sflow global-settings vlan": "sys_sflow_global_settings_vlan",
    # Bundle 40 — sys software (4).
    "software hotfix": "sys_software_hotfix",
    "software image": "sys_software_image",
    "software signature": "sys_software_signature",
    "software volume": "sys_software_volume",
    # Bundle 41 — sys runtime-adjacent config (12).
    "alert lcd": "sys_alert_lcd",
    "aom": "sys_aom",
    "appiq config": "sys_appiq_config",
    "cluster": "sys_cluster",
    "config": "sys_config",
    "default-config": "sys_default_config",
    "failover": "sys_failover",
    "internal-proxy": "sys_internal_proxy",
    "traffic": "sys_traffic",
    "tmm-traffic": "sys_tmm_traffic",
    "turboflex profile-config": "sys_turboflex_profile_config",
    "fpga firmware-config": "sys_fpga_firmware_config",
    # Audit follow-ups — kinds found in real BIG-IP configs.
    "ecm cloud-provider": "sys_ecm_cloud_provider",
    "software update": "sys_software_update",
    "dynad settings": "sys_dynad_settings",
    "compatibility-level": "sys_compatibility_level",
    "diags ihealth": "sys_diags_ihealth",
}


_parse_sys_minimal = _parse_minimal


_VCMP_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 42 — vcmp.* (4 kinds).
    "guest": "vcmp_guests",
    "traffic-profile": "vcmp_traffic_profiles",
    "virtual-disk": "vcmp_virtual_disks",
    "virtual-disk-template": "vcmp_virtual_disk_templates",
}


_parse_vcmp_minimal = _parse_minimal


_CM_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 43 — cm.* follow-ons (2 kinds).
    "ha-group": "cm_ha_groups",
    "config-sync": "cm_config_sync",
}


_parse_cm_minimal = _parse_minimal


_CLI_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 44 — cli.* (8 kinds).
    "admin-partitions": "cli_admin_partitions",
    "alias private": "cli_alias_private",
    "alias shared": "cli_alias_shared",
    "global-settings": "cli_global_settings",
    "preference": "cli_preference",
    "script": "cli_script",
    "transaction": "cli_transaction",
    "version": "cli_version",
}


_parse_cli_minimal = _parse_minimal


_API_PROTECTION_MINIMAL_DISPATCH: dict[str, str] = {
    # Bundle 45 — api-protection.* (3 kinds).
    "profile apiprotection": "api_protection_profile_apiprotection",
    "response": "api_protection_response",
    "server": "api_protection_server",
}


# Audit follow-up modules — asm.*, ilx.*, wom.*.
_ASM_MINIMAL_DISPATCH: dict[str, str] = {
    "policy": "asm_policies",
}

_ILX_MINIMAL_DISPATCH: dict[str, str] = {
    "global-settings": "ilx_global_settings",
}

_WOM_MINIMAL_DISPATCH: dict[str, str] = {
    "endpoint-discovery": "wom_endpoint_discovery",
}


_parse_asm_minimal = _parse_minimal
_parse_ilx_minimal = _parse_minimal
_parse_wom_minimal = _parse_minimal


# Sibling-completeness follow-up: top-level ``analytics`` module
# (distinct from ``ltm dns analytics global-settings``).
_ANALYTICS_MINIMAL_DISPATCH: dict[str, str] = {
    "global-settings": "analytics_global_settings",
}


_parse_analytics_minimal = _parse_minimal


# Module -> (dispatch table, parser function).  Used by the generic
# minimal-dispatch pre-pass in ``parse_bigip_conf`` so every
# bundles 17-45 minimal kind routes through the same code path.
_MinimalParserFn = Callable[[str, str, str, DocumentBuffer, "_Block"], object]
_MINIMAL_DISPATCH_BY_MODULE: dict[str, tuple[dict[str, str], _MinimalParserFn]] = {}


_parse_api_protection_minimal = _parse_minimal


_MINIMAL_DISPATCH_BY_MODULE.update(
    {
        "net": (_NET_MINIMAL_DISPATCH, _parse_net_minimal),
        "apm": (_APM_MINIMAL_DISPATCH, _parse_apm_minimal),
        "pem": (_PEM_MINIMAL_DISPATCH, _parse_pem_minimal),
        "sys": (_SYS_MINIMAL_DISPATCH, _parse_sys_minimal),
        "vcmp": (_VCMP_MINIMAL_DISPATCH, _parse_vcmp_minimal),
        "cm": (_CM_MINIMAL_DISPATCH, _parse_cm_minimal),
        "cli": (_CLI_MINIMAL_DISPATCH, _parse_cli_minimal),
        "api-protection": (_API_PROTECTION_MINIMAL_DISPATCH, _parse_api_protection_minimal),
        # Audit follow-up modules.
        "asm": (_ASM_MINIMAL_DISPATCH, _parse_asm_minimal),
        "ilx": (_ILX_MINIMAL_DISPATCH, _parse_ilx_minimal),
        "wom": (_WOM_MINIMAL_DISPATCH, _parse_wom_minimal),
        # Sibling-completeness follow-up: top-level ``analytics`` module.
        "analytics": (_ANALYTICS_MINIMAL_DISPATCH, _parse_analytics_minimal),
    }
)


# Bundle 16 — ltm auth.* profiles (11 kinds).  All share the
# minimal shape; ``defaults_from`` is captured because most auth
# kinds inherit from a system-default profile chain.

_LTM_AUTH_DISPATCH: dict[str, str] = {
    "auth profile": "ltm_auth_profiles",
    "auth ldap": "ltm_auth_ldap",
    "auth radius": "ltm_auth_radius",
    "auth radius-server": "ltm_auth_radius_servers",
    "auth tacacs": "ltm_auth_tacacs",
    "auth crldp-server": "ltm_auth_crldp_servers",
    "auth ocsp-responder": "ltm_auth_ocsp_responders",
    "auth kerberos-delegation": "ltm_auth_kerberos_delegations",
    "auth ssl-cc-ldap": "ltm_auth_ssl_cc_ldap",
    "auth ssl-crldp": "ltm_auth_ssl_crldp",
    "auth ssl-ocsp": "ltm_auth_ssl_ocsp",
}


def _parse_ltm_auth(
    full_path: str,
    body: str,
    kind_label: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipLtmAuthObject:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1] if full_path else ""
    return BigipLtmAuthObject(
        name=name,
        full_path=full_path,
        kind=kind_label,
        description=_description(props),
        defaults_from=props.get("defaults-from", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_ltm_message_routing(
    full_path: str,
    body: str,
    kind_label: str,
    source_map: DocumentBuffer,
    block: _Block,
) -> BigipLtmMessageRoutingObject:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1] if full_path else ""
    return BigipLtmMessageRoutingObject(
        name=name,
        full_path=full_path,
        kind=kind_label,
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Maps in-config obj_type (e.g. ``"message-routing diameter peer"``)
# to the ``BigipConfig`` attribute that holds it.  Bundle 15 covers
# 14 three-word kinds and 6 four-word ``... profile *`` kinds across
# four protocols (diameter / sip / mqtt / generic).
_LTM_MESSAGE_ROUTING_DISPATCH: dict[str, str] = {
    "message-routing diameter peer": "ltm_mr_diameter_peers",
    "message-routing diameter route": "ltm_mr_diameter_routes",
    "message-routing diameter profile router": "ltm_mr_diameter_profile_router",
    "message-routing diameter profile session": "ltm_mr_diameter_profile_session",
    "message-routing diameter transport-config": "ltm_mr_diameter_transport_config",
    "message-routing sip peer": "ltm_mr_sip_peers",
    "message-routing sip route": "ltm_mr_sip_routes",
    "message-routing sip profile router": "ltm_mr_sip_profile_router",
    "message-routing sip profile session": "ltm_mr_sip_profile_session",
    "message-routing sip transport-config": "ltm_mr_sip_transport_config",
    "message-routing mqtt peer": "ltm_mr_mqtt_peers",
    "message-routing mqtt route": "ltm_mr_mqtt_routes",
    "message-routing mqtt profile router": "ltm_mr_mqtt_profile_router",
    "message-routing mqtt profile session": "ltm_mr_mqtt_profile_session",
    "message-routing mqtt transport-config": "ltm_mr_mqtt_transport_config",
    "message-routing generic peer": "ltm_mr_generic_peers",
    "message-routing generic protocol": "ltm_mr_generic_protocols",
    "message-routing generic route": "ltm_mr_generic_routes",
    "message-routing generic router": "ltm_mr_generic_routers",
    "message-routing generic transport-config": "ltm_mr_generic_transport_config",
}


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


def _firewall_rules_summary(props: dict[str, str]) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """Walk a firewall ``rules { ... }`` block and return
    ``(rule_names, rule_list_refs)`` — top-level keys plus the
    ``rule-list /Common/...`` PathRef from each rule body, in
    document order.
    """
    raw = props.get("rules", "")
    if not raw or not raw.startswith("{"):
        return ((), ())
    inner = _strip_outer_braces(raw)
    names: list[str] = []
    refs: list[str] = []
    for sub in _parse_properties_with_spans(inner).values():
        names.append(sub.key)
        if sub.value.startswith("{"):
            inner_body = _strip_outer_braces(sub.value)
            sub_props = _parse_properties(inner_body)
            if "rule-list" in sub_props:
                refs.append(sub_props["rule-list"])
    return (tuple(names), tuple(refs))


def _parse_security_firewall_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallPolicy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    rule_names, rule_refs = _firewall_rules_summary(props)
    return BigipSecurityFirewallPolicy(
        name=name,
        full_path=full_path,
        description=_description(props),
        rules=rule_names,
        rule_lists=rule_refs,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_address_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallAddressList:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    addresses = _list_field(props, "addresses")
    address_lists = _list_field(props, "address-lists")
    fqdns = _list_field(props, "fqdns")
    return BigipSecurityFirewallAddressList(
        name=name,
        full_path=full_path,
        description=_description(props),
        addresses=addresses,
        address_lists=address_lists,
        fqdns=fqdns,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_global_rules(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallGlobalRules:
    props = _parse_properties(body)
    rule_names, _ = _firewall_rules_summary(props)
    return BigipSecurityFirewallGlobalRules(
        description=_description(props),
        rules=rule_names,
        enforced_policy=props.get("enforced-policy", ""),
        staged_policy=props.get("staged-policy", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_management_ip_rules(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallManagementIpRules:
    props = _parse_properties(body)
    rule_names, _ = _firewall_rules_summary(props)
    return BigipSecurityFirewallManagementIpRules(
        description=_description(props),
        rules=rule_names,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_schedule(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallSchedule:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityFirewallSchedule(
        name=name,
        full_path=full_path,
        description=_description(props),
        daily_hour_end=props.get("daily-hour-end", ""),
        daily_hour_start=props.get("daily-hour-start", ""),
        days_of_week=_list_field(props, "days-of-week"),
        date_valid_end=props.get("date-valid-end", ""),
        date_valid_start=props.get("date-valid-start", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_user_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallUserList:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityFirewallUserList(
        name=name,
        full_path=full_path,
        description=_description(props),
        users=_list_field(props, "users"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_user_domain(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallUserDomain:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityFirewallUserDomain(
        name=name,
        full_path=full_path,
        description=_description(props),
        domain=props.get("domain", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_global_fqdn_policy(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallGlobalFqdnPolicy:
    props = _parse_properties(body)
    return BigipSecurityFirewallGlobalFqdnPolicy(
        description=_description(props),
        context=props.get("context", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_port_misuse_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallPortMisusePolicy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityFirewallPortMisusePolicy(
        name=name,
        full_path=full_path,
        description=_description(props),
        default_log=props.get("default-log", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_on_demand_compilation(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallOnDemandCompilation:
    props = _parse_properties(body)
    return BigipSecurityFirewallOnDemandCompilation(
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_on_demand_rule_deploy(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallOnDemandRuleDeploy:
    props = _parse_properties(body)
    return BigipSecurityFirewallOnDemandRuleDeploy(
        description=_description(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_uuid_default_autogenerate(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallUuidDefaultAutogenerate:
    props = _parse_properties(body)
    return BigipSecurityFirewallUuidDefaultAutogenerate(
        description=_description(props),
        auto_generate_uuid=props.get("auto-generate-uuid", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_firewall_config_change_log(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityFirewallConfigChangeLog:
    props = _parse_properties(body)
    return BigipSecurityFirewallConfigChangeLog(
        description=_description(props),
        log_publisher=props.get("log-publisher", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Bundle 10a parsers.


def _parse_security_nat_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityNatPolicy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    rule_names, rule_refs = _firewall_rules_summary(props)
    return BigipSecurityNatPolicy(
        name=name,
        full_path=full_path,
        description=_description(props),
        rules=rule_names,
        rule_lists=rule_refs,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_nat_source_translation(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityNatSourceTranslation:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityNatSourceTranslation(
        name=name,
        full_path=full_path,
        description=_description(props),
        type_=props.get("type", ""),
        addresses=_list_field(props, "addresses"),
        ports=_list_field(props, "ports"),
        traffic_group=props.get("traffic-group", ""),
        egress_interfaces_disabled="egress-interfaces-disabled" in props,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_nat_destination_translation(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityNatDestinationTranslation:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityNatDestinationTranslation(
        name=name,
        full_path=full_path,
        description=_description(props),
        type_=props.get("type", ""),
        addresses=_list_field(props, "addresses"),
        ports=_list_field(props, "ports"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_log_profile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityLogProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityLogProfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        application_data=props.get("application", ""),
        network_data=props.get("network", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_dos_profile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityDosProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityDosProfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        app_service=props.get("app-service", ""),
        threshold_sensitivity=props.get("threshold-sensitivity", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_ip_intelligence_feed_list(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityIpIntelligenceFeedList:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityIpIntelligenceFeedList(
        name=name,
        full_path=full_path,
        description=_description(props),
        feeds=_list_field(props, "feeds"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_ip_intelligence_global_policy(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityIpIntelligenceGlobalPolicy:
    props = _parse_properties(body)
    return BigipSecurityIpIntelligenceGlobalPolicy(
        description=_description(props),
        log_blacklist_category=props.get("log-blacklist-category", ""),
        log_publisher=props.get("log-publisher", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_zone(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityZone:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityZone(
        name=name,
        full_path=full_path,
        description=_description(props),
        vlans=_list_field(props, "vlans"),
        tunnels=_list_field(props, "tunnels"),
        interfaces=_list_field(props, "interfaces"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_protected_zone(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityProtectedZone:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityProtectedZone(
        name=name,
        full_path=full_path,
        description=_description(props),
        enabled=_state_flag(props),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_packet_filter_policy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityPacketFilterPolicy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    rule_names, _ = _firewall_rules_summary(props)
    return BigipSecurityPacketFilterPolicy(
        name=name,
        full_path=full_path,
        description=_description(props),
        rules=rule_names,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_packet_filter_default_rules(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityPacketFilterDefaultRules:
    props = _parse_properties(body)
    return BigipSecurityPacketFilterDefaultRules(
        description=_description(props),
        action=props.get("action", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_ssh_profile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecuritySshProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecuritySshProfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        defaults_from=props.get("defaults-from", ""),
        timeout=props.get("timeout", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_http_profile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityHttpProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityHttpProfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        defaults_from=props.get("defaults-from", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_security_bot_defense_profile(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipSecurityBotDefenseProfile:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipSecurityBotDefenseProfile(
        name=name,
        full_path=full_path,
        description=_description(props),
        app_service=props.get("app-service", ""),
        template=props.get("template", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# Bundle 10b — uses the same shared minimal parser as every other
# module.  Kept as an alias so the existing call site at
# ``module == "security"`` stays grep-able.
_parse_security_minimal = _parse_minimal


# bundle-10b dispatch table — maps the in-config two-word obj_type
# (e.g. ``"dos virtual"``) to the ``BigipConfig`` attribute that
# stores it.  Used by the ``module == "security"`` block below.
_SECURITY_MINIMAL_DISPATCH: dict[str, str] = {
    "analytics settings": "security_analytics_settings",
    "anti-fraud profile": "security_anti_fraud_profiles",
    "anti-fraud signatures-update": "security_anti_fraud_signatures_update",
    "blacklist-publisher category": "security_blacklist_publisher_categories",
    "blacklist-publisher profile": "security_blacklist_publisher_profiles",
    "bot-defense signature": "security_bot_defense_signatures",
    "bot-defense signature-category": "security_bot_defense_signature_categories",
    "cloud-services connector": "security_cloud_services_connectors",
    "datasync background-tasks": "security_datasync_background_tasks",
    "datasync global-profile": "security_datasync_global_profiles",
    "datasync local-profile": "security_datasync_local_profiles",
    "debug drop-redirect-stats": "security_debug_drop_redirect_stats",
    "debug matcher": "security_debug_matcher",
    "debug register": "security_debug_register",
    "device device-context": "security_device_device_context",
    "dos autodos-file-object": "security_dos_autodos_file_objects",
    "dos behavioral-signature": "security_dos_behavioral_signatures",
    "dos bot-signature": "security_dos_bot_signatures",
    "dos bot-signature-category": "security_dos_bot_signature_categories",
    "dos device-config": "security_dos_device_config",
    "dos dns-nxdomain-stat": "security_dos_dns_nxdomain_stat",
    "dos dos-signature": "security_dos_dos_signatures",
    "dos dynamic-signatures": "security_dos_dynamic_signatures",
    "dos ip-uncommon-protolist": "security_dos_ip_uncommon_protolists",
    "dos l4bdos-file-object": "security_dos_l4bdos_file_objects",
    "dos network-whitelist": "security_dos_network_whitelists",
    "dos stress-stats": "security_dos_stress_stats",
    "dos udp-portlist": "security_dos_udp_portlists",
    "dos virtual": "security_dos_virtuals",
    "flowspec-route-injector profile": "security_flowspec_route_injector_profiles",
    "ip-intelligence blacklist-category": "security_ip_intelligence_blacklist_categories",
    "protocol-inspection common-config": "security_protocol_inspection_common_config",
    "protocol-inspection learning-stats": "security_protocol_inspection_learning_stats",
    "protocol-inspection profile": "security_protocol_inspection_profiles",
    "protocol-inspection signature": "security_protocol_inspection_signatures",
    "scrubber profile": "security_scrubber_profiles",
    "ssh ciphers": "security_ssh_ciphers",
    # Audit follow-ups.
    "shared-objects port-list": "security_shared_objects_port_lists",
    "shared-objects address-list": "security_shared_objects_address_lists",
    "dos ipv6-ext-hdr": "security_dos_ipv6_ext_hdr",
    # Sibling-completeness follow-up.
    "dos profile-signatures": "security_dos_profile_signatures",
}


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


# Bundle 12 parsers — gtm listeners / link / topology /
# distributed-app / global-settings singletons.


def _parse_gtm_listener(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmListener:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmListener(
        name=name,
        full_path=full_path,
        description=_description(props),
        address=props.get("address", ""),
        port=props.get("port", ""),
        ip_protocol=props.get("ip-protocol", ""),
        mask=props.get("mask", ""),
        pool=props.get("pool", ""),
        profiles=_list_field(props, "profiles"),
        rules=_list_field(props, "rules"),
        source_address_translation=props.get("source-address-translation", ""),
        state=_state_flag(props),
        vlans=_list_field(props, "vlans"),
        vlans_disabled="vlans-disabled" in props,
        vlans_enabled="vlans-enabled" in props,
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_listener_doh_proxy(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmListenerDohProxy:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmListenerDohProxy(
        name=name,
        full_path=full_path,
        description=_description(props),
        address=props.get("address", ""),
        port=props.get("port", ""),
        pool=props.get("pool", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_listener_doh_server(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmListenerDohServer:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmListenerDohServer(
        name=name,
        full_path=full_path,
        description=_description(props),
        address=props.get("address", ""),
        port=props.get("port", ""),
        pool=props.get("pool", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_link(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmLink:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmLink(
        name=name,
        full_path=full_path,
        description=_description(props),
        datacenter=props.get("datacenter", ""),
        monitor=props.get("monitor", ""),
        prober_pool=props.get("prober-pool", ""),
        state=_state_flag(props),
        weight=props.get("weight", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_distributed_app(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmDistributedApp:
    props = _parse_properties(body)
    name = full_path.rsplit("/", 1)[-1]
    return BigipGtmDistributedApp(
        name=name,
        full_path=full_path,
        description=_description(props),
        wide_ips=_list_field(props, "wide-ips"),
        persist_cidr=props.get("persist-cidr", ""),
        dependency_level=props.get("dependency-level", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_topology(
    identifier: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmTopology:
    """Parse a ``gtm topology`` stanza.

    Unlike every other kind, the header carries a multi-token
    condition (``ldns: subnet 10.0.0.0/8 server: subnet 10.1.0.0/16``)
    in place of a full-path; the caller passes that condition as the
    identifier.
    """
    props = _parse_properties(body)
    return BigipGtmTopology(
        name=identifier,
        full_path=identifier,
        description=_description(props),
        order=props.get("order", ""),
        score=props.get("score", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_global_settings_general(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmGlobalSettingsGeneral:
    props = _parse_properties(body)
    return BigipGtmGlobalSettingsGeneral(
        description=_description(props),
        auto_discovery=props.get("auto-discovery", ""),
        synchronization=props.get("synchronization", ""),
        synchronization_group_name=props.get("synchronization-group-name", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_global_settings_load_balancing(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmGlobalSettingsLoadBalancing:
    props = _parse_properties(body)
    return BigipGtmGlobalSettingsLoadBalancing(
        description=_description(props),
        topology_longest_match=props.get("topology-longest-match", ""),
        ignore_path_ttl=props.get("ignore-path-ttl", ""),
        respect_dependent_objects=props.get("respect-dependent-objects", ""),
        verify_vs_availability=props.get("verify-vs-availability", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_global_settings_metrics(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmGlobalSettingsMetrics:
    props = _parse_properties(body)
    return BigipGtmGlobalSettingsMetrics(
        description=_description(props),
        metrics_collection_protocols=_list_field(props, "metrics-collection-protocols"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_gtm_global_settings_metrics_exclusions(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipGtmGlobalSettingsMetricsExclusions:
    props = _parse_properties(body)
    return BigipGtmGlobalSettingsMetricsExclusions(
        description=_description(props),
        addresses=props.get("addresses", ""),
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


# auth.* parsers


def _list_field(props: dict[str, str], key: str) -> tuple[str, ...]:
    """Extract a brace-delimited list field, returning ``()`` when absent.

    Handles both ``key { a b c }`` (flat list) and the nested-block
    form used for sub-objects (which falls back to top-level keys).
    """
    raw = props.get(key, "")
    if not raw:
        return ()
    if raw.startswith("{"):
        return tuple(_parse_list_block(raw))
    # Bare-value form ``servers a.b.c.d`` — surface as a single entry.
    return (raw,)


def _parse_auth_partition(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthPartition:
    props = _parse_properties(body)
    return BigipAuthPartition(
        name=full_path,
        full_path=full_path,
        description=_description(props),
        default_route_domain=props.get("default-route-domain", ""),
        inherited_traffic_group=props.get("inherited-traffic-group", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_user(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthUser:
    props = _parse_properties(body)
    return BigipAuthUser(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        description=_description(props),
        partition=props.get("partition", ""),
        shell=props.get("shell", ""),
        encrypted_password=props.get("encrypted-password", ""),
        partition_access=_list_field(props, "partition-access"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_password(body: str, source_map: DocumentBuffer, block: _Block) -> BigipAuthPassword:
    props = _parse_properties(body)
    return BigipAuthPassword(
        expiration_warning=props.get("expiration-warning", ""),
        minimum_length=props.get("minimum-length", ""),
        policy=props.get("policy", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_password_policy(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthPasswordPolicy:
    props = _parse_properties(body)
    return BigipAuthPasswordPolicy(
        expiration_warning=props.get("expiration-warning", ""),
        max_duration=props.get("max-duration", ""),
        max_login_failures=props.get("max-login-failures", ""),
        min_duration=props.get("min-duration", ""),
        minimum_length=props.get("minimum-length", ""),
        minimum_regular_characters=props.get("minimum-regular-characters", ""),
        password_memory=props.get("password-memory", ""),
        policy_enforcement=props.get("policy-enforcement", ""),
        required_lowercase=props.get("required-lowercase", ""),
        required_numeric=props.get("required-numeric", ""),
        required_special=props.get("required-special", ""),
        required_uppercase=props.get("required-uppercase", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_source(body: str, source_map: DocumentBuffer, block: _Block) -> BigipAuthSource:
    props = _parse_properties(body)
    return BigipAuthSource(
        fallback=props.get("fallback", ""),
        type_=props.get("type", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_remote_role(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthRemoteRole:
    props = _parse_properties(body)
    return BigipAuthRemoteRole(
        role_info=_list_field(props, "role-info"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_remote_user(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthRemoteUser:
    props = _parse_properties(body)
    return BigipAuthRemoteUser(
        default_partition=props.get("default-partition", ""),
        default_role=props.get("default-role", ""),
        remote_console_access=props.get("remote-console-access", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_login_failures(
    body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthLoginFailures:
    del body
    return BigipAuthLoginFailures(
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_ldap(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthLdap:
    props = _parse_properties(body)
    return BigipAuthLdap(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        bind_dn=props.get("bind-dn", ""),
        bind_pw=props.get("bind-pw", ""),
        bind_timeout=props.get("bind-timeout", ""),
        check_host_attr=props.get("check-host-attr", ""),
        check_roles_group=props.get("check-roles-group", ""),
        filter_=props.get("filter", ""),
        group_dn=props.get("group-dn", ""),
        group_member_attribute=props.get("group-member-attribute", ""),
        idle_timeout=props.get("idle-timeout", ""),
        ignore_auth_info_unavail=props.get("ignore-auth-info-unavail", ""),
        ignore_unknown_user=props.get("ignore-unknown-user", ""),
        login_attribute=props.get("login-attribute", ""),
        port=props.get("port", ""),
        scope=props.get("scope", ""),
        search_base_dn=props.get("search-base-dn", ""),
        search_timeout=props.get("search-timeout", ""),
        servers=_list_field(props, "servers"),
        ssl=props.get("ssl", ""),
        ssl_ca_cert=props.get("ssl-ca-cert", ""),
        ssl_check_peer=props.get("ssl-check-peer", ""),
        ssl_client_cert=props.get("ssl-client-cert", ""),
        ssl_client_key=props.get("ssl-client-key", ""),
        user_template=props.get("user-template", ""),
        version=props.get("version", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_radius(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthRadius:
    props = _parse_properties(body)
    return BigipAuthRadius(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        service_type=props.get("service-type", ""),
        servers=_list_field(props, "servers"),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_radius_server(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthRadiusServer:
    props = _parse_properties(body)
    return BigipAuthRadiusServer(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        server=props.get("server", ""),
        port=props.get("port", ""),
        secret=props.get("secret", ""),
        timeout=props.get("timeout", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_tacacs(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthTacacs:
    props = _parse_properties(body)
    return BigipAuthTacacs(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        protocol=props.get("protocol", ""),
        secret=props.get("secret", ""),
        service=props.get("service", ""),
        servers=_list_field(props, "servers"),
        accounting=props.get("accounting", ""),
        authentication=props.get("authentication", ""),
        debug=props.get("debug", ""),
        encryption=props.get("encryption", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_cert_ldap(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthCertLdap:
    props = _parse_properties(body)
    return BigipAuthCertLdap(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        bind_dn=props.get("bind-dn", ""),
        bind_pw=props.get("bind-pw", ""),
        bind_timeout=props.get("bind-timeout", ""),
        idle_timeout=props.get("idle-timeout", ""),
        login_attribute=props.get("login-attribute", ""),
        port=props.get("port", ""),
        scope=props.get("scope", ""),
        search_base_dn=props.get("search-base-dn", ""),
        search_timeout=props.get("search-timeout", ""),
        servers=_list_field(props, "servers"),
        ssl=props.get("ssl", ""),
        user_template=props.get("user-template", ""),
        version=props.get("version", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


def _parse_auth_apm_auth(
    full_path: str, body: str, source_map: DocumentBuffer, block: _Block
) -> BigipAuthApmAuth:
    props = _parse_properties(body)
    return BigipAuthApmAuth(
        name=full_path.rsplit("/", 1)[-1],
        full_path=full_path,
        profile=props.get("profile", ""),
        range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
    )


# ---------------------------------------------------------------------------
# Typed dispatch tables
# ---------------------------------------------------------------------------
#
# Every typed kind whose parser fits the canonical signature
# ``(full_path, body, source_map, block) -> Type`` lives in
# :data:`_NAMED_DISPATCH` keyed by ``(module, obj_type)``.  Singleton
# typed kinds whose parser is just ``(body, source_map, block) -> Type``
# live in :data:`_SINGLETON_DISPATCH`.  Both replace the long
# ``if module == "X": if obj_type == "Y": ...`` chains in
# :func:`parse_bigip_conf`.
#
# Family parsers that need an extra TMSH sub-type argument (``ltm
# profile <type>``, ``ltm monitor <type>``, ``gtm pool <record-type>``,
# ``pem profile <variant>``, …) and the gtm-topology special case are
# kept as inline branches in ``parse_bigip_conf``.

_NamedTypedParserFn = Callable[[str, str, DocumentBuffer, "_Block"], object]
_SingletonTypedParserFn = Callable[[str, DocumentBuffer, "_Block"], object]

_NAMED_DISPATCH: dict[tuple[str, str], tuple[str, _NamedTypedParserFn]] = {
    # net.*
    ("net", "route"): ("net_routes", _parse_net_route),
    ("net", "vlan"): ("net_vlans", _parse_net_vlan),
    ("net", "self"): ("net_selves", _parse_net_self),
    ("net", "route-domain"): ("net_route_domains", _parse_net_route_domain),
    ("net", "port-list"): ("net_port_lists", _parse_net_port_list),
    ("net", "interface"): ("net_interfaces", _parse_net_interface),
    ("net", "dns-resolver"): ("net_dns_resolvers", _parse_net_dns_resolver),
    ("net", "tunnels tunnel"): ("net_tunnels", _parse_net_tunnel),
    ("net", "stp"): ("net_stps", _parse_net_stp),
    # sys.*
    ("sys", "provision"): ("sys_provisions", _parse_sys_provision),
    ("sys", "folder"): ("sys_folders", _parse_sys_folder),
    ("sys", "file ssl-cert"): ("sys_file_ssl_certs", _parse_sys_file_ssl_cert),
    ("sys", "file ssl-key"): ("sys_file_ssl_keys", _parse_sys_file_ssl_key),
    ("sys", "management-route"): ("sys_management_routes", _parse_sys_management_route),
    # security.*  (named — full_path required)
    ("security", "firewall port-list"): (
        "security_firewall_port_lists",
        _parse_security_firewall_port_list,
    ),
    ("security", "firewall rule-list"): (
        "security_firewall_rule_lists",
        _parse_security_firewall_rule_list,
    ),
    ("security", "firewall config-entity-id"): (
        "security_firewall_config_entity_ids",
        _parse_security_firewall_config_entity_id,
    ),
    ("security", "firewall policy"): (
        "security_firewall_policies",
        _parse_security_firewall_policy,
    ),
    ("security", "firewall address-list"): (
        "security_firewall_address_lists",
        _parse_security_firewall_address_list,
    ),
    ("security", "firewall schedule"): (
        "security_firewall_schedules",
        _parse_security_firewall_schedule,
    ),
    ("security", "firewall user-list"): (
        "security_firewall_user_lists",
        _parse_security_firewall_user_list,
    ),
    ("security", "firewall user-domain"): (
        "security_firewall_user_domains",
        _parse_security_firewall_user_domain,
    ),
    ("security", "firewall port-misuse-policy"): (
        "security_firewall_port_misuse_policies",
        _parse_security_firewall_port_misuse_policy,
    ),
    ("security", "ip-intelligence policy"): (
        "security_ip_intelligence_policies",
        _parse_security_ip_intelligence_policy,
    ),
    ("security", "protocol-inspection compliance-map"): (
        "security_pi_compliance_maps",
        _parse_security_pi_compliance_map,
    ),
    ("security", "protocol-inspection compliance-objects"): (
        "security_pi_compliance_objects",
        _parse_security_pi_compliance_object,
    ),
    ("security", "device-id attribute"): (
        "security_device_id_attributes",
        _parse_security_device_id_attribute,
    ),
    ("security", "nat policy"): ("security_nat_policies", _parse_security_nat_policy),
    ("security", "nat source-translation"): (
        "security_nat_source_translations",
        _parse_security_nat_source_translation,
    ),
    ("security", "nat destination-translation"): (
        "security_nat_destination_translations",
        _parse_security_nat_destination_translation,
    ),
    ("security", "log profile"): ("security_log_profiles", _parse_security_log_profile),
    ("security", "dos profile"): ("security_dos_profiles", _parse_security_dos_profile),
    ("security", "ip-intelligence feed-list"): (
        "security_ip_intelligence_feed_lists",
        _parse_security_ip_intelligence_feed_list,
    ),
    ("security", "zone"): ("security_zones", _parse_security_zone),
    ("security", "protected zone"): (
        "security_protected_zones",
        _parse_security_protected_zone,
    ),
    ("security", "packet-filter policy"): (
        "security_packet_filter_policies",
        _parse_security_packet_filter_policy,
    ),
    ("security", "ssh profile"): ("security_ssh_profiles", _parse_security_ssh_profile),
    ("security", "http profile"): ("security_http_profiles", _parse_security_http_profile),
    ("security", "bot-defense profile"): (
        "security_bot_defense_profiles",
        _parse_security_bot_defense_profile,
    ),
    # apm.*
    ("apm", "ephemeral-auth ssh-security-config"): (
        "apm_ephemeral_auth_ssh_security_configs",
        _parse_apm_ssh_security_config,
    ),
    ("apm", "oauth db-instance"): ("apm_oauth_db_instances", _parse_apm_oauth_db_instance),
    ("apm", "policy access-policy"): (
        "apm_policy_access_policies",
        _parse_apm_policy_access_policy,
    ),
    ("apm", "policy customization-source"): (
        "apm_policy_customization_sources",
        _parse_apm_policy_customization_source,
    ),
    ("apm", "policy policy-item"): ("apm_policy_items", _parse_apm_policy_item),
    # cm.*
    ("cm", "cert"): ("cm_certs", _parse_cm_cert),
    ("cm", "key"): ("cm_keys", _parse_cm_key),
    ("cm", "device"): ("cm_devices", _parse_cm_device),
    ("cm", "device-group"): ("cm_device_groups", _parse_cm_device_group),
    ("cm", "traffic-group"): ("cm_traffic_groups", _parse_cm_traffic_group),
    ("cm", "trust-domain"): ("cm_trust_domains", _parse_cm_trust_domain),
    # gtm.*  (most are named, singletons go in _SINGLETON_DISPATCH)
    ("gtm", "datacenter"): ("gtm_datacenters", _parse_gtm_datacenter),
    ("gtm", "server"): ("gtm_servers", _parse_gtm_server),
    ("gtm", "prober-pool"): ("gtm_prober_pools", _parse_gtm_prober_pool),
    ("gtm", "region"): ("gtm_regions", _parse_gtm_region),
    ("gtm", "rule"): ("gtm_rules", _parse_gtm_rule),
    ("gtm", "listener"): ("gtm_listeners", _parse_gtm_listener),
    ("gtm", "listener-doh-proxy"): (
        "gtm_listener_doh_proxies",
        _parse_gtm_listener_doh_proxy,
    ),
    ("gtm", "listener-doh-server"): (
        "gtm_listener_doh_servers",
        _parse_gtm_listener_doh_server,
    ),
    ("gtm", "link"): ("gtm_links", _parse_gtm_link),
    ("gtm", "distributed-app"): ("gtm_distributed_apps", _parse_gtm_distributed_app),
    # pem.*
    ("pem", "policy"): ("pem_policies", _parse_pem_policy),
    ("pem", "irule"): ("pem_rules", _parse_pem_irule),
    ("pem", "listener"): ("pem_listeners", _parse_pem_listener),
    ("pem", "forwarding-endpoint"): (
        "pem_forwarding_endpoints",
        _parse_pem_forwarding_endpoint,
    ),
    ("pem", "interception-endpoint"): (
        "pem_interception_endpoints",
        _parse_pem_interception_endpoint,
    ),
    ("pem", "service-chain-endpoint"): (
        "pem_service_chain_endpoints",
        _parse_pem_service_chain_endpoint,
    ),
    ("pem", "quota-mgmt rating-group"): ("pem_rating_groups", _parse_pem_rating_group),
    # auth.* (named — singletons handled separately)
    ("auth", "partition"): ("auth_partitions", _parse_auth_partition),
    ("auth", "user"): ("auth_users", _parse_auth_user),
    ("auth", "ldap"): ("auth_ldaps", _parse_auth_ldap),
    ("auth", "radius"): ("auth_radius", _parse_auth_radius),
    ("auth", "radius-server"): ("auth_radius_servers", _parse_auth_radius_server),
    ("auth", "tacacs"): ("auth_tacacs", _parse_auth_tacacs),
    ("auth", "cert-ldap"): ("auth_cert_ldaps", _parse_auth_cert_ldap),
    ("auth", "apm-auth"): ("auth_apm_auths", _parse_auth_apm_auth),
    # ltm.*  (named — kinds that take only the canonical 4-arg parser
    # signature; the family parsers that need a sub-type stay inline)
    ("ltm", "cipher group"): ("ltm_cipher_groups", _parse_ltm_cipher_group),
    ("ltm", "cipher rule"): ("ltm_cipher_rules", _parse_ltm_cipher_rule),
    ("ltm", "nat"): ("ltm_nats", _parse_ltm_nat),
    ("ltm", "snat"): ("ltm_snats", _parse_ltm_snat),
    ("ltm", "snat-translation"): ("ltm_snat_translations", _parse_ltm_snat_translation),
    ("ltm", "policy-strategy"): ("ltm_policy_strategies", _parse_ltm_policy_strategy),
    ("ltm", "traffic-class"): ("ltm_traffic_classes", _parse_ltm_traffic_class),
    ("ltm", "traffic-matching-criteria"): (
        "ltm_traffic_matching_criteria",
        _parse_ltm_traffic_matching_criteria,
    ),
    ("ltm", "ifile"): ("ltm_ifiles", _parse_ltm_ifile),
    ("ltm", "eviction-policy"): ("ltm_eviction_policies", _parse_ltm_eviction_policy),
    ("ltm", "dns nameserver"): ("ltm_dns_nameservers", _parse_ltm_dns_nameserver),
    ("ltm", "dns tsig-key"): ("ltm_dns_tsig_keys", _parse_ltm_dns_tsig_key),
    ("ltm", "dns zone"): ("ltm_dns_zones", _parse_ltm_dns_zone),
    ("ltm", "dns dnssec key"): ("ltm_dns_dnssec_keys", _parse_ltm_dns_dnssec_key),
    ("ltm", "dns dnssec zone"): ("ltm_dns_dnssec_zones", _parse_ltm_dns_dnssec_zone),
    ("ltm", "dns cache resolver"): (
        "ltm_dns_cache_resolvers",
        _parse_ltm_dns_cache_resolver,
    ),
    ("ltm", "dns cache transparent"): (
        "ltm_dns_cache_transparent",
        _parse_ltm_dns_cache_transparent,
    ),
    ("ltm", "dns cache validating-resolver"): (
        "ltm_dns_cache_validating_resolvers",
        _parse_ltm_dns_cache_validating_resolver,
    ),
    ("ltm", "dns hpke key"): ("ltm_dns_hpke_keys", _parse_ltm_dns_hpke_key),
    ("ltm", "dns hpke profile"): ("ltm_dns_hpke_profiles", _parse_ltm_dns_hpke_profile),
}

_SINGLETON_DISPATCH: dict[tuple[str, str], tuple[str, _SingletonTypedParserFn]] = {
    # sys.*
    ("sys", "dns"): ("sys_dns", _parse_sys_dns),
    ("sys", "ntp"): ("sys_ntp", _parse_sys_ntp),
    ("sys", "snmp"): ("sys_snmp", _parse_sys_snmp),
    ("sys", "global-settings"): ("sys_global_settings", _parse_sys_global_settings),
    # security.* (singleton-only kinds — no full_path)
    ("security", "firewall global-rules"): (
        "security_firewall_global_rules",
        _parse_security_firewall_global_rules,
    ),
    ("security", "firewall management-ip-rules"): (
        "security_firewall_management_ip_rules",
        _parse_security_firewall_management_ip_rules,
    ),
    ("security", "firewall global-fqdn-policy"): (
        "security_firewall_global_fqdn_policy",
        _parse_security_firewall_global_fqdn_policy,
    ),
    ("security", "firewall on-demand-compilation"): (
        "security_firewall_on_demand_compilation",
        _parse_security_firewall_on_demand_compilation,
    ),
    ("security", "firewall on-demand-rule-deploy"): (
        "security_firewall_on_demand_rule_deploy",
        _parse_security_firewall_on_demand_rule_deploy,
    ),
    ("security", "firewall uuid-default-autogenerate"): (
        "security_firewall_uuid_default_autogenerate",
        _parse_security_firewall_uuid_default_autogenerate,
    ),
    ("security", "firewall config-change-log"): (
        "security_firewall_config_change_log",
        _parse_security_firewall_config_change_log,
    ),
    ("security", "ip-intelligence global-policy"): (
        "security_ip_intelligence_global_policy",
        _parse_security_ip_intelligence_global_policy,
    ),
    ("security", "packet-filter default-rules"): (
        "security_packet_filter_default_rules",
        _parse_security_packet_filter_default_rules,
    ),
    # apm.*
    ("apm", "report default-report"): (
        "apm_report_default_report",
        _parse_apm_report_default_report,
    ),
    # gtm.* singletons.
    ("gtm", "global-settings general"): (
        "gtm_global_settings_general",
        _parse_gtm_global_settings_general,
    ),
    ("gtm", "global-settings load-balancing"): (
        "gtm_global_settings_load_balancing",
        _parse_gtm_global_settings_load_balancing,
    ),
    ("gtm", "global-settings metrics"): (
        "gtm_global_settings_metrics",
        _parse_gtm_global_settings_metrics,
    ),
    ("gtm", "global-settings metrics-exclusions"): (
        "gtm_global_settings_metrics_exclusions",
        _parse_gtm_global_settings_metrics_exclusions,
    ),
    # auth.* singletons.
    ("auth", "password"): ("auth_password", _parse_auth_password),
    ("auth", "password-policy"): ("auth_password_policy", _parse_auth_password_policy),
    ("auth", "source"): ("auth_source", _parse_auth_source),
    ("auth", "remote-role"): ("auth_remote_role", _parse_auth_remote_role),
    ("auth", "remote-user"): ("auth_remote_user", _parse_auth_remote_user),
    ("auth", "login-failures"): ("auth_login_failures", _parse_auth_login_failures),
    # ltm.* singletons.
    ("ltm", "dns cache global-settings"): (
        "ltm_dns_cache_global_settings",
        _parse_ltm_dns_cache_global_settings,
    ),
    ("ltm", "dns analytics global-settings"): (
        "ltm_dns_analytics_global_settings",
        _parse_ltm_dns_analytics_global_settings,
    ),
}


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
        # ``gtm topology`` carries a multi-token condition rather than a
        # full-path (``gtm topology ldns: subnet 10.0.0.0/8 server:
        # subnet 10.1.0.0/16 { ... }``); the standard header parser
        # would mis-tokenise it.  Pre-extract the condition and treat
        # the whole thing as the topology identifier.
        if block.header.startswith("gtm topology "):
            topo_id = block.header[len("gtm topology ") :].strip()
            config.gtm_topologies[topo_id] = _parse_gtm_topology(
                topo_id, block.body, source_map, block
            )
            continue
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
            # Two-token / three-token bare singleton headers (e.g.
            # ``sys dns {``, ``auth password {``, ``net lacp-globals
            # {``) that the strict header parser rejects.  Dispatch
            # off ``generic`` via the unified singleton-typed and
            # minimal tables.
            if generic is not None:
                module_g, obj_type_g, identifier_g = generic
                if identifier_g != "":
                    continue
                singleton = _SINGLETON_DISPATCH.get((module_g, obj_type_g))
                if singleton is not None:
                    attr, parser_fn = singleton
                    getattr(config, attr)[""] = parser_fn(block.body, source_map, block)
                    continue
                # Bare-singleton minimal kinds (``sys httpd``, ``net
                # lacp-globals``, ``cm config-sync``, …) — route via
                # the per-module minimal dispatch.
                minimal_table = _MINIMAL_DISPATCH_BY_MODULE.get(module_g)
                if minimal_table is not None and obj_type_g in minimal_table[0]:
                    attr = minimal_table[0][obj_type_g]
                    parser_fn = minimal_table[1]
                    getattr(config, attr)[""] = parser_fn(
                        "",
                        block.body,
                        f"{module_g} {obj_type_g}",
                        source_map,
                        block,
                    )
                    continue
                # ``ltm`` minimal kinds aren't keyed in
                # ``_MINIMAL_DISPATCH_BY_MODULE``; their dispatch
                # table is :data:`_LTM_MINIMAL_DISPATCH`.
                if module_g == "ltm" and obj_type_g in _LTM_MINIMAL_DISPATCH:
                    attr = _LTM_MINIMAL_DISPATCH[obj_type_g]
                    getattr(config, attr)[""] = _parse_ltm_minimal(
                        "",
                        block.body,
                        f"ltm {obj_type_g}",
                        source_map,
                        block,
                    )
            continue
        module, obj_type, full_path = parsed

        # Generic minimal-dispatch pre-pass.  Runs first so that the
        # bundles 17-45 minimal kinds for non-ltm modules (net.* /
        # sys.* / apm.* / pem.* / cm.* / vcmp / cli / api-protection)
        # are dispatched without each module needing its own elif
        # chain.  Falls through to the module-specific blocks when
        # the kind isn't in any minimal table.
        _minimal_table = _MINIMAL_DISPATCH_BY_MODULE.get(module)
        if _minimal_table is not None and obj_type in _minimal_table[0]:
            _attr = _minimal_table[0][obj_type]
            _parser_fn = _minimal_table[1]
            getattr(config, _attr)[full_path] = _parser_fn(
                full_path,
                block.body,
                f"{module} {obj_type}",
                source_map,
                block,
            )
            continue

        # Named-typed dispatch — every (module, obj_type) pair listed
        # in :data:`_NAMED_DISPATCH` routes here with the canonical
        # ``(full_path, body, source_map, block)`` parser signature.
        # Singleton typed kinds (no full_path) are in
        # :data:`_SINGLETON_DISPATCH` but parsed via the same
        # branch when full_path is empty.
        named = _NAMED_DISPATCH.get((module, obj_type))
        if named is not None:
            attr, parser_fn = named
            getattr(config, attr)[full_path] = parser_fn(full_path, block.body, source_map, block)
            continue
        singleton = _SINGLETON_DISPATCH.get((module, obj_type))
        if singleton is not None and full_path == "":
            attr, singleton_parser = singleton
            getattr(config, attr)[""] = singleton_parser(block.body, source_map, block)
            continue

        # Family parsers that take an extra TMSH sub-type argument —
        # the parser signature deviates from the canonical 4-arg form
        # so they're handled inline rather than via dispatch tables.
        if module == "security" and obj_type in _SECURITY_MINIMAL_DISPATCH:
            attr = _SECURITY_MINIMAL_DISPATCH[obj_type]
            getattr(config, attr)[full_path] = _parse_security_minimal(
                full_path,
                block.body,
                f"security {obj_type}",
                source_map,
                block,
            )
            continue
        if module == "apm" and obj_type.startswith("policy agent "):
            agent_type = obj_type.rsplit(" ", 1)[-1]
            config.apm_policy_agents[full_path] = _parse_apm_policy_agent(
                full_path, block.body, source_map, block, agent_type
            )
            continue
        if module == "gtm" and obj_type.startswith("pool "):
            record_type = obj_type.split(" ", 1)[1]
            config.gtm_pools[full_path] = _parse_gtm_pool(
                full_path, block.body, record_type, source_map, block
            )
            continue
        if module == "gtm" and obj_type.startswith("wideip "):
            record_type = obj_type.split(" ", 1)[1]
            config.gtm_wideips[full_path] = _parse_gtm_wideip(
                full_path, block.body, record_type, source_map, block
            )
            continue
        if module == "pem" and obj_type in (
            "profile diameter-endpoint",
            "profile radius-aaa",
            "profile spm",
            "profile subscriber-mgmt",
        ):
            profile_type = obj_type.split(" ", 1)[1]
            config.pem_profiles[full_path] = _parse_pem_profile(
                full_path, profile_type, block.body, source_map, block
            )
            continue

        # Non-shared modules end here — only ``ltm`` and ``gtm`` fall
        # through to the legacy ``match`` block below (some kinds
        # like ``rule`` exist under both modules and dispatch by
        # ``module`` inside the match arms).
        if module not in ("ltm", "gtm"):
            continue

        # ltm/gtm shared kinds — ``data-group``, ``virtual``, ``pool``,
        # ``rule``, ``policy`` etc. that can appear under both modules.
        # The dispatch arms gate on ``module`` where the kind is
        # ltm-only.  Family parsers (profile / persistence / monitor
        # / dns-cache-records) stay below because they need an extra
        # sub-type argument.
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
            case _ if obj_type.startswith("dns cache records ") and module == "ltm":
                # Four-word kind: all five record sub-kinds (all / key /
                # msg / nameserver / rrset) merge into one container.
                record_kind = obj_type.rsplit(" ", 1)[1]
                config.ltm_dns_cache_records[full_path] = _parse_ltm_dns_cache_record(
                    full_path, block.body, record_kind, source_map, block
                )
            case _ if (
                module == "ltm" and obj_type.startswith("auth ") and obj_type in _LTM_AUTH_DISPATCH
            ):
                # Bundle 16 — ltm auth.*  All 11 kinds share one
                # parser; the ``kind`` field carries the full
                # ``ltm auth X`` label.
                attr = _LTM_AUTH_DISPATCH[obj_type]
                getattr(config, attr)[full_path] = _parse_ltm_auth(
                    full_path,
                    block.body,
                    f"ltm {obj_type}",
                    source_map,
                    block,
                )
            case _ if module == "ltm" and obj_type in _LTM_MINIMAL_DISPATCH:
                # Bundles 17-20 — generic ltm.* minimal-shape kinds
                # (CGNAT/LSN, global-settings singletons,
                # classification, tacdb).  Routed via the dispatch
                # table; ``kind`` field carries the TMSH label.
                attr = _LTM_MINIMAL_DISPATCH[obj_type]
                getattr(config, attr)[full_path] = _parse_ltm_minimal(
                    full_path,
                    block.body,
                    f"ltm {obj_type}",
                    source_map,
                    block,
                )
            case _ if (
                module == "ltm"
                and obj_type.startswith("message-routing ")
                and obj_type in _LTM_MESSAGE_ROUTING_DISPATCH
            ):
                # Bundle 15 — ltm message-routing.*  All 20 kinds
                # share one parser; the obj_type label picks the
                # right ``BigipConfig`` attribute via the dispatch
                # table.  The ``kind`` field on each instance carries
                # the full ``ltm message-routing X Y`` label.
                attr = _LTM_MESSAGE_ROUTING_DISPATCH[obj_type]
                getattr(config, attr)[full_path] = _parse_ltm_message_routing(
                    full_path,
                    block.body,
                    f"ltm {obj_type}",
                    source_map,
                    block,
                )
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
                # Route by module — ``gtm monitor <type>`` lands in
                # ``gtm_monitors`` so it doesn't collide with the
                # identically-named LTM monitor (same path is valid
                # under both modules; TMSH enforces uniqueness per
                # (module, kind, full-path) tuple).
                if module == "gtm":
                    config.gtm_monitors[full_path] = monitor
                else:
                    config.monitors[full_path] = monitor

    return config
