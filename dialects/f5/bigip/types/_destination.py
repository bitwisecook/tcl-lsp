"""Destination value type — F5 ``[folder/]address[%route-domain][:port]`` triples.

This is the gnarly one.  F5 destination syntax is the union of:

- bare IPv4 with port:                    ``10.0.0.1:80``
- bare IPv6 with ``.``-port (no brackets):``2001:db8::1.80``
- bracketed IPv6 with ``.``-port:         ``[2001:db8::1].80``
- bracketed IPv6 with ``:``-port:         ``[2001:db8::1]:80``
- partition-prefixed:                     ``/Common/10.0.0.1:80``
- partition-prefixed bracketed IPv6:      ``/Common/[2001:db8::1].80``
- folder-nested:                          ``/Common/Application_X/10.0.0.1:80``
- IPv4 with route-domain:                 ``10.0.0.1%5:80``
- IPv6 with route-domain:                 ``2001:db8::1%5.80``
- wildcards:                              ``0.0.0.0:any`` / ``::.0`` / ``*.*``

Pool members add FQDN spellings:

- bare FQDN with port:                    ``host.example.com:443``
- partition-prefixed FQDN:                ``/Common/host.example.com:443``
- folder-nested FQDN:                     ``/Common/iApps/Tenant.app/host.example.com:443``

The parser is hand-written, not regex-driven — it splits the input
on the structural anchors (``/``, ``[``, ``]``, ``%``, the
*last* ``.`` or ``:`` for the port separator) and delegates each
piece to its dedicated value type's parser.

Folder support: the segment between the leading ``/`` and the
host segment may be a multi-level folder path
(``/Common/Application_X/host:port``).  We detect the host
segment as "the first segment that parses as an address (or
starts with ``[``)" and treat everything before it as the folder
path.
"""

from __future__ import annotations

from dataclasses import dataclass

from ._address import Address, IPAddress, parse_address
from ._folder import Folder
from ._partition import Partition
from ._port import Port
from ._route_domain import RouteDomain


