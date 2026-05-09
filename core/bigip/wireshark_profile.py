"""Generate a Wireshark *profile* directory from a BIG-IP config.

Powers ``f5 enrich-wireshark``.

A Wireshark profile is a small directory that lives under
``~/.config/wireshark/profiles/<name>/`` (Linux/macOS) or
``%APPDATA%\\Wireshark\\profiles\\<name>\\`` (Windows).  Each file in
the directory teaches Wireshark how to label some part of a packet:

================  ==========================================================
``hosts``         IPv4/IPv6 → name (alternative to a PCAPNG NRB; works on
                  plain libpcap captures too).
``subnets``       ``<network>/<prefix>  <name>`` — e.g. tag every host on
                  ``10.0.5.0/24`` with ``net-common-external-self``.
``vlans``         ``<vlan_tag> <name>`` — derived from ``net vlan { tag N }``.
``dfilters``      Saved display filters (``"label" filter-expr``).  We
                  emit one per virtual-server destination, one per
                  self-IP CIDR, plus "BIG-IP client side" and "BIG-IP
                  server side" buttons that match the front- and
                  back-end of the proxy respectively.
``services``      Static map of well-known BIG-IP control-plane ports
                  (mcpd, cmi, iControl REST, …) so they show up by
                  name in Wireshark's port columns.
``ethers``        ``MAC <name>`` — derived from every ``net arp`` static
                  ARP entry in the input.
``colorfilters``  Front-end / back-end / self-IP / SNAT / wide-IP
                  coloring rules so the row colour tells you instantly
                  which side of the proxy a packet sits on.
``preferences``   Adds a "Proxy Side" custom column (sourced from
                  ``frame.coloring_rule.name``) right next to Source /
                  Destination so the matching colour rule's name shows
                  up alongside every packet.
================  ==========================================================

The verb is purely additive — it never touches an existing profile
directory unless the user passes ``--force``.
"""

from __future__ import annotations

import ipaddress
import re
from dataclasses import dataclass, field
from pathlib import Path

from .model import BigipConfig, ProfileType
from .parser import _extract_blocks
from .pcap_enrich import (
    NameIndex,
    _extract_self_ips,
    _label,
    _split_destination,
    build_merged_name_index,
)


@dataclass(frozen=True, slots=True)
class WiresharkProfileWriteResult:
    hosts_lines: int = 0
    subnets_lines: int = 0
    vlans_lines: int = 0
    dfilters_lines: int = 0
    services_lines: int = 0
    ethers_lines: int = 0
    colorfilter_rules: int = 0
    files_written: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class ColorRule:
    """A single Wireshark coloring rule with 16-bit RGB foreground/background.

    Wireshark's ``colorfilters`` file format encodes colours as decimal
    16-bit channel values (0–65535) so the rules look the same on every
    display.  ``disabled`` rules are still written but prefixed with
    ``!`` so the user can re-enable them in *View → Coloring Rules*.
    """

    name: str
    filter: str
    bg: tuple[int, int, int]
    fg: tuple[int, int, int] = (0, 0, 0)
    disabled: bool = False


