"""Enrich a PCAPNG capture with BIG-IP-derived metadata.

Powers ``f5 enrich-pcapng``.

For each unique IPv4/IPv6 address that appears as a virtual-server
destination, pool member, SNAT-pool member, or node in a parsed
:class:`BigipConfig`, we synthesise a hostname-like label (e.g.
``vs-common-vs1``, ``snat-common-my_snatpool-192-168-1-1``,
``pool-common-web_pool-10-0-1-10``) and emit a single PCAPNG **Name
Resolution Block** (NRB) that maps each address to its label set.
Wireshark will then display those names alongside the addresses in
its packet view, exactly as if reverse DNS had returned them.

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
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO

from . import pcapng as _pcapng
from .model import BigipConfig


@dataclass(frozen=True, slots=True)
class EnrichResult:
    """Stats reported back to the CLI after a successful enrichment."""

    ipv4_records: int = 0
    ipv6_records: int = 0
    names_total: int = 0
    keylog_bytes: int = 0
    converted_from_libpcap: bool = False


@dataclass(slots=True)
class NameIndex:
    """Maps IP addresses (as ``str``) to one or more annotation names."""

    v4: dict[str, list[str]] = field(default_factory=dict)
    v6: dict[str, list[str]] = field(default_factory=dict)

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

    def __bool__(self) -> bool:
        return bool(self.v4) or bool(self.v6)

    def total_names(self) -> int:
        return sum(len(v) for v in self.v4.values()) + sum(len(v) for v in self.v6.values())


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


def _split_partition_name(full_path: str) -> tuple[str, str]:
    """``/Common/foo`` -> ``("common", "foo")``; ``foo`` -> ``("", "foo")``."""
    if not full_path:
        return "", ""
    if full_path.startswith("/"):
        parts = full_path.lstrip("/").split("/", 1)
        if len(parts) == 2:
            return parts[0].lower(), parts[1]
        return "", parts[0]
    return "", full_path


def _ip_label(addr: str) -> str:
    """``192.168.1.1`` -> ``192-168-1-1``; v6 ``2001:db8::1`` -> ``2001-db8--1``."""
    return addr.replace(".", "-").replace(":", "-")


def _name(prefix: str, partition: str, name: str, *suffix: str) -> str:
    pieces = [prefix]
    if partition:
        pieces.append(_slug(partition))
    if name:
        pieces.append(_slug(name))
    pieces.extend(_slug(s) for s in suffix if s)
    return "-".join(p for p in pieces if p)


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
    # Try node lookup by short or full name, dropping any port suffix first.
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


def build_name_index(config: BigipConfig) -> NameIndex:
    """Build a name index from a parsed :class:`BigipConfig`."""
    index = NameIndex()

    for full_path, vs in config.virtual_servers.items():
        partition, name = _split_partition_name(full_path)
        addr = _split_destination(vs.destination)
        if addr:
            index.add(addr, _name("vs", partition, name))

    for full_path, pool in config.pools.items():
        partition, name = _split_partition_name(full_path)
        for member in pool.members:
            addr = member.address or _resolve_pool_member_address(member.name, config)
            if addr:
                index.add(addr, _name("pool", partition, name, _ip_label(addr)))

    for full_path, snat in config.snat_pools.items():
        partition, name = _split_partition_name(full_path)
        for raw in snat.members:
            addr = _split_destination(raw)
            if addr:
                index.add(addr, _name("snat", partition, name, _ip_label(addr)))

    for full_path, node in config.nodes.items():
        if not node.address:
            continue
        partition, name = _split_partition_name(full_path)
        index.add(node.address, _name("node", partition, name))

    return index


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


def enrich_pcapng(
    in_fh: BinaryIO,
    out_fh: BinaryIO,
    name_index: NameIndex,
    *,
    keylog_text: str | bytes | None = None,
) -> EnrichResult:
    """Insert NRB (and optional DSB) blocks into a PCAPNG stream.

    The NRB and DSB are written directly after the first IDB of the
    first section so they precede every EPB and Wireshark applies them
    before dissecting any packet.  All other blocks are passed through
    byte-for-byte.

    Raises :class:`ValueError` if *in_fh* is libpcap; convert to PCAPNG
    first via :func:`_try_libpcap_to_pcapng` (the verb does this
    automatically when ``editcap`` / ``tshark`` is on PATH).
    """
    first4 = in_fh.read(4)
    in_fh.seek(0)
    if not _pcapng.is_pcapng_magic(first4):
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

    v4_packed: dict[bytes, list[str]] = {
        _packed(addr, v6=False): list(names) for addr, names in name_index.v4.items()
    }
    v6_packed: dict[bytes, list[str]] = {
        _packed(addr, v6=True): list(names) for addr, names in name_index.v6.items()
    }

    inserted = False
    blocks_iter = _pcapng.read_blocks(in_fh)
    pending_section_endian: str | None = None

    for block in blocks_iter:
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
        # No IDB ever appeared (degenerate input).  Fall back to writing
        # the NRB/DSB at the very end of the section so the output stays
        # valid pcapng even if Wireshark prefers them earlier.
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

    return EnrichResult(
        ipv4_records=len(v4_packed),
        ipv6_records=len(v6_packed),
        names_total=name_index.total_names(),
        keylog_bytes=len(keylog_bytes_blob) if keylog_bytes_blob is not None else 0,
        converted_from_libpcap=False,
    )


def enrich_capture_file(
    in_path: Path,
    out_path: Path,
    name_index: NameIndex,
    *,
    keylog_text: str | bytes | None = None,
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
            result = enrich_pcapng(in_fh, out_fh, name_index, keylog_text=keylog_text)
    finally:
        if tmp_dir is not None:
            tmp_dir.cleanup()

    return EnrichResult(
        ipv4_records=result.ipv4_records,
        ipv6_records=result.ipv6_records,
        names_total=result.names_total,
        keylog_bytes=result.keylog_bytes,
        converted_from_libpcap=converted,
    )