@dataclass(frozen=True, slots=True)
class Destination:
    """An F5 destination triple: optional folder, address, optional
    route-domain, port.

    Every field except ``address`` and ``port`` is optional; the
    canonical form drops them when absent.  When present they
    serialise as ``<folder>/<address>%<rd>:<port>`` (with the
    appropriate IPv6 bracketing / dot-port separator).
    """

    address: Address
    port: Port
    folder: Folder | None = None
    route_domain: RouteDomain | None = None
    # ``ipv6_brackets`` records whether the input wrapped an IPv6
    # address in ``[...]`` so the canonical render preserves it.
    ipv6_brackets: bool = False
    # ``port_separator`` records ``"."`` vs ``":"`` for IPv6
    # destinations (both are valid F5 spellings).
    port_separator: str = ":"

    @property
    def partition(self) -> Partition | None:
        """Convenience: the partition portion of the folder, if any."""
        return self.folder.partition if self.folder is not None else None

    @classmethod
    def parse(cls, text: str) -> "Destination":
        """Parse *text* as an F5 destination.

        Raises :class:`ValueError` for unparseable input.  Pool-member
        ``host.example.com:443`` style FQDNs are accepted via
        :func:`parse_address`'s fallback.  Folder-nested paths
        (``/Common/Application_X/...``) are detected automatically.
        """
        text = text.strip()
        if not text:
            raise ValueError("Destination: empty input")

        # Folder prefix: split on ``/`` but stop at the FIRST segment
        # that looks like an address (starts with ``[``, contains
        # digits + dots/colons, or doesn't look like a folder name).
        # Everything before that segment is the folder path.
        folder, rest = _split_folder_prefix(text)

        # IPv6 in brackets.  ``[ADDR]`` followed by ``:port`` or ``.port``.
        if rest.startswith("["):
            close = rest.find("]")
            if close == -1:
                raise ValueError(f"Destination: unmatched '[' in {text!r}")
            addr_text = rest[1:close]
            port_part = rest[close + 1 :]
            addr_clean, route_domain, _ = _split_route_domain(addr_text)
            ip = IPAddress.try_parse(addr_clean)
            if ip is None:
                raise ValueError(f"Destination: bracketed value isn't a valid IP ({addr_text!r})")
            if not port_part:
                return cls(
                    address=ip,
                    port=Port(port=0, spelling=""),
                    folder=folder,
                    route_domain=route_domain,
                    ipv6_brackets=True,
                )
            if port_part[0] not in (":", "."):
                raise ValueError(f"Destination: missing port separator after ']' in {text!r}")
            port_separator = port_part[0]
            port = Port.parse(port_part[1:])
            return cls(
                address=ip,
                port=port,
                folder=folder,
                route_domain=route_domain,
                ipv6_brackets=True,
                port_separator=port_separator,
            )

        # No brackets — split address from port.  IPv6 without
        # brackets uses ``.`` as the port separator (because ``:`` is
        # part of the address); IPv4 / FQDN use ``:``.
        addr_part, route_domain, _ = _split_route_domain(rest)
        if addr_part.count(":") >= 2:
            sep = addr_part.rfind(".")
            if sep == -1:
                ip = IPAddress.try_parse(addr_part)
                if ip is None:
                    raise ValueError(f"Destination: invalid IPv6 ({addr_part!r})")
                return cls(
                    address=ip,
                    port=Port(port=0, spelling=""),
                    folder=folder,
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
                folder=folder,
                route_domain=route_domain,
                port_separator=".",
            )

        # IPv4 / FQDN — port separator is the LAST ``:``.
        sep = addr_part.rfind(":")
        if sep == -1:
            address_value = parse_address(addr_part)
            return cls(
                address=address_value,
                port=Port(port=0, spelling=""),
                folder=folder,
                route_domain=route_domain,
            )
        addr_text = addr_part[:sep]
        port_text = addr_part[sep + 1 :]
        address_value = parse_address(addr_text)
        return cls(
            address=address_value,
            port=Port.parse(port_text),
            folder=folder,
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
            if self.ipv6_brackets:
                addr_text = addr_text[:-1] + str(self.route_domain) + "]"
            else:
                addr_text = addr_text + str(self.route_domain)
        out = addr_text
        if self.port.is_any and not self.port.spelling:
            pass
        else:
            out = f"{out}{self.port_separator}{self.port}"
        if self.folder is not None:
            out = f"{self.folder}/{out}"
        return out


def _split_folder_prefix(text: str) -> tuple[Folder | None, str]:
    """Split a leading folder path off *text*.

    Walks segments left-to-right; the FIRST segment that looks like
    an address (starts with ``[``, contains a digit, or fails to
    parse as a folder segment) is the host segment.  Everything
    before it is the folder.

    Returns ``(folder, remainder)`` where *remainder* is the text
    starting at the host segment.  A bare destination with no
    leading ``/`` returns ``(None, text)``.
    """
    if not text.startswith("/"):
        return None, text
    parts = text.lstrip("/").split("/")
    # The first segment is always the partition (must be a folder
    # segment).  After that, walk forward until we see something
    # that isn't a folder-shaped name.
    if len(parts) < 2:
        # ``/Common`` alone — no host segment.  Treat as no folder.
        return None, text
    folder_segments = [parts[0]]
    host_index = 1
    for i in range(1, len(parts)):
        seg = parts[i]
        if seg.startswith("[") or _looks_like_address(seg):
            host_index = i
            break
        folder_segments.append(seg)
    else:
        # Loop fell through — every segment was folder-shaped, no
        # host found.  Treat the last as the host.
        host_index = len(parts) - 1
        folder_segments = parts[:host_index]
    folder_text = "/" + "/".join(folder_segments)
    folder = Folder.try_parse(folder_text)
    if folder is None:
        return None, text
    remainder = "/".join(parts[host_index:])
    return folder, remainder


def _looks_like_address(segment: str) -> bool:
    """True when *segment* looks like a host (address[:port] or
    bracketed IPv6) rather than a folder name.

    Heuristic but driven by structural cues, not regex pattern
    matching: contains a colon (port separator or IPv6), starts
    with a digit (IPv4 or numeric host), or contains a dot in a
    position that makes it FQDN-shaped.  Folder names are
    alphanumeric + ``_-.`` with no colon.
    """
    if ":" in segment:
        return True
    if segment and segment[0].isdigit():
        return True
    return False


def _split_route_domain(text: str) -> tuple[str, RouteDomain | None, str]:
    """Strip a trailing ``%N`` route-domain off *text*.

    Returns ``(addr-without-rd, route-domain, original-rd-text)``.
    """
    if "%" not in text:
        return text, None, ""
    addr, _, rd_text = text.partition("%")
    rd_digits = ""
    for ch in rd_text:
        if ch.isdigit():
            rd_digits += ch
        else:
            break
    remainder = rd_text[len(rd_digits) :]
    if not rd_digits:
        return text, None, ""
    return addr + remainder, RouteDomain.parse(rd_digits), rd_digits
