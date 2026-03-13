"""IPv4 and IPv6 address utilities for diagnostics and hover.

Uses the Python ``ipaddress`` standard library module for parsing and
classification, with helper functions tailored to Tcl/iRules/BIG-IP use.
"""

from __future__ import annotations

import ipaddress
import re
from dataclasses import dataclass
from enum import Enum, auto

# Matches dotted-quad IPv4 addresses, optionally with /prefix.
IPV4_RE = re.compile(
    r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})(?:/(\d{1,2}))?\b",
)

# Matches IPv6 addresses (simplified — delegates real validation to stdlib).
# Covers full, compressed, and IPv4-mapped forms.
IPV6_RE = re.compile(
    r"(?<![:\w])"  # negative lookbehind: not preceded by colon or word char
    r"("
    r"(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}"  # full
    r"|(?:[0-9a-fA-F]{1,4}:){1,7}:"  # trailing ::
    r"|:(?::[0-9a-fA-F]{1,4}){1,7}"  # leading ::
    r"|(?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}"  # :: in middle
    r"|(?:[0-9a-fA-F]{1,4}:){1,5}(?::[0-9a-fA-F]{1,4}){1,2}"
    r"|(?:[0-9a-fA-F]{1,4}:){1,4}(?::[0-9a-fA-F]{1,4}){1,3}"
    r"|(?:[0-9a-fA-F]{1,4}:){1,3}(?::[0-9a-fA-F]{1,4}){1,4}"
    r"|(?:[0-9a-fA-F]{1,4}:){1,2}(?::[0-9a-fA-F]{1,4}){1,5}"
    r"|[0-9a-fA-F]{1,4}:(?::[0-9a-fA-F]{1,4}){1,6}"
    r"|::(?:ffff(?::0{1,4})?:)?(?:\d{1,3}\.){3}\d{1,3}"  # ::ffff:d.d.d.d
    r"|(?:[0-9a-fA-F]{1,4}:){1,4}:(?:\d{1,3}\.){3}\d{1,3}"  # mixed
    r"|::"  # unspecified
    r")"
    r"(?:/(\d{1,3}))?"  # optional /prefix
    r"(?![:\w])",  # negative lookahead
)


class IPClass(Enum):
    """High-level classification of an IP address."""

    LOOPBACK = auto()
    PRIVATE = auto()
    LINK_LOCAL = auto()
    MULTICAST = auto()
    RESERVED = auto()
    DOCUMENTATION = auto()
    CARRIER_GRADE_NAT = auto()
    UNSPECIFIED = auto()
    BROADCAST = auto()
    PUBLIC = auto()


@dataclass(frozen=True, slots=True)
class IPInfo:
    """Parsed information about an IP address or network."""

    address: ipaddress.IPv4Address | ipaddress.IPv6Address
    network: ipaddress.IPv4Network | ipaddress.IPv6Network | None
    version: int  # 4 or 6
    classification: IPClass
    rfc: str  # primary RFC reference
    rfc_title: str  # short RFC description
    is_subnet_mask: bool  # True if valid IPv4 subnet mask
    cidr_prefix: int | None  # CIDR prefix length (from subnet mask or /prefix notation)
    ipv4_mapped: ipaddress.IPv4Address | None  # mapped IPv4 if IPv6 v4-mapped
    ipv6_mapped: ipaddress.IPv6Address | None  # v6-mapped form if IPv4


