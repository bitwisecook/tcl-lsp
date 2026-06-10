"""IPAddress / FQDN value types — the host half of a destination."""

from __future__ import annotations

import ipaddress
from dataclasses import dataclass
from typing import Union


@dataclass(frozen=True, slots=True)
class IPAddress:
    """An IPv4 or IPv6 address with an optional route-domain id.

    Stored as a ``ipaddress.IPv4Address`` or ``IPv6Address`` so all
    classification (loopback / private / link-local / multicast),
    arithmetic, and CIDR membership flows through the stdlib.

    F5 tmsh writes route-domain-qualified addresses as
    ``<host>%<rd>`` — e.g. ``10.0.0.1%10`` is the IP ``10.0.0.1``
    living on route-domain ``10``.  When that suffix is present
    :attr:`route_domain` carries the numeric id; otherwise the
    field is ``None`` (the default Common-partition / RD-0 case).
    The :func:`str` form round-trips the suffix verbatim so
    ``str(IPAddress.parse("10.0.0.1%10")) == "10.0.0.1%10"``.
    """

    addr: ipaddress.IPv4Address | ipaddress.IPv6Address
    # Optional ``%RD`` suffix.  ``None`` for default-RD (Common /
    # route-domain 0).  Stored as an int because tmsh only writes
    # numeric route-domain ids — distinct from the IPv6 zone-id
    # form ``fe80::1%eth0`` which Python's ``ipaddress`` rejects
    # outright.
    route_domain: int | None = None

    @classmethod
    def parse(cls, text: str) -> "IPAddress":
        """Parse *text* as an IPv4 or IPv6 host address with an
        optional ``%<rd>`` route-domain suffix.

        Raises :class:`ValueError` for invalid input.  Wildcard
        spellings (``"any"`` / ``"*"``) are NOT accepted here —
        :class:`Destination` handles those at the higher level.
        """
        text = text.strip()
        rd: int | None = None
        if "%" in text:
            host, _, rd_text = text.rpartition("%")
            if rd_text.isdigit():
                rd = int(rd_text)
                text = host
            # ``rd_text`` non-numeric (``fe80::1%eth0``) — leave
            # *text* intact and let ``ipaddress`` reject it
            # naturally, matching the original behaviour.
        return cls(addr=ipaddress.ip_address(text), route_domain=rd)

    @classmethod
    def try_parse(cls, text: str) -> "IPAddress | None":
        """Same as :meth:`parse` but returns ``None`` instead of raising."""
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def is_ipv4(self) -> bool:
        return isinstance(self.addr, ipaddress.IPv4Address)

    @property
    def is_ipv6(self) -> bool:
        return isinstance(self.addr, ipaddress.IPv6Address)

    @property
    def is_loopback(self) -> bool:
        return self.addr.is_loopback

    @property
    def is_private(self) -> bool:
        return self.addr.is_private

    @property
    def is_unspecified(self) -> bool:
        """``0.0.0.0`` / ``::`` — F5 uses these as listen-on-any wildcards."""
        return self.addr.is_unspecified

    @property
    def is_multicast(self) -> bool:
        """RFC 1112 (v4) / RFC 4291 (v6) multicast range."""
        return self.addr.is_multicast

    @property
    def is_link_local(self) -> bool:
        """RFC 3927 (169.254.0.0/16) and RFC 4291 (fe80::/10) link-local."""
        return self.addr.is_link_local

    @property
    def is_reserved(self) -> bool:
        """IANA-reserved range (240.0.0.0/4 et al, plus various v6).

        Distinct from :attr:`is_private` — reserved is "set aside by
        IANA, no current use" while private is "carved out for
        intra-network use".
        """
        return self.addr.is_reserved

    @property
    def is_public(self) -> bool:
        """Globally routable as a *unicast* address on the public
        internet.

        Stricter than :pyattr:`ipaddress.IPv4Address.is_global`:
        also rejects multicast, link-local, loopback,
        unspecified, and reserved ranges.  Use this to decide
        "should we ever expect to see this address on the outside
        as a routable unicast target?" without spelling out every
        negation by hand.

        Note: ``is_global`` in the stdlib reports multicast as
        global because multicast isn't IETF-private; that's
        semantically correct but rarely what an audit query
        means by "public".
        """
        return (
            self.addr.is_global
            and not self.addr.is_multicast
            and not self.addr.is_link_local
            and not self.addr.is_loopback
            and not self.addr.is_unspecified
            and not self.addr.is_reserved
        )

    @property
    def is_documentation(self) -> bool:
        """RFC 5737 / RFC 3849 documentation-example range
        (``192.0.2.0/24``, ``198.51.100.0/24``, ``203.0.113.0/24``,
        ``2001:db8::/32``).  Configs that pin these are almost
        always lab / template / dummy values, not production."""
        # ``ipaddress`` exposes ``is_global`` and ``is_private``
        # but not a dedicated documentation predicate; check the
        # canonical ranges directly.
        if isinstance(self.addr, ipaddress.IPv4Address):
            doc_ranges = (
                ipaddress.ip_network("192.0.2.0/24"),
                ipaddress.ip_network("198.51.100.0/24"),
                ipaddress.ip_network("203.0.113.0/24"),
            )
        else:
            doc_ranges = (ipaddress.ip_network("2001:db8::/32"),)
        return any(self.addr in net for net in doc_ranges)

    def __str__(self) -> str:
        if self.route_domain is None:
            return self.addr.compressed
        return f"{self.addr.compressed}%{self.route_domain}"


