"""Enrich a PCAPNG capture with BIG-IP-derived metadata.

Powers ``f5 enrich-pcapng``.

For each unique IPv4/IPv6 address that **appears in the capture** as a
packet src/dst, we synthesise hostname-style labels from the BIG-IP
inventory:

- ``vs-<partition>-<name>`` for virtual-server destinations
- ``pool-<partition>-<pool>-<ip>`` for pool members
- ``snat-<partition>-<snatpool>-<ip>`` for SNAT-pool members
- ``node-<partition>-<name>`` for nodes
- ``self-<partition>-<name>`` for ``net self`` self-IPs
- ``net-<partition>-<self>`` for any address that falls inside a
  self-IP's subnet (so unknown 10.0.1.x addresses still get tagged
  with the VLAN they're on)

These mappings are emitted as a PCAPNG **Name Resolution Block**
(NRB).  By default we walk the capture first and only emit records
for IPs that actually appear in a packet — the NRB stays small and
relevant.  Pass ``include_unobserved=True`` (CLI: ``--all``) to emit
every IP from the inventory regardless of whether it shows up.

Optionally, a TLS NSS-format keylog file can be injected as a
**Decryption Secrets Block** (DSB), letting Wireshark decrypt TLS
sessions directly from the capture (no external ``SSLKEYLOGFILE``).

The NRB / DSB are inserted right after the first IDB of the first
section so they precede every EPB and Wireshark picks them up before
parsing packets.  Every other block — SHB, IDB, EPB, ISB, Custom,
existing NRB/DSB, etc. — is round-tripped byte-for-byte.

Plain libpcap input is converted to PCAPNG on the fly via ``editcap``
or ``tshark`` if either is on PATH; otherwise the verb refuses
because libpcap can't carry NRB/DSB blocks.
"""

from __future__ import annotations

import ipaddress
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO

from . import pcapng as _pcapng
from .model import BigipConfig
from .parser import _extract_blocks, _parse_list_block, _parse_properties
from .pcap_remap import _find_ip_offset


@dataclass(frozen=True, slots=True)
class EnrichResult:
    """Stats reported back to the CLI after a successful enrichment."""

    ipv4_records: int = 0
    ipv6_records: int = 0
    names_total: int = 0
    keylog_bytes: int = 0
    converted_from_libpcap: bool = False
    observed_ipv4: int = 0
    observed_ipv6: int = 0


_V4Net = ipaddress.IPv4Network
_V6Net = ipaddress.IPv6Network


@dataclass(slots=True)
class NameIndex:
    """Maps IP addresses (as ``str``) to one or more annotation names.

    Exact-IP entries live in :attr:`v4` / :attr:`v6`.  CIDR fallback
    labels (e.g. self-IP subnets) live in :attr:`v4_subnets` /
    :attr:`v6_subnets` and are applied to any address whose containing
    network is in the list — used so unknown hosts inside a known
    VLAN still get tagged.
    """

    v4: dict[str, list[str]] = field(default_factory=dict)
    v6: dict[str, list[str]] = field(default_factory=dict)
    v4_subnets: list[tuple[_V4Net, str]] = field(default_factory=list)
    v6_subnets: list[tuple[_V6Net, str]] = field(default_factory=list)

    def add(self, address: str, name: str) -> None:
        if not address or not name:
            return
        try:
            ip = ipaddress.ip_address(address)
        except ValueError:
            return
        bucket = self.v4 if isinstance(ip, ipaddress.IPv4Address) else self.v6
        names = bucket.setdefault(str(ip), [])
        if name not in names:
            names.append(name)

    def add_subnet(
        self,
        network: _V4Net | _V6Net,
        name: str,
    ) -> None:
        if not name:
            return
        bucket = self.v4_subnets if isinstance(network, _V4Net) else self.v6_subnets
        entry = (network, name)
        # Dedupe by exact ``(network, name)`` so multi-config merges don't
        # accumulate identical subnet rows; ``build_merged_name_index``
        # passes the same ``combined_source`` to every per-config call,
        # which would otherwise re-add every ``net self`` block N times.
        if entry not in bucket:
            bucket.append(entry)

    def update(self, other: NameIndex) -> None:
        """Merge *other* into ``self`` — used to combine multi-config inventories."""
        for addr, names in other.v4.items():
            for name in names:
                self.add(addr, name)
        for addr, names in other.v6.items():
            for name in names:
                self.add(addr, name)
        for net, name in other.v4_subnets:
            self.add_subnet(net, name)
        for net, name in other.v6_subnets:
            self.add_subnet(net, name)

    def lookup(self, address: str) -> list[str]:
        """Return every label that applies to *address* (exact + subnet)."""
        try:
            ip = ipaddress.ip_address(address)
        except ValueError:
            return []
        if isinstance(ip, ipaddress.IPv4Address):
            names = list(self.v4.get(str(ip), ()))
            for net, label in self.v4_subnets:
                if ip in net and label not in names:
                    names.append(label)
            return names
        names = list(self.v6.get(str(ip), ()))
        for net, label in self.v6_subnets:
            if ip in net and label not in names:
                names.append(label)
        return names

    def __bool__(self) -> bool:
        return bool(self.v4) or bool(self.v6) or bool(self.v4_subnets) or bool(self.v6_subnets)

    def total_names(self) -> int:
        exact = sum(len(v) for v in self.v4.values()) + sum(len(v) for v in self.v6.values())
        return exact + len(self.v4_subnets) + len(self.v6_subnets)


