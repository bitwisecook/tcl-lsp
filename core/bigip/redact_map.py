"""Stable, reversible IP-address redaction map.

Powers ``f5 redact``, ``f5 unredact``, and ``f5 pcap-remap``.

Design goals
------------

1. **CIDR preservation.** If two real IPs share a /24, their redacted
   counterparts must also share a /24 — operators reading the redacted
   config should see the same subnet relationships as the real one.

2. **Reversibility.** A sidecar map file (TOML) records every
   real↔redacted pair plus enough metadata to reverse the mapping
   exactly, including any keyed shuffle.

3. **Determinism.** Same inputs (source CIDRs encountered in the same
   order, same target pool, same shuffle key) produce the same map —
   important for round-tripping a config through redact → edit →
   unredact.

The mapping operates at the *source-CIDR* granularity inferred from the
input.  Without explicit hints we infer source CIDRs from the data:

- Every IPv4 literal is grouped into the smallest enclosing /24
  (or /64 for IPv6).  Two IPs in the same /24 land in the same
  target /24.
- Callers can supply explicit ``source_cidrs`` (e.g. parsed from
  ``ltm route-domain`` or ``net self`` declarations) for finer control.

Host-bit allocation modes
-------------------------

- ``direct``   — host bits are preserved (10.0.0.5/24 → 192.0.2.5/24).
- ``shuffle``  — host bits within the source CIDR are permuted via a
                 keyed Feistel-style permutation; the per-source-CIDR
                 key is recorded in the map so :func:`unmap_address`
                 can invert it.
"""

from __future__ import annotations

import hashlib
import ipaddress
import re
import sys
from dataclasses import dataclass, field
from typing import Iterable, Sequence

if sys.version_info >= (3, 11):
    import tomllib as _TOMLLIB
else:  # pragma: no cover — repo runs on 3.11+ in CI
    import tomli as _TOMLLIB  # type: ignore[no-redef]

# Default RFC1918 pool, walked in order.  Operators on internal LANs
# routinely use 10/8 but rarely 172.16/12 or 192.168/16 at any scale,
# so the order is deliberate.
DEFAULT_TARGET_CIDRS_V4: tuple[str, ...] = (
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
)
DEFAULT_TARGET_CIDRS_V6: tuple[str, ...] = ("fd00::/8",)

_IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
_IPV6_RE = re.compile(
    r"(?<![A-Za-z0-9:])(?:[0-9a-fA-F]{1,4}:){2,7}[0-9a-fA-F]{0,4}(?![A-Za-z0-9:])"
)


@dataclass(slots=True)
class CidrAssignment:
    """One source CIDR mapped to one target CIDR of equal prefix length."""

    source: str
    target: str
    shuffle_key: str | None = None  # hex; None means direct mapping


