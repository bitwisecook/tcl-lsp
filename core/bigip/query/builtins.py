"""Builtin function library for the query DSL.

Every builtin is registered with a decorator that captures the
signature, summary, category, and at least one example.  The runtime
dispatch table and the ``f5 query --help-builtins`` text are generated
from the same registry, so the docs and the code cannot drift.

Functions come in two flavours:

- **Plain builtins** receive their arguments eagerly evaluated.  Most
  fall in this bucket — ``ip``, ``net``, ``partition``, ``length``,
  string predicates, and so on.
- **Special-form builtins** (``select``, ``map``) need to evaluate
  their argument against a re-bound ``.`` per input value, so they are
  flagged with ``special_form=True`` and receive the unevaluated AST
  plus an :class:`.evaluator.EvalContext`.  The evaluator drives the
  binding loop.

Builtins raise :class:`.errors.BuiltinError` for argument-type mistakes
so the CLI can map them to ``error:`` messages without distinguishing
them from other query failures.
"""

from __future__ import annotations

import ipaddress
import re
from dataclasses import dataclass
from typing import Any, Callable

from .errors import BuiltinError
from .values import ObjectRef, PathRef, Stream

# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class BuiltinSpec:
    name: str
    summary: str
    signatures: tuple[str, ...]
    examples: tuple[str, ...]
    category: str
    impl: Callable[..., Any]
    # Multi-paragraph reference prose — semantics, return type, error
    # cases, related builtins.  Surfaced verbatim by
    # ``--help-builtins NAME`` and by the generated builtin reference
    # under ``docs/design/f5-query-dsl-builtins.md``.  Markdown is
    # tolerated but the help-text path is a terminal, so keep
    # decoration light (paragraphs, bullets, inline ``code``).
    details: str = ""
    special_form: bool = False
    # ``with_ctx`` builtins receive the evaluator's :class:`EvalContext`
    # as a keyword argument so they can register edits or look up the
    # active :class:`Root`.  Used by cascading operations like
    # ``rename_partition``.
    with_ctx: bool = False
    min_args: int = 0
    max_args: int | None = 0


_REGISTRY: dict[str, BuiltinSpec] = {}
# Categories rendered in the order they appear in --help-builtins.
_CATEGORY_ORDER = (
    "stream",
    "string",
    "path",
    "rename",
    "net",
    "graph",
    "value",
)


def _register(
    name: str,
    *,
    summary: str,
    signatures: tuple[str, ...],
    examples: tuple[str, ...],
    category: str,
    min_args: int,
    max_args: int | None,
    details: str = "",
    special_form: bool = False,
    with_ctx: bool = False,
) -> Callable[[Callable], Callable]:
    def decorator(fn: Callable) -> Callable:
        if name in _REGISTRY:
            raise RuntimeError(f"duplicate builtin registration: {name}")
        _REGISTRY[name] = BuiltinSpec(
            name=name,
            summary=summary,
            signatures=signatures,
            examples=examples,
            category=category,
            impl=fn,
            details=_dedent_details(details),
            special_form=special_form,
            with_ctx=with_ctx,
            min_args=min_args,
            max_args=max_args,
        )
        return fn

    return decorator


def _dedent_details(text: str) -> str:
    """Strip the leading-whitespace common to every line of a details block.

    Lets us write the per-function reference prose as indented
    triple-quoted strings without dragging the indentation into the
    rendered output.
    """
    import textwrap

    return textwrap.dedent(text).strip()


def lookup(name: str) -> BuiltinSpec | None:
    return _REGISTRY.get(name)


def list_builtins() -> list[BuiltinSpec]:
    """Return every builtin, sorted by category then name."""
    by_cat: dict[str, list[BuiltinSpec]] = {}
    for spec in _REGISTRY.values():
        by_cat.setdefault(spec.category, []).append(spec)
    out: list[BuiltinSpec] = []
    seen: set[str] = set()
    for cat in _CATEGORY_ORDER:
        for spec in sorted(by_cat.get(cat, ()), key=lambda s: s.name):
            out.append(spec)
            seen.add(spec.name)
    # Defensive: surface any builtin in an unknown category at the end.
    for spec in _REGISTRY.values():
        if spec.name not in seen:
            out.append(spec)
    return out


def format_builtins(name: str | None = None) -> str:
    """Render the builtin reference for ``f5 query --help-builtins``.

    When *name* is given, render only that builtin's entry, including
    the deep ``details`` block (so users can drill down with
    ``--help-builtins ip``).  Otherwise render every builtin grouped
    by category, with the summary and examples only — the full
    catalogue is large and the details are one keystroke away.
    """
    if name is not None:
        spec = lookup(name)
        if spec is None:
            return f"no such builtin: {name}\n"
        return _render_one(spec, with_details=True) + "\n"

    out: list[str] = ["BUILTIN FUNCTIONS", ""]
    last_cat: str | None = None
    for spec in list_builtins():
        if spec.category != last_cat:
            out.append(f"  [{spec.category}]")
            last_cat = spec.category
        out.append(_render_one(spec, indent="    ", with_details=False))
    out.append("")
    out.append("  Use --help-builtins <name> for the full reference of one builtin.")
    return "\n".join(out) + "\n"


def _render_one(spec: BuiltinSpec, indent: str = "", *, with_details: bool = True) -> str:
    """Render one builtin's reference block.

    Brief output (used by ``--help-builtins`` without a name) drops the
    multi-paragraph ``details`` so the catalogue stays scannable.  The
    per-name form (``--help-builtins ip``) and the generated reference
    doc both pass ``with_details=True`` so users get the full deep
    explanation.
    """
    lines = [f"{indent}{spec.name}"]
    for sig in spec.signatures:
        lines.append(f"{indent}    {sig}")
    lines.append(f"{indent}    {spec.summary}")
    if with_details and spec.details:
        lines.append("")
        for para in spec.details.split("\n"):
            lines.append(f"{indent}    {para}" if para else "")
        lines.append("")
    for ex in spec.examples:
        lines.append(f"{indent}    e.g. {ex}")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Argument helpers
# ---------------------------------------------------------------------------


def _as_str(value: object, *, name: str, arg: int) -> str:
    if isinstance(value, PathRef):
        return value.full_path
    if isinstance(value, str):
        return value
    raise BuiltinError(f"{name}: argument {arg} must be a string, got {_type_name(value)}")


def _as_int(value: object, *, name: str, arg: int) -> int:
    if isinstance(value, bool):
        # Bool is a subclass of int — disallow to avoid silent surprises.
        raise BuiltinError(f"{name}: argument {arg} must be an integer, got bool")
    if isinstance(value, int):
        return value
    raise BuiltinError(f"{name}: argument {arg} must be an integer, got {_type_name(value)}")


# Maximum length of a user-supplied regex pattern.  Anything longer
# is almost certainly malformed or pathological — the DSL is for
# F5-config queries, not arbitrary text mining, so a pattern past
# this length is a strong signal of misuse rather than a legitimate
# search.  Cheap to enforce and rules out the most trivial DoS shape
# (a 1 MB ``(a+)+`` blob).
_MAX_REGEX_PATTERN_LENGTH = 1024

# Coarse syntactic block on the most common catastrophic-backtracking
# shapes — nested quantifiers like ``(a+)+`` / ``(a*)*`` / ``(a*)+``
# / ``(a+)*``.  Not a full ReDoS detector (that's undecidable in the
# general case); just refuses the textbook patterns so a copy-pasted
# CVE doesn't immediately hang the process.  Users with a legitimate
# use case can always pre-process their input outside the DSL.
_PATHOLOGICAL_REGEX = re.compile(r"\([^)]*[+*]\)\s*[+*]")


def _safe_regex_compile(pattern: str, *, name: str) -> re.Pattern[str]:
    """Compile *pattern* with length and shape guards.

    The DSL exposes ``match`` / ``sub`` / ``gsub`` and the regex
    container subscript to the query author.  Local CLI usage is
    trusted (the query author is the operator), but the same code
    path is reachable from MCP / chat / editor command surfaces
    where the pattern can come from untrusted input.  This helper
    is the single chokepoint: enforces a length cap, refuses obvious
    catastrophic-backtracking shapes, and translates ``re.error`` to
    the ``BuiltinError`` shape the DSL already raises elsewhere.
    """
    if len(pattern) > _MAX_REGEX_PATTERN_LENGTH:
        raise BuiltinError(
            f"{name}: regex pattern too long "
            f"({len(pattern)} chars > {_MAX_REGEX_PATTERN_LENGTH} char limit) — "
            "the DSL caps pattern length to bound regex compile / match cost; "
            "split the search into smaller patterns or pre-filter the input"
        )
    if _PATHOLOGICAL_REGEX.search(pattern):
        raise BuiltinError(
            f"{name}: pattern {pattern!r} contains a nested quantifier "
            "shape (``(...+)+``, ``(...*)*``, …) that triggers catastrophic "
            "backtracking — refused to keep the engine responsive.  Rewrite "
            "to use possessive quantifiers, atomic groups, or a non-nested form"
        )
    try:
        return re.compile(pattern)
    except re.error as exc:
        raise BuiltinError(f"{name}: invalid pattern {pattern!r}: {exc}") from exc


def _as_sequence(value: object, *, name: str, arg: int) -> list[Any]:
    if isinstance(value, Stream):
        return list(value.items)
    if isinstance(value, list):
        return list(value)
    if isinstance(value, tuple):
        return list(value)
    raise BuiltinError(f"{name}: argument {arg} must be a list or stream, got {_type_name(value)}")


def _flatten_one_level(value: object, *, name: str) -> list[Any]:
    """``_as_sequence`` plus one level of list-flattening.

    Piping a stream into ``map(predicate)`` invokes ``map`` once per
    item, and each call returns a single-element list
    ``[predicate(item)]``.  When the result is then fed to ``any`` /
    ``all`` / similar aggregates, the per-item list wrappers
    confuse the truthiness check (``[False]`` is non-empty hence
    truthy).  Flattening one level here recovers the "did the
    predicate hold for any/all items?" semantics users expect from
    ``any(stream | map(predicate))``.

    Single-level: ``[a, b, c]`` stays ``[a, b, c]``.
    Per-iteration ``map`` output ``[[a], [b], [c]]`` becomes
    ``[a, b, c]``.  Mixed shapes are left untouched so callers
    that actually want list-of-lists semantics keep working.
    """
    seq = _as_sequence(value, name=name, arg=1)
    if seq and all(isinstance(item, list) for item in seq):
        flat: list[Any] = []
        for sub in seq:
            flat.extend(sub)
        return flat
    return seq