# Naming helpers
#
# Goal: produce labels Wireshark can treat as hostnames — alphanumerics,
# dashes, dots — that read like the user's example (``vs-common-vs1``,
# ``snat-automap-self-192-168-1-1``).  We lowercase, strip the leading
# partition prefix from full paths, and translate IP dots to dashes
# inside the host-name body so the result fits typical DNS-label
# constraints.

_LABEL_SAFE = set("abcdefghijklmnopqrstuvwxyz0123456789-")


def _slug(text: str) -> str:
    """Lowercase *text*, replacing anything that isn't ``[a-z0-9-]`` with ``-``."""
    out = []
    prev_dash = False
    for ch in text.lower():
        if ch in _LABEL_SAFE:
            out.append(ch)
            prev_dash = ch == "-"
        else:
            if not prev_dash:
                out.append("-")
                prev_dash = True
    return "".join(out).strip("-")


def _label(purpose: str, *objname_parts: str) -> str:
    """Build a ``purpose-objname`` label, lowercased with ``-`` separators.

    Concatenates *purpose* with every non-empty *objname_parts* slug,
    joined by ``-``.  Each part is normalised individually so callers
    can pass full paths, raw IPs, or short names interchangeably and
    get back a single, stable, DNS-label-ish string.
    """
    pieces = [purpose]
    for part in objname_parts:
        slug = _slug(part)
        if slug:
            pieces.append(slug)
    return "-".join(pieces)


def _split_destination(dest: str) -> str:
    """Extract the address portion of a ``[/Common/]ADDR[:port]`` destination.

    BIG-IP separates address and port with ``:`` for IPv4 and ``.`` for
    IPv6 (e.g. ``/Common/2001:db8::1.443``).  We try to be lenient: the
    last colon is treated as the port separator only if what follows
    parses as an integer.
    """
    if not dest:
        return ""
    base = dest.rsplit("/", 1)[-1]
    if ":" in base:
        host, _, port = base.rpartition(":")
        if port.isdigit() and host:
            try:
                ipaddress.ip_address(host)
                return host
            except ValueError:
                pass
    if "." in base:
        # BIG-IP uses `.port` rather than `:port` for IPv6 destinations.
        host, _, port = base.rpartition(".")
        if port.isdigit() and host:
            try:
                ipaddress.ip_address(host)
                return host
            except ValueError:
                pass
    try:
        ipaddress.ip_address(base)
        return base
    except ValueError:
        return ""


def _resolve_pool_member_address(member_name: str, config: BigipConfig) -> str:
    """Resolve a pool member's IP from its name, falling back to the node table."""
    addr = _split_destination(member_name)
    if addr:
        return addr
    base = member_name.rsplit("/", 1)[-1]
    short_no_port = base.split(":", 1)[0].split(".", 1)[0]
    full_no_port = member_name.split(":", 1)[0].split(".", 1)[0]
    for candidate in (full_no_port, member_name, short_no_port):
        resolved = config.resolve_name(candidate, config.nodes)
        if resolved:
            node = config.nodes[resolved]
            if node.address:
                return node.address
    return ""