@dataclass(slots=True)
class RedactionMap:
    """Bidirectional record of every real↔redacted IP encountered."""

    cidr_assignments: list[CidrAssignment] = field(default_factory=list)
    # Eager forward/reverse caches (full IPs, populated on demand).
    forward: dict[str, str] = field(default_factory=dict)
    reverse: dict[str, str] = field(default_factory=dict)
    target_pool_v4: tuple[str, ...] = DEFAULT_TARGET_CIDRS_V4
    target_pool_v6: tuple[str, ...] = DEFAULT_TARGET_CIDRS_V6
    mode: str = "direct"  # "direct" | "shuffle"
    seed: str = ""

    def to_toml(self) -> str:
        """Serialise to TOML for the sidecar map file."""
        lines: list[str] = []
        lines.append("# f5 redact / unredact map file")
        lines.append("# Generated automatically — do not edit by hand unless you")
        lines.append("# accept that round-tripping may then produce different output.")
        lines.append("")
        lines.append('schema = "f5-redact-map/v1"')
        lines.append(f'mode = "{self.mode}"')
        lines.append(f'seed = "{self.seed}"')
        lines.append("target_pool_v4 = [" + ", ".join(f'"{c}"' for c in self.target_pool_v4) + "]")
        lines.append("target_pool_v6 = [" + ", ".join(f'"{c}"' for c in self.target_pool_v6) + "]")
        lines.append("")
        for assignment in self.cidr_assignments:
            lines.append("[[cidr]]")
            lines.append(f'source = "{assignment.source}"')
            lines.append(f'target = "{assignment.target}"')
            if assignment.shuffle_key is not None:
                lines.append(f'shuffle_key = "{assignment.shuffle_key}"')
            lines.append("")
        # Eager IP cache so reverse-mapping doesn't require re-deriving
        # every host shuffle.
        if self.forward:
            lines.append("[ips]")
            for real, fake in sorted(self.forward.items()):
                lines.append(f'"{real}" = "{fake}"')
        return "\n".join(lines).rstrip() + "\n"

    @classmethod
    def from_toml(cls, text: str) -> RedactionMap:
        data = _TOMLLIB.loads(text)
        if data.get("schema") != "f5-redact-map/v1":
            raise ValueError(f"unknown map schema: {data.get('schema')!r}")
        rm = cls(
            cidr_assignments=[
                CidrAssignment(
                    source=item["source"],
                    target=item["target"],
                    shuffle_key=item.get("shuffle_key"),
                )
                for item in data.get("cidr", [])
            ],
            target_pool_v4=tuple(data.get("target_pool_v4", DEFAULT_TARGET_CIDRS_V4)),
            target_pool_v6=tuple(data.get("target_pool_v6", DEFAULT_TARGET_CIDRS_V6)),
            mode=data.get("mode", "direct"),
            seed=data.get("seed", ""),
        )
        forward = data.get("ips", {})
        rm.forward = dict(forward)
        rm.reverse = {v: k for k, v in forward.items()}
        return rm


# ── Allocation ──────────────────────────────────────────────────────


_IPNet = ipaddress.IPv4Network | ipaddress.IPv6Network
_IPAddr = ipaddress.IPv4Address | ipaddress.IPv6Address


def _is_already_private(addr: _IPAddr) -> bool:
    return addr.is_private or addr.is_loopback or addr.is_link_local or addr.is_multicast


def _enclosing_cidr(addr: _IPAddr, prefix: int) -> _IPNet:
    return ipaddress.ip_network(f"{addr}/{prefix}", strict=False)


class Allocator:
    """Allocate target CIDRs out of one or more pools, deterministically."""

    def __init__(self, pools_v4: Sequence[str], pools_v6: Sequence[str]) -> None:
        self._pools_v4 = [ipaddress.IPv4Network(p) for p in pools_v4]
        self._pools_v6 = [ipaddress.IPv6Network(p) for p in pools_v6]
        # Each pool tracks the next host-aligned offset (in network-bits
        # space) at which to carve the next sub-CIDR.
        self._used_v4: dict[ipaddress.IPv4Network, int] = {p: 0 for p in self._pools_v4}
        self._used_v6: dict[ipaddress.IPv6Network, int] = {p: 0 for p in self._pools_v6}

    def allocate(self, prefix: int, version: int) -> _IPNet:
        if version == 4:
            return self._allocate_v4(prefix)
        return self._allocate_v6(prefix)

    def _allocate_v4(self, prefix: int) -> ipaddress.IPv4Network:
        for pool in self._pools_v4:
            if prefix < pool.prefixlen:
                continue
            offset = self._used_v4[pool]
            block_size = 1 << (pool.max_prefixlen - prefix)
            if offset + block_size > pool.num_addresses:
                continue
            net_int = int(pool.network_address) + offset
            target = ipaddress.IPv4Network((net_int, prefix), strict=False)
            self._used_v4[pool] = offset + block_size
            return target
        raise RuntimeError(f"target pool exhausted for v4 /{prefix}")

    def _allocate_v6(self, prefix: int) -> ipaddress.IPv6Network:
        for pool in self._pools_v6:
            if prefix < pool.prefixlen:
                continue
            offset = self._used_v6[pool]
            block_size = 1 << (pool.max_prefixlen - prefix)
            if offset + block_size > pool.num_addresses:
                continue
            net_int = int(pool.network_address) + offset
            target = ipaddress.IPv6Network((net_int, prefix), strict=False)
            self._used_v6[pool] = offset + block_size
            return target
        raise RuntimeError(f"target pool exhausted for v6 /{prefix}")

    def consume(self, target: _IPNet) -> None:
        """Mark *target* as already-issued so future allocations skip past it.

        Used when extending an existing :class:`RedactionMap`: every prior
        target CIDR is replayed through the allocator before any new IPs
        are mapped, so we never hand out the same target twice.
        """
        if isinstance(target, ipaddress.IPv4Network):
            for pool in self._pools_v4:
                if target.subnet_of(pool):
                    high = int(target.network_address) + target.num_addresses
                    used_offset = high - int(pool.network_address)
                    if used_offset > self._used_v4[pool]:
                        self._used_v4[pool] = used_offset
                    return
        else:
            for pool in self._pools_v6:
                if target.subnet_of(pool):
                    high = int(target.network_address) + target.num_addresses
                    used_offset = high - int(pool.network_address)
                    if used_offset > self._used_v6[pool]:
                        self._used_v6[pool] = used_offset
                    return


