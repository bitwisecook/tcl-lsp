"""Parser for the F5 BIG-IP Ethernet trailer (HSB / "noise") added by
``tcpdump -i 0.0:nnn[p]`` captures.

Two on-the-wire formats coexist on production fleets:

- **Legacy** (TMOS 9.4–13.x): a chain of variable-length entries
  ``[type:1][length:1][version:1][value: length-3]``.  Type 1 = LOW
  (VIP name + flags), 2 = MED (TMM/CPU/timing), 3 = HIGH (peer flow
  info; fixed length 42 in version 0).  The chain ends when the next
  ``type`` is not 1/2/3 or ``length`` falls outside [7..140].

- **DPT** (TMOS 14+): an 8-byte header
  ``[magic:4=0xf5deb0f5][length:2][version:2]`` followed by a list of
  TLVs ``[provider:2][type:2][length:2][version:2][value]``.  Provider
  1 = NOISE; types within NOISE are 1=LOW, 2=MED, 3=HIGH (matching the
  legacy types).

This module:

1.  **Frames** the trailer (``parse_trailer``) by walking the TLV
    chain in whichever format is in use.  Returns one
    :class:`ParsedTLV` per entry plus an ``unknown`` flag for any TLV
    that doesn't have a registered schema.

2.  **Locates IP fields** within each TLV via a registered schema.
    The schema for `(format, type, version, length)` returns a list
    of `(offset, kind)` pairs telling the caller where to find each
    IP address.  ``kind`` is ``"v6_or_v4mapped"`` for the 16-byte
    fields the F5 trailer uses for both IPv4 and IPv6 (Wireshark's
    ``displayIPv6as4`` heuristic — IPv4-mapped IPv6 prefix
    ``::ffff:`` or the F5 route-domain prefix
    ``2620:0:c10:f501:0000:<rd>``).

3.  **Allows extension**.  External schemas can be loaded from a TOML
    file via :func:`load_schema_overlay` so operators can extend
    coverage without forking the code.

Field offsets and the TLV framing constants are ported from
Wireshark's ``epan/dissectors/packet-f5ethtrailer.c`` (GPL-2.0-or-later
licence; values are facts about the wire format and so freely
re-stated here).
"""

from __future__ import annotations

import struct
import sys
from dataclasses import dataclass, field

if sys.version_info >= (3, 11):
    import tomllib
else:  # pragma: no cover
    import tomli as tomllib  # type: ignore[no-redef]  # ty: ignore[unresolved-import]  # tomli is a fallback for py<3.11; declared as conditional dep

# ── format-detection constants (ported from packet-f5ethtrailer.c) ──

# DPT (new) format
DPT_HDR_MAGIC = 0xF5DEB0F5  # F5_DPT_V1_HDR_MAGIC
DPT_HDR_LEN = 8  # magic(4) + length(2) + version(2)
DPT_TLV_HDR_LEN = 8  # provider(2) + type(2) + length(2) + version(2)
DPT_PROVIDER_NOISE = 1  # F5_DPT_PROVIDER_NOISE

# Legacy types (also used as DPT NOISE TLV types)
LEGACY_TYPE_LOW = 1
LEGACY_TYPE_MED = 2
LEGACY_TYPE_HIGH = 3
LEGACY_MIN_SANE = 7  # F5_MIN_SANE
LEGACY_MAX_SANE = 140  # F5_MAX_SANE

# IPv6 prefix that signals "this 16-byte address actually carries an IPv4
# address in its last 4 bytes" (Wireshark: ipv4as6prefix).
IPV4_MAPPED_PREFIX = bytes.fromhex("00000000000000000000ffff")