# net self extraction
#
# `net self` blocks aren't modelled in BigipConfig, but they carry the
# only addresses that let us answer "what subnet is this packet on?" —
# i.e. the BIG-IP's own VLAN-attached IPs.  We do a small text-level
# pass to extract them.  Route-domain suffixes (``%N``) are stripped
# before parsing the address.

_ADDR_LINE = re.compile(r"^\s*address\s+(\S+)\s*$", re.MULTILINE)


@dataclass(frozen=True, slots=True)
class _SelfIp:
    full_path: str
    address: str
    network: _V4Net | _V6Net | None


def _parse_named_subblocks(braced_text: str) -> list[tuple[str, str]]:
    """Parse ``{ name1 { body1 } name2 { body2 } ... }`` into ``[(name, body), …]``.

    Used to walk the named sub-stanzas inside ``addresses`` /
    ``virtual-servers`` / ``members`` blocks where each entry is itself
    a brace-delimited mini-block keyed by name.
    """
    inner = braced_text.strip()
    if inner.startswith("{"):
        inner = inner[1:]
    if inner.endswith("}"):
        inner = inner[:-1]

    out: list[tuple[str, str]] = []
    pos = 0
    n = len(inner)
    while pos < n:
        while pos < n and inner[pos] in " \t\n\r":
            pos += 1
        if pos >= n:
            break
        name_start = pos
        while pos < n and inner[pos] not in " \t\n\r{":
            pos += 1
        name = inner[name_start:pos]
        while pos < n and inner[pos] in " \t":
            pos += 1
        body = ""
        if pos < n and inner[pos] == "{":
            body_start = pos + 1
            pos += 1
            depth = 1
            while pos < n and depth > 0:
                ch = inner[pos]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                pos += 1
            body = inner[body_start : pos - 1]
        if name:
            out.append((name, body))
    return out


def _extract_self_ips(source: str) -> list[_SelfIp]:
    """Pull every ``net self`` block's address (and CIDR, if present) from source."""
    found: list[_SelfIp] = []
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "net" or parts[1] != "self":
            continue
        full_path = parts[2]
        m = _ADDR_LINE.search(block.body)
        if m is None:
            continue
        raw = m.group(1).strip()
        if "/" in raw:
            addr_part, _, cidr = raw.partition("/")
        else:
            addr_part, cidr = raw, ""
        addr_no_rd = addr_part.split("%", 1)[0]
        try:
            ipaddress.ip_address(addr_no_rd)
        except ValueError:
            continue
        network: _V4Net | _V6Net | None = None
        if cidr:
            try:
                network = ipaddress.ip_network(f"{addr_no_rd}/{cidr}", strict=False)
            except ValueError:
                network = None
        found.append(_SelfIp(full_path=full_path, address=addr_no_rd, network=network))
    return found


# GTM extraction
#
# `gtm wideip → gtm pool → gtm server` isn't modelled in BigipConfig,
# so we walk the source text to resolve every wide-IP back to the IPs
# its members will hand out and tag those IPs with a ``wideip-…``
# label.  The chain is:
#
#   gtm wideip <type> <wideip>      pools { /<part>/<gtmpool> { … } }
#   gtm pool   <type> <gtmpool>     members { /<part>/<server>:<vs> … }
#   gtm server <server>             addresses { 10.0.0.1 { … } }
#                                   virtual-servers { <vs> { destination 10.0.0.1:443 } }
#
# We accept either source: a wide-IP claims every server-VS destination
# in its pool members; if a member's VS isn't found, we fall back to
# every address in the server's ``addresses`` block.


@dataclass(frozen=True, slots=True)
class _GtmServer:
    full_path: str
    addresses: tuple[str, ...]  # bare IPs in the addresses block
    virtual_servers: dict[str, str]  # vs_name -> address (port stripped)