# ── Permutation (shuffle mode) ──────────────────────────────────────


def _derive_key(seed: str, source_cidr: str) -> bytes:
    return hashlib.sha256(f"{seed}::{source_cidr}".encode()).digest()


_SHUFFLE_MAX_WIDTH = 20  # 2**20 = 1,048,576 host bits per CIDR; bigger -> direct.

_PERM_CACHE: dict[tuple[int, bytes], tuple[list[int], list[int]]] = {}


def _build_permutation(host_width: int, key: bytes) -> tuple[list[int], list[int]]:
    """Return ``(forward, inverse)`` permutations for ``host_width`` bits.

    Materialises the full permutation array (size 2**host_width).  Cheap
    for /24-and-smaller networks; for wider ones we cap at
    :data:`_SHUFFLE_MAX_WIDTH` and the caller falls back to a direct
    mapping.

    The shuffle is a Fisher-Yates over a deterministic RNG seeded by the
    SHA-256 of *key*, so the same *key* always yields the same output.
    """
    import random

    cache_key = (host_width, key)
    cached = _PERM_CACHE.get(cache_key)
    if cached is not None:
        return cached
    size = 1 << host_width
    seed_int = int.from_bytes(hashlib.sha256(key).digest()[:16], "big")
    rng = random.Random(seed_int)
    forward = list(range(size))
    rng.shuffle(forward)
    inverse = [0] * size
    for i, j in enumerate(forward):
        inverse[j] = i
    _PERM_CACHE[cache_key] = (forward, inverse)
    return forward, inverse


def _shuffle_host(
    addr: _IPAddr, source: _IPNet, target: _IPNet, key: bytes, *, invert: bool
) -> _IPAddr:
    host_width = addr.max_prefixlen - source.prefixlen
    if host_width == 0:
        return target.network_address
    if host_width > _SHUFFLE_MAX_WIDTH:
        # Permutation table too large; degrade to direct mapping silently.
        host_bits = int(addr) & ((1 << host_width) - 1)
        return _direct_host(host_bits, source, target)
    forward, inverse = _build_permutation(host_width, key)
    table = inverse if invert else forward
    host_bits = int(addr) & ((1 << host_width) - 1)
    return _direct_host(table[host_bits], source, target)


def _direct_host(host_bits: int, source: _IPNet, target: _IPNet) -> _IPAddr:
    cls = type(target.network_address)
    return cls(int(target.network_address) | host_bits)


# ── Map operations ──────────────────────────────────────────────────


def _normalise_ipv4_token(token: str) -> ipaddress.IPv4Address | None:
    try:
        return ipaddress.IPv4Address(token)
    except ValueError:
        return None


def _normalise_ipv6_token(token: str) -> ipaddress.IPv6Address | None:
    try:
        return ipaddress.IPv6Address(token)
    except ValueError:
        return None