@dataclass(slots=True)
class WiresharkProfile:
    """In-memory model of every file the profile directory will contain."""

    hosts: list[tuple[str, str]] = field(default_factory=list)
    subnets: list[tuple[str, str]] = field(default_factory=list)
    vlans: list[tuple[int, str]] = field(default_factory=list)
    dfilters: list[tuple[str, str]] = field(default_factory=list)
    # ``services`` rows are ``(name, port, proto)`` triples (proto in {tcp, udp, sctp}).
    services: list[tuple[str, int, str]] = field(default_factory=list)
    ethers: list[tuple[str, str]] = field(default_factory=list)
    colorfilters: list[ColorRule] = field(default_factory=list)
    # ``preferences`` is rendered verbatim — one line per key=value entry.
    preferences: list[str] = field(default_factory=list)

    def extend(self, other: WiresharkProfile) -> None:
        """Append every entry from *other* to ``self``."""
        self.hosts.extend(other.hosts)
        self.subnets.extend(other.subnets)
        self.vlans.extend(other.vlans)
        self.dfilters.extend(other.dfilters)
        self.services.extend(other.services)
        self.ethers.extend(other.ethers)
        self.colorfilters.extend(other.colorfilters)
        self.preferences.extend(other.preferences)

    def deduplicate(self) -> None:
        """Drop duplicate rows while preserving insertion order."""
        self.hosts = list(dict.fromkeys(self.hosts))
        self.subnets = list(dict.fromkeys(self.subnets))
        self.vlans = list(dict.fromkeys(self.vlans))
        self.dfilters = list(dict.fromkeys(self.dfilters))
        self.services = list(dict.fromkeys(self.services))
        self.ethers = list(dict.fromkeys(self.ethers))
        # ColorRule is a dataclass — dedup by name to preserve filter order.
        seen: set[str] = set()
        unique_rules: list[ColorRule] = []
        for rule in self.colorfilters:
            if rule.name in seen:
                continue
            seen.add(rule.name)
            unique_rules.append(rule)
        self.colorfilters = unique_rules
        self.preferences = list(dict.fromkeys(self.preferences))

    def write_to(self, directory: Path) -> WiresharkProfileWriteResult:
        directory.mkdir(parents=True, exist_ok=True)
        files: list[str] = []

        def _write(name: str, content: str) -> None:
            (directory / name).write_text(content, encoding="utf-8")
            files.append(name)

        if self.hosts:
            _write("hosts", _format_hosts(self.hosts))
        if self.subnets:
            _write("subnets", _format_subnets(self.subnets))
        if self.vlans:
            _write("vlans", _format_vlans(self.vlans))
        if self.dfilters:
            _write("dfilters", _format_dfilters(self.dfilters))
        if self.services:
            _write("services", _format_services(self.services))
        if self.ethers:
            _write("ethers", _format_ethers(self.ethers))
        if self.colorfilters:
            _write("colorfilters", _format_colorfilters(self.colorfilters))
        if self.preferences:
            _write("preferences", _format_preferences(self.preferences))

        _write("README.md", _format_readme(directory.name, files))

        return WiresharkProfileWriteResult(
            hosts_lines=len(self.hosts),
            subnets_lines=len(self.subnets),
            vlans_lines=len(self.vlans),
            dfilters_lines=len(self.dfilters),
            services_lines=len(self.services),
            ethers_lines=len(self.ethers),
            colorfilter_rules=len(self.colorfilters),
            files_written=tuple(files),
        )


# Formatters
#
# Wireshark accepts whitespace-separated columns in hosts / subnets /
# vlans / ethers files.  Comments start with ``#``.  dfilters uses
# ``"name" filter-expr`` per line.


def _format_hosts(entries: list[tuple[str, str]]) -> str:
    lines = ["# Generated by f5 enrich-wireshark — IP -> BIG-IP object label"]
    for addr, name in entries:
        lines.append(f"{addr}\t{name}")
    return "\n".join(lines) + "\n"


def _format_subnets(entries: list[tuple[str, str]]) -> str:
    lines = [
        "# Generated by f5 enrich-wireshark — CIDR -> BIG-IP self-IP label",
        "# Hosts inside these networks pick up the corresponding name.",
    ]
    for cidr, name in entries:
        lines.append(f"{cidr}\t{name}")
    return "\n".join(lines) + "\n"


def _format_vlans(entries: list[tuple[int, str]]) -> str:
    lines = ["# Generated by f5 enrich-wireshark — VLAN tag -> BIG-IP vlan name"]
    for tag, name in entries:
        lines.append(f"{tag}\t{name}")
    return "\n".join(lines) + "\n"


def _format_dfilters(entries: list[tuple[str, str]]) -> str:
    lines = ["# Generated by f5 enrich-wireshark — saved display filters"]
    for label, expr in entries:
        # Escape any embedded double-quote in the label (rare).
        safe_label = label.replace('"', "'")
        lines.append(f'"{safe_label}" {expr}')
    return "\n".join(lines) + "\n"