@dataclass(frozen=True, slots=True)
class FQDN:
    """A fully-qualified domain name used by FQDN-based pool members.

    F5 pool members can be ``ltm pool /Common/p { members {
    /Common/host.example.com:443 { fqdn { name host.example.com } } } }``;
    the ``host.example.com`` part lands here.

    The parser is conservative: it accepts dotted labels of letters /
    digits / hyphens (per RFC 1035), refuses leading or trailing
    hyphens / dots, and refuses anything that looks like an IPv4
    address (so callers can disambiguate against :class:`IPAddress`
    by trying ``IPAddress.try_parse`` first).
    """

    name: str

    @classmethod
    def parse(cls, text: str) -> "FQDN":
        """Parse *text* as an FQDN.  Raises :class:`ValueError` if invalid."""
        text = text.strip()
        if not text:
            raise ValueError("FQDN: empty input")
        if text.startswith(".") or text.endswith("."):
            raise ValueError(f"FQDN: leading/trailing dot in {text!r}")
        labels = text.split(".")
        if len(labels) < 2:
            raise ValueError(f"FQDN: must have at least one dot ({text!r})")
        for label in labels:
            if not label:
                raise ValueError(f"FQDN: empty label in {text!r}")
            if len(label) > 63:
                raise ValueError(f"FQDN: label longer than 63 chars in {text!r}")
            if label.startswith("-") or label.endswith("-"):
                raise ValueError(f"FQDN: label leading/trailing hyphen in {text!r}")
            if not all(c.isalnum() or c == "-" for c in label):
                raise ValueError(f"FQDN: invalid char in label of {text!r}")
        # Refuse a value that's actually an IPv4 address (all-numeric
        # labels with three dots).  Callers should try :class:`IPAddress`
        # first — and our :meth:`Address.parse` does exactly that.
        if all(label.isdigit() for label in labels) and len(labels) == 4:
            raise ValueError(f"FQDN: looks like an IPv4 address ({text!r})")
        return cls(name=text)

    @classmethod
    def try_parse(cls, text: str) -> "FQDN | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    def __str__(self) -> str:
        return self.name


# Sum type for "the host part of a pool member or other address-bearing
# field".  IP addresses are tried first because they're more constrained;
# anything that doesn't parse as an IP falls through to FQDN.
Address = Union[IPAddress, FQDN]


def parse_address(text: str) -> Address:
    """Parse *text* as either an :class:`IPAddress` or :class:`FQDN`.

    Raises :class:`ValueError` if the input is neither.
    """
    text = text.strip()
    ip = IPAddress.try_parse(text)
    if ip is not None:
        return ip
    return FQDN.parse(text)


def try_parse_address(text: str) -> Address | None:
    """Like :func:`parse_address` but returns ``None`` on failure."""
    try:
        return parse_address(text)
    except (ValueError, TypeError):
        return None