def _addr_only(text: str) -> str:
    """Strip a ``:port`` or ``.port`` suffix and return just the IP literal."""
    return _split_destination(text) or text


def _extract_gtm_servers(source: str) -> dict[str, _GtmServer]:
    servers: dict[str, _GtmServer] = {}
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "gtm" or parts[1] != "server":
            continue
        full_path = parts[2]
        props = _parse_properties(block.body)

        addresses: list[str] = []
        addresses_block = props.get("addresses")
        if addresses_block:
            for raw, _body in _parse_named_subblocks(addresses_block):
                addr = _addr_only(raw)
                try:
                    ipaddress.ip_address(addr)
                except ValueError:
                    continue
                addresses.append(addr)

        vs_addrs: dict[str, str] = {}
        vs_block = props.get("virtual-servers")
        if vs_block:
            for vs_name, body in _parse_named_subblocks(vs_block):
                inner = _parse_properties(body)
                dest = inner.get("destination", "")
                addr = _addr_only(dest)
                try:
                    ipaddress.ip_address(addr)
                except ValueError:
                    continue
                vs_addrs[vs_name] = addr

        servers[full_path] = _GtmServer(
            full_path=full_path,
            addresses=tuple(addresses),
            virtual_servers=vs_addrs,
        )
    return servers


def _extract_gtm_pools(source: str) -> dict[str, list[tuple[str, str]]]:
    """Return ``pool_full_path -> [(server_full_path, vs_name), …]``.

    Members come from every ``gtm pool`` regardless of record type
    (a / aaaa / cname / mx / naptr / srv) — multiple pools sharing a
    full-path are merged.
    """
    pools: dict[str, list[tuple[str, str]]] = {}
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "gtm" or parts[1] != "pool":
            continue
        # `gtm pool <type> <name>` — type is optional pre-11.x and the
        # record's full path is always the *last* token.
        full_path = parts[-1]
        props = _parse_properties(block.body)
        members_block = props.get("members")
        if not members_block:
            continue
        bucket = pools.setdefault(full_path, [])
        for member_name in _parse_list_block(members_block):
            # Member format is ``<server_full_path>:<vs_name>``.  The
            # server path can contain ``/`` so we split on the last ``:``.
            head, sep, vs_name = member_name.rpartition(":")
            if not sep or not head or not vs_name:
                continue
            bucket.append((head, vs_name))
    return pools


def _extract_gtm_wideips(source: str) -> dict[str, list[str]]:
    """Return ``wideip_full_path -> [pool_full_path, …]``."""
    wideips: dict[str, list[str]] = {}
    for block in _extract_blocks(source):
        parts = block.header.split()
        if len(parts) < 3 or parts[0] != "gtm" or parts[1] != "wideip":
            continue
        full_path = parts[-1]
        props = _parse_properties(block.body)
        pools_block = props.get("pools")
        if not pools_block:
            continue
        wideips[full_path] = list(_parse_list_block(pools_block))
    return wideips


def _resolve_gtm_path(name: str, pool: dict[str, object]) -> str | None:
    """Resolve a GTM reference; tries exact, /Common/<name>, then suffix match."""
    if name in pool:
        return name
    candidate = f"/Common/{name}"
    if candidate in pool:
        return candidate
    suffix = f"/{name}"
    for key in pool:
        if key.endswith(suffix):
            return key
    return None


def _wideip_addresses(
    wideips: dict[str, list[str]],
    gtm_pools: dict[str, list[tuple[str, str]]],
    servers: dict[str, _GtmServer],
) -> dict[str, set[str]]:
    """For each wide-IP, return the set of IPs its pool chain resolves to."""
    out: dict[str, set[str]] = {}
    for wideip_path, pool_refs in wideips.items():
        addrs: set[str] = set()
        for pool_ref in pool_refs:
            resolved_pool = _resolve_gtm_path(pool_ref, gtm_pools)
            if resolved_pool is None:
                continue
            for server_ref, vs_name in gtm_pools[resolved_pool]:
                resolved_server = _resolve_gtm_path(server_ref, servers)
                if resolved_server is None:
                    continue
                server = servers[resolved_server]
                if vs_name in server.virtual_servers:
                    addrs.add(server.virtual_servers[vs_name])
                else:
                    addrs.update(server.addresses)
        if addrs:
            out[wideip_path] = addrs
    return out