def _format_services(entries: list[tuple[str, int, str]]) -> str:
    lines = [
        "# Generated by f5 enrich-wireshark — well-known BIG-IP service ports",
        "# Static F5 control-plane mapping; safe to edit/extend by hand.",
    ]
    for name, port, proto in entries:
        lines.append(f"{name}\t{port}/{proto}")
    return "\n".join(lines) + "\n"


def _format_ethers(entries: list[tuple[str, str]]) -> str:
    lines = ["# Generated by f5 enrich-wireshark — MAC -> BIG-IP object label"]
    for mac, name in entries:
        lines.append(f"{mac}\t{name}")
    return "\n".join(lines) + "\n"


def _format_colorfilters(rules: list[ColorRule]) -> str:
    """Render :class:`ColorRule` objects in Wireshark's colorfilters format.

    Format::

        @<name>@<filter>@[<bg_r>,<bg_g>,<bg_b>][<fg_r>,<fg_g>,<fg_b>]

    Disabled rules are written with a leading ``!`` per Wireshark's
    convention so the user can flip them on without re-typing the rule.
    """
    lines = [
        "# Generated by f5 enrich-wireshark — front-end / back-end / role coloring",
        "# Rules are applied top-down; the first match wins, so per-VS rules above",
        "# the broad client/server-side rules give a more specific colour.",
    ]
    for rule in rules:
        prefix = "!" if rule.disabled else ""
        bg = ",".join(str(c) for c in rule.bg)
        fg = ",".join(str(c) for c in rule.fg)
        lines.append(f"{prefix}@{rule.name}@{rule.filter}@[{bg}][{fg}]")
    return "\n".join(lines) + "\n"


def _format_preferences(lines_in: list[str]) -> str:
    out = ["# Generated by f5 enrich-wireshark — preferences"]
    out.extend(lines_in)
    return "\n".join(out) + "\n"


def _format_readme(profile_name: str, files: list[str]) -> str:
    file_list = "\n".join(f"- `{f}`" for f in files)
    return (
        f"# `{profile_name}` Wireshark profile\n"
        f"\n"
        f"Generated by `f5 enrich-wireshark`.  Drop this directory into "
        f"your Wireshark profiles folder and select it from "
        f"**Edit → Configuration Profiles**.\n"
        f"\n"
        f"## Install\n"
        f"\n"
        f"- **Linux/macOS:** copy to "
        f"`~/.config/wireshark/profiles/{profile_name}/`\n"
        f"- **Windows:** copy to "
        f"`%APPDATA%\\Wireshark\\profiles\\{profile_name}\\`\n"
        f"\n"
        f"## Files\n"
        f"\n"
        f"{file_list}\n"
    )


# Source-text extractors


_VLAN_TAG_LINE = re.compile(r"^\s*tag\s+(\d+)\s*$", re.MULTILINE)
_ARP_IP_LINE = re.compile(r"^\s*ip-address\s+(\S+)\s*$", re.MULTILINE)
_ARP_MAC_LINE = re.compile(r"^\s*mac-address\s+(\S+)\s*$", re.MULTILINE)


def _extract_arp_entries(source: str) -> list[tuple[str, str, str]]:
    """Pull ``net arp`` static-ARP entries: ``[(mac, ip, full_path), …]``.

    Returns the canonical lowercase MAC, the raw IP literal (route-domain
    suffix stripped), and the ARP entry's full path so callers can build
    a per-entry label.
    """
    entries: list[tuple[str, str, str]] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "net" or parts[1] != "arp":
            continue
        full_path = parts[2]
        ip_match = _ARP_IP_LINE.search(block.body)
        mac_match = _ARP_MAC_LINE.search(block.body)
        if ip_match is None or mac_match is None:
            continue
        ip_str = ip_match.group(1).split("%", 1)[0]
        mac_str = mac_match.group(1).lower()
        entries.append((mac_str, ip_str, full_path))
    return entries


