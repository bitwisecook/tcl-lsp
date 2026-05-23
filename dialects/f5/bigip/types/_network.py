"""Network value type — IPv4 or IPv6 CIDR."""

from __future__ import annotations

import ipaddress
from dataclasses import dataclass

_DEFAULT_V4 = ipaddress.ip_network("0.0.0.0/0")
_DEFAULT_V6 = ipaddress.ip_network("::/0")


@dataclass(frozen=True, slots=True)
class Network:
    """An IPv4 or IPv6 network in CIDR notation.

    Wraps ``ipaddress.IPv4Network`` / ``IPv6Network`` so all
    membership checks and supernet / subnet operations flow through
    the stdlib.  The stdlib value lives in *canonical* form (host
    bits masked out); we preserve the original textual host-bit
    spelling on ``original`` so things like ``net self`` addresses
    (``203.0.113.5/24``) round-trip without losing the interface
    address.  The ``default`` keyword (BIG-IP shorthand for
    ``0.0.0.0/0`` in route tables) is preserved as-is in ``original``.
    """

    network: ipaddress.IPv4Network | ipaddress.IPv6Network
    # Original textual spelling (e.g. ``203.0.113.5/24`` with host
    # bits set, or the literal keyword ``default``).  ``str()`` and
    # the projection layer surface this when present; equality /
    # containment / supernet checks still flow through ``network``.
    original: str = ""

    @classmethod
    def parse(cls, text: str, *, strict: bool = False) -> "Network":
        """Parse *text* as a CIDR network.

        Accepts every spelling F5 emits:

        - **Integer CIDR**: ``10.0.0.0/24`` or ``2001:db8::/32`` —
          the standard prefix-length form.
        - **Dotted-quad netmask** (IPv4 only): ``10.0.0.0/255.255.255.0``
          or ``10.0.0.0 255.255.255.0`` (space-separated, the form
          ``net route`` and ``net self`` use in some configs).
        - **``default`` keyword** — shorthand for the default route
          (``0.0.0.0/0`` for v4 contexts, ``::/0`` for v6 contexts;
          we resolve to v4 by convention since the keyword is more
          common there).  ``original`` keeps the literal so it
          re-emits as ``default`` rather than ``0.0.0.0/0``.

        ``strict=True`` rejects input with host bits set; the
        default accepts ``10.0.0.5/24`` and keeps the textual
        spelling on ``original`` while normalising the stored
        :class:`ipaddress.IPv4Network` to ``10.0.0.0/24``.
        """
        original = text.strip()
        text = original
        if text == "default":
            return cls(network=_DEFAULT_V4, original="default")
        if text == "default-inet6":
            return cls(network=_DEFAULT_V6, original="default-inet6")
        # Space-separated ``ADDR MASK`` form (BIG-IP routing tables).
        if " " in text and "/" not in text:
            addr_part, _, mask_part = text.partition(" ")
            text = f"{addr_part.strip()}/{mask_part.strip()}"
        # ``ipaddress`` accepts ``10.0.0.0/255.255.255.0`` natively
        # for IPv4, so the integer-CIDR and dotted-quad forms both
        # flow through the same call after normalising the separator.
        return cls(network=ipaddress.ip_network(text, strict=strict), original=original)

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

    @property
    def is_default(self) -> bool:
        """True when the original spelling was the ``default`` keyword.

        Useful to ``net route`` consumers that want to distinguish a
        BIG-IP default route from an explicit ``0.0.0.0/0`` stanza
        even though both resolve to the same network.
        """
        return self.original in ("default", "default-inet6")

    # ------------------------------------------------------------------
    # Classification predicates.
    #
    # All delegate to the stdlib ``ipaddress`` network attributes,
    # which classify a *whole network* as belonging to a category
    # when its address range falls entirely inside one of the
    # reserved CIDRs IANA / the RFCs publish.  This is the natural
    # CIDR-level analogue of the per-address predicates on
    # :class:`IPAddress`.
    # ------------------------------------------------------------------

    @property
    def is_private(self) -> bool:
        """RFC 1918 / RFC 4193 / link-local / reserved private range."""
        return self.network.is_private

    @property
    def is_public(self) -> bool:
        """Globally routable *unicast* network on the public
        internet.

        Stricter than :pyattr:`ipaddress.IPv4Network.is_global`:
        excludes multicast, link-local, loopback, unspecified,
        and reserved ranges (the stdlib counts multicast as
        global because it isn't IETF-private — semantically
        correct but rarely what an audit means by "public").
        """
        return (
            self.network.is_global
            and not self.network.is_multicast
            and not self.network.is_link_local
            and not self.network.is_loopback
            and not self.network.is_unspecified
            and not self.network.is_reserved
        )

    @property
    def is_loopback(self) -> bool:
        return self.network.is_loopback

    @property
    def is_link_local(self) -> bool:
        return self.network.is_link_local

    @property
    def is_multicast(self) -> bool:
        return self.network.is_multicast

    @property
    def is_reserved(self) -> bool:
        return self.network.is_reserved

    @property
    def is_unspecified(self) -> bool:
        return self.network.is_unspecified

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
        # Preserve the user's original spelling when we have it —
        # this keeps ``net self`` interface addresses (``203.0.113.5/24``)
        # and routing-table ``default`` keywords round-tripping
        # without losing host bits or the keyword form.
        if self.original:
            return self.original
        return self.network.with_prefixlen