def build_name_index(config: BigipConfig, source: str | None = None) -> NameIndex:
    """Build a name index from a parsed :class:`BigipConfig`.

    Labels follow the user-facing rule ``purpose-objname`` where the
    purpose prefix is as short as possible while still being obvious:

    - ``vs-…`` — virtual server destination
    - ``pool-…`` — pool member (uses the member's own path, e.g.
      ``/Common/web1:80`` -> ``pool-common-web1-80``)
    - ``snat-…`` — SNAT-pool member (uses the snatpool path, applied
      to every member of the pool)
    - ``self-…`` — ``net self`` self-IP exact match
    - ``net-…`` — any address inside a self-IP CIDR (subnet label)
    - ``node-…`` — node
    - ``irule-…`` — additional label on a VS destination, one per
      iRule attached to that VS

    If *source* is provided, ``net self`` blocks are also scanned for
    exact-IP and subnet labels so even addresses that are not in the
    LTM inventory still get tagged with the VLAN they belong to.
    """
    index = NameIndex()

    for full_path, vs in config.virtual_servers.items():
        addr = _split_destination(vs.destination)
        if not addr:
            continue
        index.add(addr, _label("vs", full_path))
        for rule_ref in vs.rules:
            resolved = config.resolve_rule(rule_ref) or rule_ref
            index.add(addr, _label("irule", resolved))

    for pool in config.pools.values():
        for member in pool.members:
            addr = member.address or _resolve_pool_member_address(member.name, config)
            if not addr:
                continue
            index.add(addr, _label("pool", member.name or addr))

    for full_path, snat in config.snat_pools.items():
        for raw in snat.members:
            addr = _split_destination(raw)
            if not addr:
                continue
            index.add(addr, _label("snat", full_path))

    for full_path, node in config.nodes.items():
        if node.address:
            index.add(node.address, _label("node", full_path))

    if source is not None:
        for self_ip in _extract_self_ips(source):
            index.add(self_ip.address, _label("self", self_ip.full_path))
            if self_ip.network is not None:
                index.add_subnet(self_ip.network, _label("net", self_ip.full_path))

        servers = _extract_gtm_servers(source)
        gtm_pools = _extract_gtm_pools(source)
        wideips = _extract_gtm_wideips(source)
        for wideip_path, addrs in _wideip_addresses(wideips, gtm_pools, servers).items():
            label = _label("wideip", wideip_path)
            for addr in addrs:
                index.add(addr, label)

    return index


def build_merged_name_index(
    configs_with_sources: list[tuple[BigipConfig, str]],
) -> NameIndex:
    """Build a merged :class:`NameIndex` from multiple ``(config, source)`` pairs.

    GTM wide-IPs in one input often resolve through pool members hosted
    by ``gtm server`` blocks defined in *another* input; we resolve
    each pair against the union of *all* inputs so cross-file references
    work transparently.
    """
    if not configs_with_sources:
        return NameIndex()

    combined_source = "\n".join(src for _cfg, src in configs_with_sources)
    merged = NameIndex()
    for config, _src in configs_with_sources:
        merged.update(build_name_index(config, source=combined_source))
    return merged


# Pcapng walking — observed IPs


def _scan_packet_ips(packet: bytes, linktype: int, v4: set[str], v6: set[str]) -> None:
    pos = _find_ip_offset(packet, linktype)
    if pos is None:
        return
    ip_off, is_v6 = pos
    if is_v6:
        if ip_off + 40 > len(packet):
            return
        v6.add(str(ipaddress.IPv6Address(bytes(packet[ip_off + 8 : ip_off + 24]))))
        v6.add(str(ipaddress.IPv6Address(bytes(packet[ip_off + 24 : ip_off + 40]))))
    else:
        if ip_off + 20 > len(packet):
            return
        v4.add(str(ipaddress.IPv4Address(bytes(packet[ip_off + 12 : ip_off + 16]))))
        v4.add(str(ipaddress.IPv4Address(bytes(packet[ip_off + 16 : ip_off + 20]))))