def _extract_vlans(source: str) -> list[tuple[str, int]]:
    """Return ``[(full_path, tag), …]`` for every ``net vlan`` block."""
    out: list[tuple[str, int]] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "net" or parts[1] != "vlan":
            continue
        full_path = parts[2]
        m = _VLAN_TAG_LINE.search(block.body)
        if m is None:
            continue
        try:
            tag = int(m.group(1))
        except ValueError:
            continue
        out.append((full_path, tag))
    return out


# Static BIG-IP control-plane services
#
# These ports are consistent across TMOS versions and don't depend on
# the customer config.  ``services`` is a global port → name map in
# Wireshark — we keep this list short and authoritative so it doesn't
# stomp on standard ``http`` / ``https`` / ``dns`` / ``ssh`` mappings
# that Wireshark already ships in its built-in ``services`` file.
_BIGIP_CONTROL_SERVICES: tuple[tuple[str, int, str], ...] = (
    ("f5-iquery", 4353, "tcp"),         # GTM iQuery / sync between GTM peers
    ("f5-iquery", 4353, "udp"),
    ("f5-cmi", 6699, "tcp"),            # config sync (cluster master)
    ("f5-cmi-conf-replicator", 6700, "tcp"),
    ("f5-cmi-config", 6701, "tcp"),
    ("f5-cmi-mcpd", 6702, "tcp"),
    ("f5-cmi-statedb", 6703, "tcp"),
    ("f5-icontrol-rest", 8443, "tcp"),  # iControl REST
    ("f5-tmsh", 22, "tcp"),             # tmsh over SSH
    ("f5-snmp", 161, "udp"),
    ("f5-snmp-trap", 162, "udp"),
)


# Builder


def _hosts_from_index(index: NameIndex) -> list[tuple[str, str]]:
    """Flatten ``NameIndex`` exact entries into ``(IP, name)`` pairs.

    Each label gets its own line — Wireshark accepts repeated keys and
    treats each as an alias, which keeps the file mirror-image-readable
    and lets every label show up in the *Resolved Addresses* dialog.
    """
    pairs: list[tuple[str, str]] = []
    for addr, names in index.v4.items():
        for name in names:
            pairs.append((addr, name))
    for addr, names in index.v6.items():
        for name in names:
            pairs.append((addr, name))
    return pairs


def _subnets_from_index(index: NameIndex) -> list[tuple[str, str]]:
    pairs: list[tuple[str, str]] = []
    for net, name in index.v4_subnets:
        pairs.append((str(net), name))
    for net, name in index.v6_subnets:
        pairs.append((str(net), name))
    return pairs


def _dfilters_for_virtual_servers(config: BigipConfig) -> list[tuple[str, str]]:
    """One display-filter button per virtual server (``ip.addr == <dest>``)."""
    out: list[tuple[str, str]] = []
    for full_path, vs in config.virtual_servers.items():
        addr = _split_destination(vs.destination)
        if not addr:
            continue
        try:
            ip = ipaddress.ip_address(addr)
        except ValueError:
            continue
        field_name = "ipv6.addr" if isinstance(ip, ipaddress.IPv6Address) else "ip.addr"
        label = _label("vs", full_path)
        out.append((label, f"{field_name} == {addr}"))
    return out


def _dfilters_for_self_ips(source: str) -> list[tuple[str, str]]:
    """One display filter per ``net self`` block, scoped to its CIDR."""
    out: list[tuple[str, str]] = []
    for self_ip in _extract_self_ips(source):
        if self_ip.network is None:
            continue
        net = self_ip.network
        field_name = (
            "ipv6.addr"
            if isinstance(net, ipaddress.IPv6Network)
            else "ip.addr"
        )
        label = _label("net", self_ip.full_path)
        out.append((label, f"{field_name} == {net}"))
    return out