def _classify_ipv4(addr: ipaddress.IPv4Address) -> tuple[IPClass, str, str]:
    """Classify an IPv4 address and return (class, rfc, title)."""
    if addr.is_loopback:
        return IPClass.LOOPBACK, "RFC 1122", "Loopback"
    if addr.is_link_local:
        return IPClass.LINK_LOCAL, "RFC 3927", "Link-Local"
    if addr.is_multicast:
        return IPClass.MULTICAST, "RFC 5771", "Multicast"
    if addr.is_unspecified:
        return IPClass.UNSPECIFIED, "RFC 1122", "Unspecified"

    packed = int(addr)

    # Broadcast (255.255.255.255) — check before is_reserved
    if packed == 0xFFFFFFFF:
        return IPClass.BROADCAST, "RFC 919", "Limited Broadcast"

    if addr.is_reserved:
        return IPClass.RESERVED, "RFC 1112", "Reserved"

    # RFC 1918 private ranges
    # 10.0.0.0/8
    if (packed >> 24) == 10:
        return IPClass.PRIVATE, "RFC 1918", "Private (10.0.0.0/8)"
    # 172.16.0.0/12
    if (packed >> 20) == 0xAC1:
        return IPClass.PRIVATE, "RFC 1918", "Private (172.16.0.0/12)"
    # 192.168.0.0/16
    if (packed >> 16) == 0xC0A8:
        return IPClass.PRIVATE, "RFC 1918", "Private (192.168.0.0/16)"

    # Carrier-grade NAT (100.64.0.0/10)
    if (packed >> 22) == 0x191:
        return IPClass.CARRIER_GRADE_NAT, "RFC 6598", "Shared Address / CGN (100.64.0.0/10)"

    # Documentation ranges
    # 192.0.2.0/24 (TEST-NET-1)
    if (packed >> 8) == 0xC00002:
        return IPClass.DOCUMENTATION, "RFC 5737", "Documentation (TEST-NET-1)"
    # 198.51.100.0/24 (TEST-NET-2)
    if (packed >> 8) == 0xC63364:
        return IPClass.DOCUMENTATION, "RFC 5737", "Documentation (TEST-NET-2)"
    # 203.0.113.0/24 (TEST-NET-3)
    if (packed >> 8) == 0xCB0071:
        return IPClass.DOCUMENTATION, "RFC 5737", "Documentation (TEST-NET-3)"

    return IPClass.PUBLIC, "", "Public"


def _classify_ipv6(addr: ipaddress.IPv6Address) -> tuple[IPClass, str, str]:
    """Classify an IPv6 address and return (class, rfc, title)."""
    if addr.is_loopback:
        return IPClass.LOOPBACK, "RFC 4291", "Loopback (::1)"
    if addr.is_link_local:
        return IPClass.LINK_LOCAL, "RFC 4291", "Link-Local (fe80::/10)"
    if addr.is_multicast:
        return IPClass.MULTICAST, "RFC 4291", "Multicast (ff00::/8)"
    if addr.is_unspecified:
        return IPClass.UNSPECIFIED, "RFC 4291", "Unspecified (::)"
    if addr.is_reserved:
        return IPClass.RESERVED, "RFC 4291", "Reserved"

    packed = int(addr)

    # Unique Local Address (fc00::/7) — RFC 4193
    if (packed >> 121) == 0x7E or (packed >> 121) == 0x7F:
        return IPClass.PRIVATE, "RFC 4193", "Unique Local Address (fc00::/7)"

    # Documentation (2001:db8::/32) — RFC 3849
    if (packed >> 96) == 0x20010DB8:
        return IPClass.DOCUMENTATION, "RFC 3849", "Documentation (2001:db8::/32)"

    # 6to4 (2002::/16)
    if (packed >> 112) == 0x2002:
        return IPClass.PUBLIC, "RFC 3056", "6to4 Relay (2002::/16)"

    return IPClass.PUBLIC, "", "Public"


def _is_valid_subnet_mask_int(val: int) -> bool:
    """Return True if the 32-bit integer is a valid contiguous subnet mask."""
    if val == 0:
        return True
    inverted = val ^ 0xFFFFFFFF
    return (inverted & (inverted + 1)) == 0


def _cidr_from_mask_int(val: int) -> int:
    """Return the CIDR prefix length from a valid 32-bit mask."""
    if val == 0:
        return 0
    inverted = val ^ 0xFFFFFFFF
    return 32 - inverted.bit_length()


def parse_ipv4(text: str) -> IPInfo | None:
    """Parse an IPv4 address string and return classification info."""
    try:
        addr = ipaddress.IPv4Address(text)
    except (ipaddress.AddressValueError, ValueError):
        return None

    classification, rfc, rfc_title = _classify_ipv4(addr)
    packed = int(addr)

    is_mask = _is_valid_subnet_mask_int(packed)
    cidr = _cidr_from_mask_int(packed) if is_mask else None

    # Generate IPv4-mapped IPv6 form
    ipv6_mapped = ipaddress.IPv6Address(f"::ffff:{addr}")

    return IPInfo(
        address=addr,
        network=None,
        version=4,
        classification=classification,
        rfc=rfc,
        rfc_title=rfc_title,
        is_subnet_mask=is_mask,
        cidr_prefix=cidr,
        ipv4_mapped=None,
        ipv6_mapped=ipv6_mapped,
    )