def collect_observed_ips(in_fh: BinaryIO) -> tuple[set[str], set[str]]:
    """Walk every EPB in *in_fh* and return ``(ipv4, ipv6)`` address sets."""
    v4: set[str] = set()
    v6: set[str] = set()
    interface_linktypes: list[int] = []
    for block in _pcapng.read_blocks(in_fh):
        if block.block_type == _pcapng.BLOCK_TYPE_SHB:
            interface_linktypes = []
        elif block.block_type == _pcapng.BLOCK_TYPE_IDB:
            interface_linktypes.append(block.linktype or 0)
        elif block.block_type == _pcapng.BLOCK_TYPE_EPB and block.packet_data:
            iface = block.interface_id or 0
            if iface < len(interface_linktypes):
                _scan_packet_ips(
                    bytes(block.packet_data), interface_linktypes[iface], v4, v6
                )
    return v4, v6


# PCAPNG enrichment driver


def _packed(addr_str: str, *, v6: bool) -> bytes:
    if v6:
        return ipaddress.IPv6Address(addr_str).packed
    return ipaddress.IPv4Address(addr_str).packed


def _try_libpcap_to_pcapng(in_path: Path, out_path: Path) -> bool:
    """Convert a libpcap file to PCAPNG using editcap or tshark.

    Returns True on success.  Does not raise on missing tools — the
    caller decides whether to error or fall back.
    """
    editcap = shutil.which("editcap")
    if editcap:
        result = subprocess.run(  # noqa: S603 — args are fully controlled below.
            [editcap, "-F", "pcapng", str(in_path), str(out_path)],
            capture_output=True,
            check=False,
        )
        if result.returncode == 0 and out_path.exists() and out_path.stat().st_size > 0:
            return True
    tshark = shutil.which("tshark")
    if tshark:
        result = subprocess.run(  # noqa: S603 — args are fully controlled below.
            [tshark, "-F", "pcapng", "-r", str(in_path), "-w", str(out_path)],
            capture_output=True,
            check=False,
        )
        if result.returncode == 0 and out_path.exists() and out_path.stat().st_size > 0:
            return True
    return False


def _build_packed_records(
    name_index: NameIndex,
    *,
    observed_v4: set[str] | None,
    observed_v6: set[str] | None,
) -> tuple[dict[bytes, list[str]], dict[bytes, list[str]]]:
    """Resolve labels for *every* observed IP (or every inventory IP if None).

    For observed-IP mode, even addresses with no exact-IP entry get a
    record if they fall inside a known subnet (self-IP CIDR).  For the
    full-inventory mode we keep emitting only exact entries — subnet
    labels apply to specific hosts, not to ``the whole subnet network``.

    The output dict is built in a stable, sorted order so two
    enrichment runs on the same input produce byte-identical NRB
    blocks (Python's set hash randomisation would otherwise shuffle
    record order between processes).
    """
    v4_packed: dict[bytes, list[str]] = {}
    v6_packed: dict[bytes, list[str]] = {}

    if observed_v4 is not None or observed_v6 is not None:
        for addr in sorted(
            observed_v4 or (),
            key=lambda a: ipaddress.IPv4Address(a).packed,
        ):
            names = name_index.lookup(addr)
            if names:
                v4_packed[_packed(addr, v6=False)] = names
        for addr in sorted(
            observed_v6 or (),
            key=lambda a: ipaddress.IPv6Address(a).packed,
        ):
            names = name_index.lookup(addr)
            if names:
                v6_packed[_packed(addr, v6=True)] = names
        return v4_packed, v6_packed

    for addr in sorted(
        name_index.v4.keys(),
        key=lambda a: ipaddress.IPv4Address(a).packed,
    ):
        names = name_index.v4[addr]
        if names:
            v4_packed[_packed(addr, v6=False)] = list(names)
    for addr in sorted(
        name_index.v6.keys(),
        key=lambda a: ipaddress.IPv6Address(a).packed,
    ):
        names = name_index.v6[addr]
        if names:
            v6_packed[_packed(addr, v6=True)] = list(names)
    return v4_packed, v6_packed