def _vs_port_proto(dest: str) -> tuple[int, str] | None:
    """Extract ``(port, transport)`` from a BIG-IP destination string.

    The transport piece comes from inspection of the VS's profiles by
    the caller; this helper just isolates the port number from
    ``[/Common/]ADDR:PORT`` (IPv4) or ``[/Common/]ADDR.PORT`` (IPv6).
    Returns ``None`` for wildcard / port-less destinations.
    """
    if not dest:
        return None
    base = dest.rsplit("/", 1)[-1]
    for sep in (":", "."):
        if sep not in base:
            continue
        _host, _, port_str = base.rpartition(sep)
        if port_str.isdigit():
            try:
                port = int(port_str)
            except ValueError:
                continue
            if 0 < port < 65536:
                return port, "tcp"
    return None


def _virtual_server_protocol(config: BigipConfig, vs_full_path: str) -> str:
    """Pick TCP / UDP for a VS based on its attached profiles.

    Defaults to TCP — every BIG-IP profile-stack we've seen uses TCP
    unless an explicit UDP profile is attached.  Returns lowercase to
    match the Wireshark services-file convention (``443/tcp``).
    """
    types = config.profile_types_for_virtual(vs_full_path)
    if ProfileType.UDP in types:
        return "udp"
    return "tcp"


def _services_from_virtuals(config: BigipConfig) -> list[tuple[str, int, str]]:
    out: list[tuple[str, int, str]] = []
    for full_path, vs in config.virtual_servers.items():
        port_proto = _vs_port_proto(vs.destination)
        if port_proto is None:
            continue
        port, default_proto = port_proto
        proto = _virtual_server_protocol(config, full_path) or default_proto
        out.append((_label("vs", full_path), port, proto))
    return out


def _services_from_monitors(source: str) -> list[tuple[str, int, str]]:
    """Pull ``destination *:PORT`` from every ``ltm monitor`` / ``gtm monitor``.

    The transport is inferred from the monitor's record type — types
    containing ``udp``/``dns``/``radius``/``sip-udp``/``snmp`` are UDP,
    everything else defaults to TCP.  Monitors without an explicit
    port (ICMP, ``destination *:*``, ``destination *``) are skipped.
    """
    udp_hint = re.compile(r"\b(udp|dns|radius|sip-udp|snmp|wmi)\b")
    out: list[tuple[str, int, str]] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 4 or parts[0] not in ("ltm", "gtm") or parts[1] != "monitor":
            continue
        monitor_type = " ".join(parts[2:-1])
        full_path = parts[-1]
        m = re.search(r"^\s*destination\s+(\S+)\s*$", block.body, re.MULTILINE)
        if m is None:
            continue
        dest = m.group(1)
        port_proto = _vs_port_proto(dest)
        if port_proto is None:
            continue
        port, _ = port_proto
        proto = "udp" if udp_hint.search(monitor_type.lower()) else "tcp"
        out.append((_label("monitor", full_path), port, proto))
    return out


_ALLOW_SERVICE_TOKEN = re.compile(r"\b(tcp|udp|sctp):(\d+)\b")


def _services_from_self_allow(source: str) -> list[tuple[str, int, str]]:
    """``net self { allow-service { tcp:443 udp:161 ... } }`` → service entries.

    The bare keywords ``all`` / ``default`` / ``none`` are skipped — they
    don't carry a specific port; the explicit ``proto:port`` pairs do.
    Each entry is labelled after the self-IP it belongs to so the user
    can see exactly where ``161/udp`` is exposed.
    """
    out: list[tuple[str, int, str]] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "net" or parts[1] != "self":
            continue
        full_path = parts[2]
        m = re.search(r"allow-service\s*\{(.*?)\}", block.body, re.DOTALL)
        if m is None:
            continue
        for proto, port_str in _ALLOW_SERVICE_TOKEN.findall(m.group(1)):
            try:
                port = int(port_str)
            except ValueError:
                continue
            if not (0 < port < 65536):
                continue
            out.append((_label("self", full_path), port, proto.lower()))
    return out


