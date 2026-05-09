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

import io
import ipaddress
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO

from . import pcapng as _pcapng
from .model import BigipConfig
from .parser import _extract_blocks
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
        if isinstance(network, _V4Net):
            self.v4_subnets.append((network, name))
        else:
            self.v6_subnets.append((network, name))

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

    return index


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
    """
    v4_packed: dict[bytes, list[str]] = {}
    v6_packed: dict[bytes, list[str]] = {}

    if observed_v4 is not None or observed_v6 is not None:
        for addr in observed_v4 or ():
            names = name_index.lookup(addr)
            if names:
                v4_packed[_packed(addr, v6=False)] = names
        for addr in observed_v6 or ():
            names = name_index.lookup(addr)
            if names:
                v6_packed[_packed(addr, v6=True)] = names
        return v4_packed, v6_packed

    for addr, names in name_index.v4.items():
        if names:
            v4_packed[_packed(addr, v6=False)] = list(names)
    for addr, names in name_index.v6.items():
        if names:
            v6_packed[_packed(addr, v6=True)] = list(names)
    return v4_packed, v6_packed


def _read_all_for_two_pass(in_fh: BinaryIO) -> bytes:
    """Snapshot *in_fh* so we can iterate it twice (rewind if seekable)."""
    if in_fh.seekable():
        in_fh.seek(0)
        return in_fh.read()
    return in_fh.read()


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

    Raises :class:`ValueError` if *in_fh* is libpcap; convert to PCAPNG
    first via :func:`_try_libpcap_to_pcapng` (the verb does this
    automatically when ``editcap`` / ``tshark`` is on PATH).
    """
    data = _read_all_for_two_pass(in_fh)
    if not _pcapng.is_pcapng_magic(data[:4]):
        raise ValueError(
            "enrich requires PCAPNG input (libpcap can't carry NRB/DSB blocks); "
            "install wireshark/editcap or convert manually first."
        )

    keylog_bytes_blob: bytes | None = None
    if keylog_text is not None:
        if isinstance(keylog_text, str):
            keylog_bytes_blob = keylog_text.encode("utf-8")
        else:
            keylog_bytes_blob = bytes(keylog_text)
        if not keylog_bytes_blob:
            keylog_bytes_blob = None

    observed_v4: set[str] | None = None
    observed_v6: set[str] | None = None
    if not include_unobserved:
        observed_v4, observed_v6 = collect_observed_ips(io.BytesIO(data))

    v4_packed, v6_packed = _build_packed_records(
        name_index, observed_v4=observed_v4, observed_v6=observed_v6
    )

    inserted = False
    pending_section_endian: str | None = None

    for block in _pcapng.read_blocks(io.BytesIO(data)):
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
    """
    with open(in_path, "rb") as fh:
        magic = fh.read(4)

    converted = False
    pcapng_input: Path = in_path
    tmp_dir: tempfile.TemporaryDirectory[str] | None = None
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

        with open(pcapng_input, "rb") as in_fh, open(out_path, "wb") as out_fh:
            result = enrich_pcapng(
                in_fh,
                out_fh,
                name_index,
                keylog_text=keylog_text,
                include_unobserved=include_unobserved,
            )
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