def enrich_pcapng(
    in_fh: BinaryIO,
    out_fh: BinaryIO,
    name_index: NameIndex,
    *,
    keylog_text: str | bytes | None = None,
    include_unobserved: bool = False,
) -> EnrichResult:
    """Insert NRB (and optional DSB) blocks into a PCAPNG stream.

    By default we walk the input twice — once to collect every IPv4 /
    IPv6 address that appears as a packet src or dst, then again to
    pass through every block and inject the NRB/DSB after the first
    IDB.  Only addresses that actually appear in the capture get an
    NRB record (subnet labels are applied at lookup time, so a host
    inside a self-IP subnet is still labelled even when it has no
    exact-IP entry in the index).

    Pass ``include_unobserved=True`` to skip the walk and emit an
    NRB record for every exact-IP entry in *name_index* regardless of
    whether it appears in the capture.

    Seekable inputs are walked in place (``seek(0)`` between the two
    passes) so multi-GB captures don't have to land in RAM.  Pipes
    and other non-seekable streams are spooled to a temporary file
    on disk for the second pass.

    Raises :class:`ValueError` if *in_fh* is libpcap; convert to PCAPNG
    first via :func:`_try_libpcap_to_pcapng` (the verb does this
    automatically when ``editcap`` / ``tshark`` is on PATH).
    """
    keylog_bytes_blob: bytes | None = None
    if keylog_text is not None:
        if isinstance(keylog_text, str):
            keylog_bytes_blob = keylog_text.encode("utf-8")
        else:
            keylog_bytes_blob = bytes(keylog_text)
        if not keylog_bytes_blob:
            keylog_bytes_blob = None

    # Acquire a seekable handle for two-pass reading without buffering
    # the whole capture if we don't have to.  Seekable file handles
    # rewind in place; non-seekable streams (named pipes, network
    # sockets) get spooled to a SpooledTemporaryFile that flips to
    # disk once it crosses 8 MiB so a multi-GB pipe still works.
    spooled: tempfile.SpooledTemporaryFile | None = None
    if in_fh.seekable():
        in_fh.seek(0)
        magic = in_fh.read(4)
        in_fh.seek(0)
        seekable_input: BinaryIO = in_fh
    else:
        spooled = tempfile.SpooledTemporaryFile(max_size=8 * 1024 * 1024)
        shutil.copyfileobj(in_fh, spooled)
        spooled.seek(0)
        magic = spooled.read(4)
        spooled.seek(0)
        seekable_input = spooled  # type: ignore[assignment]

    try:
        if not _pcapng.is_pcapng_magic(magic):
            raise ValueError(
                "enrich requires PCAPNG input (libpcap can't carry NRB/DSB blocks); "
                "install wireshark/editcap or convert manually first."
            )

        observed_v4: set[str] | None = None
        observed_v6: set[str] | None = None
        if not include_unobserved:
            observed_v4, observed_v6 = collect_observed_ips(seekable_input)
            seekable_input.seek(0)

        v4_packed, v6_packed = _build_packed_records(
            name_index, observed_v4=observed_v4, observed_v6=observed_v6
        )

        inserted = False
        pending_section_endian: str | None = None

        for block in _pcapng.read_blocks(seekable_input):
            _pcapng.write_block(out_fh, block)
            if not inserted and block.block_type == _pcapng.BLOCK_TYPE_SHB:
                pending_section_endian = block.endian
            elif (
                not inserted
                and pending_section_endian is not None
                and block.block_type == _pcapng.BLOCK_TYPE_IDB
            ):
                endian = block.endian
                if v4_packed or v6_packed:
                    nrb = _pcapng.build_nrb_block(endian, v4_packed, v6_packed)
                    _pcapng.write_block(out_fh, nrb)
                if keylog_bytes_blob is not None:
                    dsb = _pcapng.build_dsb_block(
                        endian, _pcapng.DSB_SECRETS_TYPE_TLS, keylog_bytes_blob
                    )
                    _pcapng.write_block(out_fh, dsb)
                inserted = True

        if not inserted:
            # No IDB ever appeared (degenerate input).  Append at end of section.
            if v4_packed or v6_packed or keylog_bytes_blob is not None:
                endian = pending_section_endian or "<"
                if v4_packed or v6_packed:
                    nrb = _pcapng.build_nrb_block(endian, v4_packed, v6_packed)
                    _pcapng.write_block(out_fh, nrb)
                if keylog_bytes_blob is not None:
                    dsb = _pcapng.build_dsb_block(
                        endian, _pcapng.DSB_SECRETS_TYPE_TLS, keylog_bytes_blob
                    )
                    _pcapng.write_block(out_fh, dsb)
    finally:
        if spooled is not None:
            spooled.close()

    names_total = sum(len(v) for v in v4_packed.values()) + sum(
        len(v) for v in v6_packed.values()
    )
    return EnrichResult(
        ipv4_records=len(v4_packed),
        ipv6_records=len(v6_packed),
        names_total=names_total,
        keylog_bytes=len(keylog_bytes_blob) if keylog_bytes_blob is not None else 0,
        converted_from_libpcap=False,
        observed_ipv4=len(observed_v4) if observed_v4 is not None else 0,
        observed_ipv6=len(observed_v6) if observed_v6 is not None else 0,
    )


