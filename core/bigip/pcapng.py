"""Minimal PCAPNG reader/writer for ``f5 pcap-remap``.

PCAPNG is the modern Wireshark default capture format.  Unlike classic
libpcap (24-byte global header + 16-byte record headers), PCAPNG is a
chain of variably-sized **blocks**, each with:

    [block_type:4][block_total_length:4][body...][block_total_length:4]

The trailing length is a redundant copy that lets readers walk the file
backwards.  Per-block bodies are 32-bit aligned with explicit padding.

For *our* purposes — apply a redaction map to the IP layer and the F5
trailer — we only need to recognise three block types:

- **Section Header Block (SHB)** type 0x0A0D0D0A
- **Interface Description Block (IDB)** type 0x00000001 — gives the
  link-layer type for subsequent EPBs in the same section.
- **Enhanced Packet Block (EPB)** type 0x00000006 — the actual frame.

Other blocks (Simple Packet, Name Resolution, Interface Stats, Custom
Block, etc.) are passed through byte-for-byte without dissection.

Endianness is per-section, set by the SHB byte-order magic
(``0x1A2B3C4D`` written natively).

This module exposes :func:`read_pcapng_packets` and
:func:`write_pcapng_packets` so :func:`core.bigip.pcap_remap.remap_pcap`
can branch on the file magic and call us instead of the libpcap path.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import BinaryIO, Iterator

# Block types we dissect; everything else is forwarded byte-for-byte.
BLOCK_TYPE_SHB = 0x0A0D0D0A
BLOCK_TYPE_IDB = 0x00000001
BLOCK_TYPE_EPB = 0x00000006
BLOCK_TYPE_SPB = 0x00000003  # Simple Packet Block — has no IDB ref; rare
BLOCK_TYPE_NRB = 0x00000004  # Name Resolution Block
BLOCK_TYPE_DSB = 0x0000000A  # Decryption Secrets Block

# NRB record types (per the PCAPNG spec).
NRB_RECORD_END = 0x0000
NRB_RECORD_IPV4 = 0x0001
NRB_RECORD_IPV6 = 0x0002

# DSB secrets-type identifiers (per the PCAPNG spec, draft-tuexen-opsawg-pcapng).
DSB_SECRETS_TYPE_TLS = 0x544C534B  # "TLSK" — NSS-format TLS keylog text
DSB_SECRETS_TYPE_SSH = 0x5353484B  # "SSHK"
DSB_SECRETS_TYPE_WIREGUARD = 0x57474B4C  # "WGKL"


SHB_BYTE_ORDER_MAGIC = 0x1A2B3C4D


@dataclass(slots=True)
class PcapngBlock:
    """One raw block read from a PCAPNG file.

    For most block types we don't dissect the body — the rewriter only
    needs to mutate EPB packet data, and even then only at known
    offsets.  Everything else round-trips by carrying the original
    block bytes.
    """

    block_type: int
    body: bytes  # body bytes between the two block_total_length fields
    endian: str  # "<" | ">"
    # For EPB only: cached fields the caller needs.  None for non-EPB blocks.
    interface_id: int | None = None
    timestamp_high: int | None = None
    timestamp_low: int | None = None
    captured_len: int | None = None
    original_len: int | None = None
    packet_data: bytes | None = None
    # Trailing options bytes between packet data (padded) and block end.
    options: bytes | None = None
    # For SHB only: the section-wide endian.
    # For IDB only: the link-layer type.
    linktype: int | None = None


def _pad32(n: int) -> int:
    """Round n up to the next multiple of 4."""
    return (n + 3) & ~3


def _read_u32(fh: BinaryIO, endian: str) -> int | None:
    raw = fh.read(4)
    if len(raw) < 4:
        return None
    return struct.unpack(endian + "I", raw)[0]


def _peek_endian(fh: BinaryIO) -> str:
    """Determine endianness from the first SHB without consuming bytes."""
    pos = fh.tell()
    block_type = fh.read(4)
    fh.read(4)  # block_total_length — re-read in the main parser
    magic = fh.read(4)
    fh.seek(pos)
    if len(block_type) < 4 or len(magic) < 4:
        raise ValueError("pcapng: file too short for SHB")
    if struct.unpack("<I", block_type)[0] != BLOCK_TYPE_SHB:
        raise ValueError(
            f"pcapng: first block must be SHB (0x0a0d0d0a), got 0x{struct.unpack('<I', block_type)[0]:08x}"
        )
    # The byte-order magic is written natively, so reading it as "<I"
    # gives 0x1a2b3c4d on LE and 0x4d3c2b1a on BE.
    if struct.unpack("<I", magic)[0] == SHB_BYTE_ORDER_MAGIC:
        return "<"
    if struct.unpack(">I", magic)[0] == SHB_BYTE_ORDER_MAGIC:
        return ">"
    raise ValueError(f"pcapng: SHB byte-order magic not recognised: {magic.hex()}")


def read_blocks(fh: BinaryIO) -> Iterator[PcapngBlock]:
    """Yield :class:`PcapngBlock` for every block in *fh*.

    The first SHB sets the section endianness; subsequent SHBs (rare —
    PCAPNG supports concatenated sections) reset it.  The yielded
    block has its body parsed enough to identify EPB packet data and
    IDB linktype, but the original bytes are preserved verbatim for
    pass-through.
    """
    endian = _peek_endian(fh)

    while True:
        header = fh.read(8)
        if len(header) == 0:
            return
        if len(header) < 8:
            raise ValueError("pcapng: truncated block header")
        block_type, block_total_len = struct.unpack(endian + "II", header)
        if block_total_len < 12 or block_total_len % 4 != 0:
            raise ValueError(
                f"pcapng: invalid block_total_length {block_total_len} for type 0x{block_type:08x}"
            )
        body_len = block_total_len - 12  # minus type(4) + len(4) + trailing-len(4)
        body = fh.read(body_len)
        if len(body) < body_len:
            raise ValueError("pcapng: truncated block body")
        trailing = fh.read(4)
        if len(trailing) < 4:
            raise ValueError("pcapng: truncated block trailer")
        trailing_len = struct.unpack(endian + "I", trailing)[0]
        if trailing_len != block_total_len:
            raise ValueError(
                f"pcapng: trailing length 0x{trailing_len:x} != header 0x{block_total_len:x}"
            )

        if block_type == BLOCK_TYPE_SHB:
            # First 4 bytes are byte-order magic.  Re-derive endian from
            # this section's magic so concatenated sections work.
            magic = struct.unpack("<I", body[:4])[0]
            if magic == SHB_BYTE_ORDER_MAGIC:
                endian = "<"
            else:
                endian = ">"
            yield PcapngBlock(block_type=block_type, body=body, endian=endian)
            continue

        if block_type == BLOCK_TYPE_IDB:
            linktype, _reserved = struct.unpack(endian + "HH", body[:4])
            # Body: linktype(2) + reserved(2) + snaplen(4) + options(...)
            yield PcapngBlock(block_type=block_type, body=body, endian=endian, linktype=linktype)
            continue

        if block_type == BLOCK_TYPE_EPB:
            # Body: ifid(4) ts_high(4) ts_low(4) cap_len(4) orig_len(4) data... opts...
            (interface_id, ts_high, ts_low, cap_len, orig_len) = struct.unpack(
                endian + "IIIII", body[:20]
            )
            packet_off = 20
            packet_end = packet_off + cap_len
            packet_data = body[packet_off:packet_end]
            options = body[_pad32(packet_end) :]
            yield PcapngBlock(
                block_type=block_type,
                body=body,
                endian=endian,
                interface_id=interface_id,
                timestamp_high=ts_high,
                timestamp_low=ts_low,
                captured_len=cap_len,
                original_len=orig_len,
                packet_data=packet_data,
                options=options,
            )
            continue

        # Other block types (NRB, ISB, SPB, custom, ...) — pass through.
        yield PcapngBlock(block_type=block_type, body=body, endian=endian)


def write_block(fh: BinaryIO, block: PcapngBlock) -> None:
    """Serialise *block* back to *fh*.

    For EPB blocks where the caller mutated ``packet_data``, the body
    is rebuilt from the cached fields plus the new packet bytes.  For
    every other block type the original ``body`` bytes are written
    verbatim.
    """
    endian = block.endian
    if (
        block.block_type == BLOCK_TYPE_EPB
        and block.packet_data is not None
        and block.captured_len is not None
        and block.original_len is not None
        and block.options is not None
    ):
        # Rebuild EPB body with potentially-modified packet bytes.
        # captured_len is unchanged because all our rewrites are
        # length-preserving, but we re-read it from the block to keep
        # the writer agnostic.
        cap_len = len(block.packet_data)
        if cap_len != block.captured_len:
            raise ValueError(
                f"pcapng: EPB packet length changed ({cap_len} != {block.captured_len}); "
                f"the rewriter must be length-preserving"
            )
        body = (
            struct.pack(
                endian + "IIIII",
                block.interface_id or 0,
                block.timestamp_high or 0,
                block.timestamp_low or 0,
                cap_len,
                block.original_len or 0,
            )
            + block.packet_data
            + b"\x00" * (_pad32(cap_len) - cap_len)
            + block.options
        )
    else:
        body = block.body

    block_total_len = 12 + len(body)
    fh.write(struct.pack(endian + "II", block.block_type, block_total_len))
    fh.write(body)
    fh.write(struct.pack(endian + "I", block_total_len))


def is_pcapng_magic(four_bytes: bytes) -> bool:
    """True iff *four_bytes* is the PCAPNG SHB block-type magic."""
    if len(four_bytes) < 4:
        return False
    return struct.unpack("<I", four_bytes)[0] == BLOCK_TYPE_SHB


def build_nrb_block(
    endian: str,
    v4_names: dict[bytes, list[str]] | None = None,
    v6_names: dict[bytes, list[str]] | None = None,
) -> PcapngBlock:
    """Build a Name Resolution Block (NRB) from per-address name lists.

    *v4_names* maps 4-byte big-endian IPv4 packed addresses to a list of
    UTF-8 names.  *v6_names* maps 16-byte big-endian IPv6 packed
    addresses similarly.  At least one ``nrb_record_end`` (type 0,
    length 0) record is always emitted to terminate the record list,
    per the PCAPNG spec.

    Each record is::

        [type:2][length:2][value:length, padded to 4 bytes]

    For an IPv4 record the value is ``[4-byte addr][name1\\0][name2\\0]…``;
    for IPv6, ``[16-byte addr][name1\\0]…``.  *length* is the unpadded
    value length; the padding bytes are excluded from the count.
    """
    body = bytearray()

    def _append_record(rec_type: int, addr: bytes, names: list[str]) -> None:
        names_blob = b"".join(n.encode("utf-8") + b"\x00" for n in names if n)
        if not names_blob:
            return
        value = addr + names_blob
        body.extend(struct.pack(endian + "HH", rec_type, len(value)))
        body.extend(value)
        pad = (-len(value)) & 3
        if pad:
            body.extend(b"\x00" * pad)

    if v4_names:
        for addr, names in v4_names.items():
            if len(addr) != 4:
                raise ValueError(f"v4 record address must be 4 bytes, got {len(addr)}")
            _append_record(NRB_RECORD_IPV4, addr, names)

    if v6_names:
        for addr, names in v6_names.items():
            if len(addr) != 16:
                raise ValueError(f"v6 record address must be 16 bytes, got {len(addr)}")
            _append_record(NRB_RECORD_IPV6, addr, names)

    # End-of-records sentinel — required even when there are no name records.
    body.extend(struct.pack(endian + "HH", NRB_RECORD_END, 0))

    return PcapngBlock(block_type=BLOCK_TYPE_NRB, body=bytes(body), endian=endian)


def build_dsb_block(endian: str, secrets_type: int, secrets_data: bytes) -> PcapngBlock:
    """Build a Decryption Secrets Block (DSB) carrying *secrets_data*.

    Used to inline TLS / SSH / WireGuard key material into a capture so
    Wireshark can decrypt it without an external SSLKEYLOGFILE.  For TLS
    NSS-format keylogs, pass *secrets_type* = :data:`DSB_SECRETS_TYPE_TLS`
    and the raw keylog file content as *secrets_data*.

    Body layout::

        [secrets_type:4][secrets_length:4][secrets_data:secrets_length, padded to 4]
    """
    if not isinstance(secrets_data, (bytes, bytearray)):
        raise TypeError("secrets_data must be bytes")
    body = bytearray()
    body.extend(struct.pack(endian + "II", secrets_type, len(secrets_data)))
    body.extend(secrets_data)
    pad = (-len(secrets_data)) & 3
    if pad:
        body.extend(b"\x00" * pad)
    return PcapngBlock(block_type=BLOCK_TYPE_DSB, body=bytes(body), endian=endian)