def _services_from_gtm_servers(source: str) -> list[tuple[str, int, str]]:
    """Pull ``destination IP:PORT`` from each ``gtm server { virtual-servers { vsN { destination ... } } }``.

    Mirrors :func:`_services_from_virtuals` for the GTM-side view
    (LTM and GTM tiers often live in separate inputs).  The label
    follows ``gtmvs-<server-path>-<vsname>`` so it doesn't collide with
    the LTM ``vs-…`` namespace.
    """
    out: list[tuple[str, int, str]] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "gtm" or parts[1] != "server":
            continue
        server_path = parts[2]
        vs_block_match = re.search(
            r"virtual-servers\s*\{(.*)\}\s*$", block.body, re.DOTALL
        )
        if vs_block_match is None:
            continue
        from .parser import _parse_properties
        from .pcap_enrich import _parse_named_subblocks

        for vs_name, body in _parse_named_subblocks("{" + vs_block_match.group(1) + "}"):
            inner = _parse_properties(body)
            dest = inner.get("destination", "")
            port_proto = _vs_port_proto(dest)
            if port_proto is None:
                continue
            port, _ = port_proto
            label = _label("gtmvs", server_path, vs_name)
            out.append((label, port, "tcp"))
    return out


# Best-effort port extraction from iRule bodies + ltm policy conditions.
# Both spaces have many syntactic variants and we don't aim for full
# coverage — we catch the common forms and tag them as ``irule-…`` /
# ``policy-…`` so the operator can see exactly which object referenced
# the port.

# Match: ``node IP PORT`` or ``pool POOL member IP PORT`` or
# ``connect IP PORT`` — port is the trailing number.
_IRULE_HOST_PORT = re.compile(
    r"\b(?:node|connect)\s+\S+\s+(\d{1,5})\b"
    r"|\bpool\s+\S+\s+member\s+\S+\s+(\d{1,5})\b"
)
# ``HTTP::redirect "https://host:443/..."`` and similar URL literals.
_IRULE_URL_PORT = re.compile(r"://[^/\"\s]+:(\d{1,5})\b")
# ``TCP::respond ... port N`` and similar named-arg uses.
_IRULE_NAMED_PORT = re.compile(r"\bport\s+(\d{1,5})\b")


def _services_from_irules(config: BigipConfig) -> list[tuple[str, int, str]]:
    out: list[tuple[str, int, str]] = []
    for full_path, rule in config.rules.items():
        body = rule.source or ""
        ports: set[int] = set()
        for groups in _IRULE_HOST_PORT.findall(body):
            for token in groups:
                if token:
                    try:
                        ports.add(int(token))
                    except ValueError:
                        continue
        for token in _IRULE_URL_PORT.findall(body):
            try:
                ports.add(int(token))
            except ValueError:
                continue
        for token in _IRULE_NAMED_PORT.findall(body):
            try:
                ports.add(int(token))
            except ValueError:
                continue
        for port in sorted(p for p in ports if 0 < p < 65536):
            out.append((_label("irule", full_path), port, "tcp"))
    return out


# Match ``port-equals N`` or ``port equals N`` inside an ltm policy
# condition block.  The default protocol for policy port match is TCP
# (HTTP / HTTPS / TCP listeners are by far the common case).
_POLICY_PORT_EQUALS = re.compile(r"port[-\s]equals\s+(\d{1,5})\b")


def _services_from_ltm_policies(source: str) -> list[tuple[str, int, str]]:
    out: list[tuple[str, int, str]] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "ltm" or parts[1] != "policy":
            continue
        full_path = parts[2]
        ports: set[int] = set()
        for token in _POLICY_PORT_EQUALS.findall(block.body):
            try:
                ports.add(int(token))
            except ValueError:
                continue
        for port in sorted(p for p in ports if 0 < p < 65536):
            out.append((_label("policy", full_path), port, "tcp"))
    return out


def _services_from_pool_members(config: BigipConfig) -> list[tuple[str, int, str]]:
    """Each pool member with an explicit port becomes a services entry.

    Pool members don't carry their own transport, so we tag them as TCP
    — every pool we see in practice is TCP-based; UDP pools (rare) get
    relabelled by hand.
    """
    out: list[tuple[str, int, str]] = []
    for pool in config.pools.values():
        for member in pool.members:
            port_proto = _vs_port_proto(member.name)
            if port_proto is None:
                continue
            port, _proto = port_proto
            out.append((_label("pool", member.name), port, "tcp"))
    return out