# The F5 route-domain prefix (Wireshark: f5rtdomprefix).  Matches the
# first 10 bytes of an IPv6 address; bytes 10-11 are the 16-bit route
# domain ID; bytes 12-15 are the IPv4 address.
F5_RTDOM_PREFIX = bytes.fromhex("262000000c10f501")  # 8 bytes; full prefix is 10
# Wireshark's prefix is actually 10 bytes: the 8 above followed by 0x00 0x00.
F5_RTDOM_PREFIX_10 = F5_RTDOM_PREFIX + b"\x00\x00"


# ── parse output ────────────────────────────────────────────────────


@dataclass(slots=True)
class IpFieldRef:
    """A single IP-address field located inside a parsed TLV."""

    offset: int  # absolute offset within the trailer bytes
    kind: str  # "v4" | "v6" | "v6_or_v4mapped"


@dataclass(slots=True)
class ParsedTLV:
    """One entry in the trailer chain."""

    fmt: str  # "legacy" | "dpt"
    type_: int
    version: int
    length: int  # full entry length (header + value)
    offset: int  # absolute offset within the trailer bytes
    ip_fields: list[IpFieldRef] = field(default_factory=list)
    schema_known: bool = True  # False -> no schema; caller decides policy


@dataclass(slots=True)
class TrailerParse:
    """Result of :func:`parse_trailer`."""

    fmt: str | None  # "legacy" | "dpt" | None (no trailer detected)
    tlvs: list[ParsedTLV] = field(default_factory=list)
    bytes_consumed: int = 0  # how many bytes of *data* the trailer occupies


# ── schema registry ────────────────────────────────────────────────


# Legacy schema key: (type, version, length) → list[IpFieldRef relative-to-TLV].
# Length is part of the key because some legacy types have multiple
# (length, version) tuples that mean different layouts.
_LEGACY_SCHEMAS: dict[tuple[int, int, int], list[tuple[int, str]]] = {}

# DPT schema key: (provider, type, version) → list[IpFieldRef relative-to-TLV].
# Length isn't a key here because the DPT length is explicit in the
# header and one (provider, type, version) tuple = one layout.
_DPT_SCHEMAS: dict[tuple[int, int, int], list[tuple[int, str]]] = {}


def register_legacy_schema(
    type_: int, version: int, length: int, ip_fields: list[tuple[int, str]]
) -> None:
    """Register an IP-field layout for a legacy F5 trailer entry.

    *ip_fields* is a list of ``(offset_within_tlv, kind)`` pairs where
    ``kind`` is one of ``"v4"`` (4 bytes), ``"v6"`` (16 bytes), or
    ``"v6_or_v4mapped"`` (16 bytes; we detect IPv4-mapped at runtime).
    """
    _LEGACY_SCHEMAS[(type_, version, length)] = list(ip_fields)


def register_dpt_schema(
    provider: int, type_: int, version: int, ip_fields: list[tuple[int, str]]
) -> None:
    """Register an IP-field layout for a DPT TLV."""
    _DPT_SCHEMAS[(provider, type_, version)] = list(ip_fields)


# ── built-in schemas (ported from Wireshark) ───────────────────────


# Legacy HIGH v0 (length 42; F5_HIV0_LEN).
# After the 3-byte legacy header [type, length, version]:
#   +3  ipproto (1)
#   +4  vlan (2)
#   +6  peer_remote_addr (16-byte IPv6 / IPv4-mapped)
#   +22 peer_local_addr  (16-byte IPv6 / IPv4-mapped)
#   +38 peer_remote_port (2)
#   +40 peer_local_port  (2)
register_legacy_schema(
    LEGACY_TYPE_HIGH,
    0,
    42,
    [
        (6, "v6_or_v4mapped"),
        (22, "v6_or_v4mapped"),
    ],
)


# DPT NOISE HIGH v1 (after the 8-byte TLV header):
#   +8  ipproto (1)
#   +9  vlan (2)
#   +11 peer_remote_addr (16)
#   +27 peer_local_addr  (16)
#   +43 peer_remote_port (2)
#   +45 peer_local_port  (2)
register_dpt_schema(
    DPT_PROVIDER_NOISE,
    LEGACY_TYPE_HIGH,
    1,
    [
        (11, "v6_or_v4mapped"),
        (27, "v6_or_v4mapped"),
    ],
)


