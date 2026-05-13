"""Destination value type — F5 ``[partition/]address[%route-domain][:port]`` triples.

This is the gnarly one.  F5 destination syntax is the union of:

- bare IPv4 with port:                    ``10.0.0.1:80``
- bare IPv6 with ``.``-port (no brackets):``2001:db8::1.80``
- bracketed IPv6 with ``.``-port:         ``[2001:db8::1].80``
- bracketed IPv6 with ``:``-port:         ``[2001:db8::1]:80``
- partition-prefixed:                     ``/Common/10.0.0.1:80``
- partition-prefixed bracketed IPv6:      ``/Common/[2001:db8::1].80``
- IPv4 with route-domain:                 ``10.0.0.1%5:80``
- IPv6 with route-domain:                 ``2001:db8::1%5.80``
- wildcards:                              ``0.0.0.0:any`` / ``::.0`` / ``*.*``

Pool members add FQDN spellings:

- bare FQDN with port:                    ``host.example.com:443``
- partition-prefixed FQDN:                ``/Common/host.example.com:443``

The parser is hand-written, not regex-driven — it splits the input
on the structural anchors (``/``, ``[``, ``]``, ``%``, the
*last* ``.`` or ``:`` for the port separator) and delegates each
piece to its dedicated value type's parser.

All seven variants round-trip: ``str(Destination.parse(s)) == s``
when the input is already in canonical form (one of the spellings
above).  Loose input (extra whitespace, missing port) is normalised
to canonical on render.
"""

from __future__ import annotations

from dataclasses import dataclass

from ._address import Address, IPAddress, parse_address
from ._partition import Partition
from ._port import Port
from ._route_domain import RouteDomain


