"""IPRange value type — arbitrary first..last IP ranges.

Represents an inclusive range of IPv4 or IPv6 addresses that may
not align with any single CIDR.  Parses ``"192.168.9.77-192.168.9.83"``-
style ``low-high`` spans.  Use :meth:`to_cidrs` to summarise as the
minimum CIDR set covering the range, or :meth:`as_supernet` for the
smallest single enclosing CIDR.
"""

from __future__ import annotations

import ipaddress
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class IPRange:
    """Inclusive ``[first, last]`` range of IPv4 or IPv6 addresses.

    Stored as the two endpoint :class:`ipaddress` values so range
    arithmetic delegates to the stdlib.  ``first`` and ``last``
    must agree on address family (both v4 or both v6).
    """

    first: ipaddress.IPv4Address | ipaddress.IPv6Address
    last: ipaddress.IPv4Address | ipaddress.IPv6Address

    def __post_init__(self) -> None:
        if type(self.first) is not type(self.last):
            raise ValueError(
                f"IPRange: mixed address families ({type(self.first).__name__} / "
                f"{type(self.last).__name__})"
            )
        if int(self.first) > int(self.last):
            raise ValueError(f"IPRange: first > last ({self.first} > {self.last})")

    @classmethod
    def parse(cls, text: str) -> "IPRange":
        """Parse ``"first-last"`` (or a single address, which yields
        a one-IP range).

        Raises :class:`ValueError` for malformed input or mixed
        IPv4/IPv6 endpoints.
        """
        text = (text or "").strip()
        if not text:
            raise ValueError("IPRange: empty input")
        if "-" not in text:
            single = ipaddress.ip_address(text)
            return cls(first=single, last=single)
        # Split on the LAST hyphen so IPv6 addresses with internal
        # dashes (which aren't legal, but be defensive) don't
        # misparse.  IPv6 endpoints never contain ``-`` so this is
        # safe.
        lo_text, _, hi_text = text.rpartition("-")
        lo = ipaddress.ip_address(lo_text.strip())
        hi = ipaddress.ip_address(hi_text.strip())
        return cls(first=lo, last=hi)

    @classmethod
    def try_parse(cls, text: str) -> "IPRange | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def is_ipv4(self) -> bool:
        return isinstance(self.first, ipaddress.IPv4Address)

    @property
    def is_ipv6(self) -> bool:
        return isinstance(self.first, ipaddress.IPv6Address)

    @property
    def count(self) -> int:
        """Number of addresses in the range (inclusive)."""
        return int(self.last) - int(self.first) + 1

    def contains(self, addr: object) -> bool:
        """``True`` when *addr* lies inside the range.  Accepts
        :class:`IPAddress`, stdlib v4/v6 address types, or a string."""
        # Avoid circular import on IPAddress.
        from ._address import IPAddress

        candidate: ipaddress.IPv4Address | ipaddress.IPv6Address | None = None
        if isinstance(addr, IPAddress):
            candidate = addr.addr
        elif isinstance(addr, (ipaddress.IPv4Address, ipaddress.IPv6Address)):
            candidate = addr
        elif isinstance(addr, str):
            try:
                candidate = ipaddress.ip_address(addr)
            except ValueError:
                return False
        if candidate is None:
            return False
        if type(candidate) is not type(self.first):
            return False
        return int(self.first) <= int(candidate) <= int(self.last)

    def overlaps(self, other: "IPRange") -> bool:
        """``True`` when *self* and *other* share at least one
        address.  Ranges of different families never overlap."""
        if self.is_ipv4 != other.is_ipv4:
            return False
        return not (int(self.last) < int(other.first) or int(other.last) < int(self.first))

    def to_cidrs(self) -> list[ipaddress.IPv4Network | ipaddress.IPv6Network]:
        """Decompose the range into the minimum set of CIDR
        networks that exactly covers it.

        Wraps :func:`ipaddress.summarize_address_range`.  Useful
        for converting a free-form range into firewall-rule
        ``address-list`` entries.
        """
        return list(ipaddress.summarize_address_range(self.first, self.last))

    def as_supernet(self) -> ipaddress.IPv4Network | ipaddress.IPv6Network:
        """The smallest single CIDR that wholly contains the
        range.  Always at least as large as the range itself —
        may include addresses outside the original ``[first, last]``
        endpoints.
        """
        cidrs = self.to_cidrs()
        if len(cidrs) == 1:
            return cidrs[0]
        # Iteratively widen until one network covers both endpoints.
        family_max = 32 if self.is_ipv4 else 128
        for prefix in range(family_max, -1, -1):
            net = ipaddress.ip_network((int(self.first), prefix), strict=False)
            if int(net.network_address) <= int(self.first) and int(net.broadcast_address) >= int(
                self.last
            ):
                return net
        # Theoretically unreachable: ``/0`` always covers everything.
        return ipaddress.ip_network("0.0.0.0/0") if self.is_ipv4 else ipaddress.ip_network("::/0")

    def __str__(self) -> str:
        if self.first == self.last:
            return str(self.first)
        return f"{self.first}-{self.last}"
