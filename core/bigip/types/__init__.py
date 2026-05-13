"""Typed BIG-IP scalar values — IP addresses, networks, ports, partitions.

Domain-specific value types layered on top of Python's ``ipaddress``
standard library so the rest of the codebase can stop string-bashing
F5 destination syntax.

Design principles:

- **Parse, don't regex.**  Every text → value conversion goes through
  a hand-written parser that delegates real validation to ``ipaddress``
  for IPs and to integer parsing for ports.  No best-effort regex
  prefiltering.
- **Round-trip clean.**  Every value renders back to the canonical F5
  text representation it was parsed from (or the F5-preferred
  spelling when parsed loosely).  The same value never round-trips
  through two different spellings.
- **One canonical home.**  Every other module that needs to talk
  about an IP / network / port / FQDN / partition / route-domain
  should import these types instead of re-parsing from string.

Public surface:

- :class:`IPAddress`   — IPv4 or IPv6 host.
- :class:`Network`     — IPv4 or IPv6 CIDR.
- :class:`Port`        — single port (1-65535, or wildcard ``0`` /
  ``"any"`` / ``"*"``).
- :class:`PortRange`   — inclusive port range.
- :class:`Partition`   — F5 partition (``/Common`` / ``/Tenant_A``).
- :class:`RouteDomain` — F5 route-domain (``%5`` suffix).
- :class:`FQDN`        — fully-qualified domain name (used by
  fqdn-based pool members).
- :class:`Address`     — sum type covering :class:`IPAddress` /
  :class:`FQDN`.
- :class:`Destination` — F5 virtual-server / pool-member
  ``[partition/]address[%route-domain][.|:port]`` triple.

Every class is a frozen dataclass, hashable, equality-comparable,
and ``str()``-able to its canonical F5 spelling.
"""

from __future__ import annotations

from ._address import FQDN, Address, IPAddress, parse_address, try_parse_address
from ._destination import Destination
from ._folder import Folder, ObjectPath
from ._network import Network
from ._partition import Partition
from ._port import Port, PortRange
from ._route_domain import RouteDomain

__all__ = [
    "Address",
    "Destination",
    "FQDN",
    "Folder",
    "IPAddress",
    "Network",
    "ObjectPath",
    "Partition",
    "Port",
    "PortRange",
    "RouteDomain",
    "parse_address",
    "try_parse_address",
]