@dataclass(frozen=True, slots=True)
class Destination:
    """An F5 destination triple: optional partition, address, optional
    route-domain, port.

    Every field except ``address`` and ``port`` is optional; the
    canonical form drops them when absent.  When present they
    serialise as ``<partition>/<address>%<rd>:<port>`` (with the
    appropriate IPv6 bracketing / dot-port separator).
    """

    address: Address
    port: Port
    partition: Partition | None = None
    route_domain: RouteDomain | None = None
    # ``ipv6_brackets`` records whether the input wrapped an IPv6
    # address in ``[...]`` so the canonical render preserves it.
    ipv6_brackets: bool = False
    # ``port_separator`` records ``"."`` vs ``":"`` for IPv6
    # destinations (both are valid F5 spellings).
    port_separator: str = ":"

    @classmethod
    def parse(cls, text: str) -> "Destination":
        """Parse *text* as an F5 destination.

        Raises :class:`ValueError` for unparseable input.  Pool-member
        ``host.example.com:443`` style FQDNs are accepted via
        :func:`parse_address`'s fallback.
        """
        text = text.strip()
        if not text:
            raise ValueError("Destination: empty input")

        rest = text

        # Partition prefix: ``/Common/...``.  The first ``/`` is the
        # separator, the second ``/`` ends the partition name.
        partition: Partition | None = None
        if rest.startswith("/") and rest.count("/") >= 2:
            # Split at the second ``/``: rest[0]='/' rest[1..k-1]=name rest[k]='/'.
            second_slash = rest.find("/", 1)
            if second_slash > 1:
                part_text = rest[:second_slash]
                rest = rest[second_slash + 1 :]
                partition = Partition.parse(part_text)

        # IPv6 in brackets.  ``[ADDR]`` followed by ``:port`` or ``.port``.
        ipv6_brackets = False
        port_separator = ":"
        if rest.startswith("["):
            close = rest.find("]")
            if close == -1:
                raise ValueError(f"Destination: unmatched '[' in {text!r}")
            addr_text = rest[1:close]
            port_part = rest[close + 1 :]
            ipv6_brackets = True
            address, route_domain, _ = _split_route_domain(addr_text)
            ip = IPAddress.try_parse(address)
            if ip is None:
                raise ValueError(f"Destination: bracketed value isn't a valid IP ({addr_text!r})")
            address_value: Address = ip
            if not port_part:
                # ``[ADDR]`` with no port — accept as port=any.
                return cls(
                    address=address_value,
                    port=Port(port=0, spelling=""),
                    partition=partition,
                    route_domain=route_domain,
                    ipv6_brackets=True,
                )
            if port_part[0] not in (":", "."):
                raise ValueError(f"Destination: missing port separator after ']' in {text!r}")
            port_separator = port_part[0]
            port = Port.parse(port_part[1:])
            return cls(
                address=address_value,
                port=port,
                partition=partition,
                route_domain=route_domain,
                ipv6_brackets=True,
                port_separator=port_separator,
            )

        # No brackets — split address from port.  IPv6 without brackets
        # uses ``.`` as the port separator (because ``:`` is part of
        # the address); IPv4 / FQDN use ``:``.  Detect by counting
        # colons in *rest* before the route-domain.
        addr_part, route_domain, route_domain_text = _split_route_domain(rest)
        # If there are 2+ colons in *addr_part*, it's an unbracketed
        # IPv6 address; the port separator is the LAST ``.``.
        if addr_part.count(":") >= 2:
            port_separator = "."
            sep = addr_part.rfind(".")
            if sep == -1:
                # No port — entire string is the address.
                ip = IPAddress.try_parse(addr_part)
                if ip is None:
                    raise ValueError(f"Destination: invalid IPv6 ({addr_part!r})")
                return cls(
                    address=ip,
                    port=Port(port=0, spelling=""),
                    partition=partition,
                    route_domain=route_domain,
                    port_separator=".",
                )
            addr_text = addr_part[:sep]
            port_text = addr_part[sep + 1 :]
            ip = IPAddress.try_parse(addr_text)
            if ip is None:
                raise ValueError(f"Destination: invalid IPv6 ({addr_text!r})")
            return cls(
                address=ip,
                port=Port.parse(port_text),
                partition=partition,
                route_domain=route_domain,
                port_separator=".",
            )

        # IPv4 / FQDN — port separator is the LAST ``:``.
        sep = addr_part.rfind(":")
        if sep == -1:
            # No port.  Try address-only.
            address_value = parse_address(addr_part)
            return cls(
                address=address_value,
                port=Port(port=0, spelling=""),
                partition=partition,
                route_domain=route_domain,
            )
        addr_text = addr_part[:sep]
        port_text = addr_part[sep + 1 :]
        address_value = parse_address(addr_text)
        return cls(
            address=address_value,
            port=Port.parse(port_text),
            partition=partition,
            route_domain=route_domain,
        )

    @classmethod
    def try_parse(cls, text: str) -> "Destination | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    def __str__(self) -> str:
        addr_text = str(self.address)
        if isinstance(self.address, IPAddress) and self.address.is_ipv6:
            if self.ipv6_brackets:
                addr_text = f"[{addr_text}]"
        if self.route_domain is not None and not self.route_domain.is_default:
            # Route-domain attaches to the address before any brackets close.
            if self.ipv6_brackets:
                # Insert ``%N`` before the closing ``]``.
                addr_text = addr_text[:-1] + str(self.route_domain) + "]"
            else:
                addr_text = addr_text + str(self.route_domain)
        out = addr_text
        if self.port.is_any and not self.port.spelling:
            # Originally parsed without an explicit port — render
            # without a separator, matching the input shape.
            pass
        else:
            out = f"{out}{self.port_separator}{self.port}"
        if self.partition is not None:
            out = f"{self.partition}/{out}"
        return out


def _split_route_domain(text: str) -> tuple[str, RouteDomain | None, str]:
    """Strip a trailing ``%N`` route-domain off *text*.

    Returns ``(addr-without-rd, route-domain, original-rd-text)``.
    """
    if "%" not in text:
        return text, None, ""
    addr, _, rd_text = text.partition("%")
    # The route-domain token is digits only — stop at the first
    # non-digit so a port separator after the rd works (``%5:80``,
    # ``%5.80``).
    rd_digits = ""
    for ch in rd_text:
        if ch.isdigit():
            rd_digits += ch
        else:
            break
    remainder = rd_text[len(rd_digits) :]
    if not rd_digits:
        # ``%`` followed by non-digit — not a route-domain.
        return text, None, ""
    return addr + remainder, RouteDomain.parse(rd_digits), rd_digits