def parse_ipv6(text: str) -> IPInfo | None:
    """Parse an IPv6 address string and return classification info."""
    try:
        addr = ipaddress.IPv6Address(text)
    except (ipaddress.AddressValueError, ValueError):
        return None

    classification, rfc, rfc_title = _classify_ipv6(addr)

    # Check for IPv4-mapped (::ffff:x.x.x.x)
    ipv4_mapped: ipaddress.IPv4Address | None = None
    if addr.ipv4_mapped is not None:
        ipv4_mapped = addr.ipv4_mapped

    return IPInfo(
        address=addr,
        network=None,
        version=6,
        classification=classification,
        rfc=rfc,
        rfc_title=rfc_title,
        is_subnet_mask=False,
        cidr_prefix=None,
        ipv4_mapped=ipv4_mapped,
        ipv6_mapped=None,
    )


def parse_ip(text: str) -> IPInfo | None:
    """Parse either an IPv4 or IPv6 address and return classification info.

    Tries IPv4 first (most common in Tcl/iRules), then IPv6.
    Strips optional /prefix before parsing the address.
    """
    # Strip /prefix for address parsing
    addr_text = text.split("/")[0] if "/" in text else text
    prefix_len: int | None = None
    if "/" in text:
        try:
            prefix_len = int(text.split("/")[1])
        except (ValueError, IndexError):
            pass

    info = parse_ipv4(addr_text)
    if info is None:
        info = parse_ipv6(addr_text)
    if info is None:
        return None

    # Attach network if /prefix was given
    if prefix_len is not None:
        try:
            net = ipaddress.ip_network(f"{addr_text}/{prefix_len}", strict=False)
            info = IPInfo(
                address=info.address,
                network=net,
                version=info.version,
                classification=info.classification,
                rfc=info.rfc,
                rfc_title=info.rfc_title,
                is_subnet_mask=info.is_subnet_mask,
                cidr_prefix=prefix_len,
                ipv4_mapped=info.ipv4_mapped,
                ipv6_mapped=info.ipv6_mapped,
            )
        except (ValueError, ipaddress.AddressValueError):
            pass

    return info


def ipv4_to_ipv6_mapped(addr: ipaddress.IPv4Address) -> str:
    """Convert an IPv4 address to its IPv4-mapped IPv6 representation."""
    return str(ipaddress.IPv6Address(f"::ffff:{addr}"))


def ipv6_mapped_to_ipv4(addr: ipaddress.IPv6Address) -> str | None:
    """Convert an IPv4-mapped IPv6 address back to plain IPv4, or None."""
    if addr.ipv4_mapped is not None:
        return str(addr.ipv4_mapped)
    return None


def format_ip_hover(info: IPInfo) -> str:
    """Format an IPInfo into markdown hover text."""
    parts: list[str] = []

    if info.version == 4:
        parts.append(f"**IPv4:** `{info.address}`")
    else:
        parts.append(f"**IPv6:** `{info.address}`")

    # Network info
    if info.network is not None:
        parts.append(f"**Network:** `{info.network}`")

    # Classification
    label = info.classification.name.replace("_", " ").title()
    if info.rfc:
        parts.append(f"**Class:** {info.rfc_title} ({info.rfc})")
    else:
        parts.append(f"**Class:** {label}")

    # Subnet mask info
    if info.is_subnet_mask and info.cidr_prefix is not None:
        parts.append(f"**Subnet mask:** /{info.cidr_prefix}")

    # IPv4-mapped info
    if info.ipv4_mapped is not None:
        parts.append(f"**IPv4 equivalent:** `{info.ipv4_mapped}`")
    if info.ipv6_mapped is not None:
        parts.append(f"**IPv6 mapped:** `{info.ipv6_mapped}`")

    # Binary representation for IPv4
    if info.version == 4:
        packed = int(info.address)
        octets = [
            f"{(packed >> 24) & 0xFF:08b}",
            f"{(packed >> 16) & 0xFF:08b}",
            f"{(packed >> 8) & 0xFF:08b}",
            f"{packed & 0xFF:08b}",
        ]
        parts.append(f"**Binary:** `{'.'.join(octets)}`")

    return "\n\n".join(parts)


def validate_ipv4_octets(text: str) -> str | None:
    """Validate an IPv4 dotted-quad and return an error message if invalid.

    Checks:
    - Octets > 255
    - Leading zeros (octal ambiguity)
    """
    parts = text.split(".")
    if len(parts) != 4:
        return None

    for i, part in enumerate(parts):
        if not part:
            return f"Octet {i + 1} is empty"
        if not part.isdigit():
            return None  # not a plain dotted-quad

        val = int(part)
        if val > 255:
            return f"Octet {i + 1} ({part}) exceeds 255"

        if len(part) > 1 and part[0] == "0" and all(c in "01234567" for c in part):
            return (
                f"Octet {i + 1} ({part}) has a leading zero "
                "— may be interpreted as octal in some contexts"
            )

    return None