def _ethers_from_arp(source: str) -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    for mac, _ip, full_path in _extract_arp_entries(source):
        out.append((mac, _label("arp", full_path)))
    return out


def _addresses_for_label_prefix(index: NameIndex, prefix: str) -> tuple[set[str], set[str]]:
    """Collect every IP from *index* whose labels include one matching *prefix*.

    Returns ``(v4_addrs, v6_addrs)``.  Used to drive the front-end /
    back-end / self / SNAT / wide-IP coloring rules from the same
    NameIndex that already powers the ``hosts`` file.
    """
    v4: set[str] = set()
    v6: set[str] = set()
    needle = f"{prefix}-"
    for addr, names in index.v4.items():
        if any(name.startswith(needle) for name in names):
            v4.add(addr)
    for addr, names in index.v6.items():
        if any(name.startswith(needle) for name in names):
            v6.add(addr)
    return v4, v6


def _ip_set_filter(addrs_v4: set[str], addrs_v6: set[str]) -> str:
    """Build a Wireshark display filter matching any of *addrs_{v4,v6}*.

    Uses the ``ip.addr in {a, b, c}`` set syntax — Wireshark's coloring
    rule parser only accepts the comma-separated form (the
    space-separated variant works in display filters, but not here).
    """
    parts: list[str] = []
    if addrs_v4:
        joined = ", ".join(sorted(addrs_v4))
        parts.append(f"ip.addr in {{{joined}}}")
    if addrs_v6:
        joined = ", ".join(sorted(addrs_v6))
        parts.append(f"ipv6.addr in {{{joined}}}")
    return " or ".join(parts)


# Coloring palette — all 16-bit RGB.  Backgrounds are deliberately
# pastel so default-black foreground text stays readable, and the five
# proxy-side roles get visually distinct hues that read at a glance:
#
#   client side : light blue        — packets with a VS in src/dst.
#   server side : light orange      — packets with a pool member / node.
#   self IP     : light green       — packets touching a self-IP.
#   SNAT        : light pink        — SNAT-translated packets.
#   wide-IP     : light yellow      — GTM-resolved packets.
_COLOR_CLIENT_SIDE = (49152, 57344, 65535)
_COLOR_SERVER_SIDE = (65535, 57344, 49152)
_COLOR_SELF = (49152, 65535, 49152)
_COLOR_SNAT = (65535, 52428, 58982)
_COLOR_WIDEIP = (65535, 65535, 49152)


def _build_proxy_side_color_rules(index: NameIndex) -> list[ColorRule]:
    """Return coloring rules for the five proxy-side roles in priority order.

    Wireshark applies rules top-down and stops at the first match, so
    order encodes how we want overlapping addresses resolved.  In a
    typical BIG-IP → pool-member packet the source is a self-IP and
    the destination is a pool member — both rules would fire, and
    we want **server side** to win because that's the actionable
    label.  Hence the order is *server-side → client-side → SNAT →
    wide-IP → self*: the data-plane proxy-side labels (server / client)
    take precedence over the administrative ones (SNAT, self, wide-IP).
    """
    rules: list[ColorRule] = []

    pool_v4, pool_v6 = _addresses_for_label_prefix(index, "pool")
    node_v4, node_v6 = _addresses_for_label_prefix(index, "node")
    server_filter = _ip_set_filter(pool_v4 | node_v4, pool_v6 | node_v6)
    if server_filter:
        rules.append(
            ColorRule(name="BIG-IP server side", filter=server_filter, bg=_COLOR_SERVER_SIDE)
        )

    vs_v4, vs_v6 = _addresses_for_label_prefix(index, "vs")
    client_filter = _ip_set_filter(vs_v4, vs_v6)
    if client_filter:
        rules.append(
            ColorRule(name="BIG-IP client side", filter=client_filter, bg=_COLOR_CLIENT_SIDE)
        )

    def _maybe(prefix: str, label: str, bg: tuple[int, int, int]) -> None:
        v4, v6 = _addresses_for_label_prefix(index, prefix)
        flt = _ip_set_filter(v4, v6)
        if flt:
            rules.append(ColorRule(name=label, filter=flt, bg=bg))

    _maybe("snat", "BIG-IP SNAT", _COLOR_SNAT)
    _maybe("wideip", "BIG-IP wide-IP", _COLOR_WIDEIP)
    _maybe("self", "BIG-IP self IP", _COLOR_SELF)
    return rules


