"""Network value type — IPv4 or IPv6 CIDR."""

from __future__ import annotations

import ipaddress
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Network:
    """An IPv4 or IPv6 network in CIDR notation.

    Wraps ``ipaddress.IPv4Network`` / ``IPv6Network`` so all
    membership checks and supernet / subnet operations flow through
    the stdlib.  Always stored in *canonical* form (host bits
    masked out); accept loose input (host bits set) and normalise.
    """

    network: ipaddress.IPv4Network | ipaddress.IPv6Network

    @classmethod
    def parse(cls, text: str, *, strict: bool = False) -> "Network":
        """Parse *text* as a CIDR network.

        Accepts both spellings F5 emits:

        - **Integer CIDR**: ``10.0.0.0/24`` or ``2001:db8::/32`` —
          the standard prefix-length form.
        - **Dotted-quad netmask** (IPv4 only): ``10.0.0.0/255.255.255.0``
          or ``10.0.0.0 255.255.255.0`` (space-separated, the form
          ``net route`` and ``net self`` use in some configs).

        ``strict=True`` rejects input with host bits set; the default
        accepts ``10.0.0.5/24`` and normalises to ``10.0.0.0/24``.
        """
        text = text.strip()
        # Space-separated ``ADDR MASK`` form (BIG-IP routing tables).
        if " " in text and "/" not in text:
            addr_part, _, mask_part = text.partition(" ")
            text = f"{addr_part.strip()}/{mask_part.strip()}"
        # ``ipaddress`` accepts ``10.0.0.0/255.255.255.0`` natively
        # for IPv4, so the integer-CIDR and dotted-quad forms both
        # flow through the same call after normalising the separator.
        return cls(network=ipaddress.ip_network(text, strict=strict))

    @classmethod
    def try_parse(cls, text: str, *, strict: bool = False) -> "Network | None":
        try:
            return cls.parse(text, strict=strict)
        except (ValueError, TypeError):
            return None

    @property
    def is_ipv4(self) -> bool:
        return isinstance(self.network, ipaddress.IPv4Network)

    @property
    def is_ipv6(self) -> bool:
        return isinstance(self.network, ipaddress.IPv6Network)

    @property
    def prefix_length(self) -> int:
        return self.network.prefixlen

    def __contains__(self, other: object) -> bool:
        """``ip in network`` membership; accepts :class:`IPAddress` or stdlib types."""
        if isinstance(other, Network):
            return other.network.subnet_of(self.network)
        # Avoid circular import — IPAddress also lives in this package.
        from ._address import IPAddress

        if isinstance(other, IPAddress):
            return other.addr in self.network
        if isinstance(other, (ipaddress.IPv4Address, ipaddress.IPv6Address)):
            return other in self.network
        return False

    def __str__(self) -> str:
        return self.network.with_prefixlen