def collect_addresses(text: str) -> tuple[list[ipaddress.IPv4Address], list[ipaddress.IPv6Address]]:
    """Find every IPv4 and IPv6 literal in *text* in order of first appearance."""
    seen_v4: dict[ipaddress.IPv4Address, None] = {}
    for token in _IPV4_RE.findall(text):
        addr = _normalise_ipv4_token(token)
        if addr is None:
            continue
        if _is_already_private(addr):
            continue
        seen_v4.setdefault(addr, None)
    seen_v6: dict[ipaddress.IPv6Address, None] = {}
    for token in _IPV6_RE.findall(text):
        addr = _normalise_ipv6_token(token)
        if addr is None:
            continue
        if _is_already_private(addr):
            continue
        seen_v6.setdefault(addr, None)
    return list(seen_v4), list(seen_v6)


def build_map(
    *,
    text: str,
    target_pool_v4: Sequence[str] = DEFAULT_TARGET_CIDRS_V4,
    target_pool_v6: Sequence[str] = DEFAULT_TARGET_CIDRS_V6,
    mode: str = "direct",
    seed: str = "",
    source_cidr_v4_prefix: int = 24,
    source_cidr_v6_prefix: int = 64,
    explicit_source_cidrs: Iterable[str] = (),
    existing: RedactionMap | None = None,
) -> RedactionMap:
    """Build (or extend) a :class:`RedactionMap` covering every public IP in *text*.

    Per-CIDR allocation is greedy by first-seen order so the same input
    text always produces the same map.

    When *existing* is supplied, every prior CIDR assignment, IP mapping,
    target pool, mode, and seed is reused — only IPs / CIDRs not seen
    before get fresh assignments.  This makes a sequence of
    redactions stable: the IP a customer sees in week-1 keeps mapping
    to the same redacted address in week-2, so support emails continue
    to round-trip cleanly.
    """
    if mode not in {"direct", "shuffle"}:
        raise ValueError(f"unknown mode {mode!r}")

    if existing is not None:
        # Inherit settings from the prior map; ignore any conflicting
        # arguments so the user can't accidentally desync a continuation.
        rm = RedactionMap(
            cidr_assignments=list(existing.cidr_assignments),
            forward=dict(existing.forward),
            reverse=dict(existing.reverse),
            target_pool_v4=existing.target_pool_v4,
            target_pool_v6=existing.target_pool_v6,
            mode=existing.mode,
            seed=existing.seed,
        )
        target_pool_v4 = existing.target_pool_v4
        target_pool_v6 = existing.target_pool_v6
        mode = existing.mode
        seed = existing.seed
    else:
        rm = RedactionMap(
            target_pool_v4=tuple(target_pool_v4),
            target_pool_v6=tuple(target_pool_v6),
            mode=mode,
            seed=seed,
        )
    allocator = Allocator(target_pool_v4, target_pool_v6)

    # Replay prior CIDR assignments through the allocator so new ones
    # don't collide with already-issued targets.
    cidr_for: dict[_IPNet, _IPNet] = {}
    for assignment in rm.cidr_assignments:
        src = ipaddress.ip_network(assignment.source, strict=False)
        tgt = ipaddress.ip_network(assignment.target, strict=False)
        cidr_for[src] = tgt
        # Bump allocator past this target so we don't double-issue.
        allocator.consume(tgt)

    explicit: list[_IPNet] = [ipaddress.ip_network(c, strict=False) for c in explicit_source_cidrs]

    def _source_cidr_for(addr: _IPAddr) -> _IPNet:
        for net in explicit:
            if (
                isinstance(net, ipaddress.IPv4Network)
                and isinstance(addr, ipaddress.IPv4Address)
                and addr in net
            ) or (
                isinstance(net, ipaddress.IPv6Network)
                and isinstance(addr, ipaddress.IPv6Address)
                and addr in net
            ):
                return net
        prefix = (
            source_cidr_v4_prefix
            if isinstance(addr, ipaddress.IPv4Address)
            else source_cidr_v6_prefix
        )
        return _enclosing_cidr(addr, prefix)

    addresses_v4, addresses_v6 = collect_addresses(text)
    for addr in [*addresses_v4, *addresses_v6]:
        source_net = _source_cidr_for(addr)
        if source_net not in cidr_for:
            target_net = allocator.allocate(source_net.prefixlen, addr.version)
            shuffle_key = _derive_key(seed, str(source_net)).hex() if mode == "shuffle" else None
            rm.cidr_assignments.append(
                CidrAssignment(
                    source=str(source_net),
                    target=str(target_net),
                    shuffle_key=shuffle_key,
                )
            )
            cidr_for[source_net] = target_net

        target_net = cidr_for[source_net]
        if mode == "shuffle":
            mapped = _shuffle_host(
                addr, source_net, target_net, _derive_key(seed, str(source_net)), invert=False
            )
        else:
            host_width = addr.max_prefixlen - source_net.prefixlen
            host_bits = int(addr) & ((1 << host_width) - 1)
            mapped = _direct_host(host_bits, source_net, target_net)

        rm.forward[str(addr)] = str(mapped)
        rm.reverse[str(mapped)] = str(addr)

    return rm