def _type_name(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        return "int"
    if isinstance(value, float):
        return "float"
    if isinstance(value, str):
        return "string"
    if isinstance(value, PathRef):
        return "path-ref"
    if isinstance(value, ObjectRef):
        return "object"
    if isinstance(value, Stream):
        return "stream"
    if isinstance(value, list):
        return "list"
    return type(value).__name__


# ---------------------------------------------------------------------------
# Network / address helpers
# ---------------------------------------------------------------------------

# A BIG-IP "destination" is ``[/Partition/]address[%route-domain][:port]``.
# Route domains attach to an address with a ``%<n>`` suffix and stay
# with the address through every transform — they are part of the
# routable identity, not part of the port.
_DEST_RE = re.compile(
    r"^(?P<partition>(?:/[^/]+/)?)"
    r"(?P<addr>[^%:/\s]+)"
    r"(?:%(?P<rd>[A-Za-z0-9_-]+))?"
    r"(?::(?P<port>\d+))?$"
)


def _split_destination(value: str) -> tuple[str, str, str, str]:
    """Return ``(partition_prefix, address, route_domain, port)``.

    Delegates to :class:`core.bigip.types.Destination` so every
    documented F5 spelling — bracketed IPv6, unbracketed IPv6 with
    ``.``-port, folder-nested paths, FQDN pool members, wildcard
    ports — works the same way the model dataclasses' typed fields
    do.  Falls back to the legacy regex match for non-destination
    strings (bare addresses that don't fit any destination shape)
    so the existing query surface stays stable for edge cases.

    ``partition_prefix`` includes the surrounding slashes
    (``"/Common/"``) or is empty.  Each of the other parts is the
    bare value (no ``%``, no ``:``) or empty when absent.  When the
    address is an IPv6 host, it's returned without brackets even
    when the source spelt it as ``[::1]:80`` — callers that need
    the bracket form can wrap it themselves.
    """
    from ..types import Destination

    dest = Destination.try_parse(value)
    if dest is not None:
        partition_prefix = ""
        if dest.folder is not None:
            # Render the folder portion + trailing slash so the
            # ``partition_prefix`` includes every path segment up to
            # the host.  ``Folder.__str__`` already handles the
            # nested-folder case (``/Common/Application_X``).
            partition_prefix = str(dest.folder) + "/"
        addr_text = str(dest.address)
        rd_text = (
            ""
            if dest.route_domain is None or dest.route_domain.is_default
            else str(dest.route_domain.id)
        )
        port_text = ""
        if not (dest.port.is_any and not dest.port.spelling):
            port_text = str(dest.port.port) if not dest.port.is_any else dest.port.spelling
        return partition_prefix, addr_text, rd_text, port_text

    # Legacy regex fallback for inputs that don't parse as a full
    # destination (e.g. just an IPv4 address with no port, or
    # text-shaped tokens an iRule body might carry).
    m = _DEST_RE.match(value)
    if not m:
        return "", value, "", ""
    return (
        m.group("partition") or "",
        m.group("addr"),
        m.group("rd") or "",
        m.group("port") or "",
    )


def _rebuild_destination(partition: str, address: str, route_domain: str, port: str) -> str:
    """Inverse of :func:`_split_destination`."""
    out = f"{partition}{address}"
    if route_domain:
        out = f"{out}%{route_domain}"
    if port:
        out = f"{out}:{port}"
    return out


@_register(
    "ip",
    summary=(
        "Construct an IP-address string from a single string argument, or "
        "from a network + a source address whose host bits should be "
        "preserved (the readdressing helper)."
    ),
    signatures=(
        "ip(addr: string) -> string",
        "ip(network: string, source: string) -> string",
    ),
    details="""
    The one-argument form normalises a destination string to its bare
    address: ``ip("/Common/192.168.1.1:80")`` returns ``"192.168.1.1"``,
    stripping the partition prefix, the route domain, and the port.
    Use the dedicated helpers (``partition``, ``port``,
    ``route_domain``) to recover those parts.

    The two-argument form is the **readdressing helper** and is what
    most query-driven migrations use.  It takes the host bits of
    *source* and joins them to *network*'s prefix, producing a new
    address in *network* that occupies the same host position as the
    original.  Crucially, the partition prefix, route domain, and
    port on *source* are **preserved**:

    - ``ip("192.168.9.0/24", "/Common/10.10.0.5%5:443")`` returns
      ``"/Common/192.168.9.5%5:443"``.
    - The host portion of ``10.10.0.5`` in ``/24`` is ``.5``; the
      result lands ``.5`` into the new network.

    Address-family mismatch raises ``BuiltinError`` (an IPv4 host
    cannot land in an IPv6 network).  An unparseable network or
    source address likewise raises with the offending token in the
    message.

    Pair with ``|=`` to readdress every VS in one statement:
    ``.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)``.

    Related: ``net``, ``host``, ``port``, ``route_domain``,
    ``with_route_domain``, ``in_cidr``.
    """,
    examples=(
        'ip("10.0.0.1")',
        'ip("/Common/10.10.0.5%5:443")           # -> "10.10.0.5"',
        'ip("192.168.9.0/24", .destination)      # rebase, keep host bits',
        'ip("192.168.9.0/24", "10.10.0.5%5:443") # -> "192.168.9.5%5:443"',
    ),
    category="net",
    min_args=1,
    max_args=2,
)
def _builtin_ip(*args: Any) -> str:
    if len(args) == 1:
        s = _as_str(args[0], name="ip", arg=1)
        _, addr, _, _ = _split_destination(s)
        try:
            ipaddress.ip_address(addr)
        except ValueError as exc:
            raise BuiltinError(f"ip: invalid address {addr!r}: {exc}") from exc
        # ``ip(x)`` normalises to the bare address — partition, route
        # domain, and port are stripped.  Use ``host`` / ``port`` /
        # ``route_domain`` to recover them.
        return addr
    # Two-arg form: rebase ``source``'s host bits into ``network``,
    # preserving partition prefix, route domain, and port from the source.
    net_str = _as_str(args[0], name="ip", arg=1)
    src_str = _as_str(args[1], name="ip", arg=2)
    try:
        network = ipaddress.ip_network(net_str, strict=False)
    except ValueError as exc:
        raise BuiltinError(f"ip: invalid network {net_str!r}: {exc}") from exc
    partition, src_addr, rd, port = _split_destination(src_str)
    try:
        src_ip = ipaddress.ip_address(src_addr)
    except ValueError as exc:
        raise BuiltinError(f"ip: invalid source address {src_addr!r}: {exc}") from exc
    if isinstance(network, ipaddress.IPv4Network) and not isinstance(src_ip, ipaddress.IPv4Address):
        raise BuiltinError("ip: cannot rebase an IPv6 address into an IPv4 network")
    if isinstance(network, ipaddress.IPv6Network) and not isinstance(src_ip, ipaddress.IPv6Address):
        raise BuiltinError("ip: cannot rebase an IPv4 address into an IPv6 network")
    host_bits = int(src_ip) & (~int(network.netmask))
    new_int = int(network.network_address) | host_bits
    new_addr = type(src_ip)(new_int)
    return _rebuild_destination(partition, str(new_addr), rd, port)


@_register(
    "net",
    summary="Return the network portion of an IP/CIDR string as ``addr/prefix``.",
    signatures=("net(value: string) -> string",),
    details="""
    Parses *value* as a network (``addr/prefix``) and returns its
    canonical form.  Host bits in the input are masked off, so
    ``net("192.168.9.42/24")`` returns ``"192.168.9.0/24"``.

    Useful as a normaliser when you want every VS in the same /24 to
    report the same network string: ``.ltm.virtual[] | .destination
    | host(.) + "/24" | net(.)``.

    Unparseable input raises ``BuiltinError``.  IPv4 and IPv6 networks
    are both accepted; the prefix is required.

    Related: ``ip``, ``in_cidr``, ``host``.
    """,
    examples=(
        'net("192.168.9.0/24")           # -> "192.168.9.0/24"',
        'net("192.168.9.42/24")          # -> "192.168.9.0/24"',
        'net("2001:db8::42/64")          # -> "2001:db8::/64"',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_net(value: Any) -> str:
    s = _as_str(value, name="net", arg=1)
    try:
        network = ipaddress.ip_network(s, strict=False)
    except ValueError as exc:
        raise BuiltinError(f"net: invalid value {s!r}: {exc}") from exc
    return str(network)


@_register(
    "host",
    summary=(
        "Return just the address half of a BIG-IP destination, stripping "
        "any partition prefix, route domain, and ``:port`` suffix."
    ),
    signatures=("host(value: string) -> string",),
    details="""
    BIG-IP destinations are spelt
    ``[/Partition/]address[%route-domain][:port]``.  ``host`` extracts
    just the address — the partition prefix, route-domain suffix,
    and port are all dropped:

    - ``host("/Common/10.0.0.1%5:80")`` returns ``"10.0.0.1"``.
    - Use ``route_domain``, ``port``, and ``partition`` to recover
      the parts ``host`` strips.

    Falls back to returning the input verbatim when the string does
    not parse as a destination, so it's safe to apply to fields that
    might already be bare addresses.

    Related: ``ip`` (one-arg form does the same normalisation),
    ``port``, ``route_domain``, ``partition``.
    """,
    examples=(
        "host(.destination)",
        'host("/Common/192.168.1.1:80")           # -> "192.168.1.1"',
        'host("/Common/10.0.0.1%5:443")           # -> "10.0.0.1"',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_host(value: Any) -> str:
    s = _as_str(value, name="host", arg=1)
    _, addr, _, _ = _split_destination(s)
    return addr


@_register(
    "port",
    summary=(
        "Return the port half of a BIG-IP destination as an integer, or "
        "``null`` if no port is present."
    ),
    signatures=("port(value: string) -> integer | null",),
    details="""
    Extracts the ``:port`` suffix from a destination string and
    returns it as an integer.  Returns ``null`` (not ``0``) when no
    port is present, so ``port(.destination) | defined(.)`` is the
    natural way to filter VSes that explicitly target a port.

    Partition prefix and route domain on the input are ignored.  A
    malformed port (non-numeric) returns ``null`` rather than
    raising — the destination simply doesn't have a recognisable
    port.

    Related: ``host``, ``ip``, ``route_domain``.
    """,
    examples=(
        "port(.destination)",
        'port("192.168.1.1:80")                   # -> 80',
        'port("/Common/10.0.0.1%5:443")           # -> 443',
        'port("/Common/10.0.0.1")                 # -> null',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_port(value: Any) -> int | None:
    s = _as_str(value, name="port", arg=1)
    _, _, _, port = _split_destination(s)
    return int(port) if port else None


@_register(
    "partition",
    summary="Return the partition name of a full-path (``/Common/foo`` → ``Common``).",
    signatures=("partition(path: string) -> string",),
    details="""
    Extracts the partition segment of a BIG-IP full-path — the bit
    between the first and second ``/``.  An input that does not begin
    with ``/`` (a relative reference, or a bare name) returns the
    empty string.

    Useful for group-by aggregates: ``[.ltm.virtual[].name |
    partition(.)] | unique | sort`` enumerates every partition
    that owns at least one virtual server.

    Related: ``basename`` (the inverse — last segment),
    ``with_partition`` (replace the partition), ``rename_partition``
    (move every object in a partition).
    """,
    examples=(
        'partition("/Common/web_pool")            # -> "Common"',
        'partition("/Tenant_A/web_pool")          # -> "Tenant_A"',
        'partition("relative_name")               # -> ""',
    ),
    category="path",
    min_args=1,
    max_args=1,
)
def _builtin_partition(value: Any) -> str:
    s = _as_str(value, name="partition", arg=1)
    if not s.startswith("/"):
        return ""
    parts = s.split("/", 2)
    return parts[1] if len(parts) >= 2 else ""


@_register(
    "basename",
    summary="Return the last segment of a full-path (``/Common/foo`` → ``foo``).",
    signatures=("basename(path: string) -> string",),
    details="""
    Returns everything after the last ``/`` in a path string.  For a
    bare name (no slashes) the input is returned unchanged.

    Pairs naturally with ``|=`` to strip the partition prefix from
    every reference in one statement:
    ``.ltm.virtual[].pool |= basename(.)``.

    Related: ``partition`` (the inverse — partition segment),
    ``with_partition`` (replace partition, preserve basename).
    """,
    examples=(
        'basename("/Common/web_pool")             # -> "web_pool"',
        'basename("/Tenant_A/api_pool")           # -> "api_pool"',
        'basename("relative_name")                # -> "relative_name"',
    ),
    category="path",
    min_args=1,
    max_args=1,
)
def _builtin_basename(value: Any) -> str:
    s = _as_str(value, name="basename", arg=1)
    return s.rsplit("/", 1)[-1]


@_register(
    "with_partition",
    summary="Replace the partition of a full-path, preserving the basename.",
    signatures=("with_partition(path: string, partition: string) -> string",),
    details="""
    Returns ``/<partition>/<basename(path)>``.  This is a *string*
    transform — by itself it just builds a new path string, it does
    NOT move the underlying object.  Pair it with ``|=`` on an
    identity field to actually move objects (the ``|=`` routes
    through ``rename_object``):
    ``.ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")``.

    For a whole-partition migration (every object, not just pools),
    reach for ``rename_partition`` — the cascade rewrites compound
    values like destination addresses and pool-member names too,
    which ``with_partition`` alone can't reach because they aren't
    standalone object identifiers.

    Raises ``BuiltinError`` when the new partition is empty.

    Related: ``partition``, ``basename``, ``rename``,
    ``rename_partition``.
    """,
    examples=(
        'with_partition("/Common/web_pool", "Tenant_A")  # -> "/Tenant_A/web_pool"',
        '.ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")',
    ),
    category="path",
    min_args=2,
    max_args=2,
)
def _builtin_with_partition(path: Any, partition: Any) -> str:
    s = _as_str(path, name="with_partition", arg=1)
    p = _as_str(partition, name="with_partition", arg=2)
    if not p:
        raise BuiltinError("with_partition: partition must not be empty")
    base = s.rsplit("/", 1)[-1]
    return f"/{p}/{base}"


@_register(
    "in_cidr",
    summary=(
        "Test whether an address (or destination) lies within a CIDR network.  "
        "Partition prefixes and ``:port`` suffixes on the address are ignored."
    ),
    signatures=("in_cidr(addr: string, network: string) -> boolean",),
    details="""
    Strips any partition prefix and ``:port`` suffix from *addr*,
    parses what's left as an IP, and tests for membership in
    *network*.  An unparseable address returns ``false`` (not an
    error) so the helper is safe to use as a stream filter without
    pre-validation.  An unparseable *network* raises
    ``BuiltinError`` — the network is supplied by the query author,
    so a typo there should fail loudly.

    Address-family mismatches return ``false``: an IPv4 host in an
    IPv6 network is just "not in the network", not an error.

    The route-domain portion of *addr* (``%5``) is ignored for the
    membership test — RDs don't take part in the prefix arithmetic.

    Related: ``ip``, ``net``, ``host``, ``route_domain``.
    """,
    examples=(
        'in_cidr("10.0.0.5", "10.0.0.0/8")              # -> true',
        'in_cidr("/Common/10.0.0.5:80", "10.0.0.0/8")   # -> true',
        '.ltm.virtual[] | select(in_cidr(.destination, "10.0.0.0/8")) | .name',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_in_cidr(addr: Any, network: Any) -> bool:
    addr_s = _as_str(addr, name="in_cidr", arg=1)
    net_s = _as_str(network, name="in_cidr", arg=2)
    _, host, _, _ = _split_destination(addr_s)
    try:
        ip = ipaddress.ip_address(host)
    except ValueError:
        return False
    try:
        net = ipaddress.ip_network(net_s, strict=False)
    except ValueError as exc:
        raise BuiltinError(f"in_cidr: invalid network {net_s!r}: {exc}") from exc
    if isinstance(ip, ipaddress.IPv4Address) and not isinstance(net, ipaddress.IPv4Network):
        return False
    if isinstance(ip, ipaddress.IPv6Address) and not isinstance(net, ipaddress.IPv6Network):
        return False
    return ip in net


@_register(
    "route_domain",
    summary=(
        "Return the route-domain number of a destination / address string "
        "(``10.0.0.1%5:80`` -> ``5``), or null when none is present."
    ),
    signatures=("route_domain(value: string) -> string | null",),
    details="""
    Extracts the ``%<rd>`` portion of a BIG-IP destination.  Returns
    the route domain as a string (not an integer) because RDs may be
    spelled with leading zeros or non-numeric tokens in some
    configs; cast to int when you need to compare numerically.

    Returns ``null`` when no route domain is present, so ``select(
    route_domain(.destination) | defined(.))`` filters VSes that
    explicitly bind to a non-default route domain.

    Related: ``with_route_domain`` (set / replace / strip),
    ``host``, ``port``.
    """,
    examples=(
        "route_domain(.destination)",
        'route_domain("10.0.0.1%5:80")            # -> "5"',
        'route_domain("/Common/10.0.0.1:80")      # -> null',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_route_domain(value: Any) -> str | None:
    s = _as_str(value, name="route_domain", arg=1)
    _, _, rd, _ = _split_destination(s)
    return rd or None


@_register(
    "with_route_domain",
    summary=(
        "Set, replace, or strip the route-domain on a destination / address.  "
        "Pass an empty string (or null) as the second argument to strip the "
        "route-domain entirely.  Partition prefix and port are preserved."
    ),
    signatures=("with_route_domain(value: string, rd: string | integer | null) -> string",),
    details="""
    Edits the route-domain portion of a destination string in place.
    Accepts an integer (``with_route_domain(.dest, 7)``), a string
    (``with_route_domain(.dest, "7")``), or null/empty-string to
    strip the route domain entirely.

    Partition prefix and port survive the edit unchanged — this
    helper only touches the ``%<rd>`` segment.

    Booleans are rejected (``with_route_domain(.dest, true)`` raises
    ``BuiltinError``) so accidental coercions don't produce
    nonsense addresses.

    Common pattern: ``.ltm.virtual[] | .destination |=
    with_route_domain(., 7)`` rebinds every VS to RD 7 in one
    statement.

    Related: ``route_domain`` (read), ``ip`` (rebase, preserves RD),
    ``host``, ``port``.
    """,
    examples=(
        "with_route_domain(.destination, 5)",
        'with_route_domain("/Common/10.0.0.1:80", 7)        # -> "/Common/10.0.0.1%7:80"',
        'with_route_domain("/Common/10.0.0.1%5:80", "")     # -> "/Common/10.0.0.1:80"',
        ".ltm.virtual[] | .destination |= with_route_domain(., 7)",
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_with_route_domain(value: Any, rd: Any) -> str:
    s = _as_str(value, name="with_route_domain", arg=1)
    partition, addr, _, port = _split_destination(s)
    if rd is None or rd == "":
        new_rd = ""
    elif isinstance(rd, bool):
        raise BuiltinError("with_route_domain: rd cannot be a boolean")
    elif isinstance(rd, int):
        new_rd = str(rd)
    elif isinstance(rd, str):
        new_rd = rd
    else:
        raise BuiltinError(
            f"with_route_domain: rd must be a string, integer, or null, got {_type_name(rd)}"
        )
    return _rebuild_destination(partition, addr, new_rd, port)


# ---------------------------------------------------------------------------
# IP / address classification (typed-layer helpers)
# ---------------------------------------------------------------------------
#
# Every builtin below routes its input through ``core.bigip.types``
# so IPv6 / FQDN / folder-nested destinations work without the
# caller having to think about parsing.


def _typed_address(value: Any, *, name: str, arg: int = 1):
    """Coerce *value* to a typed :class:`Address` (``IPAddress`` |
    ``FQDN``).

    Strings flow through ``Destination.try_parse`` first (so
    ``/Common/10.0.0.1:80`` extracts the host) and fall back to
    ``parse_address`` for bare hosts.  Returns ``None`` for inputs
    that can't be coerced.
    """
    from ..types import Destination, parse_address

    if value is None:
        return None
    s = _as_str(value, name=name, arg=arg)
    if not s:
        return None
    dest = Destination.try_parse(s)
    if dest is not None:
        return dest.address
    try:
        return parse_address(s)
    except ValueError:
        return None


def _typed_network(value: Any, *, name: str, arg: int = 1):
    """Coerce *value* to a typed :class:`Network`.

    Accepts integer CIDR (``10.0.0.0/24``) and dotted-quad netmask
    (``10.0.0.0/255.255.255.0``) — same shapes the parser does.
    """
    from ..types import Network

    if value is None:
        return None
    s = _as_str(value, name=name, arg=arg)
    if not s:
        return None
    return Network.try_parse(s)


@_register(
    "is_ipv4",
    summary="True when *value* parses as an IPv4 address.",
    signatures=("is_ipv4(value: string) -> boolean",),
    details="""
    Accepts a bare IPv4 (``10.0.0.1``) or a destination string
    (``/Common/10.0.0.1:80`` — the host portion is extracted).
    Returns ``false`` for IPv6, FQDN, or unparseable input.

    Pairs with :func:`is_ipv6` to branch on address family without
    pattern-matching the string.

    Related: ``is_ipv6``, ``is_fqdn``, ``is_private``,
    ``is_loopback``, ``is_unspecified``.
    """,
    examples=(
        "is_ipv4(.destination)",
        ".ltm.virtual[] | select(is_ipv4(.destination)) | .name",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_ipv4(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_ipv4")
    return isinstance(a, IPAddress) and a.is_ipv4


@_register(
    "is_ipv6",
    summary="True when *value* parses as an IPv6 address.",
    signatures=("is_ipv6(value: string) -> boolean",),
    details="""
    Accepts every documented F5 spelling — bare (``2001:db8::1``),
    bracketed (``[2001:db8::1]``), with ``.``-port
    (``[2001:db8::1].80`` / ``2001:db8::1.80``), with ``:``-port
    (``[2001:db8::1]:80``), partition-prefixed, folder-nested.
    Returns ``false`` for IPv4 / FQDN / unparseable input.

    Related: ``is_ipv4``, ``is_fqdn``.
    """,
    examples=(
        "is_ipv6(.destination)",
        ".ltm.virtual[] | select(is_ipv6(.destination)) | .name",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_ipv6(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_ipv6")
    return isinstance(a, IPAddress) and a.is_ipv6


@_register(
    "is_fqdn",
    summary="True when *value*'s host portion is an FQDN (not an IP).",
    signatures=("is_fqdn(value: string) -> boolean",),
    details="""
    Distinguishes FQDN pool members (``/Common/host.example.com:443``)
    from IP-based ones.  Returns ``false`` for IPv4 / IPv6 / empty
    / unparseable input.

    Useful for branching when a pool has a mix of IP and FQDN
    members — typically the FQDN form needs DNS-resolution checks
    while IP-form members get straight reachability checks.

    Related: ``is_ipv4``, ``is_ipv6``.
    """,
    examples=(
        "is_fqdn(.address)",
        ".ltm.pool[] | .members[] | select(is_fqdn(.address)) | .name",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_fqdn(value: Any) -> bool:
    from ..types import FQDN

    return isinstance(_typed_address(value, name="is_fqdn"), FQDN)


@_register(
    "is_private",
    summary="True when *value* is an RFC-1918 / RFC-4193 private IP.",
    signatures=("is_private(value: string) -> boolean",),
    details="""
    Classifies through Python's ``ipaddress`` stdlib —
    ``10.0.0.0/8``, ``172.16.0.0/12``, ``192.168.0.0/16`` for IPv4;
    ``fc00::/7`` for IPv6 ULAs; plus a handful of other "non-global"
    ranges per the IANA registries.

    Returns ``false`` for FQDN, public IPs, and unparseable input.

    Related: ``is_loopback``, ``is_unspecified``, ``in_cidr``.
    """,
    examples=(
        "is_private(.destination)",
        ".ltm.virtual[] | select(is_private(.destination)) | .name",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_private(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_private")
    return isinstance(a, IPAddress) and a.is_private


@_register(
    "is_loopback",
    summary="True when *value* is a loopback address (``127.0.0.0/8`` / ``::1``).",
    signatures=("is_loopback(value: string) -> boolean",),
    details="""
    Returns ``false`` for non-loopback IPs, FQDNs, and unparseable
    input.

    Related: ``is_private``, ``is_unspecified``.
    """,
    examples=(".ltm.virtual[] | select(is_loopback(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_loopback(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_loopback")
    return isinstance(a, IPAddress) and a.is_loopback


@_register(
    "is_unspecified",
    summary="True when *value* is the unspecified-host wildcard (``0.0.0.0`` / ``::``).",
    signatures=("is_unspecified(value: string) -> boolean",),
    details="""
    F5 uses ``0.0.0.0`` / ``::`` as the listen-on-any host wildcard
    on virtual servers.  This predicate filters those out cleanly:

    ``.ltm.virtual[] | select(is_unspecified(.destination)) | .name``

    Returns ``false`` for any non-wildcard IP, FQDN, or
    unparseable input.

    Related: ``is_wildcard_port`` (the partner for the port half),
    ``is_loopback``, ``is_private``.
    """,
    examples=(".ltm.virtual[] | select(is_unspecified(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_unspecified(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_unspecified")
    return isinstance(a, IPAddress) and a.is_unspecified


@_register(
    "is_wildcard_port",
    summary="True when *value*'s port portion is the wildcard (``any`` / ``*`` / ``0``).",
    signatures=("is_wildcard_port(value: string) -> boolean",),
    details="""
    F5 virtual-server destinations carrying port wildcards
    (``/Common/0.0.0.0:any`` / ``/Common/10.0.0.1:0``) match every
    incoming port; surface them with this predicate rather than
    matching a string suffix.

    Related: ``port``, ``is_unspecified`` (the host half).
    """,
    examples=(".ltm.virtual[] | select(is_wildcard_port(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_wildcard_port(value: Any) -> bool:
    from ..types import Destination

    if value is None:
        return False
    s = _as_str(value, name="is_wildcard_port", arg=1)
    dest = Destination.try_parse(s)
    return dest is not None and dest.port.is_any


@_register(
    "prefix_length",
    summary="Return the CIDR prefix length of a network string.",
    signatures=("prefix_length(value: string) -> integer | null",),
    details="""
    Accepts both integer CIDR (``10.0.0.0/24``) and dotted-quad
    netmask (``10.0.0.0/255.255.255.0``) — both render the same
    prefix length.  Returns ``null`` for inputs that aren't
    networks.

    Pairs with :func:`subnet_of` for CIDR algebra.

    Related: ``in_cidr``, ``subnet_of``.
    """,
    examples=(
        "prefix_length(.net.self[].address)",
        ".net.self[] | select(prefix_length(.address) >= 24) | .name",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_prefix_length(value: Any) -> int | None:
    n = _typed_network(value, name="prefix_length")
    if n is None:
        return None
    return n.prefix_length


@_register(
    "subnet_of",
    summary="True when *subnet* lies entirely inside *supernet*.",
    signatures=("subnet_of(subnet: string, supernet: string) -> boolean",),
    details="""
    Wraps ``ipaddress.IPv4Network.subnet_of`` /
    ``IPv6Network.subnet_of``.  Both arguments must be networks
    (CIDR or dotted-quad netmask form).  IPv4 ↔ IPv6 comparison
    returns ``false`` rather than raising — different families are
    never subsets.

    Related: ``in_cidr`` (single-host membership), ``prefix_length``.
    """,
    examples=(
        'subnet_of("10.1.0.0/16", "10.0.0.0/8")               # -> true',
        'subnet_of(.net.self[].address, "10.0.0.0/8")',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_subnet_of(subnet: Any, supernet: Any) -> bool:
    sub = _typed_network(subnet, name="subnet_of", arg=1)
    sup = _typed_network(supernet, name="subnet_of", arg=2)
    if sub is None or sup is None:
        return False
    try:
        return sub.network.subnet_of(sup.network)
    except TypeError:
        # Different families (IPv4 vs IPv6).
        return False


@_register(
    "overlaps",
    summary="True when two networks overlap (share at least one address).",
    signatures=("overlaps(net1: string, net2: string) -> boolean",),
    details="""
    Useful for finding self-IP / route-domain conflicts:
    ``.net.self[] | combinations(2) | select(overlaps(.[0].address,
    .[1].address))``.

    IPv4 ↔ IPv6 comparison returns ``false``.

    Related: ``subnet_of``, ``in_cidr``.
    """,
    examples=(
        'overlaps("10.0.0.0/24", "10.0.0.0/16")              # -> true',
        'overlaps("10.0.0.0/24", "10.1.0.0/24")              # -> false',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_overlaps(net1: Any, net2: Any) -> bool:
    a = _typed_network(net1, name="overlaps", arg=1)
    b = _typed_network(net2, name="overlaps", arg=2)
    if a is None or b is None:
        return False
    try:
        return a.network.overlaps(b.network)
    except TypeError:
        return False


@_register(
    "with_port",
    summary="Return *dest* with its port replaced by *port*.",
    signatures=("with_port(dest: string, port: integer | string) -> string",),
    details="""
    Preserves every other component of the destination — partition,
    folder, address, route-domain, IPv6 brackets, and the
    ``.``-vs-``:`` port separator.  ``port`` can be an integer
    (``443``), the wildcard string (``"any"`` / ``"*"`` / ``"0"``),
    or the empty string to strip the port entirely.

    Inverse of :func:`port`; pairs with ``with_partition`` /
    ``with_route_domain`` for full destination editing.

    Related: ``port``, ``with_host``, ``with_partition``,
    ``with_route_domain``.
    """,
    examples=(
        "with_port(.destination, 8443)",
        'with_port("/Common/10.0.0.1:80", "any")              # -> "/Common/10.0.0.1:any"',
        ".ltm.virtual[] | .destination |= with_port(., 443)",
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_with_port(dest: Any, port: Any) -> str:
    from ..types import Destination, Port

    s = _as_str(dest, name="with_port", arg=1)
    parsed = Destination.try_parse(s)
    if parsed is None:
        raise BuiltinError(f"with_port: cannot parse destination {s!r}")
    if port is None or port == "":
        new_port = Port(port=0, spelling="")
    elif isinstance(port, bool):
        raise BuiltinError("with_port: port cannot be a boolean")
    elif isinstance(port, int):
        if not 0 <= port <= 65535:
            raise BuiltinError(f"with_port: port out of range ({port})")
        new_port = Port(port=port, spelling=str(port))
    elif isinstance(port, str):
        new_port = Port.parse(port)
    else:
        raise BuiltinError(
            f"with_port: port must be a string, integer, or null, got {_type_name(port)}"
        )
    return str(
        Destination(
            address=parsed.address,
            port=new_port,
            folder=parsed.folder,
            route_domain=parsed.route_domain,
            ipv6_brackets=parsed.ipv6_brackets,
            port_separator=parsed.port_separator,
        )
    )


@_register(
    "with_host",
    summary="Return *dest* with its host replaced by *host*.",
    signatures=("with_host(dest: string, host: string) -> string",),
    details="""
    Preserves the partition, folder, route-domain, port, and IPv6
    bracket form; replaces only the address.  ``host`` may be an
    IPv4, IPv6, or FQDN string.

    Inverse of :func:`host`; pairs with :func:`with_port` for full
    destination editing.

    Related: ``host``, ``with_port``, ``with_partition``.
    """,
    examples=(
        'with_host(.destination, "10.0.0.2")',
        'with_host("/Common/10.0.0.1:80", "host.example.com")   # -> "/Common/host.example.com:80"',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_with_host(dest: Any, host: Any) -> str:
    from ..types import Destination, parse_address

    s = _as_str(dest, name="with_host", arg=1)
    h = _as_str(host, name="with_host", arg=2)
    parsed = Destination.try_parse(s)
    if parsed is None:
        raise BuiltinError(f"with_host: cannot parse destination {s!r}")
    try:
        new_addr = parse_address(h)
    except ValueError as exc:
        raise BuiltinError(f"with_host: cannot parse host {h!r}: {exc}") from exc
    # Switching IPv4 ↔ IPv6 — drop the bracket form and ``.``-port
    # separator if it no longer applies.
    from ..types import IPAddress

    new_brackets = parsed.ipv6_brackets
    new_separator = parsed.port_separator
    if not (isinstance(new_addr, IPAddress) and new_addr.is_ipv6):
        new_brackets = False
        new_separator = ":"
    return str(
        Destination(
            address=new_addr,
            port=parsed.port,
            folder=parsed.folder,
            route_domain=parsed.route_domain,
            ipv6_brackets=new_brackets,
            port_separator=new_separator,
        )
    )


@_register(
    "folder",
    summary="Return the folder portion of a TMSH path (``/Common/Application_X``).",
    signatures=("folder(value: string) -> string",),
    details="""
    Extracts the folder path from a full BIG-IP object path.  Bare
    partition (``/Common/pool``) → ``"/Common"`` (just the
    partition root); nested-folder (``/Common/iApps/Tenant.app/p``)
    → ``"/Common/iApps/Tenant.app"``.  Returns ``""`` for non-path
    input.

    Sibling to :func:`partition` (which returns just the partition
    name without the slash).

    Related: ``partition``, ``basename``, ``with_partition``,
    ``with_folder``.
    """,
    examples=(
        'folder(."full-path")',
        '.ltm.virtual[] | select(folder(."full-path") == "/Common/iApps/Tenant.app") | .name',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_folder(value: Any) -> str:
    from ..types import ObjectPath

    s = _as_str(value, name="folder", arg=1)
    obj = ObjectPath.try_parse(s)
    if obj is None:
        return ""
    return str(obj.folder)


@_register(
    "with_folder",
    summary="Return *path* with its folder portion replaced by *folder*.",
    signatures=("with_folder(path: string, folder: string) -> string",),
    details="""
    Replaces every segment from the leading slash up to (but not
    including) the leaf name.  ``folder`` may be a single
    partition (``/Common``) or a nested folder
    (``/Common/Application_X``); the leaf is kept exactly.

    Related: ``folder``, ``with_partition``, ``basename``.
    """,
    examples=(
        'with_folder("/Common/web_pool", "/Tenant_A")              # -> "/Tenant_A/web_pool"',
        'with_folder("/Common/iApps/old.app/pool_1", "/Common/iApps/new.app")  # -> "/Common/iApps/new.app/pool_1"',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_with_folder(path: Any, folder: Any) -> str:
    from ..types import Folder, ObjectPath

    p = _as_str(path, name="with_folder", arg=1)
    f = _as_str(folder, name="with_folder", arg=2)
    obj = ObjectPath.try_parse(p)
    if obj is None:
        raise BuiltinError(f"with_folder: cannot parse path {p!r}")
    new_folder = Folder.try_parse(f)
    if new_folder is None:
        raise BuiltinError(f"with_folder: cannot parse folder {f!r}")
    return str(ObjectPath(folder=new_folder, name=obj.name))


# ---------------------------------------------------------------------------
# Rename — single-object and whole-partition cascade
# ---------------------------------------------------------------------------


@_register(
    "rename",
    summary=(
        "Rename a BIG-IP object full-path and update every reference to it.  "
        "Routes through the same engine ``f5 rename`` uses (token-bounded "
        "regex substitution across the whole source, covering iRule body "
        "references and pool-member identifiers)."
    ),
    signatures=("rename(old: string, new: string) -> integer",),
    details="""
    Schedules a token-bounded source rewrite that replaces every
    occurrence of *old* with *new*.  The substitution is the same one
    ``rename_object`` performs:

    - The match is **token-bounded**, so renaming ``/Common/foo`` does
      not touch ``/Common/foobar`` or ``/Common/foo_extra``.
    - **References inside iRule bodies** are rewritten too —
      ``pool /Common/foo``, ``persist add ... /Common/foo``,
      ``class match ... /Common/foo``, and so on.  Short-name
      references (``foo`` instead of ``/Common/foo``) are *not*
      rewritten; they're unsafe to handle by regex.
    - **Pool-member identifiers** that embed the renamed name are
      rewritten (``/Common/foo:80`` → ``/Common/new:80``).

    Unlike the DSL form ``.<kind>["/Common/old"].name = "/Common/new"``
    (which raises when the LHS resolves to nothing), ``rename()`` is
    **tolerant**: a zero-occurrence outcome yields a no-op
    ``AppliedSource`` with no rename report.  The ``f5 rename`` CLI
    detects the no-op and surfaces it as ``warning: no occurrences
    of <old> found`` with exit code 1 — matching its historical
    behaviour.

    Pre-flight checks: empty old/new names raise ``BuiltinError``;
    ``old == new`` is a no-op that returns 0 without scheduling an
    edit.

    Common patterns:

    - ``f5 rename /Common/old /Common/new bigip.conf`` is exactly
      ``f5 query 'rename("/Common/old", "/Common/new")' bigip.conf``.
    - Chain with a property edit using ``;`` so the second statement
      sees the renamed object:
      ``rename("/Common/old", "/Common/new") ;
      .ltm.pool["/Common/new"].monitor = "/Common/tcp"``.

    Related: ``rename_partition`` (whole-partition cascade), the DSL
    form ``.<kind>[X].name = Y`` (strict variant — errors when X
    doesn't exist).
    """,
    examples=(
        'rename("/Common/old_pool", "/Common/new_pool")',
        'rename("/Common/log_rule", "/Common/audit_rule")',
        'rename("/Common/old", "/Common/new") ; .ltm.pool["/Common/new"].monitor = "/Common/tcp"',
    ),
    category="rename",
    min_args=2,
    max_args=2,
    with_ctx=True,
)
def _builtin_rename(old: Any, new: Any, *, ctx: Any) -> int:
    from ..grep import compute_grep
    from ..types import ObjectPath
    from .edit_plan import EditOp

    old_s = _as_str(old, name="rename", arg=1).strip()
    new_s = _as_str(new, name="rename", arg=2).strip()
    if not old_s:
        raise BuiltinError("rename: old name must not be empty")
    if not new_s:
        raise BuiltinError("rename: new name must not be empty")
    if old_s == new_s:
        return 0

    # Partition-visibility check.  When the rename CHANGES the
    # target's partition, every existing referrer must still be
    # able to see the new partition under the F5 visibility rules.
    # If any referrer would lose visibility, refuse the move with
    # an explicit list — the operator can drop the offending refs,
    # move the referrers too, or pick a different target partition.
    old_path = ObjectPath.try_parse(old_s)
    new_path = ObjectPath.try_parse(new_s)
    if old_path is not None and new_path is not None and old_path.partition != new_path.partition:
        report = compute_grep(
            sources={ctx.root.uri: ctx.root.source},
            configs={ctx.root.uri: ctx.root.config},
            pattern=old_s,
            use_regex=False,
            use_cidr=False,
            direction="reverse",
            max_depth=1,
            max_nodes=1024,
            include_body=False,
            recurse=True,
        )
        broken: list[str] = []
        for node in report.related:
            if node.full_path == old_s:
                continue
            referrer = ObjectPath.try_parse(node.full_path)
            if referrer is None:
                continue
            if not referrer.partition.can_see(new_path.partition):
                broken.append(node.full_path)
        if broken:
            shown = ", ".join(sorted(set(broken))[:10])
            extra = f" (+{len(set(broken)) - 10} more)" if len(set(broken)) > 10 else ""
            raise BuiltinError(
                f"rename: moving {old_s!r} to {new_s!r} would break "
                f"partition visibility for {len(set(broken))} referrer(s): "
                f"{shown}{extra}.  Move the referrer(s) to "
                f"{new_path.partition!s} first, drop the offending "
                f"references, or pick a target partition the referrers "
                f"can see."
            )

    ctx.edits.add(
        EditOp(
            source_uri=ctx.root.uri,
            object_path=old_s,
            object_kind="",
            field_name="name",
            operator="=",
            old_value=old_s,
            new_value=new_s,
            field_slot=None,
            stanza_slot=None,
            strict=False,
        )
    )
    return 1


@_register(
    "rename_partition",
    summary=(
        "Rename a BIG-IP partition by rewriting every textual occurrence "
        "of the ``/<old>/`` prefix across the whole source.  Token-bounded "
        "and covers object headers, references in config properties, "
        "destination address prefixes, pool-member identifiers, iRule "
        "body literals, and the ``auth partition`` stanza header."
    ),
    details="""
    A whole-partition migration: every textual ``/<old>/`` occurrence
    in the source becomes ``/<new>/`` in one atomic rewrite.  The
    pattern is token-bounded the same way ``rename`` is, so:

    - Neighbouring identifiers like ``/<old>Ext/...`` are not touched.
    - The trailing lookahead requires the next character to be the
      start of an identifier or address, so bare standalone
      occurrences of the partition name (which appear as property
      values in some kinds of objects) are not rewritten.

    Crucially, this covers **compound values** that ``rename`` cannot:

    - Destination addresses: ``destination /Common/10.10.0.5%5:443``
      — the prefix part of an address isn't a standalone object
      identifier, so ``rename`` won't touch it.  ``rename_partition``
      will.
    - Pool-member identifiers: ``/Common/n1%5:80``.
    - Bare ``/Common/`` mentions inside iRule body literals.

    The ``auth partition Common { ... }`` stanza header is also
    renamed when present — both halves of the migration land in one
    statement.

    Route domains, ports, and the bits inside compound values that
    don't reference the partition (the host address, the port
    number) are preserved exactly.

    Pre-flight checks: empty names raise ``BuiltinError``; old
    names containing ``/`` raise ``BuiltinError`` (pass bare
    partition names, not paths); names not matching
    ``[A-Za-z0-9_.-]+`` raise.  ``old == new`` is a no-op.

    The applier rejects mixing ``rename_partition`` with field edits
    in the *same* statement — the prefix rewrite shifts byte offsets
    and field-slot ranges captured at projection time would target
    the wrong span.  Split them with ``;`` and the runner applies
    each statement against the post-rewrite source.

    Returns the count of textual matches the cascade will land on
    (computed against the source as the builtin runs, before any
    edits apply).

    Related: ``rename`` (single-object), ``with_partition`` (string
    transform, doesn't migrate references).
    """,
    signatures=("rename_partition(old: string, new: string) -> integer",),
    examples=(
        'rename_partition("Common", "Tenant_A")',
        'rename_partition("staging", "prod")',
    ),
    category="rename",
    min_args=2,
    max_args=2,
    with_ctx=True,
)
def _builtin_rename_partition(old: Any, new: Any, *, ctx: Any) -> int:
    from .edit_plan import PrefixRewrite

    old_name = _as_str(old, name="rename_partition", arg=1).strip()
    new_name = _as_str(new, name="rename_partition", arg=2).strip()
    if not old_name or not new_name:
        raise BuiltinError("rename_partition: partition names must not be empty")
    if "/" in old_name or "/" in new_name:
        raise BuiltinError("rename_partition: pass bare partition names, not paths")
    if not re.fullmatch(r"[A-Za-z0-9_.\-]+", old_name) or not re.fullmatch(
        r"[A-Za-z0-9_.\-]+", new_name
    ):
        raise BuiltinError("rename_partition: partition names must match [A-Za-z0-9_.-]+")
    if old_name == new_name:
        return 0
    # F5 partition-visibility constraint: only ``/Common`` is
    # visible to every other partition.  Renaming the system
    # ``Common`` partition to a tenant name (or vice versa) is
    # ambiguous — every cross-partition reference now points at
    # something with different visibility rules, and the operator
    # may not have intended the consequence.  Refuse the rename
    # rather than silently breaking visibility for downstream
    # objects; the operator can drop the offending references
    # first (or use a per-object rename for the specific objects
    # they want to move).
    if old_name == "Common":
        raise BuiltinError(
            "rename_partition: refusing to rename /Common — "
            "tenant partitions reference /Common one-way (the "
            "F5 partition-visibility model), and renaming it "
            "would silently break every cross-partition reference.  "
            "Migrate the specific objects with rename(...) instead."
        )
    if new_name == "Common":
        raise BuiltinError(
            "rename_partition: refusing to rename a tenant partition to "
            "/Common — /Common cannot reference tenant partitions, "
            "so any cross-partition references in this config would "
            "be silently invalidated.  Use check_partition_visibility() "
            "first to audit existing references."
        )

    # ``/Old/...`` -> ``/New/...``: token-bounded so a longer name
    # ("/OldExt/...") isn't matched.  The trailing lookahead requires
    # an identifier or address character after the prefix, so bare
    # occurrences of the partition name on their own do not match.
    prefix_pattern = re.compile(rf"(?<![A-Za-z0-9_/.\-])/{re.escape(old_name)}/(?=[A-Za-z0-9_])")
    # ``auth partition Old { ... }`` — the standalone partition stanza.
    header_pattern = re.compile(
        rf"(?<![A-Za-z0-9_/.\-])(auth\s+partition\s+){re.escape(old_name)}"
        rf"(?![A-Za-z0-9_/.\-])"
    )

    ctx.edits.add_prefix(
        PrefixRewrite(
            source_uri=ctx.root.uri,
            label=f"partition /{old_name}/",
            pattern=prefix_pattern,
            replacement=f"/{new_name}/",
            human_new=f"/{new_name}/",
        )
    )
    ctx.edits.add_prefix(
        PrefixRewrite(
            source_uri=ctx.root.uri,
            label=f"auth partition {old_name}",
            pattern=header_pattern,
            replacement=rf"\g<1>{new_name}",
            human_new=f"auth partition {new_name}",
        )
    )

    # Return the total textual-match count across both rewrites — the
    # ``/Old/`` prefix occurrences (object headers, references,
    # compound values) plus the ``auth partition <Old>`` stanza
    # header rewrite when present.  Matches the stderr summary the
    # CLI prints after the rewrites apply.
    return len(prefix_pattern.findall(ctx.root.source)) + len(
        header_pattern.findall(ctx.root.source)
    )


@_register(
    "rename_folder",
    summary="Move every object from one folder path to another.",
    details="""
    The folder-level sibling of :func:`rename_partition`.  ``old``
    and ``new`` are folder paths (``/Common/iApps/Tenant.app`` /
    ``/Tenant_A/iApps/Tenant.app``) — every reference whose path
    starts with ``<old>/`` is rewritten to start with ``<new>/``.

    Cascades into every place a TMSH path appears in the source:

    - object stanza headers (``ltm pool /Common/iApps/old.app/p1``);
    - reference properties (``pool /Common/iApps/old.app/p1``);
    - destinations that embed the folder
      (``destination /Common/iApps/old.app/10.0.0.1:80``);
    - iRule body literals.

    Uses the same token-bounded prefix-cascade machinery
    ``rename_partition`` uses — so an unrelated path
    ``/Common/iApps/old.app.bak/p1`` doesn't accidentally match.

    Pre-flight checks: both arguments must be parseable folder
    paths (``/<partition>[/<segment>...]``); empty names raise
    ``BuiltinError``.  ``old == new`` is a no-op.

    Returns the count of textual matches the cascade landed on.

    Related: ``rename_partition`` (partition-level),
    ``rename`` (single-object), ``with_folder`` (string transform,
    doesn't migrate references), ``folder`` (extract folder).
    """,
    signatures=("rename_folder(old: string, new: string) -> integer",),
    examples=(
        'rename_folder("/Common/iApps/old.app", "/Common/iApps/new.app")',
        'rename_folder("/Common/iApps/Tenant.app", "/Tenant_A/iApps/Tenant.app")',
    ),
    category="rename",
    min_args=2,
    max_args=2,
    with_ctx=True,
)
def _builtin_rename_folder(old: Any, new: Any, *, ctx: Any) -> int:
    from ..types import Folder
    from .edit_plan import PrefixRewrite

    old_text = _as_str(old, name="rename_folder", arg=1).strip()
    new_text = _as_str(new, name="rename_folder", arg=2).strip()
    if not old_text or not new_text:
        raise BuiltinError("rename_folder: folder paths must not be empty")
    old_folder = Folder.try_parse(old_text)
    new_folder = Folder.try_parse(new_text)
    if old_folder is None:
        raise BuiltinError(f"rename_folder: cannot parse old folder {old_text!r}")
    if new_folder is None:
        raise BuiltinError(f"rename_folder: cannot parse new folder {new_text!r}")
    old_canonical = str(old_folder)
    new_canonical = str(new_folder)
    if old_canonical == new_canonical:
        return 0

    # Token-bounded: a longer folder path with the same prefix
    # (``/Common/iApps/old.app.bak/...``) must not match.  The
    # lookahead requires ``/`` (the next path segment) so we're
    # only matching exact folder-path tokens followed by a sub-path.
    prefix_pattern = re.compile(
        rf"(?<![A-Za-z0-9_/.\-]){re.escape(old_canonical)}/(?=[A-Za-z0-9_])"
    )
    ctx.edits.add_prefix(
        PrefixRewrite(
            source_uri=ctx.root.uri,
            label=f"folder {old_canonical}/",
            pattern=prefix_pattern,
            replacement=f"{new_canonical}/",
            human_new=f"{new_canonical}/",
        )
    )
    return len(prefix_pattern.findall(ctx.root.source))


# ---------------------------------------------------------------------------
# Object-path / graph predicates
# ---------------------------------------------------------------------------
#
# The ``ObjectPath`` typed value layer underpins these — every input
# parses through ``ObjectPath.try_parse`` first so folder-nested paths
# work the same way as flat ones.


@_register(
    "with_name",
    summary="Return *path* with its leaf name replaced by *name*.",
    signatures=("with_name(path: string, name: string) -> string",),
    details="""
    Preserves the partition + every folder segment; replaces only
    the final segment (the object's bare name).  Useful for
    relocating an object inside its existing folder context:
    ``with_name("/Common/iApps/Tenant.app/old_pool", "new_pool")``
    → ``"/Common/iApps/Tenant.app/new_pool"``.

    Related: ``basename`` (extract leaf), ``with_partition``,
    ``with_folder``.
    """,
    examples=(
        'with_name("/Common/old_pool", "new_pool")',
        'with_name(."full-path", "renamed")',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_with_name(path: Any, name: Any) -> str:
    from ..types import ObjectPath

    p = _as_str(path, name="with_name", arg=1)
    n = _as_str(name, name="with_name", arg=2).strip()
    if not n:
        raise BuiltinError("with_name: new name must not be empty")
    obj = ObjectPath.try_parse(p)
    if obj is None:
        raise BuiltinError(f"with_name: cannot parse path {p!r}")
    return str(ObjectPath(folder=obj.folder, name=n))


@_register(
    "in_partition",
    summary="True when *path* belongs to *partition*.",
    signatures=("in_partition(path: string, partition: string) -> boolean",),
    details="""
    Accepts both spellings of the partition argument: bare
    (``"Common"``) and slash-prefixed (``"/Common"``).  Returns
    ``false`` for inputs that aren't TMSH paths.

    Symbolic alternative to ``partition(.) == "Common"`` — reads
    better in filters and avoids the bare-name vs path-shape
    pitfall.

    Related: ``partition``, ``in_folder``.
    """,
    examples=(
        '.ltm.pool[] | select(in_partition(."full-path", "Common")) | .name',
        'in_partition("/Common/web_pool", "Common")',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_in_partition(path: Any, partition: Any) -> bool:
    from ..types import ObjectPath, Partition

    p = _as_str(path, name="in_partition", arg=1)
    part_text = _as_str(partition, name="in_partition", arg=2)
    obj = ObjectPath.try_parse(p)
    if obj is None:
        return False
    target = Partition.try_parse(part_text)
    if target is None:
        return False
    return obj.partition == target


@_register(
    "in_folder",
    summary="True when *path* lives at-or-below *folder*.",
    signatures=("in_folder(path: string, folder: string) -> boolean",),
    details="""
    Matches paths whose folder prefix equals *folder* OR has
    *folder* as an ancestor.  ``in_folder(
    "/Common/iApps/Tenant.app/pool_1", "/Common/iApps")`` →
    ``true``; ``in_folder("/Common/web_pool",
    "/Common/iApps")`` → ``false``.

    Symbolic alternative to ``startswith(folder(.), "/Common/iApps")``
    — does the right thing on folder boundaries (won't match
    ``/Common/iApps_bak/...``).

    Related: ``folder``, ``in_partition``, ``startswith``.
    """,
    examples=('.ltm.pool[] | select(in_folder(."full-path", "/Common/iApps")) | .name',),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_in_folder(path: Any, folder: Any) -> bool:
    from ..types import Folder, ObjectPath

    p = _as_str(path, name="in_folder", arg=1)
    f = _as_str(folder, name="in_folder", arg=2)
    obj = ObjectPath.try_parse(p)
    target_folder = Folder.try_parse(f)
    if obj is None or target_folder is None:
        return False
    # Match when target_folder is a prefix (incl. equal) of obj.folder.
    if obj.folder.partition != target_folder.partition:
        return False
    target_segs = target_folder.segments
    obj_segs = obj.folder.segments
    if len(target_segs) > len(obj_segs):
        return False
    return obj_segs[: len(target_segs)] == target_segs


@_register(
    "references_to",
    summary="Return every object in this config that references *path*.",
    signatures=("references_to(path: string) -> list",),
    details="""
    Walks the parsed BIG-IP config for the current document and
    returns every object whose body contains a token-bounded
    reference to *path*.  Routes through the same engine
    ``f5 grep`` uses, so the search picks up references in:

    - property values (``pool /Common/p``);
    - compound values (destination prefixes,
      pool-member partition prefixes, profile attachment lists);
    - iRule body command arguments (``pool $member`` /
      ``class match …`` / ``persist …``).

    Multi-file workspaces: only the current document's graph is
    walked, mirroring the per-file semantics of mutating queries.

    Related: ``refs``, ``referenced_by`` (object-relative graph
    forms — pass an object value, get its forward / reverse
    edges).
    """,
    examples=(
        'references_to("/Common/web_pool")',
        'count(references_to("/Common/log_irule"))',
    ),
    category="graph",
    min_args=1,
    max_args=1,
    with_ctx=True,
)
def _builtin_references_to(path: Any, *, ctx: Any) -> list[str]:
    from ..grep import compute_grep

    target = _as_str(path, name="references_to", arg=1).strip()
    if not target:
        return []
    report = compute_grep(
        sources={ctx.root.uri: ctx.root.source},
        configs={ctx.root.uri: ctx.root.config},
        pattern=target,
        use_regex=False,
        use_cidr=False,
        direction="reverse",
        max_depth=1,
        max_nodes=1024,
        include_body=False,
        recurse=True,
    )
    seen: list[str] = []
    for node in report.related:
        if node.full_path == target:
            continue
        if node.full_path not in seen:
            seen.append(node.full_path)
    return seen


@_register(
    "can_see",
    summary="True when *referrer_path*'s partition may reference *target_path*'s partition.",
    signatures=("can_see(referrer_path: string, target_path: string) -> boolean",),
    details="""
    F5 partition visibility is **directional**:

    - Objects in any partition may reference objects in ``/Common``
      (one-way visibility).
    - Objects in ``/Common`` may **not** reference objects in any
      tenant partition.
    - Cross-tenant references (``/Tenant_A/...`` ↔ ``/Tenant_B/...``)
      are **not** allowed.
    - Same partition is always visible to itself.

    Use this predicate to validate that a proposed rename or
    cross-config reference is legal *before* applying it.  Example:
    "find every iRule reference that would break partition
    visibility":

    ``.ltm.rule[] | .refs.pools[] | select(not can_see(.., .))``

    Related: ``partition``, ``in_partition``,
    ``check_partition_visibility``.
    """,
    examples=(
        'can_see("/Tenant_A/vs1", "/Common/web_pool")  # true — Tenant_A can see /Common',
        'can_see("/Common/vs1", "/Tenant_A/web_pool")  # false — /Common cannot see /Tenant_A',
        'can_see("/Tenant_A/vs1", "/Tenant_B/web_pool")  # false — cross-tenant',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_can_see(referrer_path: Any, target_path: Any) -> bool:
    from ..types import ObjectPath

    r_text = _as_str(referrer_path, name="can_see", arg=1)
    t_text = _as_str(target_path, name="can_see", arg=2)
    r = ObjectPath.try_parse(r_text)
    t = ObjectPath.try_parse(t_text)
    if r is None or t is None:
        return False
    return r.partition.can_see(t.partition)


@_register(
    "check_partition_visibility",
    summary="Return every reference in this config that violates F5 partition visibility rules.",
    signatures=("check_partition_visibility() -> list",),
    details="""
    Walks the parsed config and surfaces every reference whose
    *referrer* partition can't see the *target* partition under the
    F5 partition-visibility rules (see :func:`can_see`).  Returns a
    list of ``"<referrer> -> <target>"`` strings — empty list when
    every reference is legal.

    Used to validate a config before applying a partition-level
    refactor, or to audit a config that was hand-edited and may
    have grown invalid cross-partition refs over time.

    Related: ``can_see``, ``references_to``, ``rename_partition``.
    """,
    examples=(
        "check_partition_visibility()",
        "count(check_partition_visibility())  # 0 → config is partition-clean",
    ),
    category="graph",
    min_args=0,
    max_args=0,
    with_ctx=True,
)
def _builtin_check_partition_visibility(*, ctx: Any) -> list[str]:
    from ..grep import compute_grep
    from ..types import ObjectPath

    violations: list[str] = []
    seen: set[tuple[str, str]] = set()
    # For each typed-projectable object in the config, run a forward
    # grep to find every other object it references; flag pairings
    # where the referrer's partition can't see the target's.
    cfg = ctx.root.config
    from dataclasses import fields

    for fld in fields(cfg):
        if fld.name == "generic_objects":
            continue
        kind_dict = getattr(cfg, fld.name)
        if not isinstance(kind_dict, dict):
            continue
        for referrer_path in kind_dict:
            referrer = ObjectPath.try_parse(referrer_path)
            if referrer is None:
                continue
            report = compute_grep(
                sources={ctx.root.uri: ctx.root.source},
                configs={ctx.root.uri: ctx.root.config},
                pattern=referrer_path,
                use_regex=False,
                use_cidr=False,
                direction="forward",
                max_depth=1,
                max_nodes=1024,
                include_body=False,
                recurse=True,
            )
            for node in report.related:
                if node.full_path == referrer_path:
                    continue
                target = ObjectPath.try_parse(node.full_path)
                if target is None:
                    continue
                if referrer.partition.can_see(target.partition):
                    continue
                key = (referrer_path, node.full_path)
                if key in seen:
                    continue
                seen.add(key)
                violations.append(f"{referrer_path} -> {node.full_path}")
    return sorted(violations)


# ---------------------------------------------------------------------------
# String helpers
# ---------------------------------------------------------------------------


@_register(
    "length",
    summary="Length of a string, list, stream, or object's field map.",
    signatures=("length(value: any) -> integer",),
    details="""
    Returns the size of *value*:

    - **string** / :class:`PathRef`: character count of the string.
    - **list** / **stream**: number of items.
    - **object**: number of TMSH fields (uncommonly used; mostly for
      introspection of unknown kinds).
    - **null**: returns 0.

    Raises ``BuiltinError`` for any other type (numbers, booleans).

    Pairs naturally with comparisons for predicates: ``select(.rules
    | length > 0)`` keeps every VS that has at least one attached
    iRule.

    Related: ``count`` (alias for list/stream only).
    """,
    examples=(
        "length(.rules)",
        ".rules | length",
        ".ltm.virtual[] | select(.rules | length > 0) | .name",
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_length(value: Any) -> int:
    if value is None:
        return 0
    if isinstance(value, (str, list, tuple, Stream)):
        return len(value)
    if isinstance(value, PathRef):
        return len(value.full_path)
    if isinstance(value, ObjectRef):
        return len(value.fields)
    if isinstance(value, dict):
        return len(value)
    raise BuiltinError(f"length: cannot take length of {_type_name(value)}")


@_register(
    "str",
    summary="Convert any scalar to its string form.",
    signatures=("str(value: any) -> string",),
    details="""
    Coerces a scalar value to its string representation.  Useful for
    building report-style output where a number or boolean needs to
    appear next to text:
    ``.ltm.pool[] | .name + ": " + str(count(.members)) + " members"``.

    The ``+`` operator also auto-coerces scalars when one side is
    already a string, so ``str()`` is typically only needed when
    both sides are non-strings (e.g. building a key out of two
    numbers).

    Rendering:

    - **string** / :class:`PathRef`: returned as-is (PathRef → full-path).
    - **integers** and **floats**: their decimal form.
    - **booleans**: ``"true"`` / ``"false"``.
    - **null**: ``"null"``.

    Raises ``BuiltinError`` for objects, lists, and streams — those
    have no single-line canonical form and the user should pick
    explicit fields instead.

    Related: ``+`` (string concat coerces scalars), ``length``,
    ``basename``.
    """,
    examples=(
        '.ltm.pool[] | .name + ": " + str(count(.members))',
        "str(42)",
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_str(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, PathRef):
        return value.full_path
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    raise BuiltinError(
        f"str: cannot stringify {_type_name(value)} — pick a scalar field "
        f"(e.g. ``.name`` / ``.full-path``) or use ``--json`` for full objects"
    )


@_register(
    "startswith",
    summary="Test whether a string starts with a prefix.",
    signatures=("startswith(value: string, prefix: string) -> boolean",),
    details="""
    Returns ``true`` when *value* begins with *prefix*.  Accepts
    :class:`PathRef` for either argument (compared via the
    ``full_path``), so ``startswith(.pool, "/Common/")`` works even
    though ``.pool`` is a path-ref, not a plain string.

    Use ``match`` when you need pattern-based matching.

    Related: ``endswith``, ``contains``, ``match``.
    """,
    examples=(
        'startswith(.name, "vs_prod_")',
        '.ltm.virtual[] | select(startswith(.name, "vs_dev_")) | .name',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_startswith(value: Any, prefix: Any) -> bool:
    return _as_str(value, name="startswith", arg=1).startswith(
        _as_str(prefix, name="startswith", arg=2)
    )


@_register(
    "endswith",
    summary="Test whether a string ends with a suffix.",
    signatures=("endswith(value: string, suffix: string) -> boolean",),
    details="""
    Returns ``true`` when *value* ends with *suffix*.  Accepts
    :class:`PathRef` for either argument; compared via ``full_path``.

    Related: ``startswith``, ``contains``, ``match``.
    """,
    examples=(
        'endswith(.name, "_pool")',
        '.ltm.virtual[] | select(endswith(.destination, ":443"))',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_endswith(value: Any, suffix: Any) -> bool:
    return _as_str(value, name="endswith", arg=1).endswith(_as_str(suffix, name="endswith", arg=2))


@_register(
    "contains",
    summary="Test whether a string contains a substring, or a list contains a value.",
    signatures=(
        "contains(value: string, needle: string) -> boolean",
        "contains(value: list, needle: any) -> boolean",
    ),
    details="""
    Overloaded by the type of *value*:

    - When *value* is a **string** (or :class:`PathRef`), tests
      substring membership: ``contains(.destination, ":443")``.
    - When *value* is a **list / stream** (such as ``.rules`` —
      a list of :class:`PathRef`), tests element membership.
      :class:`PathRef` items and string needles are compared on
      their ``full_path`` so ``contains(.rules,
      "/Common/log_rule")`` works against the streamed list of
      path-refs.

    Raises ``BuiltinError`` if *value* is neither a string nor a
    list-like value.

    Related: ``startswith``, ``endswith``, ``match``, ``any`` /
    ``all`` (for more general predicates over a stream).
    """,
    examples=(
        'contains(.destination, ":443")',
        'contains(.rules, "/Common/log_rule")',
        '.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_contains(value: Any, needle: Any) -> bool:
    def _eq(item: Any, target: Any) -> bool:
        a = item.full_path if isinstance(item, PathRef) else item
        b = target.full_path if isinstance(target, PathRef) else target
        return a == b

    if isinstance(value, str):
        return _as_str(needle, name="contains", arg=2) in value
    if isinstance(value, (list, tuple, Stream)):
        items = value.items if isinstance(value, Stream) else value
        return any(_eq(item, needle) for item in items)
    if isinstance(value, PathRef):
        return _as_str(needle, name="contains", arg=2) in value.full_path
    raise BuiltinError(f"contains: cannot search inside {_type_name(value)}")


@_register(
    "match",
    summary="Regex-match a string; returns true when the pattern matches anywhere.",
    signatures=("match(value: string, pattern: string) -> boolean",),
    details="""
    Tests whether *pattern* (a Python regex) matches anywhere in
    *value* (semantically ``re.search``, not ``re.match``).  Use
    ``^`` / ``$`` to anchor.

    **jq users note.**  This DSL's ``match`` is a *boolean
    predicate* — it corresponds to jq's ``test(pattern)`` builtin,
    not jq's ``match(pattern)`` (which returns rich match objects
    with capture groups, byte offsets, and named groups).  This DSL
    has no equivalent of jq's match-object output; if you need
    capture groups, use ``sub`` / ``gsub`` with a replacement
    template instead.

    An invalid regex raises ``BuiltinError`` with the underlying
    ``re.error`` reason — the pattern comes from the query author,
    so a typo should fail loudly.

    **Trust boundary.** ``match`` / ``sub`` / ``gsub`` and the
    ``[~"pattern"]`` regex subscript route their patterns through a
    central guard that caps pattern length and refuses obvious
    catastrophic-backtracking shapes (``(a+)+`` etc.).  Local CLI
    use is trusted (the query author is the operator); the same
    guard makes it safe to expose the DSL through MCP / chat /
    editor command surfaces where the pattern can come from
    untrusted input.  See ``_safe_regex_compile`` for the exact
    shape filter.

    For pure prefix/suffix or substring tests, prefer ``startswith``
    / ``endswith`` / ``contains`` — they're cheaper and read better.

    Note: the **regex subscript** form ``.ltm.virtual["~pattern"]``
    is a separate, more efficient mechanism for filtering keys
    inside a container — reach for ``match`` when you need to test
    a *value* against a pattern, and for the subscript when you're
    selecting *keys*.

    Related: ``sub``, ``gsub``, ``startswith``, ``endswith``,
    ``contains``.
    """,
    examples=(
        'match(.name, "^vs_prod_.*")',
        '.ltm.virtual[] | select(match(.destination, ":(80|443)$")) | .name',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_match(value: Any, pattern: Any) -> bool:
    s = _as_str(value, name="match", arg=1)
    p = _as_str(pattern, name="match", arg=2)
    rx = _safe_regex_compile(p, name="match")
    return rx.search(s) is not None


@_register(
    "sub",
    summary="Replace the first regex match in a string.",
    signatures=("sub(value: string, pattern: string, replacement: string) -> string",),
    details="""
    Replaces the **first** occurrence of *pattern* in *value* with
    *replacement* and returns the new string.  *pattern* is a Python
    regex; *replacement* may use ``\\1`` / ``\\g<name>`` backrefs.

    Use ``gsub`` to replace every match instead.  An invalid pattern
    raises ``BuiltinError``.

    Pairs naturally with ``|=`` to rewrite a property in place:
    ``.ltm.virtual[].name |= sub(., "^vs_dev_", "vs_qa_")``.  When
    the LHS is a stream of identity-field paths, each match is
    rewritten through ``rename_object`` — references update along
    with the headers.

    Related: ``gsub``, ``match``, ``rename`` (for full-path
    identity renames the engine already understands).
    """,
    examples=(
        'sub(.name, "^vs_dev_", "vs_qa_")',
        '.ltm.virtual[].destination |= sub(., ":443$", ":8443")',
    ),
    category="string",
    min_args=3,
    max_args=3,
)
def _builtin_sub(value: Any, pattern: Any, repl: Any) -> str:
    s = _as_str(value, name="sub", arg=1)
    p = _as_str(pattern, name="sub", arg=2)
    r = _as_str(repl, name="sub", arg=3)
    rx = _safe_regex_compile(p, name="sub")
    return rx.sub(r, s, count=1)


@_register(
    "gsub",
    summary="Replace every regex match in a string.",
    signatures=("gsub(value: string, pattern: string, replacement: string) -> string",),
    details="""
    Like ``sub`` but replaces **every** occurrence of *pattern* in
    *value*.  Useful for blanket string rewrites inside iRule bodies
    or data-group values.

    For object full-path renames, prefer ``rename`` or
    ``rename_partition`` over a raw ``gsub`` — those route through a
    token-bounded engine that won't touch substring collisions or
    short-name references in unsafe contexts.

    Related: ``sub``, ``match``, ``rename``, ``rename_partition``.
    """,
    examples=(
        'gsub(.body, "/Common/old_", "/Common/new_")',
        '.ltm.virtual[].destination |= gsub(., "%5", "%7")  # bulk RD change',
    ),
    category="string",
    min_args=3,
    max_args=3,
)
def _builtin_gsub(value: Any, pattern: Any, repl: Any) -> str:
    s = _as_str(value, name="gsub", arg=1)
    p = _as_str(pattern, name="gsub", arg=2)
    r = _as_str(repl, name="gsub", arg=3)
    rx = _safe_regex_compile(p, name="gsub")
    return rx.sub(r, s)


@_register(
    "split",
    summary="Split a string on a separator.  Returns a list.",
    signatures=("split(value: string, separator: string) -> list[string]",),
    details="""
    Splits *value* on every occurrence of *separator*, returning a
    Python list of substrings.  The separator is not a regex — use
    a literal string.

    Common pattern: project a single string field, split it, and
    extract a component.  ``.ltm.virtual[].destination | split(., ":")
    | last(.)`` projects the port part of every destination.

    Related: ``join`` (the inverse), ``sub`` / ``gsub`` (for regex
    rewrites).
    """,
    examples=(
        'split(.destination, ":")',
        'split(.destination, ":") | last(.)        # port portion',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_split(value: Any, sep: Any) -> list[str]:
    return _as_str(value, name="split", arg=1).split(_as_str(sep, name="split", arg=2))


@_register(
    "join",
    summary="Join a list of strings with a separator.",
    signatures=("join(values: list, separator: string) -> string",),
    details="""
    Joins a list (or stream) of strings into one string, separated
    by *separator*.  :class:`PathRef` items are coerced to their
    ``full_path``, so ``join(.rules, ", ")`` works on the streamed
    list of attached iRule references.

    Useful for ad-hoc reports: ``.ltm.virtual[] | "\\(.name): \\(join
    (.rules, ", "))"`` (when string interpolation lands) or
    ``join(map(.name, .ltm.virtual[]), "\\n")`` to flatten a stream
    of names.

    Related: ``split``, ``map``, ``sort``.
    """,
    examples=(
        'join(.rules, ", ")',
        'join(sort([.ltm.virtual[].name]), ", ")',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_join(values: Any, sep: Any) -> str:
    items = _as_sequence(values, name="join", arg=1)
    s = _as_str(sep, name="join", arg=2)
    return s.join(_as_str(v, name="join", arg=1) for v in items)


@_register(
    "upcase",
    summary="Uppercase a string.",
    signatures=("upcase(value: string) -> string",),
    details="""
    Returns *value* with every ASCII letter converted to uppercase.
    Accepts :class:`PathRef`; the result is a plain string (the path
    is normalised).  Use locale-aware casing helpers in Python if
    you need them — this wrapper just calls ``str.upper``.

    Related: ``downcase``.
    """,
    examples=(
        "upcase(.name)",
        'upcase("vs_prod_web")                    # -> "VS_PROD_WEB"',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_upcase(value: Any) -> str:
    return _as_str(value, name="upcase", arg=1).upper()


@_register(
    "downcase",
    summary="Lowercase a string.",
    signatures=("downcase(value: string) -> string",),
    details="""
    Returns *value* with every ASCII letter converted to lowercase.
    Accepts :class:`PathRef`.

    Related: ``upcase``.
    """,
    examples=(
        "downcase(.name)",
        'downcase("VS_PROD_WEB")                  # -> "vs_prod_web"',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_downcase(value: Any) -> str:
    return _as_str(value, name="downcase", arg=1).lower()


# ---------------------------------------------------------------------------
# Stream / list helpers
# ---------------------------------------------------------------------------


@_register(
    "keys",
    summary="Return the field names of an object as a sorted list.",
    signatures=("keys(value: object) -> list[string]",),
    details="""
    Returns the field-name keys of an :class:`ObjectRef` (or a plain
    ``dict``) as a sorted list.  Useful for introspecting unfamiliar
    object kinds or for projecting "which fields does each kind
    expose?".

    Returns the keys, not the values — pair with ``values`` (or just
    index back through the object) to fetch the values too.

    Raises ``BuiltinError`` for non-object inputs.

    Related: ``values``, ``length``, ``type``.
    """,
    examples=(
        "keys(.ltm.virtual.web_vs)                # all field names of one VS",
        "[.ltm.virtual[]] | first | keys          # discover the VS field set",
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_keys(value: Any) -> list[str]:
    if isinstance(value, ObjectRef):
        return sorted(value.fields)
    if isinstance(value, dict):
        return sorted(value)
    raise BuiltinError(f"keys: argument 1 must be an object, got {_type_name(value)}")


@_register(
    "values",
    summary="Return the field values of an object as a list.",
    signatures=("values(value: object) -> list",),
    details="""
    Returns the values of an :class:`ObjectRef`'s fields, ordered by
    sorted field name.  Pairs with ``keys`` for matched
    ``(name, value)`` traversal.

    The returned list mixes types — most BIG-IP objects carry a mix
    of strings, path-refs, and nested lists — so subsequent
    operations should be type-aware (``select(. != "")`` etc.).

    Raises ``BuiltinError`` for non-object inputs.

    Related: ``keys``, ``length``.
    """,
    examples=(
        "values(.ltm.virtual.web_vs)",
        ".ltm.virtual.web_vs | values | map(type)   # type signature of one VS",
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_values(value: Any) -> list[Any]:
    if isinstance(value, ObjectRef):
        return [value.fields[k] for k in sorted(value.fields)]
    if isinstance(value, dict):
        return [value[k] for k in sorted(value)]
    raise BuiltinError(f"values: argument 1 must be an object, got {_type_name(value)}")


@_register(
    "first",
    summary="Return the first item of a list or stream, or null when empty.",
    signatures=("first(value: list | stream) -> any",),
    details="""
    Returns the first element of a list or stream.  Returns ``null``
    (not an error) when the input is empty, so it's safe to apply to
    fields that may have no entries (``first(.rules)`` on a VS with
    no attached iRules returns null).

    Useful in combination with sorting / unique-ing to pick the
    "smallest" or "first by name" entry.

    Related: ``last``, ``count``, ``length``, ``sort``.
    """,
    examples=(
        "first(.rules)",
        "[.ltm.virtual[].name] | sort | first     # alphabetical first VS",
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_first(value: Any) -> Any:
    items = _as_sequence(value, name="first", arg=1)
    return items[0] if items else None


@_register(
    "last",
    summary="Return the last item of a list or stream, or null when empty.",
    signatures=("last(value: list | stream) -> any",),
    details="""
    Returns the last element of a list or stream.  Returns ``null``
    when the input is empty.  Idiomatic for splitting "address:port"
    style destinations: ``split(.destination, ":") | last``.

    Related: ``first``, ``count``, ``sort``.
    """,
    examples=(
        "last(.rules)",
        'split(.destination, ":") | last(.)        # port portion',
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_last(value: Any) -> Any:
    items = _as_sequence(value, name="last", arg=1)
    return items[-1] if items else None


@_register(
    "count",
    summary="Count the items in a list or stream.",
    signatures=("count(value: list | stream) -> integer",),
    details="""
    Alias for ``length`` restricted to lists and streams.  Reads
    naturally in filter prose: ``select(.rules | count > 0)``.

    Related: ``length``.
    """,
    examples=(
        "[.ltm.virtual[]] | count                 # number of VSes",
        ".ltm.virtual[] | select(.rules | count > 0) | .name",
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_count(value: Any) -> int:
    return len(_as_sequence(value, name="count", arg=1))


@_register(
    "unique",
    summary="Return the unique items of a list, preserving first-seen order.",
    signatures=("unique(value: list | stream) -> list",),
    details="""
    De-duplicates a list or stream while preserving the original
    order of first occurrence.  :class:`PathRef` items are compared
    on their ``full_path``, so a stream that pulls the same pool
    reference from many VSes collapses to one entry.

    Unhashable items (rare — usually nested lists) fall back to a
    linear scan, so worst-case is O(n^2); for the typical case of
    strings, integers, and path-refs it's O(n).

    Pairs nicely with ``sort`` for stable de-duplicated output:
    ``[.ltm.virtual[].pool] | unique | sort``.

    Related: ``sort``, ``count``, ``map``.
    """,
    examples=(
        "[.ltm.virtual[].pool] | unique           # every distinct default pool",
        "[.ltm.virtual[].name | partition(.)] | unique  # used partitions",
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_unique(value: Any) -> list[Any]:
    items = _as_sequence(value, name="unique", arg=1)
    seen: set = set()
    out: list[Any] = []
    for item in items:
        key = item.full_path if isinstance(item, PathRef) else item
        if isinstance(key, list):
            key = tuple(key)
        try:
            if key in seen:
                continue
            seen.add(key)
        except TypeError:
            # Unhashable — fall back to linear scan.
            if any(item == prior for prior in out):
                continue
        out.append(item)
    return out


@_register(
    "sort",
    summary="Return a sorted list.  Strings sort lexicographically; numbers numerically.",
    signatures=("sort(value: list | stream) -> list",),
    details="""
    Sorts the items of a list or stream and returns a list.
    :class:`PathRef` items sort on their ``full_path``.  Heterogeneous
    types in the same list raise — sort what comes back from a
    projection (always one type) rather than mixed object/scalar
    streams.

    Stable (Python's ``sorted`` is).  Use the list-literal collection
    idiom ``[.X[].name] | sort`` to gather a stream from ``[]`` before
    sorting — bare ``... | sort`` after a stream would sort each item
    individually, not the stream as a whole.

    Related: ``unique``, ``first``, ``last``.
    """,
    examples=(
        "[.ltm.virtual[].name] | sort",
        "[.ltm.virtual[].pool] | unique | sort    # sorted distinct pools",
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_sort(value: Any) -> list[Any]:
    items = _as_sequence(value, name="sort", arg=1)
    return sorted(
        items,
        key=lambda v: v.full_path if isinstance(v, PathRef) else v,
    )


@_register(
    "any",
    summary="True when at least one item of a list or stream is truthy.",
    signatures=("any(value: list | stream) -> boolean",),
    details="""
    Tests whether **any** item in a list or stream is truthy.
    Truthy means non-null, non-empty-string, non-empty-collection,
    and non-zero — same conventions as ``select``.

    Used most often with a per-item predicate piped through a
    stream:
    ``any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))``
    is "does any member's address lie in 10/8?".  The pipe iterates
    the stream of addresses (each becomes ``.``), produces a stream
    of booleans, and ``any`` collapses it.

    Note on ``map``: piping a stream into ``map(predicate)`` invokes
    ``map`` once **per item** — each call returns a single-element
    list ``[predicate(item)]``.  ``any`` flattens one level of
    list-of-lists so ``any(stream | map(predicate))`` Just Works,
    but the predicate form (``any(stream | predicate)``) is the
    idiomatic shape.

    Short-circuits — stops at the first truthy item.

    Related: ``all``, ``select``, ``map``.
    """,
    examples=(
        'any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))',
        '.ltm.virtual[] | select(any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))) | .name',
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_any(value: Any) -> bool:
    return any(_truthy(v) for v in _flatten_one_level(value, name="any"))


@_register(
    "all",
    summary="True when every item of a list or stream is truthy.",
    signatures=("all(value: list | stream) -> boolean",),
    details="""
    Tests whether **every** item in a list or stream is truthy.
    Short-circuits on the first falsy item.  An empty input returns
    ``true`` (vacuous truth — there's no falsy item to find).

    Common pattern: validate an invariant across the config —
    ``all(.ltm.virtual[].pool | startswith(., "/Common/"))``
    is "are all default pools in Common?".  Pipe iterates the stream
    of pools, ``startswith`` runs per item, ``all`` collapses.

    Related: ``any``, ``select``, ``map``.
    """,
    examples=(
        'all(.ltm.virtual[].pool | startswith(., "/Common/"))',
        'all(.ltm.virtual[].pool | . != "")          # every VS has a default pool?',
    ),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_all(value: Any) -> bool:
    return all(_truthy(v) for v in _flatten_one_level(value, name="all"))


# ``select`` and ``map`` are special forms — they need to evaluate their
# argument once per input value, with ``.`` re-bound to that value.  The
# evaluator handles the actual binding loop; we just declare the spec
# here so the dispatch table and the help text stay symmetric.


@_register(
    "select",
    summary="Drop the current value unless the body evaluates to a truthy result.",
    signatures=("select(body) -> any | drop",),
    details="""
    **Special form.**  ``select`` is the filter primitive — for each
    input value, it evaluates *body* against that value (with ``.``
    re-bound to the current item) and emits the current value
    unchanged when the result is truthy, dropping it otherwise.

    Truthy values: non-null, non-empty-string, non-empty-list /
    -stream, non-zero numbers, true booleans, non-empty path-refs.

    Typical use is inside a pipeline that streams objects:
    ``.ltm.virtual[] | select(.pool != "")`` keeps only VSes with a
    default pool.  Chain multiple ``select(...)`` to AND predicates
    together; use ``or`` inside one body to OR.

    Unlike most builtins, *body* may be any expression (not just a
    value) — it's the unevaluated AST and is re-evaluated per item.
    That makes ``select`` the source of every conditional flow in
    the DSL: filter, partition, branch.

    Related: ``map`` (transform every item instead of filtering),
    ``any``, ``all``, ``not``.
    """,
    examples=(
        '.ltm.virtual[] | select(.pool != "") | .name',
        '.ltm.virtual[] | select(startswith(.name, "vs_prod_"))',
        '.ltm.virtual[] | select(in_cidr(.destination, "10.0.0.0/8"))',
        ".ltm.virtual[] | select(.rules | count > 0 and .rules | count < 5)",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_select(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("select must be evaluated through the evaluator")


@_register(
    "map",
    summary="Apply the body to every item, returning the list of results.",
    signatures=("map(body) -> list",),
    details="""
    **Special form.**  ``map`` is the transform primitive — for each
    item of the *input* (which must be a list / stream), it
    evaluates *body* with ``.`` re-bound to that item and collects
    the results into a list.

    Output cardinality matches jq's ``map(f) == [.[] | f]`` rule:
    each *body* invocation flattens through the same machinery the
    pipe uses, so a body that produces

    - one value contributes one element (the common case);
    - a stream contributes every stream item;
    - the ``select`` drop sentinel contributes zero elements
      (``map(select(predicate))`` is the canonical filter idiom).

    So ``map`` is many-to-many in general, not strictly one-to-one.

    The body can be any expression: a field projection, a builtin
    call, a multi-stage pipeline, an arithmetic expression — `.` is
    the current item throughout.

    Common patterns:

    - **Project a field of a list**: ``.rules | map(basename(.))`` —
      ``.rules`` is a list, the pipe passes it whole, ``map``
      iterates it.
    - **Filter + transform**: ``map(select(.address) | .name)`` —
      drops items whose ``address`` is falsey, projects ``.name``
      on the survivors.  Zero outputs per dropped item.
    - **Predicate over a stream** (don't use ``map`` for this): pipe
      the stream through the predicate instead —
      ``.pool.members[].address | in_cidr(., "10.0.0.0/8")``
      yields a stream of booleans suitable for ``any`` / ``all``.
    - **Compose with sort + unique on a stream**: wrap with a list
      literal first so subsequent stages see one list:
      ``[.ltm.virtual[].name | partition(.)] | unique | sort``.

    Related: ``select``, ``any``, ``all``, ``unique``, ``sort``.
    """,
    examples=(
        ".rules | map(basename(.))",
        "[.ltm.virtual[].name | partition(.)] | unique | sort",
        'any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))',
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_map(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("map must be evaluated through the evaluator")


# ---------------------------------------------------------------------------
# Value introspection
# ---------------------------------------------------------------------------


@_register(
    "kind",
    summary="Return the TMSH kind of an object (``ltm virtual``, ``ltm pool``, …).",
    signatures=("kind(value: object) -> string",),
    details="""
    Returns the TMSH module+type string an :class:`ObjectRef` belongs
    to.  For a :class:`PathRef` returns the ``expected_kind`` (which
    is the kind the surrounding field declared, e.g. ``"ltm pool"``
    for ``.ltm.virtual[].pool``).

    Useful for grouping or for filtering across kinds:
    ``[.ltm.pool[] | kind(.)] | unique`` returns the single-element
    list ``["ltm pool"]``, and ``[.ltm.virtual[] | refs(.)[]]``
    surfaces every dependency path; ``kind`` distinguishes them
    downstream.

    Related: ``path``, ``type``, ``refs``.
    """,
    examples=(
        "kind(.ltm.virtual.web_vs)                  # -> 'ltm virtual'",
        "[.ltm.virtual[] | refs(.)[]] | unique | sort",
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_kind(value: Any) -> str:
    if isinstance(value, ObjectRef):
        return value.kind
    if isinstance(value, PathRef):
        return value.expected_kind
    raise BuiltinError(f"kind: argument 1 must be an object, got {_type_name(value)}")


@_register(
    "path",
    summary="Return the BIG-IP full-path of an object or path-ref.",
    signatures=("path(value: object | path-ref) -> string",),
    details="""
    Returns the ``full_path`` of an :class:`ObjectRef` or
    :class:`PathRef` as a plain string.  This is the same as reading
    ``."full-path"`` from an ObjectRef but reads more naturally in
    pipelines.

    Useful when you have a stream of mixed objects and want to print
    a flat list of paths:
    ``.ltm.virtual.web_vs | refs(.) | map(path(.))`` (refs returns a
    list, pipe passes it whole, ``map`` iterates it).

    Raises ``BuiltinError`` for scalars (use the value directly when
    it's already a string).

    Related: ``kind``, ``partition``, ``basename``.
    """,
    examples=(
        "path(.ltm.virtual.web_vs)                # -> '/Common/web_vs'",
        "[.ltm.virtual[] | path(.)]               # collect every VS full-path",
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_path(value: Any) -> str:
    if isinstance(value, ObjectRef):
        return value.full_path
    if isinstance(value, PathRef):
        return value.full_path
    raise BuiltinError(f"path: cannot take path of {_type_name(value)}")


@_register(
    "defined",
    summary="True when the argument is not null and not the empty string.",
    signatures=("defined(value: any) -> boolean",),
    details="""
    Returns ``true`` for values that are "set" — anything that is
    not ``null``, not the empty string ``""``, and not an empty
    :class:`PathRef`.

    Distinct from a general truthiness check: ``defined`` returns
    ``true`` for ``false``, ``0``, and an empty list — those are
    *defined* but falsy.  Pair with ``select`` to keep only objects
    that have a particular field populated:
    ``.ltm.virtual[] | select(defined(.snatpool))``.

    Related: ``not``, ``select``.
    """,
    examples=(
        "select(defined(.pool))",
        ".ltm.virtual[] | select(defined(.snatpool)) | .name",
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_defined(value: Any) -> bool:
    if value is None:
        return False
    if isinstance(value, str) and value == "":
        return False
    if isinstance(value, PathRef) and value.full_path == "":
        return False
    return True


@_register(
    "type",
    summary="Name of the value's runtime type (``string``, ``object``, ``stream``, ...).",
    signatures=("type(value: any) -> string",),
    details="""
    Returns the DSL-level type name for *value*.  Possible values:

    - ``"null"``, ``"bool"``, ``"int"``, ``"float"``, ``"string"``
    - ``"path-ref"``, ``"object"``, ``"stream"``, ``"list"``

    Useful for introspection and for writing queries that branch on
    type (rare — most queries know the type from context).  Mainly
    surfaces in debugging.

    Related: ``kind`` (TMSH kind, more useful for BIG-IP objects),
    ``defined``.
    """,
    examples=(
        "type(.pool)                            # -> 'path-ref'",
        "type(.destination)                     # -> 'string'",
        "type(.rules)                           # -> 'list'",
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_type(value: Any) -> str:
    return _type_name(value)


# ---------------------------------------------------------------------------
# Graph helpers — surfaced from the same edge model the grep verb walks.
# ---------------------------------------------------------------------------


@_register(
    "refs",
    summary=(
        "List the full-paths of every object the given object references "
        "(forward edges in the same graph ``f5 grep`` walks)."
    ),
    signatures=("refs(value: object) -> list[string]",),
    details="""
    Walks the same reference graph the ``f5 grep`` verb uses, one
    hop forward from the given object.  Returns a list of full-path
    strings, deduplicated and excluding the seed itself.

    Forward edges include every kind of reference ``grep`` knows
    about: a VS's pool / iRules / profiles / persist / SNAT-pool,
    a pool's monitor and member nodes, a rule's pool / persist /
    data-group references extracted from its body, and so on.

    Requires the object to have been loaded from a real config —
    hand-built :class:`ObjectRef` values without a ``config_uri``
    raise.

    Currently always one hop deep; multi-hop walks belong in
    ``f5 grep`` (which produces a structured report) until the DSL
    grows a ``depth`` argument.

    Related: ``referenced_by`` (reverse direction), ``kind``,
    ``path``.
    """,
    examples=(
        "refs(.ltm.virtual.web_vs)",
        ".ltm.virtual.web_vs | refs(.) | sort   # all dependencies, sorted",
        ".ltm.virtual.web_vs | refs(.) | count  # dependency count",
    ),
    category="graph",
    min_args=1,
    max_args=1,
)
def _builtin_refs(value: Any) -> list[str]:
    from .graph import forward_refs

    if not isinstance(value, ObjectRef):
        raise BuiltinError(f"refs: argument 1 must be an object, got {_type_name(value)}")
    return forward_refs(value)


@_register(
    "referenced_by",
    summary=(
        "List the full-paths of every object that references the given "
        "object (reverse edges in the ``f5 grep`` graph)."
    ),
    signatures=("referenced_by(value: object) -> list[string]",),
    details="""
    The inverse of ``refs`` — walks one hop backwards in the
    reference graph and lists the objects that depend on the seed.
    Empty list means the object is an orphan (nothing in the config
    references it).

    Useful for orphan / cleanup queries:
    ``.ltm.pool[] | select(referenced_by(.) | count == 0) | .name``
    lists every pool that no virtual / iRule / data-group attaches to.

    Like ``refs``, the object must have been loaded from a real
    config (has a ``config_uri``).

    Related: ``refs`` (forward direction), ``count``, ``select``.
    """,
    examples=(
        "referenced_by(.ltm.pool.web_pool)",
        ".ltm.pool[] | select(referenced_by(.) | count == 0) | .name  # orphan pools",
    ),
    category="graph",
    min_args=1,
    max_args=1,
)
def _builtin_referenced_by(value: Any) -> list[str]:
    from .graph import reverse_refs

    if not isinstance(value, ObjectRef):
        raise BuiltinError(f"referenced_by: argument 1 must be an object, got {_type_name(value)}")
    return reverse_refs(value)


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------


def _truthy(value: object) -> bool:
    if value is None or value is False:
        return False
    if isinstance(value, str):
        return value != ""
    if isinstance(value, (list, tuple, Stream)):
        return len(value) > 0
    if isinstance(value, PathRef):
        return value.full_path != ""
    return bool(value)