# LOW and MED entries do not carry remote-peer IP addresses (they hold
# VIP names, TMM/CPU info, RST cause strings).  They're recognised by
# the framing logic but have empty IP-field lists.
register_legacy_schema(LEGACY_TYPE_LOW, 0, 35, [])  # F5_LOWV94_LEN
register_legacy_schema(LEGACY_TYPE_LOW, 0, 22, [])  # F5_LOWV10_LEN
register_legacy_schema(LEGACY_TYPE_MED, 0, 8, [])  # F5_MEDV94_LEN
register_legacy_schema(LEGACY_TYPE_MED, 0, 21, [])  # F5_MEDV10_LEN
register_legacy_schema(LEGACY_TYPE_MED, 0, 29, [])  # F5_MEDV11_LEN
register_dpt_schema(DPT_PROVIDER_NOISE, LEGACY_TYPE_LOW, 1, [])
register_dpt_schema(DPT_PROVIDER_NOISE, LEGACY_TYPE_LOW, 2, [])
register_dpt_schema(DPT_PROVIDER_NOISE, LEGACY_TYPE_LOW, 3, [])
register_dpt_schema(DPT_PROVIDER_NOISE, LEGACY_TYPE_LOW, 4, [])
register_dpt_schema(DPT_PROVIDER_NOISE, LEGACY_TYPE_MED, 1, [])
register_dpt_schema(DPT_PROVIDER_NOISE, LEGACY_TYPE_MED, 4, [])

# DPT provider 4 = TLS keylog data (F5_DPT_PROVIDER_TLS).  Carries
# master_secret / client_random / hash bytes for SSL keylog injection;
# no IP fields but it has multiple sub-types we recognise so they
# don't trigger UnknownTrailerError under ``--on-unknown=error``.
DPT_PROVIDER_TLS = 4
for _tls_subtype in (0, 1, 2, 3):  # PRE13_STD, PRE13_EXT, 13_STD, 13_EXT
    register_dpt_schema(DPT_PROVIDER_TLS, _tls_subtype, 0, [])
    register_dpt_schema(DPT_PROVIDER_TLS, _tls_subtype, 1, [])
del _tls_subtype

# DPT provider 5 — observed in real-world TMOS captures
# (Wireshark issues 17171 and 16898) but not yet upstreamed into
# packet-f5ethtrailer.c at the time we ported the schema.  Register it
# as "no IP fields" so packets carrying it round-trip cleanly without
# triggering UnknownTrailerError.  When F5 publishes the layout we'll
# add real IP-field offsets here.
DPT_PROVIDER_5 = 5
register_dpt_schema(DPT_PROVIDER_5, 1, 1, [])
register_dpt_schema(DPT_PROVIDER_5, 1, 0, [])


# ── parsing ────────────────────────────────────────────────────────


def _legacy_known_types() -> set[int]:
    """Return the set of legacy ``type`` values that have any schema.

    Always includes the three Wireshark-defined types (LOW/MED/HIGH)
    so detection works even with no overlays loaded; user-registered
    types extend the set so :func:`load_schema_overlay` can introduce
    fleet-specific layouts.
    """
    base = {LEGACY_TYPE_LOW, LEGACY_TYPE_MED, LEGACY_TYPE_HIGH}
    return base | {key[0] for key in _LEGACY_SCHEMAS}