def map_address(rm: RedactionMap, real: str) -> str:
    """Return the redacted form of *real*, or *real* unchanged if no rule applies."""
    if real in rm.forward:
        return rm.forward[real]
    addr = _normalise_ipv4_token(real) or _normalise_ipv6_token(real)
    if addr is None:
        return real
    if _is_already_private(addr):
        return real
    # Find an assignment whose source covers this address.
    for assignment in rm.cidr_assignments:
        source = ipaddress.ip_network(assignment.source, strict=False)
        if isinstance(source, ipaddress.IPv4Network) and not isinstance(
            addr, ipaddress.IPv4Address
        ):
            continue
        if isinstance(source, ipaddress.IPv6Network) and not isinstance(
            addr, ipaddress.IPv6Address
        ):
            continue
        if addr in source:
            target = ipaddress.ip_network(assignment.target, strict=False)
            if assignment.shuffle_key:
                mapped = _shuffle_host(
                    addr, source, target, bytes.fromhex(assignment.shuffle_key), invert=False
                )
            else:
                host_width = addr.max_prefixlen - source.prefixlen
                host_bits = int(addr) & ((1 << host_width) - 1)
                mapped = _direct_host(host_bits, source, target)
            rm.forward[real] = str(mapped)
            rm.reverse[str(mapped)] = real
            return str(mapped)
    return real


def unmap_address(rm: RedactionMap, fake: str) -> str:
    """Return the real form of *fake*, or *fake* unchanged if no rule applies."""
    if fake in rm.reverse:
        return rm.reverse[fake]
    addr = _normalise_ipv4_token(fake) or _normalise_ipv6_token(fake)
    if addr is None:
        return fake
    for assignment in rm.cidr_assignments:
        target = ipaddress.ip_network(assignment.target, strict=False)
        if isinstance(target, ipaddress.IPv4Network) and not isinstance(
            addr, ipaddress.IPv4Address
        ):
            continue
        if isinstance(target, ipaddress.IPv6Network) and not isinstance(
            addr, ipaddress.IPv6Address
        ):
            continue
        if addr in target:
            source = ipaddress.ip_network(assignment.source, strict=False)
            if assignment.shuffle_key:
                real = _shuffle_host(
                    addr, target, source, bytes.fromhex(assignment.shuffle_key), invert=True
                )
            else:
                host_width = addr.max_prefixlen - target.prefixlen
                host_bits = int(addr) & ((1 << host_width) - 1)
                real = _direct_host(host_bits, target, source)
            rm.reverse[fake] = str(real)
            rm.forward[str(real)] = fake
            return str(real)
    return fake


def apply_map(rm: RedactionMap, text: str, *, reverse: bool = False) -> tuple[str, int]:
    """Substitute every IPv4/IPv6 literal in *text* via *rm*. Returns (text, count)."""
    count = 0

    def _sub_v4(match: re.Match[str]) -> str:
        nonlocal count
        token = match.group(0)
        replacement = unmap_address(rm, token) if reverse else map_address(rm, token)
        if replacement != token:
            count += 1
        return replacement

    def _sub_v6(match: re.Match[str]) -> str:
        nonlocal count
        token = match.group(0)
        replacement = unmap_address(rm, token) if reverse else map_address(rm, token)
        if replacement != token:
            count += 1
        return replacement

    out = _IPV4_RE.sub(_sub_v4, text)
    out = _IPV6_RE.sub(_sub_v6, out)
    return out, count