def enrich_capture_file(
    in_path: Path,
    out_path: Path,
    name_index: NameIndex,
    *,
    keylog_text: str | bytes | None = None,
    include_unobserved: bool = False,
) -> EnrichResult:
    """Enrich a capture file on disk; auto-converts libpcap → pcapng.

    Wraps :func:`enrich_pcapng` with the libpcap conversion fallback.
    The output is staged in a temporary file alongside *out_path* and
    renamed into place atomically once the enrich step succeeds — so
    a crash mid-write can't half-overwrite the destination, and an
    invocation like ``f5 enrich-pcapng in.pcapng in.pcapng`` (same
    path for input and output) doesn't truncate the input before we
    read it.
    """
    with open(in_path, "rb") as fh:
        magic = fh.read(4)

    converted = False
    pcapng_input: Path = in_path
    tmp_dir: tempfile.TemporaryDirectory[str] | None = None

    out_path = Path(out_path)
    # Stage in the output's parent so the final rename stays on the
    # same filesystem (rename across filesystems isn't atomic).  Use a
    # ``.partial`` suffix so the file is recognisable as in-progress.
    out_dir = out_path.parent if out_path.parent != Path("") else Path(".")
    out_dir.mkdir(parents=True, exist_ok=True)
    staging_fd, staging_name = tempfile.mkstemp(
        prefix=out_path.name + ".",
        suffix=".partial",
        dir=str(out_dir),
    )
    staging_path = Path(staging_name)

    try:
        if not _pcapng.is_pcapng_magic(magic):
            tmp_dir = tempfile.TemporaryDirectory(prefix="enrich-pcapng-")
            converted_path = Path(tmp_dir.name) / (in_path.stem + ".pcapng")
            if not _try_libpcap_to_pcapng(in_path, converted_path):
                raise ValueError(
                    f"input {in_path.name} is libpcap and neither editcap nor "
                    f"tshark is available to convert it to pcapng"
                )
            pcapng_input = converted_path
            converted = True

        with open(pcapng_input, "rb") as in_fh, os.fdopen(staging_fd, "wb") as out_fh:
            result = enrich_pcapng(
                in_fh,
                out_fh,
                name_index,
                keylog_text=keylog_text,
                include_unobserved=include_unobserved,
            )
        # Atomic replace — works on POSIX and Windows alike.
        os.replace(staging_path, out_path)
    except BaseException:
        # Drop the half-written staging file; never touch out_path on failure.
        try:
            staging_path.unlink(missing_ok=True)
        except OSError:
            pass
        raise
    finally:
        if tmp_dir is not None:
            tmp_dir.cleanup()

    return EnrichResult(
        ipv4_records=result.ipv4_records,
        ipv6_records=result.ipv6_records,
        names_total=result.names_total,
        keylog_bytes=result.keylog_bytes,
        converted_from_libpcap=converted,
        observed_ipv4=result.observed_ipv4,
        observed_ipv6=result.observed_ipv6,
    )