def parse_trailer(data: bytes) -> TrailerParse:
    """Parse *data* as an F5 Ethernet trailer.

    Returns a :class:`TrailerParse` with ``fmt=None`` if the bytes
    don't look like an F5 trailer at all (the caller should then
    leave them alone).
    """
    if len(data) < 3:
        return TrailerParse(fmt=None)

    # DPT format: starts with the 4-byte magic.
    if len(data) >= DPT_HDR_LEN and struct.unpack(">I", data[:4])[0] == DPT_HDR_MAGIC:
        return _parse_dpt(data)

    # Legacy format: first byte is a known legacy type, total entry
    # length (wire-length + 2 — see _parse_legacy) is sane.
    if data[0] in _legacy_known_types():
        total = data[1] + 2
        if LEGACY_MIN_SANE <= total <= LEGACY_MAX_SANE:
            return _parse_legacy(data)

    return TrailerParse(fmt=None)


def _parse_legacy(data: bytes) -> TrailerParse:
    """Walk a legacy-format trailer chain.

    The on-wire length byte excludes the leading ``[type][length]``
    pair — Wireshark's dissector (``packet-f5ethtrailer.c`` line 2763,
    ``len = tvb_get_uint8(tvb, offset + F5_OFF_LENGTH) + F5_OFF_VERSION``)
    adds 2 back to compute the full entry size.  We do the same so
    schema keys match the Wireshark-defined constants
    (``F5_HIV0_LEN=42``, ``F5_LOWV94_LEN=35``, etc.), which are all
    full entry sizes including the 3-byte type/length/version header.
    """
    out = TrailerParse(fmt="legacy")
    pos = 0
    known_types = _legacy_known_types()
    while pos + 3 <= len(data):
        type_ = data[pos]
        wire_length = data[pos + 1]
        version = data[pos + 2]
        total_length = wire_length + 2
        # Termination conditions (mirroring dissect_old_trailer):
        if type_ not in known_types:
            break
        if total_length < LEGACY_MIN_SANE or total_length > LEGACY_MAX_SANE:
            break
        if pos + total_length > len(data):
            break

        schema = _LEGACY_SCHEMAS.get((type_, version, total_length))
        ip_fields = (
            [IpFieldRef(offset=pos + rel_off, kind=kind) for rel_off, kind in (schema or [])]
            if schema is not None
            else []
        )
        out.tlvs.append(
            ParsedTLV(
                fmt="legacy",
                type_=type_,
                version=version,
                length=total_length,
                offset=pos,
                ip_fields=ip_fields,
                schema_known=schema is not None,
            )
        )
        pos += total_length
    out.bytes_consumed = pos
    return out


def _parse_dpt(data: bytes) -> TrailerParse:
    out = TrailerParse(fmt="dpt")
    if len(data) < DPT_HDR_LEN:
        return out
    total_len = struct.unpack(">H", data[4:6])[0]
    # version field at data[6:8] is the DPT envelope version; ignored
    # here (we only know v1).  Bound the scan by total_len.
    end = min(len(data), total_len)
    pos = DPT_HDR_LEN
    while pos + DPT_TLV_HDR_LEN <= end:
        provider = struct.unpack(">H", data[pos : pos + 2])[0]
        type_ = struct.unpack(">H", data[pos + 2 : pos + 4])[0]
        length = struct.unpack(">H", data[pos + 4 : pos + 6])[0]
        version = struct.unpack(">H", data[pos + 6 : pos + 8])[0]
        if length < DPT_TLV_HDR_LEN or pos + length > end:
            break

        schema = _DPT_SCHEMAS.get((provider, type_, version))
        ip_fields = (
            [IpFieldRef(offset=pos + rel_off, kind=kind) for rel_off, kind in (schema or [])]
            if schema is not None
            else []
        )
        out.tlvs.append(
            ParsedTLV(
                fmt="dpt",
                # Mirror legacy "type" semantics so callers can switch
                # on a single field.  Provider is encoded into a tuple
                # via a small synthetic shift if needed.
                type_=type_,
                version=version,
                length=length,
                offset=pos,
                ip_fields=ip_fields,
                schema_known=schema is not None,
            )
        )
        pos += length
    out.bytes_consumed = pos
    return out


# ── address rewrite helpers ────────────────────────────────────────