def _build_proxy_side_dfilters(index: NameIndex) -> list[tuple[str, str]]:
    """Display-filter buttons for the five proxy-side groups."""
    buttons: list[tuple[str, str]] = []
    pool_v4, pool_v6 = _addresses_for_label_prefix(index, "pool")
    node_v4, node_v6 = _addresses_for_label_prefix(index, "node")
    server_filter = _ip_set_filter(pool_v4 | node_v4, pool_v6 | node_v6)
    if server_filter:
        buttons.append(("BIG-IP server side", server_filter))
    vs_v4, vs_v6 = _addresses_for_label_prefix(index, "vs")
    client_filter = _ip_set_filter(vs_v4, vs_v6)
    if client_filter:
        buttons.append(("BIG-IP client side", client_filter))
    return buttons


def _build_preferences() -> list[str]:
    """Single-line preferences override that adds a *Proxy Side* column.

    The custom column reads ``frame.coloring_rule.name`` so the matched
    coloring rule's name (``BIG-IP client side``, ``BIG-IP server side``,
    …) shows up in a dedicated column right after Destination.  Every
    other column stays at the Wireshark default so we don't overwrite
    user preferences they haven't asked us to touch.
    """
    column_format = (
        '"No.","%m",'
        '"Time","%t",'
        '"Source","%s",'
        '"Destination","%d",'
        '"Proxy Side","%Cus:frame.coloring_rule.name:0:R",'
        '"Protocol","%p",'
        '"Length","%L",'
        '"Info","%i"'
    )
    return [f"gui.column.format: {column_format}"]


def build_wireshark_profile(
    configs_with_sources: list[tuple[BigipConfig, str]],
) -> WiresharkProfile:
    """Assemble a :class:`WiresharkProfile` from one or more ``(config, source)`` pairs.

    Multi-input deployments — GTM separate from LTM, multiple LTM
    tiers, partition-split SCFs — feed every file in via this list.
    Wide-IP / pool / server cross-references resolve against the
    combined source so a wide-IP in one file can reach a server
    declared in another.
    """
    if not configs_with_sources:
        return WiresharkProfile()

    combined_source = "\n".join(src for _cfg, src in configs_with_sources)
    index = build_merged_name_index(configs_with_sources)

    profile = WiresharkProfile(
        hosts=_hosts_from_index(index),
        subnets=_subnets_from_index(index),
        vlans=[
            (tag, _label("vlan", full_path))
            for full_path, tag in _extract_vlans(combined_source)
        ],
        dfilters=(
            _build_proxy_side_dfilters(index) + _dfilters_for_self_ips(combined_source)
        ),
        services=list(_BIGIP_CONTROL_SERVICES),
        ethers=_ethers_from_arp(combined_source),
        colorfilters=_build_proxy_side_color_rules(index),
        preferences=_build_preferences(),
    )
    for config, _src in configs_with_sources:
        profile.dfilters.extend(_dfilters_for_virtual_servers(config))
        profile.services.extend(_services_from_virtuals(config))
        profile.services.extend(_services_from_pool_members(config))
        profile.services.extend(_services_from_irules(config))

    profile.services.extend(_services_from_monitors(combined_source))
    profile.services.extend(_services_from_self_allow(combined_source))
    profile.services.extend(_services_from_gtm_servers(combined_source))
    profile.services.extend(_services_from_ltm_policies(combined_source))

    profile.deduplicate()
    return profile