def looks_ipv4_mapped(sixteen: bytes) -> bool:
    """True if a 16-byte address is IPv4-mapped (``::ffff:a.b.c.d``)."""
    return len(sixteen) == 16 and sixteen[:12] == IPV4_MAPPED_PREFIX


def looks_f5_route_domain(sixteen: bytes) -> bool:
    """True if a 16-byte address carries an F5 route-domain wrapper.

    Wireshark recognises the prefix ``2620:0:c10:f501:<rd>:<v4>``
    (10 bytes of fixed prefix + 2 bytes route-domain id + 4 bytes IPv4).
    """
    return len(sixteen) == 16 and sixteen[:10] == F5_RTDOM_PREFIX_10[:10]


def classify_v6_or_v4mapped(sixteen: bytes) -> tuple[str, int]:
    """Return ``(kind, ipv4_offset_within_16)`` for a 16-byte field.

    *kind* is ``"v4"`` if the bytes are IPv4-encoded (mapped or with
    an F5 route-domain wrapper), otherwise ``"v6"``.
    """
    if looks_ipv4_mapped(sixteen):
        return "v4", 12
    if looks_f5_route_domain(sixteen):
        return "v4", 12
    return "v6", 0


# ── overlay loader ─────────────────────────────────────────────────


def load_schema_overlay(text: str) -> None:
    """Load an additional schema from a TOML string.

    Format::

        [[legacy]]
        type = 3
        version = 0
        length = 42
        ip_fields = [
            { offset = 6,  kind = "v6_or_v4mapped" },
            { offset = 22, kind = "v6_or_v4mapped" },
        ]

        [[dpt]]
        provider = 1
        type = 3
        version = 1
        ip_fields = [
            { offset = 11, kind = "v6_or_v4mapped" },
            { offset = 27, kind = "v6_or_v4mapped" },
        ]

    Existing entries are overwritten — the caller can use this both
    to add unknown (type, version) combinations and to fix up offsets
    found wrong in the field.
    """
    data = tomllib.loads(text)
    for entry in data.get("legacy", []):
        register_legacy_schema(
            type_=int(entry["type"]),
            version=int(entry["version"]),
            length=int(entry["length"]),
            ip_fields=[(int(f["offset"]), str(f["kind"])) for f in entry.get("ip_fields", [])],
        )
    for entry in data.get("dpt", []):
        register_dpt_schema(
            provider=int(entry["provider"]),
            type_=int(entry["type"]),
            version=int(entry["version"]),
            ip_fields=[(int(f["offset"]), str(f["kind"])) for f in entry.get("ip_fields", [])],
        )


def schema_summary() -> dict[str, list[str]]:
    """Return a human-readable summary of registered schemas."""
    return {
        "legacy": [
            f"type={k[0]} version={k[1]} length={k[2]} fields={len(v)}"
            for k, v in sorted(_LEGACY_SCHEMAS.items())
        ],
        "dpt": [
            f"provider={k[0]} type={k[1]} version={k[2]} fields={len(v)}"
            for k, v in sorted(_DPT_SCHEMAS.items())
        ],
    }


__all__ = [
    "DPT_HDR_LEN",
    "DPT_HDR_MAGIC",
    "DPT_PROVIDER_NOISE",
    "DPT_TLV_HDR_LEN",
    "F5_RTDOM_PREFIX_10",
    "IPV4_MAPPED_PREFIX",
    "IpFieldRef",
    "LEGACY_MAX_SANE",
    "LEGACY_MIN_SANE",
    "LEGACY_TYPE_HIGH",
    "LEGACY_TYPE_LOW",
    "LEGACY_TYPE_MED",
    "ParsedTLV",
    "TrailerParse",
    "classify_v6_or_v4mapped",
    "load_schema_overlay",
    "looks_f5_route_domain",
    "looks_ipv4_mapped",
    "parse_trailer",
    "register_dpt_schema",
    "register_legacy_schema",
    "schema_summary",
]
