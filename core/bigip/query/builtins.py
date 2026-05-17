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

import base64 as _base64
import calendar as _calendar
import html as _html
import ipaddress
import math
import re
import shlex as _shlex
import time as _time
import urllib.parse as _urlparse
from collections.abc import Iterable
from dataclasses import dataclass
from datetime import datetime as _datetime
from datetime import timezone as _timezone
from typing import Any, Callable, Protocol, runtime_checkable

from .errors import BuiltinError
from .values import ObjectRef, PathRef, QueryFieldProvider, Stream, resolve_lazy_field

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
    # under ``docs/references/f5_query/builtins.md``.  Markdown is
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
    # Most builtins are *scalar*: they accept one value per argument
    # and the call dispatcher broadcasts each :class:`Stream`
    # argument element-wise.  This makes idioms like
    # ``is_fqdn(.pool.members[].address)`` produce a stream of bools
    # rather than rejecting the stream input, matching jq's natural
    # element-wise model.  Stream-aware builtins (``count``,
    # ``sort``, ``any``, ``all``, ``first``, ``last``, ``unique``,
    # ``reverse``, ``join``, ``min``, ``max``, ``group_by``,
    # ``sort_by``) set ``stream_aware=True`` so they receive the
    # whole stream as one argument.
    stream_aware: bool = False


@runtime_checkable
class _PathResolverContext(Protocol):
    def resolve_pathref(self, ref: PathRef) -> ObjectRef | None: ...


_REGISTRY: dict[str, BuiltinSpec] = {}
# Categories rendered in the order they appear in --help-builtins.
_CATEGORY_ORDER = (
    "stream",
    "string",
    "math",
    "time",
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
    stream_aware: bool = False,
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
            stream_aware=stream_aware,
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


def builtin(
    name: str,
    *,
    summary: str = "",
    signatures: tuple[str, ...] = (),
    examples: tuple[str, ...] = (),
    category: str = "user",
    min_args: int = 0,
    max_args: int | None = None,
    details: str = "",
    special_form: bool = False,
    with_ctx: bool = False,
    stream_aware: bool = False,
) -> Callable[[Callable], Callable]:
    """Public decorator — register *fn* as the DSL builtin *name*.

    Mirrors the in-tree ``_register`` decorator but with usable
    defaults so a script can drop a one-line plugin into
    ``$XDG_CONFIG_HOME/f5q/plugins/`` and have it work:

    .. code-block:: python

        from f5q import builtin

        @builtin("uppercase", summary="ASCII upper-case a string.",
                 min_args=1, max_args=1)
        def _uppercase(s):
            return str(s).upper()

    After the plugin loads (auto-discovered on first call to
    :func:`lookup` / :func:`list_builtins`, or via
    :func:`core.bigip.query.plugins.load_user_plugins`), the DSL can
    call ``uppercase(.ltm.virtual[].name)`` from the CLI and the
    Python API alike.

    Defaults vs ``_register``:

    - *category* defaults to ``"user"``; in-tree builtins always
      specify one of the ``_CATEGORY_ORDER`` slots.
    - *min_args* / *max_args* default to ``0`` / ``None`` (variadic);
      in-tree builtins pin them so ``--help-builtins`` shows arity
      bounds.
    - *signatures* / *examples* default to empty tuples; for a quick
      plugin they're optional, but supplying them populates the
      ``--help-builtins NAME`` page.

    All other fields (``details``, ``special_form``, ``with_ctx``,
    ``stream_aware``) match ``_register`` semantics — see the
    in-tree builtins for examples of each.
    """
    return _register(
        name,
        summary=summary,
        signatures=signatures,
        examples=examples,
        category=category,
        min_args=min_args,
        max_args=max_args,
        details=details,
        special_form=special_form,
        with_ctx=with_ctx,
        stream_aware=stream_aware,
    )


def lookup(name: str) -> BuiltinSpec | None:
    _ensure_plugins_loaded()
    return _REGISTRY.get(name)


_PLUGINS_LOADED = False


def _ensure_plugins_loaded() -> None:
    """Auto-load user plugins on first registry access.

    Same pattern the renderer registry uses — the first call to
    :func:`lookup` / :func:`list_builtins` scans
    ``$XDG_CONFIG_HOME/f5q/plugins/`` and imports every ``*.py``
    file there.  Subsequent calls are a flag check.
    """
    global _PLUGINS_LOADED
    if _PLUGINS_LOADED:
        return
    _PLUGINS_LOADED = True
    from .plugins import load_user_plugins

    load_user_plugins()


def list_builtins() -> list[BuiltinSpec]:
    """Return every builtin, sorted by category then name."""
    _ensure_plugins_loaded()
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
    # Typed sub-objects (BigipGtmPoolMember, BigipPoolMember, …)
    # stringify to their ``name`` field — mirrors how PathRef
    # stringifies to ``full_path`` so a query like
    # ``join($pool.members, ",")`` produces the member-name list
    # without an explicit ``.[].name`` projection.
    if isinstance(value, QueryFieldProvider):
        qf = value.query_fields()
        member_name = qf.get("name")
        if isinstance(member_name, str) and member_name:
            return member_name
    raise BuiltinError(f"{name}: argument {arg} must be a string, got {_type_name(value)}")


def _coerce_pathlike(value: object, *, name: str, arg: int) -> str:
    """Return a path-string from any value the user could plausibly pass.

    Accepts a plain ``str``, a :class:`PathRef`, or an :class:`ObjectRef`
    (whose :attr:`full_path` names the object).  Lets users write the
    obvious ``select(in_partition(., "Common"))`` form without forcing
    an explicit ``.name`` / ``.full-path`` projection — the predicate
    is about *the object*, so the engine should accept the object.
    """
    if isinstance(value, str):
        return value
    if isinstance(value, PathRef):
        return value.full_path
    if isinstance(value, ObjectRef):
        return value.full_path
    raise BuiltinError(
        f"{name}: argument {arg} must be a path-string or object, got {_type_name(value)}"
    )


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


def _safe_regex_compile(pattern: str, *, name: str, flags: int = 0) -> re.Pattern[str]:
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
        return re.compile(pattern, flags)
    except re.error as exc:
        raise BuiltinError(f"{name}: invalid pattern {pattern!r}: {exc}") from exc


def _as_sequence(value: object, *, name: str, arg: int) -> list[Any]:
    """Coerce a list-shaped DSL value to a plain Python list.

    Every list-shaped class in the DSL (``Stream``, ``BigipList``, the
    native list/tuple, monitor-expression wrappers) implements
    :class:`collections.abc.Iterable` — so the only thing this
    function does beyond ``list(value)`` is reject scalars that
    *happen* to be iterable: strings/bytes (semantically scalar) and
    dict-likes (which go through ``_as_object``).
    """
    if isinstance(value, Iterable) and not isinstance(value, (str, bytes, bytearray, dict)):
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
    "ip_translate",
    summary=(
        "Map an address from a source network to a destination network, "
        "across address families when needed."
    ),
    signatures=("ip_translate(src_net: string, dst_net: string, addr: string) -> string",),
    details="""
    Computes the host-bit offset of *addr* within *src_net* and applies
    that same offset within *dst_net*.  When the two networks belong to
    different families (IPv4 / IPv6) this performs an address-family
    translation: the host portion of an IPv4 address can be re-emitted
    inside an IPv6 prefix and vice versa.

    *src_net* must cover *addr*: if the host bits of *addr* relative to
    *src_net* don't fit inside *dst_net* (i.e. ``dst_net`` is more
    specific than ``src_net``), :class:`BuiltinError` is raised so
    silent truncation can't slip through.

    The returned string is the bare address — partition prefix and port
    are not preserved (callers building tmsh stanzas can re-attach
    them with ``+`` concatenation).  Use ``ip(net, src)`` instead when
    the operation stays in one family and you want to keep partition /
    route-domain / port from the source.

    Related: ``ip``, ``in_cidr``, ``net``.
    """,
    examples=(
        'ip_translate("10.0.0.0/8", "2001:db8::/32", "10.1.2.3")',
        '# -> "2001:db8::1:203"',
        'ip_translate("192.168.50.0/24", "2001:db8:50::/64", "192.168.50.10")',
        '# -> "2001:db8:50::a"',
    ),
    category="net",
    min_args=3,
    max_args=3,
)
def _builtin_ip_translate(*args: Any) -> str:
    src_str = _as_str(args[0], name="ip_translate", arg=1)
    dst_str = _as_str(args[1], name="ip_translate", arg=2)
    addr_str = _as_str(args[2], name="ip_translate", arg=3)
    try:
        src_net = ipaddress.ip_network(src_str, strict=False)
    except ValueError as exc:
        raise BuiltinError(f"ip_translate: invalid src_net {src_str!r}: {exc}") from exc
    try:
        dst_net = ipaddress.ip_network(dst_str, strict=False)
    except ValueError as exc:
        raise BuiltinError(f"ip_translate: invalid dst_net {dst_str!r}: {exc}") from exc
    _, bare_addr, _, _ = _split_destination(addr_str)
    try:
        src_ip = ipaddress.ip_address(bare_addr)
    except ValueError as exc:
        raise BuiltinError(f"ip_translate: invalid addr {bare_addr!r}: {exc}") from exc
    if src_ip not in src_net:
        raise BuiltinError(f"ip_translate: {bare_addr!r} is not within src_net {src_str!r}")
    src_host_bits = src_net.max_prefixlen - src_net.prefixlen
    dst_host_bits = dst_net.max_prefixlen - dst_net.prefixlen
    if src_host_bits > dst_host_bits:
        raise BuiltinError(
            f"ip_translate: dst_net {dst_str!r} is too specific to hold "
            f"the host bits of src_net {src_str!r} "
            f"({src_host_bits}-bit host vs {dst_host_bits}-bit host)"
        )
    host_offset = int(src_ip) - int(src_net.network_address)
    new_int = int(dst_net.network_address) | host_offset
    if isinstance(dst_net, ipaddress.IPv4Network):
        return str(ipaddress.IPv4Address(new_int))
    return str(ipaddress.IPv6Address(new_int))


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

    Pass-throughs:

    - Already-typed :class:`IPAddress` / :class:`FQDN` / :class:`Address`
      instances (typed fields surface these directly through the
      projection layer) are returned as-is.
    - Strings flow through ``Destination.try_parse`` first (so
      ``/Common/10.0.0.1:80`` extracts the host) and fall back to
      ``parse_address`` for bare hosts.

    Returns ``None`` for inputs that can't be coerced.
    """
    from ..types import FQDN, Address, Destination, IPAddress, parse_address

    if value is None:
        return None
    if isinstance(value, (IPAddress, FQDN, Address)):
        return value
    s = _as_typed_str(value, name=name, arg=arg)
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
    Already-typed :class:`Network` instances pass through unchanged
    so that callers projecting from a typed field (``.net.self[].address``
    is already a ``Network``) skip a round-trip through ``str``.
    """
    from ..types import Network

    if value is None:
        return None
    if isinstance(value, Network):
        return value
    s = _as_typed_str(value, name=name, arg=arg)
    if not s:
        return None
    return Network.try_parse(s)


def _as_typed_str(value: object, *, name: str, arg: int) -> str:
    """Like :func:`_as_str` but also accepts already-typed value objects
    so the typed builtins can stringify a passed-in
    :class:`IPAddress` / :class:`Network` / :class:`Destination` /
    :class:`FQDN` / :class:`Address` without going through the
    bare-string check that :func:`_as_str` enforces.
    """
    from ..types import FQDN, Address, Destination, IPAddress, Network

    if isinstance(value, (IPAddress, Network, Destination, FQDN, Address)):
        return str(value)
    return _as_str(value, name=name, arg=arg)


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
    "is_multicast",
    summary="True when *value* is a multicast IP (``224.0.0.0/4`` / ``ff00::/8``).",
    signatures=("is_multicast(value: string) -> boolean",),
    details="""
    Classifies through Python's ``ipaddress``: IPv4 ``224.0.0.0/4``
    and IPv6 ``ff00::/8``.  Returns ``false`` for FQDNs, unicast
    IPs, and unparseable input.

    Related: ``is_link_local``, ``is_reserved``, ``is_public``.
    """,
    examples=(".ltm.virtual[] | select(is_multicast(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_multicast(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_multicast")
    return isinstance(a, IPAddress) and a.is_multicast


@_register(
    "is_link_local",
    summary=("True when *value* is link-local (``169.254.0.0/16`` IPv4 / ``fe80::/10`` IPv6)."),
    signatures=("is_link_local(value: string) -> boolean",),
    details="""
    RFC 3927 (IPv4) / RFC 4291 (IPv6) link-local — addresses that
    are only valid on the directly attached segment.  Useful when
    auditing for accidentally-leaked auto-configured addresses.

    Related: ``is_multicast``, ``is_loopback``, ``is_private``.
    """,
    examples=(".ltm.node[] | select(is_link_local(.address)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_link_local(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_link_local")
    return isinstance(a, IPAddress) and a.is_link_local


@_register(
    "is_reserved",
    summary="True when *value* is in an IANA-reserved range (no current use).",
    signatures=("is_reserved(value: string) -> boolean",),
    details="""
    Reserved means "IANA has set aside the range, no current
    allocation" — distinct from ``is_private`` (carved out for
    intra-network use).  IPv4 ``240.0.0.0/4`` and various IPv6
    blocks fall here.

    Related: ``is_public``, ``is_private``.
    """,
    examples=(".ltm.virtual[] | select(is_reserved(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_reserved(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_reserved")
    return isinstance(a, IPAddress) and a.is_reserved


@_register(
    "is_public",
    summary="True when *value* is globally routable on the public internet.",
    signatures=("is_public(value: string) -> boolean",),
    details="""
    Returns ``true`` only when *value* is **not** in any of the
    reserved / private / loopback / link-local / multicast /
    unspecified ranges — i.e. an address you might legitimately
    see on the public internet.  Backed by
    :pyattr:`ipaddress.IPv4Address.is_global`.

    Use to audit "what's actually exposed?" without spelling out
    every negation:
    ``.ltm.virtual[] | select(is_public(.destination)) | .name``.

    Related: ``is_private`` (the complement), ``is_reserved``,
    ``is_documentation``.
    """,
    examples=(".ltm.virtual[] | select(is_public(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_public(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_public")
    return isinstance(a, IPAddress) and a.is_public


@_register(
    "is_documentation",
    summary=("True when *value* is in a documentation-example range (RFC 5737 / RFC 3849)."),
    signatures=("is_documentation(value: string) -> boolean",),
    details="""
    IPv4 ``192.0.2.0/24``, ``198.51.100.0/24``, ``203.0.113.0/24``,
    and IPv6 ``2001:db8::/32``.  Catching these in production
    configs is almost always a lab-template leak.

    Related: ``is_public``, ``is_private``, ``is_reserved``.
    """,
    examples=(".ltm.virtual[] | select(is_documentation(.destination)) | .name",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_is_documentation(value: Any) -> bool:
    from ..types import IPAddress

    a = _typed_address(value, name="is_documentation")
    return isinstance(a, IPAddress) and a.is_documentation


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
    "network_address",
    summary="Return the network (``.0``) address of a CIDR.",
    signatures=("network_address(value: string) -> string | null",),
    details="""
    Strips the host bits off *value* and returns the canonical
    network address.  ``network_address("10.0.0.5/24")`` →
    ``"10.0.0.0"``.  Returns ``null`` for unparseable input.

    Related: ``broadcast_address``, ``first_host``, ``last_host``,
    ``prefix_length``.
    """,
    examples=(
        'network_address("10.0.0.5/24")               # -> "10.0.0.0"',
        ".net.self[] | network_address(.address)",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_network_address(value: Any) -> str | None:
    n = _typed_network(value, name="network_address")
    if n is None:
        return None
    return str(n.network.network_address)


@_register(
    "broadcast_address",
    summary="Return the broadcast address of a CIDR (last address in range).",
    signatures=("broadcast_address(value: string) -> string | null",),
    details="""
    For IPv4 the broadcast is the ``.255`` (or whatever the
    prefix gives); for IPv6 there is no true broadcast, but
    ``ipaddress`` exposes the last address in the range and we
    surface it here for symmetry.  Returns ``null`` for
    unparseable input.

    Related: ``network_address``, ``last_host``, ``host_count``.
    """,
    examples=(
        'broadcast_address("10.0.0.0/24")             # -> "10.0.0.255"',
        ".net.route[] | {net: network_address(.network), bcast: broadcast_address(.network)}",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_broadcast_address(value: Any) -> str | None:
    n = _typed_network(value, name="broadcast_address")
    if n is None:
        return None
    return str(n.network.broadcast_address)


@_register(
    "first_host",
    summary="Return the lowest usable host address inside a CIDR.",
    signatures=("first_host(value: string) -> string | null",),
    details="""
    For prefix lengths that yield a network and broadcast
    address (IPv4 ``/30`` or shorter, IPv6 anything), this is
    ``network + 1`` — the first address assignable to a host.
    For point-to-point ``/31`` and host ``/32`` IPv4 networks
    where ``ipaddress.hosts()`` is empty, falls back to the
    network address itself (the only / lowest address in the
    range).  Returns ``null`` for unparseable input.

    Related: ``last_host``, ``host_count``, ``network_address``.
    """,
    examples=(
        'first_host("10.0.0.0/24")                    # -> "10.0.0.1"',
        '.ltm.pool[].members[].address | first_host(. + "/24")',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_first_host(value: Any) -> str | None:
    n = _typed_network(value, name="first_host")
    if n is None:
        return None
    hosts = list(n.network.hosts())
    if hosts:
        return str(hosts[0])
    # /31 and /32 (and v6 /127, /128) have no broadcast / network
    # split — there's just the one address.
    return str(n.network.network_address)


@_register(
    "last_host",
    summary="Return the highest usable host address inside a CIDR.",
    signatures=("last_host(value: string) -> string | null",),
    details="""
    The mirror of :func:`first_host`.  For ``/30`` and shorter
    IPv4 networks this is one below the broadcast; for ``/31``
    and ``/32`` it is the network address itself.  Returns
    ``null`` for unparseable input.

    Related: ``first_host``, ``broadcast_address``, ``host_count``.
    """,
    examples=('last_host("10.0.0.0/24")                     # -> "10.0.0.254"',),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_last_host(value: Any) -> str | None:
    n = _typed_network(value, name="last_host")
    if n is None:
        return None
    hosts = list(n.network.hosts())
    if hosts:
        return str(hosts[-1])
    return str(n.network.network_address)


@_register(
    "host_count",
    summary="Count of host addresses inside a CIDR.",
    signatures=("host_count(value: string) -> integer | null",),
    details="""
    Returns the number of host-assignable addresses in *value*.
    ``host_count("10.0.0.0/24")`` → ``254`` (256 − network −
    broadcast).  ``/31`` returns 2 and ``/32`` returns 1, matching
    operational reality on point-to-point and host networks.
    Returns ``null`` for unparseable input.

    Related: ``first_host``, ``last_host``, ``prefix_length``.
    """,
    examples=(
        'host_count("10.0.0.0/24")                    # -> 254',
        'host_count("10.0.0.0/31")                    # -> 2',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_host_count(value: Any) -> int | None:
    n = _typed_network(value, name="host_count")
    if n is None:
        return None
    network = n.network
    total = network.num_addresses
    # /31 (and IPv6 /127) are point-to-point: both addresses are
    # usable hosts.  /32 (and /128) are single-host networks.
    if total <= 2:
        return total
    # Otherwise subtract network + broadcast.
    return total - 2


@_register(
    "collapse_cidrs",
    summary="Merge a list of CIDRs into the minimal set of ranges.",
    signatures=("collapse_cidrs(values: list[string]) -> list[string]",),
    details="""
    Wraps :func:`ipaddress.collapse_addresses`.  Adjacent or
    subsumed CIDRs in *values* are merged so the result is the
    smallest set of non-overlapping ranges that covers the same
    address space.  Mixed IPv4 / IPv6 lists are split and each
    family collapsed independently.

    Useful for normalising address-list and firewall-rule
    address-list payloads before diffing:
    ``collapse_cidrs([.security.firewall."address-list"[].addresses[]])``.

    Related: ``supernet_of`` (one CIDR covering everything),
    ``subnet_of``.
    """,
    examples=(
        'collapse_cidrs(["10.0.0.0/24", "10.0.1.0/24"])    # -> ["10.0.0.0/23"]',
        'collapse_cidrs(["10.0.0.0/8", "10.1.0.0/16"])     # -> ["10.0.0.0/8"]',
    ),
    category="net",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_collapse_cidrs(values: Any) -> list[str]:
    import ipaddress as _ip

    items = _as_sequence(values, name="collapse_cidrs", arg=1)
    v4: list[_ip.IPv4Network] = []
    v6: list[_ip.IPv6Network] = []
    for item in items:
        net = _typed_network(item, name="collapse_cidrs")
        if net is None:
            continue
        if isinstance(net.network, _ip.IPv4Network):
            v4.append(net.network)
        else:
            v6.append(net.network)
    out: list[str] = [str(n) for n in _ip.collapse_addresses(v4)]
    out.extend(str(n) for n in _ip.collapse_addresses(v6))
    return out


@_register(
    "supernet_of",
    summary=("Return the smallest single CIDR that covers every address or network in *values*."),
    signatures=("supernet_of(values: list[string]) -> string | null",),
    details="""
    Finds the minimal supernet that contains every input.
    Plain IPs are treated as ``/32`` (IPv4) or ``/128`` (IPv6).
    Mixed-family inputs raise — IPv4 and IPv6 are never in the
    same supernet.  Returns ``null`` when *values* is empty.

    Pairs with ``collapse_cidrs`` for the two natural CIDR-
    algebra operations: "merge what's already adjacent" versus
    "what's the bounding CIDR".

    Related: ``collapse_cidrs``, ``subnet_of``, ``in_cidr``.
    """,
    examples=(
        'supernet_of(["10.0.0.1", "10.0.1.1"])             # -> "10.0.0.0/23"',
        'supernet_of(["10.0.0.0/24", "10.0.1.0/24"])       # -> "10.0.0.0/23"',
        ".ltm.pool[].members[].address | [.] | supernet_of(.)",
    ),
    category="net",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_supernet_of(values: Any) -> str | None:
    import ipaddress as _ip

    items = _as_sequence(values, name="supernet_of", arg=1)
    nets: list[_ip.IPv4Network | _ip.IPv6Network] = []
    for item in items:
        # Try CIDR first; fall back to single-host wrapping.
        net = _typed_network(item, name="supernet_of")
        if net is not None:
            nets.append(net.network)
            continue
        addr = _typed_address(item, name="supernet_of")
        if addr is None:
            continue
        from ..types import IPAddress

        if not isinstance(addr, IPAddress):
            continue
        prefix = 32 if addr.is_ipv4 else 128
        nets.append(_ip.ip_network(f"{addr.addr}/{prefix}", strict=False))
    if not nets:
        return None
    families = {type(n) for n in nets}
    if len(families) > 1:
        raise BuiltinError("supernet_of: cannot mix IPv4 and IPv6 inputs")
    # Iteratively shrink the prefix until a single network covers
    # every input.  ``ipaddress`` doesn't expose a "minimal-cover"
    # helper but ``supernet(prefixlen_diff=N)`` lets us widen each
    # input and check for convergence.
    network_cls = next(iter(families))
    family_max_prefix = 32 if network_cls is _ip.IPv4Network else 128
    lowest = min(int(n.network_address) for n in nets)
    highest = max(int(n.broadcast_address) for n in nets)
    # Walk prefix lengths from the most specific (the shortest
    # prefix present in inputs) outward until the candidate
    # contains the full ``[lowest, highest]`` span.
    prefix = min(n.prefixlen for n in nets)
    while prefix >= 0:
        candidate = _ip.ip_network((lowest, prefix), strict=False)
        if int(candidate.network_address) <= lowest and int(candidate.broadcast_address) >= highest:
            return str(candidate)
        prefix -= 1
    # Fallback: the family's default (0.0.0.0/0 or ::/0) covers
    # everything.
    return "0.0.0.0/0" if family_max_prefix == 32 else "::/0"


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
    Useful for finding self-IP / route-domain conflicts.  The DSL
    doesn't ship a pairwise-combinations primitive yet, so the
    natural pattern uses a let-binding to cross the stream against
    itself: ``[.net.self[]] as $all | .net.self[] as $a | $all[]
    | select(. != $a) | select(overlaps($a.address, .address))
    | $a.name + " ↔ " + .name``.

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
        # ``use_exact=True``: a rename of ``/Tenant_A/p`` must not
        # be falsely refused because ``/Tenant_A/p2`` has a referrer
        # that wouldn't see the new partition.  Identity-shaped
        # safety checks need exact-path matching.
        report = compute_grep(
            sources={ctx.root.uri: ctx.root.source},
            configs={ctx.root.uri: ctx.root.config},
            pattern=old_s,
            use_regex=False,
            use_cidr=False,
            use_exact=True,
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
        'rename_partition("Tenant_A", "Tenant_B")',
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


@_register(
    "rename_prefix",
    summary="Rewrite every object whose full-path starts with *old* to start with *new*.",
    details="""
    A general-purpose sibling of :func:`rename_partition` and
    :func:`rename_folder`: where those are scoped to partition or
    folder boundaries, ``rename_prefix`` operates on arbitrary
    full-path prefixes.  Useful for moving a *family* of related
    objects together when their identifying convention is a leaf-
    name prefix that doesn't align with a partition or folder
    boundary, e.g. moving every ``/Common/app3_*`` object to
    ``/Tenant_A/app3_*``:

    ::

        rename_prefix("/Common/app3_", "/Tenant_A/app3_")

    Every full-path occurrence beginning with ``<old>`` is rewritten
    to begin with ``<new>``, cascading through:

    - object stanza headers (``ltm pool /Common/app3_p1``);
    - reference properties (``pool /Common/app3_p1``);
    - destinations that embed the prefix
      (``destination /Common/app3_vip:443``);
    - iRule body literals.

    Token-bounded so an unrelated path that *contains* the prefix
    later in the string (``/Common/old/app3_x``) doesn't accidentally
    match — the rewrite only fires when the prefix starts on a
    path-segment boundary.

    Pre-flight checks: both arguments must be non-empty.  ``old ==
    new`` is a no-op.  Mixing with field edits inside the same
    statement is rejected (byte offsets shift); split with ``;``.

    Returns the count of textual matches the cascade landed on.

    Related: ``rename_partition`` (partition-level cascade),
    ``rename_folder`` (folder-level cascade), ``rename`` (single
    object + every reference).
    """,
    signatures=("rename_prefix(old: string, new: string) -> integer",),
    examples=(
        'rename_prefix("/Common/app3_", "/Tenant_A/app3_")',
        'rename_prefix("/Common/legacy-", "/Tenant_B/legacy-")',
    ),
    category="rename",
    min_args=2,
    max_args=2,
    with_ctx=True,
)
def _builtin_rename_prefix(old: Any, new: Any, *, ctx: Any) -> int:
    from .edit_plan import PrefixRewrite

    old_text = _as_str(old, name="rename_prefix", arg=1).strip()
    new_text = _as_str(new, name="rename_prefix", arg=2).strip()
    if not old_text or not new_text:
        raise BuiltinError("rename_prefix: prefixes must not be empty")
    # Both arguments must be BIG-IP-path prefixes (start with ``/``)
    # — the docs and the "path migration" framing only make sense
    # for token-bounded path rewrites.  Without this guard, calls
    # like ``rename_prefix("pool", "X")`` would build a broad
    # textual rewrite that the token-bounded regex catches but
    # which has no clean migration semantics.  Refuse rather than
    # silently rewrite.
    if not old_text.startswith("/"):
        raise BuiltinError(
            f"rename_prefix: old prefix must start with '/' "
            f"(BIG-IP full paths) — got {old_text!r}.  Use ``rename`` "
            "for individual-object renames or ``sub``/``gsub`` for "
            "free-form text rewrites."
        )
    if not new_text.startswith("/"):
        raise BuiltinError(
            f"rename_prefix: new prefix must start with '/' (BIG-IP full paths) — got {new_text!r}."
        )
    if old_text == new_text:
        return 0
    # Token-bounded: the prefix must start on a path-segment
    # boundary (preceded by whitespace, ``/``, ``=``, ``,``, ``{``,
    # ``"``, or start-of-line) so an unrelated sub-string match deep
    # inside another token doesn't fire.  The lookahead requires a
    # name character so an exact-prefix match (no further name
    # characters after the prefix) still hits but a random ``{`` or
    # whitespace doesn't.
    prefix_pattern = re.compile(rf"(?<![A-Za-z0-9_./\-]){re.escape(old_text)}(?=[A-Za-z0-9_])")
    ctx.edits.add_prefix(
        PrefixRewrite(
            source_uri=ctx.root.uri,
            label=f"prefix {old_text}",
            pattern=prefix_pattern,
            replacement=new_text,
            human_new=new_text,
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

    Both spellings are accepted as the *path* argument:

    - **Full path** (``"/Common/old_pool"``): the partition + folder
      segments are preserved and only the leaf is replaced.
    - **Bare leaf** (``"old_pool"`` — what ``.name`` projects): no
      partition / folder context to preserve, so the result is just
      the new leaf name.  This makes ``.name |= with_name(., "X")``
      work the same way ``."full-path" |= with_name(., "X")`` does.

    Related: ``basename`` (extract leaf), ``with_partition``,
    ``with_folder``.
    """,
    examples=(
        'with_name("/Common/old_pool", "new_pool")',
        'with_name(."full-path", "renamed")',
        'with_name(.name, "renamed")',
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
        # Bare leaf — no partition / folder context to preserve.
        # ``.name`` projects this shape; accepting it lets users write
        # ``.name |= with_name(., "X")`` without remembering that the
        # full-path variant is the only "parseable" one.
        if p and "/" not in p:
            return n
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

    p = _coerce_pathlike(path, name="in_partition", arg=1)
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

    p = _coerce_pathlike(path, name="in_folder", arg=1)
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
    # ``use_exact=True`` so ``/Common/p`` doesn't also match
    # ``/Common/p2``; ``references_to`` is an identity-shaped query
    # and substring seeding produces false positives that
    # downstream renames / partition guards then misbehave on.
    report = compute_grep(
        sources={ctx.root.uri: ctx.root.source},
        configs={ctx.root.uri: ctx.root.config},
        pattern=target,
        use_regex=False,
        use_cidr=False,
        use_exact=True,
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
    "find every iRule that references a pool whose partition the
    rule itself can't see" (uses a let-binding to carry the rule's
    full path into the per-reference stream — the DSL has no jq
    ``..`` parent operator):

    ``.ltm.rule[] as $r | $r.refs.pools[] | select(not can_see($r."full-path", .))``

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
            # Exact match: ``/Common/p`` traverse-out edges must come
            # from /Common/p itself, not from /Common/p2.
            report = compute_grep(
                sources={ctx.root.uri: ctx.root.source},
                configs={ctx.root.uri: ctx.root.config},
                pattern=referrer_path,
                use_regex=False,
                use_cidr=False,
                use_exact=True,
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
    stream_aware=True,
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
    signatures=(
        "sub(value: string, pattern: string, replacement: string) -> string",
        "sub(value: string, pattern: string, replacement: string, flags: string) -> string",
    ),
    details="""
    Replaces the **first** occurrence of *pattern* in *value* with
    *replacement* and returns the new string.  *pattern* is a Python
    regex; *replacement* may use ``\\1`` / ``\\g<name>`` backrefs.

    Optional *flags* string takes the same letters as ``test`` /
    ``scan`` / ``capture`` / ``splits``: ``i`` for case-insensitive,
    ``x`` for free-spacing, ``s`` for dot-matches-newline, ``m`` for
    multi-line.

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
        'sub(.name, "^VS_", "vs_", "i")           # case-insensitive',
        '.ltm.virtual[].destination |= sub(., ":443$", ":8443")',
    ),
    category="string",
    min_args=3,
    max_args=4,
)
def _builtin_sub(value: Any, pattern: Any, repl: Any, flags: Any = "") -> str:
    s = _as_str(value, name="sub", arg=1)
    p = _as_str(pattern, name="sub", arg=2)
    r = _as_str(repl, name="sub", arg=3)
    f = _as_str(flags, name="sub", arg=4) if flags != "" else ""
    rx = _safe_regex_compile(p, name="sub", flags=_jq_regex_flags(f, name="sub"))
    return rx.sub(r, s, count=1)


@_register(
    "gsub",
    summary="Replace every regex match in a string.",
    signatures=(
        "gsub(value: string, pattern: string, replacement: string) -> string",
        "gsub(value: string, pattern: string, replacement: string, flags: string) -> string",
    ),
    details="""
    Like ``sub`` but replaces **every** occurrence of *pattern* in
    *value*.  Useful for blanket string rewrites inside iRule bodies
    or data-group values.

    Accepts the same optional *flags* string ``sub`` / ``test`` /
    ``scan`` do: ``i`` / ``x`` / ``s`` / ``m``.

    For object full-path renames, prefer ``rename`` or
    ``rename_partition`` over a raw ``gsub`` — those route through a
    token-bounded engine that won't touch substring collisions or
    short-name references in unsafe contexts.

    Related: ``sub``, ``match``, ``rename``, ``rename_partition``.
    """,
    examples=(
        'gsub(.body, "/Common/old_", "/Common/new_")',
        'gsub(.body, "old_", "new_", "i")          # case-insensitive',
        '.ltm.virtual[].destination |= gsub(., "%5", "%7")  # bulk RD change',
    ),
    category="string",
    min_args=3,
    max_args=4,
)
def _builtin_gsub(value: Any, pattern: Any, repl: Any, flags: Any = "") -> str:
    s = _as_str(value, name="gsub", arg=1)
    p = _as_str(pattern, name="gsub", arg=2)
    r = _as_str(repl, name="gsub", arg=3)
    f = _as_str(flags, name="gsub", arg=4) if flags != "" else ""
    rx = _safe_regex_compile(p, name="gsub", flags=_jq_regex_flags(f, name="gsub"))
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
    "index",
    summary="Position of a needle inside a string or list (jq-compatible).",
    signatures=(
        "index(value: string, needle: string) -> integer | null",
        "index(value: list, needle: any) -> integer | null",
    ),
    details="""
    Mirrors jq's ``index`` builtin.  Returns the zero-based offset of
    the first occurrence of *needle* inside *value*, or ``null`` when
    *needle* is not present.

    For strings, ``index`` does substring search.  For lists / streams
    / :class:`BigipList` values, ``index`` matches element-wise on
    ``full_path`` when items are :class:`PathRef`, otherwise on
    equality.

    Common predicate idiom (paralleling jq):
    ``.ltm.virtual[] | select(.name | index(":443"))`` — keeps every
    virtual whose name contains the substring.

    Related: ``contains`` (boolean variant), ``startswith`` /
    ``endswith``.
    """,
    examples=(
        '.ltm.virtual[] | select(.name | index(":443"))',
        '[.profiles[].name] | index("http")     # 0..n-1 or null',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_index(value: Any, needle: Any) -> int | None:
    def _eq(item: Any, target: Any) -> bool:
        a = item.full_path if isinstance(item, PathRef) else item
        b = target.full_path if isinstance(target, PathRef) else target
        return a == b

    if isinstance(value, str):
        nstr = _as_str(needle, name="index", arg=2)
        pos = value.find(nstr)
        return pos if pos >= 0 else None
    if isinstance(value, PathRef):
        nstr = _as_str(needle, name="index", arg=2)
        pos = value.full_path.find(nstr)
        return pos if pos >= 0 else None
    if isinstance(value, Iterable) and not isinstance(value, (str, bytes, bytearray, dict)):
        items = list(value)
    else:
        raise BuiltinError(f"index: cannot search inside {_type_name(value)}")
    for i, item in enumerate(items):
        if _eq(item, needle):
            return i
    return None


@_register(
    "source_file",
    summary="Return the source file URI of the current object.",
    signatures=("source_file(value: object) -> string | null",),
    details="""
    Resolves the source URI of the BIG-IP object passed in (the file
    a ``ltm pool`` / ``ltm virtual`` / ... stanza was parsed from).
    Most useful in ``--merge`` mode, where a single query streams
    objects from several inputs and the consumer wants to label each
    by origin: ``.ltm.virtual[] | {name: .name, src: source_file}``.

    Returns ``null`` for synthetic / non-object values.  The result is
    the source URI as stored on the underlying :class:`ObjectRef`
    (typically a ``file:///`` URL); pair with ``basename`` for a
    short filename.

    Related: ``--merge`` mode, ``$name`` for explicit per-source
    binding.
    """,
    examples=(
        ".ltm.virtual[] | {name: .name, src: source_file}",
        ".ltm.pool[] | {name: .name, file: basename(source_file)}",
    ),
    category="value",
    min_args=1,
    max_args=1,
    with_ctx=True,
)
def _builtin_source_file(value: Any, *, ctx: Any) -> str | None:
    if isinstance(value, ObjectRef):
        return value.config_uri or None
    if isinstance(value, PathRef):
        # Path-refs don't directly carry a source URI; resolve to the
        # backing ObjectRef when possible so the user gets a result
        # without an explicit ``refs`` traversal.
        target = _resolve_pathref_via_ctx(value, ctx)
        if target is not None:
            return target.config_uri or None
    return None


def _resolve_pathref_via_ctx(ref: Any, ctx: Any) -> ObjectRef | None:
    """Best-effort PathRef -> ObjectRef lookup through EvalContext."""
    if not isinstance(ref, PathRef) or not isinstance(ctx, _PathResolverContext):
        return None
    return ctx.resolve_pathref(ref)


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
    stream_aware=True,
)
def _builtin_join(values: Any, sep: Any) -> str:
    items = _as_sequence(values, name="join", arg=1)
    s = _as_str(sep, name="join", arg=2)
    return s.join(_as_str(v, name="join", arg=1) for v in items)


@_register(
    "tsv",
    summary="Join arguments with tabs for tab-separated row output.",
    signatures=(
        "tsv(*cells: any) -> string",
        "tsv(a, b, c, ...) -> string",
    ),
    details="""
    Each argument is coerced to its scalar string form (``PathRef`` →
    full-path, ``null`` → empty, bool → ``true`` / ``false``,
    numbers → their decimal form) and joined with ``\\t``.  Embedded
    tabs, newlines, and carriage returns inside cell values are
    replaced with spaces so the resulting line stays one TSV row;
    pre-quote cells explicitly if you need to retain whitespace.

    Designed to compose with stream broadcast: when any argument is a
    :class:`Stream`, ``tsv`` broadcasts element-wise so
    ``tsv(.name, .destination, .pool)`` produces one row per virtual
    server, and ``tsv(.name, .pool.members[].address)`` produces one
    row per pool member with the VS name replicated across each row
    (same semantics every other scalar builtin uses).

    Pair with ``--raw`` to print without surrounding quoting:
    ``f5 query --raw 'tsv(.name, .destination)' bigip.conf``.

    Related: ``csv`` (comma-separated, quote-aware), ``join`` (join
    one list with a separator), string concat with ``+``.
    """,
    examples=(
        ".ltm.virtual[] | tsv(.name, .destination, .pool)",
        ".ltm.pool[].members[] | tsv(.name, .address, port(.name))",
    ),
    category="string",
    min_args=1,
    max_args=None,
)
def _builtin_tsv(*cells: Any) -> str:
    return "\t".join(_csv_cell_text(c, name="tsv", arg=i) for i, c in enumerate(cells, 1))


@_register(
    "csv",
    summary="Join arguments with commas, quoting cells when necessary.",
    signatures=(
        "csv(*cells: any) -> string",
        "csv(a, b, c, ...) -> string",
    ),
    details="""
    RFC 4180-style CSV row builder.  Each argument is coerced to its
    scalar string form and emitted as a CSV field; cells containing
    ``,``, ``"``, ``\\n``, or ``\\r`` are wrapped in double quotes
    with embedded quotes doubled (``"`` → ``""``).  Empty cells emit
    as an empty field (``,``), not as ``""``.

    Broadcasts the same way as ``tsv``: when any argument is a
    :class:`Stream`, ``csv`` produces one row per element with scalar
    arguments replicated.

    Pair with ``--raw`` for clean piping into CSV consumers:
    ``f5 query --raw 'csv(.name, .destination)' bigip.conf | head``.

    Related: ``tsv``, ``join``.
    """,
    examples=(
        ".ltm.virtual[] | csv(.name, .destination, .pool)",
        ".ltm.pool[].members[] | csv(.name, .address)",
    ),
    category="string",
    min_args=1,
    max_args=None,
)
def _builtin_csv(*cells: Any) -> str:
    return ",".join(
        _csv_quote(_csv_cell_text(c, name="csv", arg=i)) for i, c in enumerate(cells, 1)
    )


def _csv_cell_text(value: Any, *, name: str, arg: int) -> str:
    """Coerce *value* to a row-cell string.

    ``PathRef`` → ``full_path``, ``None`` → empty, bool / int / float
    render through ``str``, already-typed value objects (``IPAddress``
    / ``Network`` / ``Destination`` / ``FQDN`` / ``Address``) render
    through their ``__str__``.  Tabs / newlines / carriage returns
    inside a cell are replaced with single spaces so a TSV / CSV row
    stays on one line.
    """
    from ..types import FQDN, Address, Destination, IPAddress, Network

    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    if isinstance(value, PathRef):
        s = value.full_path
    elif isinstance(value, (IPAddress, Network, Destination, FQDN, Address)):
        s = str(value)
    elif isinstance(value, str):
        s = value
    elif isinstance(value, (list, tuple)):
        # Compact rendering for list-valued fields (e.g. ``.rules``):
        # space-separated full-paths is the rendering ``f5 grep`` /
        # the rename engine already use.
        s = " ".join(_csv_cell_text(v, name=name, arg=arg) for v in value)
    else:
        raise BuiltinError(
            f"{name}: argument {arg} cannot be rendered as a row cell ({_type_name(value)})"
        )
    return s.replace("\t", " ").replace("\n", " ").replace("\r", " ")


def _csv_quote(cell: str) -> str:
    """Apply RFC 4180 quoting when *cell* contains a delimiter / quote."""
    if any(ch in cell for ch in (",", '"', "\n", "\r")):
        return '"' + cell.replace('"', '""') + '"'
    return cell


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
    stream_aware=True,
)
def _builtin_keys(value: Any) -> list[Any]:
    if isinstance(value, ObjectRef):
        return sorted(value.fields)
    if isinstance(value, dict):
        return sorted(value)
    if isinstance(value, Iterable) and not isinstance(value, (str, bytes, bytearray, dict)):
        # ``BigipList`` / ``list`` / ``tuple`` / ``Stream`` — match
        # jq, where ``keys`` on a list returns its indices.
        return list(range(len(list(value))))
    raise BuiltinError(f"keys: argument 1 must be an object or list, got {_type_name(value)}")


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
    stream_aware=True,
)
def _builtin_values(value: Any) -> list[Any]:
    if isinstance(value, ObjectRef):
        return [resolve_lazy_field(value, k, value.fields[k]) for k in sorted(value.fields)]
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
    stream_aware=True,
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
    stream_aware=True,
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
    stream_aware=True,
)
def _builtin_count(value: Any) -> int:
    return len(_as_sequence(value, name="count", arg=1))


@_register(
    "unique",
    summary="Return the unique items of a list, sorted.",
    signatures=("unique(value: list | stream) -> list",),
    details="""
    De-duplicates a list or stream and returns the unique items in
    sorted order.  Matches jq's ``unique`` exactly: input is treated
    as an array, output is the sorted unique values.  :class:`PathRef`
    items collate by their ``full_path``, mixed types fall into jq's
    cross-type ordering (``null < bool < number < string < array <
    object``).

    Unhashable items (rare — usually nested lists) collate through the
    same :func:`_sort_key` the ``sort`` builtin uses, so the result is
    deterministic across runs and Python interpreter versions.

    For grouping with a key function use ``unique_by(f)`` instead,
    which keeps one representative per equivalence class.

    Related: ``sort``, ``unique_by``, ``dupes``, ``count``, ``map``.
    """,
    examples=(
        "[.ltm.virtual[].pool] | unique           # every distinct default pool",
        "[.ltm.virtual[].name | partition(.)] | unique  # used partitions, sorted",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_unique(value: Any) -> list[Any]:
    items = _as_sequence(value, name="unique", arg=1)
    # jq's ``unique`` is "sorted unique values": sort by the jq
    # ordering, then drop adjacent duplicates.  This makes the result
    # deterministic regardless of input order, and matches the order
    # jq emits.
    ordered = sorted(items, key=_sort_key)
    out: list[Any] = []
    prev_key: Any = _MISSING
    for item in ordered:
        key = _sort_key(item)
        if prev_key is _MISSING or key != prev_key:
            out.append(item)
            prev_key = key
    return out


_MISSING: Any = object()


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
    stream_aware=True,
)
def _builtin_sort(value: Any) -> list[Any]:
    items = _as_sequence(value, name="sort", arg=1)
    return sorted(items, key=_sort_key)


def _sort_key(value: Any) -> tuple:
    """Order key compatible with jq's ordering across scalar + composite types.

    jq orders ``null < false < true < numbers < strings < arrays <
    objects``.  Inside each category lexicographic / numeric ordering
    applies.  Objects sort by their *sorted* ``[key, value]`` tuple
    sequence so two dicts with the same shape collate consistently
    rather than raising ``TypeError`` on Python's native ``<``.

    The returned tuple has the type-tag as its first slot so values of
    different families never compare against each other — they fall
    out into stable buckets.
    """
    if value is None:
        return (0,)
    if isinstance(value, bool):
        # Important: check bool before int because ``isinstance(True, int)`` is True.
        return (1, value)
    if isinstance(value, (int, float)):
        return (2, value)
    if isinstance(value, PathRef):
        return (3, value.full_path)
    if isinstance(value, str):
        return (3, value)
    if isinstance(value, (list, tuple)):
        return (4, tuple(_sort_key(v) for v in value))
    if isinstance(value, dict):
        return (5, tuple((k, _sort_key(value[k])) for k in sorted(value)))
    if isinstance(value, ObjectRef):
        return (
            5,
            tuple(
                (k, _sort_key(resolve_lazy_field(value, k, value.fields[k])))
                for k in sorted(value.fields)
            ),
        )
    # Fallback: try string repr so heterogeneous unknowns sort
    # deterministically rather than blowing up.
    return (6, str(value))


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
    stream_aware=True,
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
    stream_aware=True,
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


@_register(
    "port_set_contains",
    summary="True when *port* lies inside the comma-separated *spec*.",
    signatures=("port_set_contains(spec: string, port: integer) -> boolean",),
    details="""
    *spec* is a F5 firewall-rule port spec like ``"80-82,8081"``.
    Returns ``true`` when *port* falls inside any segment.  Use
    this to audit rules:
    ``.security.firewall.rule-list[].rules[] | select(port_set_contains(.port, 443))``.

    Related: ``port_set_count``, ``port_set_overlaps``,
    ``in_cidr`` (the address-side counterpart).
    """,
    examples=(
        'port_set_contains("80-82,8081", 81)            # -> true',
        'port_set_contains("80,443", 8080)              # -> false',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_port_set_contains(spec: Any, port: Any) -> bool:
    from ..types import PortSet

    s = _as_str(spec, name="port_set_contains", arg=1)
    p_int = _as_int(port, name="port_set_contains", arg=2)
    ps = PortSet.try_parse(s)
    if ps is None:
        return False
    return ps.contains(p_int)


@_register(
    "port_set_count",
    summary="Total number of ports across every segment of *spec*.",
    signatures=("port_set_count(spec: string) -> integer | null",),
    details="""
    Counts how many distinct ports a comma-separated port spec
    covers.  ``"80-82,8081"`` → 4.  ``"any"`` → 65536.  Returns
    ``null`` for unparseable input.

    Related: ``port_set_contains``, ``port_set_overlaps``.
    """,
    examples=(
        'port_set_count("80-82,8081")                   # -> 4',
        'port_set_count("any")                          # -> 65536',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_port_set_count(spec: Any) -> int | None:
    from ..types import PortSet

    s = _as_str(spec, name="port_set_count", arg=1)
    ps = PortSet.try_parse(s)
    if ps is None:
        return None
    return ps.count


@_register(
    "port_set_overlaps",
    summary="True when two port specs share at least one port.",
    signatures=("port_set_overlaps(a: string, b: string) -> boolean",),
    details="""
    Pair-wise overlap check between two comma-separated port
    specs.  Useful when comparing firewall-rule port windows to
    spot accidental coverage gaps or duplications.

    Related: ``port_set_contains``, ``port_set_count``.
    """,
    examples=(
        'port_set_overlaps("80-82,443", "82-100")       # -> true',
        'port_set_overlaps("80-82,443", "100-200")      # -> false',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_port_set_overlaps(a: Any, b: Any) -> bool:
    from ..types import PortSet

    sa = _as_str(a, name="port_set_overlaps", arg=1)
    sb = _as_str(b, name="port_set_overlaps", arg=2)
    pa = PortSet.try_parse(sa)
    pb = PortSet.try_parse(sb)
    if pa is None or pb is None:
        return False
    return pa.overlaps(pb)


@_register(
    "ip_range_to_cidrs",
    summary="Decompose ``first-last`` IP range into the minimum CIDR set.",
    signatures=("ip_range_to_cidrs(range: string) -> list[string]",),
    details="""
    Parses *range* (``"192.168.9.77-192.168.9.83"``) and returns
    the smallest list of CIDRs that exactly covers the range.
    Useful for converting free-form ranges into firewall
    ``address-list`` entries.

    Returns ``null`` for unparseable input.  Single-address
    inputs return a one-element list of the ``/32`` (or
    ``/128``).

    Related: ``ip_range_supernet``, ``ip_range_count``,
    ``ip_range_contains``.
    """,
    examples=(
        'ip_range_to_cidrs("192.168.9.77-192.168.9.83")  # -> 4 /29.. /30 etc.',
        'ip_range_to_cidrs("10.0.0.1-10.0.0.255")',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_ip_range_to_cidrs(value: Any) -> list[str] | None:
    from ..types import IPRange

    s = _as_str(value, name="ip_range_to_cidrs", arg=1)
    rng = IPRange.try_parse(s)
    if rng is None:
        return None
    return [str(n) for n in rng.to_cidrs()]


@_register(
    "ip_range_supernet",
    summary="Smallest single CIDR that covers an IP range.",
    signatures=("ip_range_supernet(range: string) -> string | null",),
    details="""
    The minimum-prefix CIDR containing both endpoints of *range*.
    May include addresses outside the original ``[first, last]``
    span — that's the inherent cost of summarising a free-form
    range as a single CIDR.

    Pair with :func:`ip_range_to_cidrs` (exact decomposition)
    when you need precision instead of a single bounding network.
    """,
    examples=('ip_range_supernet("192.168.9.77-192.168.9.83")  # -> "192.168.9.64/27"',),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_ip_range_supernet(value: Any) -> str | None:
    from ..types import IPRange

    s = _as_str(value, name="ip_range_supernet", arg=1)
    rng = IPRange.try_parse(s)
    if rng is None:
        return None
    return str(rng.as_supernet())


@_register(
    "ip_range_count",
    summary="Count of addresses in an IP range (inclusive).",
    signatures=("ip_range_count(range: string) -> integer | null",),
    details="""
    ``"10.0.0.5-10.0.0.9"`` → 5 (five addresses inclusive).
    Returns ``null`` for unparseable input.

    Related: ``ip_range_to_cidrs``, ``ip_range_contains``.
    """,
    examples=(
        'ip_range_count("192.168.9.77-192.168.9.83")    # -> 7',
        'ip_range_count("10.0.0.1")                     # -> 1',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_ip_range_count(value: Any) -> int | None:
    from ..types import IPRange

    s = _as_str(value, name="ip_range_count", arg=1)
    rng = IPRange.try_parse(s)
    if rng is None:
        return None
    return rng.count


@_register(
    "ip_range_contains",
    summary="True when *addr* lies inside the ``first-last`` range.",
    signatures=("ip_range_contains(range: string, addr: string) -> boolean",),
    details="""
    Inclusive membership check.  Mixed-family inputs (v4 range,
    v6 address or vice versa) always return ``false`` rather
    than raising — different families never overlap.

    Related: ``in_cidr`` (CIDR equivalent), ``ip_range_to_cidrs``.
    """,
    examples=(
        'ip_range_contains("192.168.9.77-192.168.9.83", "192.168.9.80")  # -> true',
        'ip_range_contains("10.0.0.0-10.0.0.255", "10.0.1.1")            # -> false',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_ip_range_contains(range_text: Any, addr: Any) -> bool:
    from ..types import IPRange

    s = _as_str(range_text, name="ip_range_contains", arg=1)
    a = _as_str(addr, name="ip_range_contains", arg=2)
    rng = IPRange.try_parse(s)
    if rng is None:
        return False
    return rng.contains(a)


_DNS_CACHE: dict[str, list[str]] = {}
_REV_DNS_CACHE: dict[str, list[str]] = {}


@_register(
    "dns",
    summary="Resolve a hostname to its IP addresses (A + AAAA records).",
    signatures=("dns(name: string) -> list[string]",),
    details="""
    Performs a forward DNS lookup of *name* via the system
    resolver (``socket.getaddrinfo``).  Returns the sorted list
    of unique IP addresses or an empty list when resolution
    fails.

    Results are memoised for the lifetime of the Python process
    so repeated lookups inside one query don't hammer DNS.
    Lookups are time-bounded by the resolver's default timeout
    (typically 5s).

    Pair with ``rev_dns`` for round-trip checks
    (``dns("host.example.com") | map(rev_dns(.))``).
    """,
    examples=(
        'dns("one.one.one.one")                          # -> ["1.1.1.1", "1.0.0.1"]',
        ".ltm.node[].address | {addr: ., rev: rev_dns(.)}",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_dns(value: Any) -> list[str]:
    import socket as _socket

    name = _as_str(value, name="dns", arg=1)
    if name in _DNS_CACHE:
        return list(_DNS_CACHE[name])
    try:
        infos = _socket.getaddrinfo(name, None, type=_socket.SOCK_STREAM)
    except _socket.gaierror:
        _DNS_CACHE[name] = []
        return []
    # ``getaddrinfo`` returns ``sockaddr`` tuples whose first
    # element is the address string; coerce to ``str`` so the
    # static type matches the declared ``list[str]`` return.
    out: list[str] = sorted({str(info[4][0]) for info in infos})
    _DNS_CACHE[name] = out
    return list(out)


@_register(
    "rev_dns",
    summary="Reverse-resolve an IP address to its PTR hostname.",
    signatures=("rev_dns(ip: string) -> list[string]",),
    details="""
    Performs a reverse DNS lookup (``socket.gethostbyaddr``).
    Returns the canonical hostname plus any aliases, or an
    empty list on failure.  Memoised per process; bounded by the
    resolver's default timeout.

    Related: ``dns`` (forward).
    """,
    examples=(
        'rev_dns("1.1.1.1")                              # -> ["one.one.one.one"]',
        ".ltm.node[].address | rev_dns(.)",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_rev_dns(value: Any) -> list[str]:
    import socket as _socket

    ip = _as_str(value, name="rev_dns", arg=1)
    if ip in _REV_DNS_CACHE:
        return list(_REV_DNS_CACHE[ip])
    try:
        host, aliases, _ = _socket.gethostbyaddr(ip)
    except (_socket.herror, _socket.gaierror):
        _REV_DNS_CACHE[ip] = []
        return []
    out = [host] + list(aliases)
    _REV_DNS_CACHE[ip] = out
    return list(out)


# ---------------------------------------------------------------------------
# Network probes — opt-in via --enable-probes.  Time-bounded, cached
# per process, gated by ``_probes.PROBES_ENABLED``.
# ---------------------------------------------------------------------------


@_register(
    "ping",
    summary="ICMP echo to *ip*.  Requires --enable-probes.",
    signatures=("ping(ip: string) -> object",),
    details="""
    Subprocess invocation of the system ``ping`` command.
    Returns ``{ok: bool, rtt_ms: float | null, error: string | null}``.
    Gated by ``--enable-probes`` — without the flag, raises
    ``BuiltinError`` so an offline query never hits the network
    by accident.

    Related: ``portping`` (TCP/UDP), ``traceroute``, ``dns``.
    """,
    examples=(
        'ping("10.0.0.1")',
        ".ltm.node[] | {addr: .address, reachable: (ping(.address).ok)}",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_ping(value: Any) -> dict[str, Any]:
    from ._probes import ping

    return ping(_as_str(value, name="ping", arg=1))


@_register(
    "portping",
    summary="TCP/UDP probe to *(ip, port)*.  Requires --enable-probes.",
    signatures=("portping(ip: string, port: integer[, protocol: string]) -> object",),
    details="""
    TCP-connect (default) or UDP send-receive timing.  Returns
    ``{ok, rtt_ms, error}``.  UDP is best-effort — no reply does
    not imply unreachable.  Pass ``protocol="udp"`` to switch.
    """,
    examples=(
        'portping("10.0.0.1", 443)',
        ".ltm.virtual[] | {name: .name, vip_up: portping(host(.destination), port(.destination)).ok}",
    ),
    category="net",
    min_args=2,
    max_args=3,
)
def _builtin_portping(ip: Any, port: Any, protocol: Any = "tcp") -> dict[str, Any]:
    from ._probes import portping

    return portping(
        _as_str(ip, name="portping", arg=1),
        _as_int(port, name="portping", arg=2),
        protocol=_as_str(protocol, name="portping", arg=3),
    )


@_register(
    "traceroute",
    summary="Hop-by-hop path probe to *ip*.  Requires --enable-probes.",
    signatures=("traceroute(ip: string) -> list[object]",),
    details="""
    Subprocess invocation of ``traceroute``.  Returns one record
    per hop: ``{hop: int, ip: string | null, rtt_ms: float | null}``.
    Hops the router didn't answer for show up with ``ip=null``.
    """,
    examples=('traceroute("8.8.8.8") | last(.) | .ip',),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_traceroute(value: Any) -> list[dict[str, Any]]:
    from ._probes import traceroute

    return traceroute(_as_str(value, name="traceroute", arg=1))


def _register_url_method(method: str) -> Callable[[Any], dict[str, Any]]:
    @_register(
        f"url_{method.lower()}",
        summary=f"HTTP {method.upper()} request.  Requires --enable-probes.",
        signatures=(f"url_{method.lower()}(url: string[, headers: object]) -> object",),
        details=f"""
        Issues an HTTP ``{method.upper()}`` to *url* via urllib.
        Returns ``{{status: int | null, headers: object, body: string,
        error: string | null}}``.  Default timeout 5s.

        Optional second argument is a dict of request headers.

        Related: ``url_get``, ``url_head``, ``url_post``,
        ``url_options``.
        """,
        examples=(
            f'url_{method.lower()}("https://example.com/")',
            f'url_{method.lower()}("https://api.example/v1", {{"Authorization": "Bearer X"}})',
        ),
        category="net",
        min_args=1,
        max_args=3 if method.upper() == "POST" else 2,
    )
    def _impl(*args: Any) -> dict[str, Any]:
        from ._probes import url_request

        url = _as_str(args[0], name=f"url_{method.lower()}", arg=1)
        headers: dict[str, str] | None = None
        body: str | None = None
        if method.upper() == "POST":
            body = _as_str(args[1], name="url_post", arg=2) if len(args) > 1 else None
            if len(args) > 2 and isinstance(args[2], dict):
                headers = {str(k): str(v) for k, v in args[2].items()}
        else:
            if len(args) > 1 and isinstance(args[1], dict):
                headers = {str(k): str(v) for k, v in args[1].items()}
        return url_request(method.upper(), url, body=body, headers=headers)

    return _impl


_register_url_method("get")
_register_url_method("head")
_register_url_method("options")
_register_url_method("post")


@_register(
    "socket_get",
    summary="TCP connect + read banner.  Requires --enable-probes.",
    signatures=("socket_get(host: string, port: integer[, send: string]) -> string",),
    details="""
    Opens a TCP socket to *(host, port)*, optionally sends *send*,
    reads up to 4096 bytes, and returns the response as UTF-8
    (replacement on non-text bytes).  Useful for protocol-banner
    fingerprinting — SSH versions, SMTP greetings, etc.
    """,
    examples=(
        'socket_get("ssh.example.com", 22)',
        'socket_get("smtp.example.com", 25)',
    ),
    category="net",
    min_args=2,
    max_args=3,
)
def _builtin_socket_get(host: Any, port: Any, send: Any = "") -> str:
    from ._probes import socket_get

    return socket_get(
        _as_str(host, name="socket_get", arg=1),
        _as_int(port, name="socket_get", arg=2),
        send=_as_str(send, name="socket_get", arg=3),
    )


@_register(
    "tls_handshake",
    summary="Open a TLS connection and inspect what the peer offered.",
    signatures=("tls_handshake(host: string, port: integer[, sni: string]) -> object",),
    details="""
    Performs a full TLS handshake against *(host, port)* (with
    SNI defaulting to *host*) and returns the negotiated
    protocol, cipher suite, ALPN selection, peer certificate
    dict, and verify status against the system trust store.

    Requires ``--enable-probes``.
    """,
    examples=(
        'tls_handshake("example.com", 443) | .protocol',
        'tls_handshake("example.com", 443) | .peer_cert.subject',
    ),
    category="net",
    min_args=2,
    max_args=3,
)
def _builtin_tls_handshake(host: Any, port: Any, sni: Any = None) -> dict[str, Any]:
    from ._probes import tls_handshake

    sni_text: str | None = None
    if sni is not None and sni != "":
        sni_text = _as_str(sni, name="tls_handshake", arg=3)
    return tls_handshake(
        _as_str(host, name="tls_handshake", arg=1),
        _as_int(port, name="tls_handshake", arg=2),
        sni=sni_text,
    )


@_register(
    "x509_parse",
    summary="Parse a PEM-encoded X.509 certificate.",
    signatures=("x509_parse(pem: string) -> object",),
    details="""
    Returns a dict of fields: subject, issuer, not_before,
    not_after, serial, fingerprint_sha256, sans, key_alg,
    key_size, sig_alg, version, public_key_pem.  Uses
    :mod:`cryptography` when available; falls back to stdlib
    :mod:`ssl` (a subset of fields) when not.

    Does NOT need ``--enable-probes`` — it operates on locally-
    held PEM text.  Pair with ``url_get`` or ``json_load`` to
    feed it certificate data.

    Related: ``tls_handshake`` (negotiated chain), ``json_load``.
    """,
    examples=(
        'x509_parse(json_load("/etc/ssl/cert.pem"))',
        'tls_handshake("example.com", 443).peer_cert',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_x509_parse(value: Any) -> dict[str, Any]:
    from ._probes import x509_parse

    return x509_parse(_as_str(value, name="x509_parse", arg=1))


@_register(
    "x509_from_config",
    summary="Project a BIG-IP config-object cert into the ``x509_parse`` shape.",
    signatures=("x509_from_config(cert: object) -> object",),
    details="""
    Takes a BIG-IP config object that carries cert metadata and
    returns a dict in the same shape :func:`x509_parse` produces:
    ``subject`` / ``issuer`` / ``not_after`` / ``serial`` /
    ``fingerprint_sha256`` / ``sans`` / ``key_alg`` / ``key_size``
    / ``version`` / etc.

    Supported config objects (anywhere a cert appears in the
    parsed model):

    - ``sys file ssl-cert`` — cert / chain / bundle store, the
      target of every client-ssl / server-ssl ``cert-key-chain``
      and ``ltm monitor https.cert`` PathRef.
    - ``cm cert`` — device-trust certs, the target of ``cm
      device.cert``, ``cm trust-domain.ca-cert``, ``cm
      trust-domain.ca-cert-bundle``.

    For config objects that *reference* a cert by PathRef
    (``ltm monitor https``, ``ltm profile client-ssl``,
    ``ltm profile server-ssl``, ``cm device``, ``cm
    trust-domain``), index into the referent first then pipe to
    this builtin.  ``sys crypto cert`` is a minimal projection
    that doesn't carry metadata — load its PEM with
    :func:`cert_load` instead.

    Normalisation handled at the boundary: SAN strings are split
    on commas with the ``DNS:`` / ``IP:`` prefix stripped; the
    fingerprint is converted from ``SHA256/12:34:…`` to bare
    uppercase hex; ``serial-number`` is converted from decimal to
    uppercase hex; ``version`` is mapped from ``"3"`` to ``"v3"``;
    ``key-type`` is mapped to cryptography's public-key class
    names (``rsa-public`` → ``RSAPublicKey``).  Fields BIG-IP's
    TMSH surface doesn't carry (``not_before``, ``sig_alg``,
    ``public_key_pem``) come back ``null``.

    Pair with :func:`x509_eq` to compare a BIG-IP cert against one
    fetched from a live endpoint or a PEM file.

    Related: ``x509_parse`` (parse a PEM string),
    ``x509_load_file`` (parse a PEM file), ``x509_eq``.
    """,
    examples=(
        '.sys.file.ssl-cert["/Common/example.crt"] | x509_from_config(.)',
        '.cm.cert["/Common/dtca.crt"] | x509_from_config(.)',
        "x509_eq("
        '.sys.file.ssl-cert["/Common/example.crt"] | x509_from_config(.), '
        'x509_parse(url_get("https://example.com/cert.pem").body))',
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_x509_from_config(value: Any) -> dict[str, Any]:
    from ._probes import x509_from_config

    return x509_from_config(value)


@_register(
    "x509_eq",
    summary="Compare two ``x509_parse``-shaped dicts for cert identity.",
    signatures=("x509_eq(a: object, b: object) -> boolean",),
    details="""
    Returns ``true`` when the two dicts describe the same X.509
    certificate.  Comparison order, strongest → weakest:

    1. ``fingerprint_sha256`` — the canonical identity.  Two certs
       with the same SHA-256 fingerprint are the same cert.
    2. ``subject`` + ``issuer`` + ``serial`` — the X.509-defined
       primary key.  Used when one side is a BIG-IP ``sys file
       ssl-cert`` projection that doesn't carry a SHA-256
       fingerprint.

    Plain ``==`` on the dicts compares every field — including
    ``not_before`` / ``sig_alg`` / ``public_key_pem`` which the
    BIG-IP side leaves ``null`` — and so reports certs as
    different even when they describe the same key material.  Use
    this helper instead for "same cert" semantics.

    Related: ``x509_parse``, ``x509_from_config``.
    """,
    examples=(
        "x509_eq(x509_parse(.body), x509_from_config($cert))",
        ".sys.file.ssl-cert[] | select(x509_eq(x509_from_config(.), $peer))",
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_x509_eq(left: Any, right: Any) -> bool:
    from ._probes import x509_eq

    return x509_eq(left, right)


@_register(
    "json_load",
    summary="Read a file from disk and parse it as JSON.",
    signatures=("json_load(path: string) -> any",),
    details="""
    Reads *path* from the local filesystem and returns the parsed
    JSON value.  Use this to mix external data (CMDB exports,
    vlan-to-tenant maps, signed-cert manifests) into a query:

    .. code-block:: text

       json_load("/etc/inventory/servers.json") as $inv
         | .ltm.node[]
         | {name: .name, owner: $inv[.address]}

    Tilde expansion is honoured (``json_load("~/data/x.json")``).
    Raises :class:`BuiltinError` for missing files or invalid
    JSON — failures are explicit rather than producing ``null``.
    """,
    examples=(
        'json_load("/etc/inventory/servers.json")',
        '.ltm.node[].address as $a | json_load("data.json")[$a]',
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_json_load(path: Any) -> object:
    import json as _json
    import os.path

    p = _as_str(path, name="json_load", arg=1)
    expanded = os.path.expanduser(p)
    try:
        with open(expanded, encoding="utf-8") as f:
            return _json.load(f)
    except FileNotFoundError as exc:
        raise BuiltinError(f"json_load: file not found: {expanded}") from exc
    except OSError as exc:
        raise BuiltinError(f"json_load: cannot read {expanded}: {exc}") from exc
    except _json.JSONDecodeError as exc:
        raise BuiltinError(
            f"json_load: {expanded}: invalid JSON ({exc.msg} at line {exc.lineno} col {exc.colno})"
        ) from exc


@_register(
    "cert_load",
    summary="Load and parse an X.509 cert from disk (PEM / DER / PKCS#12).",
    signatures=(
        "cert_load(path: string) -> object | list[object]",
        "cert_load(path: string, password: string) -> object | list[object]",
    ),
    details="""
    Reads *path* from disk and returns a structured cert dict in the
    same shape :func:`x509_parse` produces.  The file format is
    sniffed from the bytes — extension hints (``.crt``, ``.pem``,
    ``.cer``, ``.der``, ``.pfx``, ``.p12``) are tolerated but not
    required:

    - **PEM** (``-----BEGIN CERTIFICATE-----``): parsed directly.
      When the file contains a *chain* (multiple PEM blocks) a
      list is returned, leaf first.
    - **DER**: re-encoded to PEM and parsed.
    - **PKCS#12** (``.pfx`` / ``.p12``): unpacked into the
      end-entity cert plus any chain certs.  Pass *password* as
      the optional second argument when the bundle is encrypted;
      omit it for plain bundles.  Returns ``[leaf, *chain]`` when
      a chain is present, otherwise just the leaf dict.

    Tilde expansion is honoured.  Raises :class:`BuiltinError` for
    missing files, unreadable formats, or wrong passwords.  No
    network access — purely local file IO.

    Related: ``x509_parse`` (parse an in-memory PEM string),
    ``tls_handshake`` (peer cert pre-parsed in ``peer_cert``).
    """,
    examples=(
        'cert_load("/etc/ssl/certs/server.crt")',
        'cert_load("./bundle.pfx", "trustno1")',
        'cert_load("chain.pem") | first | .subject',
    ),
    category="value",
    min_args=1,
    max_args=2,
)
def _builtin_cert_load(*args: Any) -> Any:
    import os.path

    from ._probes import x509_parse

    p = _as_str(args[0], name="cert_load", arg=1)
    expanded = os.path.expanduser(p)
    password: bytes | None = None
    if len(args) > 1:
        password = _as_str(args[1], name="cert_load", arg=2).encode("utf-8")
    try:
        with open(expanded, "rb") as f:
            raw = f.read()
    except FileNotFoundError as exc:
        raise BuiltinError(f"cert_load: file not found: {expanded}") from exc
    except OSError as exc:
        raise BuiltinError(f"cert_load: cannot read {expanded}: {exc}") from exc
    # PEM is the easy path — split on the END markers and parse
    # each block; one block returns a dict, several return a list
    # (chain order preserved from the file).
    if b"-----BEGIN" in raw:
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise BuiltinError(f"cert_load: {expanded}: not UTF-8 PEM ({exc})") from exc
        blocks = _split_pem_blocks(text)
        if not blocks:
            raise BuiltinError(f"cert_load: {expanded}: no PEM CERTIFICATE blocks found")
        parsed = [x509_parse(b) for b in blocks]
        return parsed[0] if len(parsed) == 1 else parsed
    # PKCS#12 — needs cryptography.  Detect by the leading magic
    # (``0x30`` = ASN.1 SEQUENCE) plus the ``.pfx`` / ``.p12``
    # extension; we don't try to disambiguate from a raw DER cert
    # without the extension hint because both start with 0x30.
    suffix = os.path.splitext(expanded)[1].lower()
    if suffix in {".pfx", ".p12"}:
        return _load_pkcs12(expanded, raw, password)
    # Assume DER cert.  Re-encode to PEM and parse.
    try:
        import ssl as _ssl

        pem = _ssl.DER_cert_to_PEM_cert(raw)
    except Exception as exc:
        raise BuiltinError(f"cert_load: {expanded}: not PEM / DER / PKCS#12 ({exc})") from exc
    return x509_parse(pem)


def _split_pem_blocks(text: str) -> list[str]:
    """Split a multi-cert PEM file into one PEM string per cert.

    Only blocks tagged ``CERTIFICATE`` are returned — keys and other
    block types are skipped so a combined ``cert+key.pem`` produces
    just the certs.  Block order from the file is preserved.
    """
    blocks: list[str] = []
    lines = text.splitlines(keepends=True)
    in_cert = False
    buf: list[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("-----BEGIN CERTIFICATE-----"):
            in_cert = True
            buf = [line]
            continue
        if in_cert:
            buf.append(line)
            if stripped.startswith("-----END CERTIFICATE-----"):
                blocks.append("".join(buf))
                in_cert = False
                buf = []
    return blocks


def _load_pkcs12(expanded: str, raw: bytes, password: bytes | None) -> Any:
    """Decode a PKCS#12 bundle and return the parsed cert(s).

    Returns the end-entity cert dict when no chain is present,
    otherwise ``[leaf, *chain]`` in file order.  Requires the
    ``cryptography`` package — raises a clear ``BuiltinError`` when
    it isn't installed.
    """
    from importlib import import_module

    from ._probes import x509_parse

    try:
        pkcs12 = import_module("cryptography.hazmat.primitives.serialization.pkcs12")
        serialization = import_module("cryptography.hazmat.primitives.serialization")
    except ImportError as exc:
        raise BuiltinError(
            f"cert_load: {expanded}: PKCS#12 parsing needs the "
            "``cryptography`` package — install it to load .pfx / .p12 files"
        ) from exc
    try:
        leaf_cert, additional = _pkcs12_unpack(pkcs12, raw, password)
    except Exception as exc:
        raise BuiltinError(f"cert_load: {expanded}: failed to decode PKCS#12 ({exc})") from exc
    if leaf_cert is None and not additional:
        raise BuiltinError(f"cert_load: {expanded}: PKCS#12 bundle contains no certificates")
    pem_blocks: list[str] = []
    if leaf_cert is not None:
        pem_blocks.append(leaf_cert.public_bytes(serialization.Encoding.PEM).decode("utf-8"))
    for cert in additional:
        pem_blocks.append(cert.public_bytes(serialization.Encoding.PEM).decode("utf-8"))
    parsed = [x509_parse(b) for b in pem_blocks]
    return parsed[0] if len(parsed) == 1 else parsed


def _pkcs12_unpack(pkcs12: Any, raw: bytes, password: bytes | None) -> tuple[Any, list[Any]]:
    """Return ``(end_entity_cert, additional_certs)`` from a PKCS#12 blob.

    Tries :func:`load_key_and_certificates` first (the modern API);
    falls back to :func:`load_pkcs12` (returns a PKCS12KeyAndCertificates
    object) for older / different ``cryptography`` versions.
    """
    if hasattr(pkcs12, "load_key_and_certificates"):
        _, cert, additional = pkcs12.load_key_and_certificates(raw, password)
        return cert, list(additional or [])
    bundle = pkcs12.load_pkcs12(raw, password)
    leaf = bundle.cert.certificate if bundle.cert is not None else None
    additional = [entry.certificate for entry in (bundle.additional_certs or [])]
    return leaf, additional


@_register(
    "jsonl_load",
    summary="Read a file from disk and parse it as JSON Lines (NDJSON).",
    signatures=("jsonl_load(path: string) -> list",),
    details="""
    Reads *path* line by line and parses each non-blank line as a
    JSON value, returning the list in file order.  This is the
    natural shape for log streams, event archives, and any other
    one-record-per-line dump where loading the whole file as one
    JSON value would force every consumer to know about the
    framing.

    Blank lines are skipped.  Any line that fails to parse raises
    :class:`BuiltinError` with the offending line number so a bad
    record in a large dump is easy to find.

    Tilde expansion is honoured.

    Related: ``json_load`` (whole-file JSON), ``json_parse``
    (in-memory string), ``csv_load`` (CSV with or without headers).
    """,
    examples=(
        'jsonl_load("/var/log/events.jsonl")',
        '.ltm.virtual[].name as $n | jsonl_load("events.jsonl")[] | select(.vs == $n)',
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_jsonl_load(path: Any) -> list[Any]:
    import os.path

    from ._inputs import InputError, parse_jsonl

    p = _as_str(path, name="jsonl_load", arg=1)
    expanded = os.path.expanduser(p)
    try:
        with open(expanded, encoding="utf-8") as f:
            text = f.read()
    except FileNotFoundError as exc:
        raise BuiltinError(f"jsonl_load: file not found: {expanded}") from exc
    except OSError as exc:
        raise BuiltinError(f"jsonl_load: cannot read {expanded}: {exc}") from exc
    try:
        return parse_jsonl(text, source=expanded)
    except InputError as exc:
        raise BuiltinError(f"jsonl_load: {exc}") from exc


@_register(
    "csv_load",
    summary="Read a CSV file from disk and parse it into a list of records.",
    signatures=(
        "csv_load(path: string) -> list[object]",
        "csv_load(path: string, headers: list[string]) -> list[object]",
    ),
    details="""
    Reads *path* as CSV.  With one argument the first row of the
    file names the columns (the jq-natural shape, matches what most
    spreadsheet exports look like).  With two arguments *headers*
    is a list of column names and every row of the file is treated
    as data — use this form for header-less CSVs (firewall NAT
    exports, RFC 4180 fragments, etc.).

    Values are returned as strings.  The DSL's ``+`` operator
    coerces scalars when one side is a string, so number-shaped
    cells (``"443"``) flow through arithmetic without an explicit
    cast.  Missing trailing columns become empty strings; rows
    that overflow the header list land their extras in an
    ``_extra`` list.

    Tilde expansion is honoured.  Raises :class:`BuiltinError` for
    missing files or unreadable CSV.

    Related: ``jsonl_load``, ``json_load``, ``csv`` /
    ``tsv`` (render to one-row strings).
    """,
    examples=(
        'csv_load("/etc/inventory/servers.csv")',
        'csv_load("nats.csv", ["internal", "external"])',
        'csv_load("vips.csv") | map(.name)',
    ),
    category="value",
    min_args=1,
    max_args=2,
)
def _builtin_csv_load(*args: Any) -> list[dict[str, Any]]:
    import os.path

    from ._inputs import InputError, parse_csv

    p = _as_str(args[0], name="csv_load", arg=1)
    expanded = os.path.expanduser(p)
    headers: list[str] | None = None
    if len(args) > 1:
        raw_headers = args[1]
        if not isinstance(raw_headers, list):
            raise BuiltinError(
                f"csv_load: argument 2 must be a list of strings, got {_type_name(raw_headers)}"
            )
        headers = []
        for i, h in enumerate(raw_headers):
            if not isinstance(h, str):
                raise BuiltinError(
                    f"csv_load: argument 2: header {i} must be a string, got {_type_name(h)}"
                )
            headers.append(h)
    try:
        with open(expanded, encoding="utf-8") as f:
            text = f.read()
    except FileNotFoundError as exc:
        raise BuiltinError(f"csv_load: file not found: {expanded}") from exc
    except OSError as exc:
        raise BuiltinError(f"csv_load: cannot read {expanded}: {exc}") from exc
    try:
        return parse_csv(text, headers=headers, source=expanded)
    except InputError as exc:
        raise BuiltinError(f"csv_load: {exc}") from exc


@_register(
    "f5log_load",
    summary="Read a BIG-IP log file from disk and parse it into structured events.",
    signatures=("f5log_load(path: string) -> list[object]",),
    details="""
    Reads *path* as a BIG-IP log and parses each line into a
    structured event dict:

    .. code-block:: text

       { "timestamp": "Nov 28 09:53:00"
       , "host": "bigip01"
       , "severity": "info"
       , "daemon": "tmm"
       , "pid": 12345
       , "code": "01230140:6"
       , "module": "01230140"
       , "level": 6
       , "message": "Connection limit reached for pool /Common/web_pool"
       , "raw": "<original line>"
       }

    Handles classic syslog, RFC3164-with-PRI, and the F5
    ``XXXXXXXX:N:`` message-code form.  Lines that don't match
    land with ``message`` set to the original text and the typed
    fields blank, so a grep / filter pipeline never silently
    drops unknown shapes.

    Tilde expansion is honoured.  Pairs naturally with the
    classification predicates (``in_cidr`` / ``is_private``) when
    the message body contains an IP — split on whitespace inside
    the message and feed candidates through.

    Related: ``jsonl_load``, ``csv_load``, ``json_load``.
    """,
    examples=(
        'f5log_load("/var/log/ltm") | last',
        '[f5log_load("/var/log/tmm") | select(.severity == "err")] | count',
        'f5log_load("audit.log") | select(.daemon == "logger" and .module == "01070417")',
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_f5log_load(path: Any) -> list[dict[str, Any]]:
    import os.path

    from ._inputs import InputError, parse_f5log

    p = _as_str(path, name="f5log_load", arg=1)
    expanded = os.path.expanduser(p)
    try:
        with open(expanded, encoding="utf-8") as f:
            text = f.read()
    except FileNotFoundError as exc:
        raise BuiltinError(f"f5log_load: file not found: {expanded}") from exc
    except OSError as exc:
        raise BuiltinError(f"f5log_load: cannot read {expanded}: {exc}") from exc
    try:
        return parse_f5log(text, source=expanded)
    except InputError as exc:
        raise BuiltinError(f"f5log_load: {exc}") from exc


@_register(
    "json_parse",
    summary="Parse a JSON string into its native value.",
    signatures=("json_parse(text: string) -> any",),
    details="""
    Counterpart to :func:`json_load` for in-memory strings.
    Useful for parsing the ``body`` of an HTTP response or any
    other JSON-bearing text the query already has in hand:

    .. code-block:: text

       url_get("https://api.example/v1/inventory")
         | json_parse(.body)
         | .servers

    Raises :class:`BuiltinError` on invalid JSON.
    """,
    examples=(
        'json_parse("[1, 2, 3]")                          # -> [1, 2, 3]',
        'url_get("https://api/v1") | json_parse(.body)',
    ),
    category="value",
    min_args=1,
    max_args=1,
)
def _builtin_json_parse(value: Any) -> object:
    import json as _json

    text = _as_str(value, name="json_parse", arg=1)
    try:
        return _json.loads(text)
    except _json.JSONDecodeError as exc:
        raise BuiltinError(
            f"json_parse: invalid JSON ({exc.msg} at line {exc.lineno} col {exc.colno})"
        ) from exc


# ---------------------------------------------------------------------------
# HTTP response helpers — make ``url_*`` results ergonomic.
# ---------------------------------------------------------------------------


def _http_response(value: Any, *, name: str) -> dict[str, Any]:
    """Coerce *value* to an HTTP response dict and validate its shape."""
    if not isinstance(value, dict):
        raise BuiltinError(
            f"{name}: argument must be an HTTP response dict (got {_type_name(value)})"
        )
    return value


@_register(
    "http_status",
    summary="Status code from an HTTP response dict.",
    signatures=("http_status(response: object) -> integer | null",),
    details="""
    Accessor for the ``status`` field of an ``url_get``-style
    response.  Returns ``null`` when the request failed before
    the server responded (DNS error, connect timeout, etc.).

    Equivalent to ``response.status`` — provided for parity with
    the other ``http_*`` helpers and so audits can spell their
    intent symmetrically.
    """,
    examples=(
        'url_get("https://example.com/") | http_status(.)',
        ".urls[] | url_head(.) | {url: ., status: http_status(.)}",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_status(value: Any) -> int | None:
    resp = _http_response(value, name="http_status")
    return resp.get("status")


@_register(
    "http_body",
    summary="Response body as a string.",
    signatures=("http_body(response: object) -> string",),
    details="""
    Accessor for the response's ``body`` field.  Always a string;
    binary payloads round-trip with U+FFFD replacement.
    """,
    examples=('url_get("https://example.com/") | http_body(.)',),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_body(value: Any) -> str:
    resp = _http_response(value, name="http_body")
    body = resp.get("body", "")
    return body if isinstance(body, str) else str(body)


@_register(
    "http_body_json",
    summary="Parse the response body as JSON.",
    signatures=("http_body_json(response: object) -> any",),
    details="""
    Convenience wrapper around ``json_parse(.body)`` that adds a
    light content-type sanity check: if the response declares a
    ``content-type`` and it doesn't include ``json``, the
    builtin still parses but raises ``BuiltinError`` if the body
    isn't valid JSON.  When ``content-type`` is missing it
    silently attempts the parse.

    Use this when an API returns JSON and you want to traverse
    the parsed value without spelling out a ``json_parse(.body)``
    chain every time.
    """,
    examples=(
        'url_get("https://api/v1") | http_body_json(.) | .items',
        ".urls[] | url_get(.) | http_body_json(.).version",
    ),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_body_json(value: Any) -> object:
    import json as _json

    resp = _http_response(value, name="http_body_json")
    # Fetchers pre-parse JSON bodies into ``body_json`` when the
    # response's content-type indicates JSON, so the common path
    # never re-parses on traversal.  Synthetic responses built by
    # hand can either set ``body_json`` directly or rely on the
    # fallback that re-parses ``body``.
    if "body_json" in resp and resp["body_json"] is not None:
        return resp["body_json"]
    body = resp.get("body", "")
    if not isinstance(body, str):
        body = str(body)
    if not body:
        return None
    try:
        return _json.loads(body)
    except _json.JSONDecodeError as exc:
        raise BuiltinError(
            f"http_body_json: invalid JSON ({exc.msg} at line {exc.lineno} col {exc.colno})"
        ) from exc


@_register(
    "http_headers",
    summary="Return the response's headers as a dict (keys lowercased).",
    signatures=("http_headers(response: object) -> object",),
    details="""
    The underlying ``url_*`` builtins already store headers
    with lowercase keys so a query can do case-insensitive
    lookups directly.  This helper is the typed accessor: use
    ``http_header(resp, "name")`` for one value, or
    ``http_headers(resp)`` when you want the whole map.
    """,
    examples=('url_get("https://example.com/") | http_headers(.) | keys',),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_headers(value: Any) -> dict[str, Any]:
    resp = _http_response(value, name="http_headers")
    headers = resp.get("headers", {})
    return headers if isinstance(headers, dict) else {}


@_register(
    "http_header",
    summary="Return one header value by name (case-insensitive).",
    signatures=("http_header(response: object, name: string) -> string | null",),
    details="""
    Looks *name* up in the response's headers; the match is
    case-insensitive (``Content-Type`` finds ``content-type``).
    Returns ``null`` when the header isn't present.

    Note: HTTP allows multiple headers with the same name to
    repeat (e.g. ``Set-Cookie``).  The underlying urllib path
    collapses repeats into a single comma-separated string,
    matching the wire-format convention.
    """,
    examples=(
        'url_get("https://example.com/") | http_header(., "content-type")',
        '.urls[] | url_head(.) | http_header(., "server")',
    ),
    category="net",
    min_args=2,
    max_args=2,
)
def _builtin_http_header(value: Any, name: Any) -> str | None:
    resp = _http_response(value, name="http_header")
    headers = resp.get("headers", {})
    if not isinstance(headers, dict):
        return None
    key = _as_str(name, name="http_header", arg=2).lower()
    return headers.get(key)


def _status_in_range(value: Any, low: int, high: int, builtin_name: str) -> bool:
    resp = _http_response(value, name=builtin_name)
    status = resp.get("status")
    if not isinstance(status, int):
        return False
    return low <= status <= high


@_register(
    "http_ok",
    summary="True when the response status is 2xx.",
    signatures=("http_ok(response: object) -> boolean",),
    details="""
    Range predicate for the 200-299 success class.  Useful as
    the head of audit pipelines:
    ``.urls[] | url_get(.) | select(http_ok(.))``.

    Related: ``http_redirect``, ``http_client_error``,
    ``http_server_error``.
    """,
    examples=(".urls[] | url_get(.) | select(http_ok(.))",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_ok(value: Any) -> bool:
    return _status_in_range(value, 200, 299, "http_ok")


@_register(
    "http_redirect",
    summary="True when the response status is 3xx.",
    signatures=("http_redirect(response: object) -> boolean",),
    details="""
    Range predicate for the 300-399 redirect class.
    Pair with ``http_header(., "location")`` to extract the
    Location target.
    """,
    examples=(".urls[] | url_head(.) | select(http_redirect(.))",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_redirect(value: Any) -> bool:
    return _status_in_range(value, 300, 399, "http_redirect")


@_register(
    "http_client_error",
    summary="True when the response status is 4xx.",
    signatures=("http_client_error(response: object) -> boolean",),
    details="""
    Range predicate for the 400-499 client-error class.
    """,
    examples=(".urls[] | url_get(.) | select(http_client_error(.))",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_client_error(value: Any) -> bool:
    return _status_in_range(value, 400, 499, "http_client_error")


@_register(
    "http_server_error",
    summary="True when the response status is 5xx.",
    signatures=("http_server_error(response: object) -> boolean",),
    details="""
    Range predicate for the 500-599 server-error class.  When
    diffing an audit run, surfacing these reliably gives an
    operator the right signal — server errors typically need
    a different escalation path from 4xx client misuse.
    """,
    examples=(".urls[] | url_get(.) | select(http_server_error(.))",),
    category="net",
    min_args=1,
    max_args=1,
)
def _builtin_http_server_error(value: Any) -> bool:
    return _status_in_range(value, 500, 599, "http_server_error")


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
# Additional jq-compatible stream / list / set helpers
# ---------------------------------------------------------------------------


@_register(
    "dupes",
    summary="Return the duplicated items of a list — values that appear more than once, sorted.",
    signatures=("dupes(value: list | stream) -> list",),
    details="""
    Returns the items that appear **more than once** in *value*, with
    each duplicate represented once and the result sorted by the same
    ordering ``unique`` and ``sort`` use.

    The complement of ``unique``: where ``unique`` collapses a list to
    its distinct values, ``dupes`` keeps only the values whose count is
    at least two.  Useful for triage queries — finding shared pool
    names, repeated rule attachments, duplicated VIPs across configs.

    Empty input returns an empty list.  :class:`PathRef` items are
    compared on their ``full_path``, mixed types fall into jq's
    cross-type ordering.

    Related: ``unique``, ``unique_by``, ``group_by``, ``sort``.
    """,
    examples=(
        "[.ltm.virtual[].pool] | dupes            # pools attached to more than one VS",
        "[.ltm.virtual[].destination | host] | dupes  # IPs reused across VSes",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_dupes(value: Any) -> list[Any]:
    items = _as_sequence(value, name="dupes", arg=1)
    ordered = sorted(items, key=_sort_key)
    out: list[Any] = []
    i = 0
    n = len(ordered)
    while i < n:
        j = i + 1
        key_i = _sort_key(ordered[i])
        while j < n and _sort_key(ordered[j]) == key_i:
            j += 1
        if j - i >= 2:
            out.append(ordered[i])
        i = j
    return out


@_register(
    "reverse",
    summary="Reverse a list or string.",
    signatures=(
        "reverse(value: list | stream) -> list",
        "reverse(value: string) -> string",
    ),
    details="""
    Returns the input with its elements (or characters) in reverse
    order.  Matches jq's ``reverse``: lists and arrays reverse
    element-wise; strings reverse character-wise.

    :class:`PathRef` values are reversed as their ``full_path``
    string.  ``null`` returns ``null``.

    Related: ``sort``, ``first``, ``last``.
    """,
    examples=(
        "[.ltm.virtual[].name] | sort | reverse   # descending names",
        'reverse("abc")                           # -> "cba"',
    ),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_reverse(value: Any) -> Any:
    if value is None:
        return None
    if isinstance(value, str):
        return value[::-1]
    if isinstance(value, PathRef):
        return value.full_path[::-1]
    items = _as_sequence(value, name="reverse", arg=1)
    return list(reversed(items))


@_register(
    "add",
    summary="Combine the items of a list by ``+`` — sum numbers, concatenate strings/lists, merge objects.",
    signatures=("add(value: list | stream) -> any",),
    details="""
    Adds the elements of a list / stream together using the same
    semantics as the ``+`` operator (matches jq's ``add``):

    - **numbers**: arithmetic sum.
    - **strings**: concatenation, in order.
    - **lists**: concatenation (single level).
    - **objects**: shallow merge with later entries overwriting
      earlier ones.

    An **empty** input returns ``null`` (jq parity).  Items must be
    homogeneous; mixing types raises ``BuiltinError`` — coerce with
    ``str`` / ``tonumber`` / ``map`` first if needed.

    Related: ``flatten``, ``join`` (string-only with a separator),
    ``map``.
    """,
    examples=(
        "[1, 2, 3] | add                          # -> 6",
        '["foo", "bar"] | add                     # -> "foobar"',
        "[.ltm.virtual[].rules] | add             # flat list of every rule attachment",
        "[.ltm.virtual[].pool.members[]] | add | length",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_add(value: Any) -> Any:
    items = _as_sequence(value, name="add", arg=1)
    if not items:
        return None
    first = items[0]
    if isinstance(first, bool) or first is None:
        raise BuiltinError(
            f"add: cannot sum {_type_name(first)} values — coerce with ``str`` "
            "or ``tonumber`` first"
        )
    if isinstance(first, (int, float)):
        total: int | float = 0
        for item in items:
            if isinstance(item, bool) or not isinstance(item, (int, float)):
                raise BuiltinError(f"add: cannot mix {_type_name(first)} with {_type_name(item)}")
            total += item
        return total
    if isinstance(first, str) or isinstance(first, PathRef):
        parts: list[str] = []
        for item in items:
            if isinstance(item, PathRef):
                parts.append(item.full_path)
            elif isinstance(item, str):
                parts.append(item)
            else:
                raise BuiltinError(f"add: cannot mix string with {_type_name(item)}")
        return "".join(parts)
    if isinstance(first, (list, tuple)):
        out_list: list[Any] = []
        for item in items:
            if not isinstance(item, (list, tuple)):
                raise BuiltinError(f"add: cannot mix list with {_type_name(item)}")
            out_list.extend(item)
        return out_list
    if isinstance(first, dict):
        merged: dict[Any, Any] = {}
        for item in items:
            if not isinstance(item, dict):
                raise BuiltinError(f"add: cannot mix dict with {_type_name(item)}")
            merged.update(item)
        return merged
    raise BuiltinError(f"add: cannot sum values of type {_type_name(first)}")


@_register(
    "min",
    summary="Smallest element of a list or stream, or null when empty.",
    signatures=("min(value: list | stream) -> any",),
    details="""
    Returns the minimum element using jq's cross-type ordering
    (``null < false < true < numbers < strings < arrays < objects``).
    :class:`PathRef` collates by ``full_path``.

    Empty input returns ``null`` — matches jq.  For "smallest by a
    derived key" use ``min_by(f)``.

    Related: ``max``, ``min_by``, ``sort``, ``first``.
    """,
    examples=(
        "[1, 5, 2, 8, 3] | min                    # -> 1",
        "[.ltm.virtual[].name] | min              # alphabetically first VS name",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_min(value: Any) -> Any:
    items = _as_sequence(value, name="min", arg=1)
    if not items:
        return None
    return min(items, key=_sort_key)


@_register(
    "max",
    summary="Largest element of a list or stream, or null when empty.",
    signatures=("max(value: list | stream) -> any",),
    details="""
    Returns the maximum element using jq's cross-type ordering.
    :class:`PathRef` collates by ``full_path``.  Empty input returns
    ``null`` — matches jq.

    For "largest by a derived key" use ``max_by(f)``.

    Related: ``min``, ``max_by``, ``sort``, ``last``.
    """,
    examples=(
        "[1, 5, 2, 8, 3] | max                    # -> 8",
        "[.ltm.virtual[].name] | max              # alphabetically last VS name",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_max(value: Any) -> Any:
    items = _as_sequence(value, name="max", arg=1)
    if not items:
        return None
    return max(items, key=_sort_key)


@_register(
    "flatten",
    summary="Flatten a nested list by one level, or by *depth* when specified.",
    signatures=(
        "flatten() -> list",
        "flatten(depth: integer) -> list",
    ),
    details="""
    **Special form.**  Matches jq's ``flatten``: with no depth
    argument, flattens nested lists by exactly **one** level.  With a
    *depth* argument, flattens that many levels deep.  ``flatten(0)``
    is the identity.

    The value is always the current input — call as
    ``[X] | flatten`` or ``[X] | flatten(2)``.

    Non-list elements pass through unchanged at each level; this lets
    you flatten a mixed stream of "string or list of strings" without
    error.

    A negative depth raises ``BuiltinError`` — jq's behaviour is
    "flatten infinitely" for negative depths, but in this DSL that
    pattern is almost always a typo, so we reject it explicitly.

    Related: ``add`` (concatenates one level), ``map``.
    """,
    examples=(
        "[[1, 2], [3, 4]] | flatten               # -> [1, 2, 3, 4]",
        "[[1, [2, 3]], [4]] | flatten             # -> [1, [2, 3], 4]",
        "[[1, [2, 3]], [4]] | flatten(2)          # -> [1, 2, 3, 4]",
        "[.ltm.virtual[].rules] | flatten | unique",
    ),
    category="stream",
    min_args=0,
    max_args=1,
    special_form=True,
)
def _builtin_flatten(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("flatten must be evaluated through the evaluator")


def _flatten_value(value: Any, depth: int) -> list[Any]:
    """Helper used by the ``flatten`` special form."""
    items = _as_sequence(value, name="flatten", arg=1)
    if depth < 0:
        raise BuiltinError(f"flatten: depth must be non-negative, got {depth}")

    def _go(seq: list[Any], remaining: int) -> list[Any]:
        if remaining == 0:
            return list(seq)
        out: list[Any] = []
        for item in seq:
            if isinstance(item, (list, tuple)):
                out.extend(_go(list(item), remaining - 1))
            elif isinstance(item, Stream):
                out.extend(_go(list(item.items), remaining - 1))
            else:
                out.append(item)
        return out

    return _go(items, depth)


@_register(
    "range",
    summary="Generate a stream of integers — jq's range(); 1, 2, or 3 args.",
    signatures=(
        "range(upto: integer) -> stream[integer]",
        "range(from: integer, upto: integer) -> stream[integer]",
        "range(from: integer, upto: integer, step: integer) -> stream[integer]",
    ),
    details="""
    Matches jq's ``range`` exactly.  Emits a stream of integers:

    - One arg ``upto`` → ``0, 1, 2, … upto-1``.
    - Two args ``from, upto`` → ``from, from+1, … upto-1``.
    - Three args ``from, upto, step`` → arithmetic progression, stops
      strictly before *upto* (positive step) or strictly after *upto*
      (negative step).

    A *step* of zero raises ``BuiltinError``.  All arguments must be
    integers.

    Useful for synthetic streams: indexed enumeration, cartesian-style
    pairings with array generators, fixed-length placeholder runs.

    Related: ``limit``, ``nth``.
    """,
    examples=(
        "range(3)                                 # -> 0, 1, 2",
        "range(2, 6)                              # -> 2, 3, 4, 5",
        "range(0, 10, 2)                          # -> 0, 2, 4, 6, 8",
        "[range(5)] | map(. * 2)                  # -> [0, 2, 4, 6, 8]",
    ),
    category="stream",
    min_args=1,
    max_args=3,
)
def _builtin_range(*args: Any) -> Stream:
    if len(args) == 1:
        start = 0
        end = _as_int(args[0], name="range", arg=1)
        step = 1
    elif len(args) == 2:
        start = _as_int(args[0], name="range", arg=1)
        end = _as_int(args[1], name="range", arg=2)
        step = 1
    else:
        start = _as_int(args[0], name="range", arg=1)
        end = _as_int(args[1], name="range", arg=2)
        step = _as_int(args[2], name="range", arg=3)
    if step == 0:
        raise BuiltinError("range: step must be non-zero")
    return Stream(items=list(range(start, end, step)))


@_register(
    "nth",
    summary="The n-th element of a list or stream (0-indexed), or null when out of range.",
    signatures=("nth(value: list | stream, n: integer) -> any",),
    details="""
    Returns the *n*-th element (0-indexed) of a list or stream.  Out-
    of-range indices return ``null`` rather than raising — matches the
    "safe access" feel of ``first`` / ``last``.

    Accepts the jq-style implicit receiver: ``stream | nth(2)`` is the
    same as ``nth(stream, 2)``.  Negative indices count from the end
    (``nth(stream, -1)`` is the last item) — a convenience over jq,
    which doesn't accept negatives.

    Related: ``first``, ``last``, ``limit``, ``range``.
    """,
    examples=(
        "[.ltm.virtual[].name] | nth(0)           # first VS name",
        ".ltm.virtual.web_vs.rules | nth(0)        # first attached rule",
    ),
    category="stream",
    min_args=2,
    max_args=2,
    stream_aware=True,
)
def _builtin_nth(value: Any, n: Any) -> Any:
    items = _as_sequence(value, name="nth", arg=1)
    idx = _as_int(n, name="nth", arg=2)
    if -len(items) <= idx < len(items):
        return items[idx]
    return None


@_register(
    "limit",
    summary="Take the first *n* items of a list or stream.",
    signatures=("limit(value: list | stream, n: integer) -> list",),
    details="""
    Returns the first *n* items.  When the input has fewer than *n*
    items, returns the input unchanged; when *n* is zero or negative,
    returns an empty list.

    Convenience for "give me a preview of the result" or "cap a
    potentially-large stream" — paginate by combining with ``range``
    and slicing.

    Note: jq's ``limit(n; gen)`` is a two-argument special form that
    takes a generator expression.  This DSL's ``limit`` is the value
    form — pipe a stream / list into it, the same way ``count`` /
    ``sort`` work.

    Related: ``first``, ``nth``, ``range``.
    """,
    examples=(
        "[.ltm.virtual[].name] | sort | limit(., 5)  # first five names alphabetically",
        ".ltm.virtual[] | limit(3)                # first three VSes",
    ),
    category="stream",
    min_args=2,
    max_args=2,
    stream_aware=True,
)
def _builtin_limit(value: Any, n: Any) -> list[Any]:
    items = _as_sequence(value, name="limit", arg=1)
    count = _as_int(n, name="limit", arg=2)
    if count <= 0:
        return []
    return items[:count]


# ---------------------------------------------------------------------------
# Special-form stream helpers (sort_by, unique_by, min_by, max_by, group_by)
# ---------------------------------------------------------------------------


@_register(
    "sort_by",
    summary="Sort a list by the value of *body* evaluated against each item.",
    signatures=("sort_by(body) -> list",),
    details="""
    **Special form.**  Matches jq's ``sort_by``: for each input item,
    evaluates *body* with ``.`` re-bound to that item and uses the
    result as the sort key.  The original items are returned in order
    of their derived keys, using jq's cross-type ordering.

    The body is unevaluated AST and is re-run per item, so it may be a
    field projection (``sort_by(.name)``), a builtin call
    (``sort_by(partition(.))``), or an arithmetic expression.

    Stable: ties keep input order (Python's ``sorted`` is stable).

    Related: ``sort``, ``unique_by``, ``min_by``, ``max_by``,
    ``group_by``.
    """,
    examples=(
        "[.ltm.virtual[]] | sort_by(.name)",
        "[.ltm.pool[]] | sort_by(.members | length)  # smallest pools first",
        "[.ltm.virtual[]] | sort_by(partition(.name))",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_sort_by(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("sort_by must be evaluated through the evaluator")


@_register(
    "unique_by",
    summary="Sorted unique items, where uniqueness is determined by *body*.",
    signatures=("unique_by(body) -> list",),
    details="""
    **Special form.**  Matches jq's ``unique_by``: returns the unique
    items of the input, where two items are considered equal when
    *body* evaluates to the same value for both.  The result is
    sorted by the same key.

    Equivalent to ``[sort_by(body)] | <dedupe-by-key>``.  One
    representative per equivalence class survives — Python's ``sorted``
    is stable, so the representative is the **first** input occurrence
    of each key.

    Related: ``unique``, ``sort_by``, ``group_by``, ``dupes``.
    """,
    examples=(
        "[.ltm.virtual[]] | unique_by(.pool)      # one VS per distinct default pool",
        "[.ltm.virtual[]] | unique_by(partition(.name))",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_unique_by(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("unique_by must be evaluated through the evaluator")


@_register(
    "min_by",
    summary="Item whose *body* value is smallest under jq's cross-type ordering.",
    signatures=("min_by(body) -> any",),
    details="""
    **Special form.**  Matches jq's ``min_by``: for each input item,
    evaluates *body* with ``.`` re-bound, and returns the item whose
    derived key is the smallest under jq's cross-type ordering.

    On ties, returns the first such item (Python's ``min`` is stable).
    Empty input returns ``null``.

    Related: ``min``, ``max_by``, ``sort_by``, ``first``.
    """,
    examples=(
        "[.ltm.pool[]] | min_by(.members | length)  # smallest pool",
        "[.ltm.virtual[]] | min_by(.name)            # alphabetically first VS",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_min_by(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("min_by must be evaluated through the evaluator")


@_register(
    "max_by",
    summary="Item whose *body* value is largest under jq's cross-type ordering.",
    signatures=("max_by(body) -> any",),
    details="""
    **Special form.**  Like ``min_by`` but picks the largest.  Empty
    input returns ``null``.

    Related: ``max``, ``min_by``, ``sort_by``, ``last``.
    """,
    examples=(
        "[.ltm.pool[]] | max_by(.members | length)  # biggest pool",
        "[.ltm.virtual[]] | max_by(.name)            # alphabetically last VS",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_max_by(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("max_by must be evaluated through the evaluator")


@_register(
    "group_by",
    summary="Group items by the value of *body*; returns a list of groups sorted by key.",
    signatures=("group_by(body) -> list[list]",),
    details="""
    **Special form.**  Matches jq's ``group_by``: for each input item,
    evaluates *body* with ``.`` re-bound, then partitions the input
    into groups of items sharing the same key value.  The outer list
    is sorted by the group keys (jq's cross-type ordering); within
    each group, items preserve their input order.

    Pair with ``map(length)`` for a histogram, or ``map(first)`` for
    one representative per group (cheaper than ``unique_by`` when you
    also want the group counts).

    Related: ``sort_by``, ``unique_by``, ``map``, ``count``.
    """,
    examples=(
        "[.ltm.virtual[]] | group_by(partition(.name))",
        "[.ltm.virtual[]] | group_by(.pool) | map(length)  # VS count per pool",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_group_by(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("group_by must be evaluated through the evaluator")


# ---------------------------------------------------------------------------
# Dict / object operations (to_entries / from_entries / with_entries / has / in)
# ---------------------------------------------------------------------------


def _object_entries(value: Any, *, name: str) -> list[tuple[str, Any]]:
    """Return ``(key, value)`` pairs for an object-shaped value."""
    if isinstance(value, ObjectRef):
        return [(k, resolve_lazy_field(value, k, value.fields[k])) for k in sorted(value.fields)]
    if isinstance(value, dict):
        return [(k, value[k]) for k in sorted(value)]
    raise BuiltinError(f"{name}: argument 1 must be an object, got {_type_name(value)}")


@_register(
    "to_entries",
    summary="Convert an object to a list of ``{key, value}`` entries.",
    signatures=("to_entries(value: object) -> list[object]",),
    details="""
    Matches jq's ``to_entries``: an object ``{a: 1, b: 2}`` becomes
    the list ``[{"key": "a", "value": 1}, {"key": "b", "value": 2}]``.
    Entries are emitted in sorted key order so the result is
    deterministic.

    Useful for treating an object as a stream of named slots —
    iterate, filter, transform, then put the object back together
    with ``from_entries``.

    Related: ``from_entries``, ``with_entries``, ``keys``, ``values``.
    """,
    examples=(
        "{a: 1, b: 2} | to_entries",
        ".ltm.virtual.web_vs | to_entries | map(.key)  # field names, same as keys",
    ),
    category="value",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_to_entries(value: Any) -> list[dict[str, Any]]:
    return [{"key": k, "value": v} for k, v in _object_entries(value, name="to_entries")]


@_register(
    "from_entries",
    summary="Convert a list of ``{key, value}`` entries back into an object.",
    signatures=("from_entries(value: list[object]) -> object",),
    details="""
    Inverse of ``to_entries``.  Each entry may spell its key as
    ``key`` / ``k`` / ``name`` and its value as ``value`` / ``v``,
    matching jq's flexibility.  Missing values default to ``null``;
    missing keys raise.

    Keys are coerced to strings (jq parity — JSON object keys are
    strings).  Duplicate keys: later entries overwrite earlier ones.

    Related: ``to_entries``, ``with_entries``.
    """,
    examples=(
        '[{key: "a", value: 1}, {key: "b", value: 2}] | from_entries',
        "to_entries | from_entries                # round-trips an object",
    ),
    category="value",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_from_entries(value: Any) -> dict[str, Any]:
    items = _as_sequence(value, name="from_entries", arg=1)
    out: dict[str, Any] = {}
    for i, entry in enumerate(items):
        if isinstance(entry, dict):
            if "key" in entry:
                key = entry["key"]
            elif "k" in entry:
                key = entry["k"]
            elif "name" in entry:
                key = entry["name"]
            else:
                raise BuiltinError(
                    f"from_entries: entry {i} missing key (expected ``key``, ``k``, or ``name``)"
                )
            if "value" in entry:
                val = entry["value"]
            elif "v" in entry:
                val = entry["v"]
            else:
                val = None
        elif isinstance(entry, (list, tuple)) and len(entry) == 2:
            key, val = entry[0], entry[1]
        elif isinstance(entry, ObjectRef):
            fields = entry.fields
            if "key" in fields:
                key = resolve_lazy_field(entry, "key", fields["key"])
            elif "k" in fields:
                key = resolve_lazy_field(entry, "k", fields["k"])
            elif "name" in fields:
                key = resolve_lazy_field(entry, "name", fields["name"])
            else:
                raise BuiltinError(
                    f"from_entries: entry {i} missing key (expected ``key``, ``k``, or ``name``)"
                )
            if "value" in fields:
                val = resolve_lazy_field(entry, "value", fields["value"])
            elif "v" in fields:
                val = resolve_lazy_field(entry, "v", fields["v"])
            else:
                val = None
        else:
            raise BuiltinError(
                f"from_entries: entry {i} must be an object or 2-element list, "
                f"got {_type_name(entry)}"
            )
        if isinstance(key, PathRef):
            key = key.full_path
        if key is None:
            key = ""
        if not isinstance(key, (str, int)):
            raise BuiltinError(
                f"from_entries: entry {i} key must be a string or integer, got {_type_name(key)}"
            )
        out[str(key)] = val
    return out


@_register(
    "with_entries",
    summary="Apply *body* to each ``{key, value}`` entry of an object and reassemble.",
    signatures=("with_entries(body) -> object",),
    details="""
    **Special form.**  Matches jq's ``with_entries``: equivalent to
    ``to_entries | map(body) | from_entries``.  For each entry of the
    input object, evaluates *body* with ``.`` re-bound to a
    ``{key, value}`` object and collects the results into a new
    object.

    The body must yield ``{key, value}``-shaped objects (or the
    relaxed ``k`` / ``v`` / ``name`` spellings ``from_entries``
    accepts).  This DSL doesn't support property assignment on plain
    dicts, so use object literals to reshape entries (jq's
    ``with_entries(.key |= upcase)`` becomes
    ``with_entries({key: upcase(.key), value: .value})``).

    Returning the ``select`` drop sentinel drops the entry — handy
    for filtering object fields.

    Related: ``to_entries``, ``from_entries``, ``map``, ``select``.
    """,
    examples=(
        "with_entries({key: upcase(.key), value: .value})   # uppercase field names",
        'with_entries(select(.value | type == "string"))    # keep only string fields',
    ),
    category="value",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_with_entries(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("with_entries must be evaluated through the evaluator")


@_register(
    "has",
    summary="True when an object has the given field, or an array has the given index.",
    signatures=(
        "has(value: object, key: string) -> boolean",
        "has(value: list, index: integer) -> boolean",
    ),
    details="""
    Matches jq's ``has``:

    - For an **object** (``ObjectRef`` / ``dict``), tests whether the
      key is present.  Keys are strings; integer keys are coerced.
    - For a **list / stream**, tests whether the integer index is
      within bounds (``0 <= index < length``).

    Use the implicit-receiver form for the natural reading:
    ``.ltm.virtual[] | select(has(.snatpool))`` keeps only VSes whose
    object exposes a ``snatpool`` field.  Pair with ``defined`` when
    you also need to filter out an explicit empty string.

    Related: ``in`` (inverse), ``keys``, ``defined``.
    """,
    examples=(
        ".ltm.virtual[] | select(has(.snatpool)) | .name",
        '{a: 1, b: 2} | has("a")                  # -> true',
        "[10, 20, 30] | has(1)                    # -> true",
    ),
    category="value",
    min_args=2,
    max_args=2,
)
def _builtin_has(value: Any, key: Any) -> bool:
    if isinstance(value, ObjectRef):
        return _as_str(key, name="has", arg=2) in value.fields
    if isinstance(value, dict):
        if isinstance(key, PathRef):
            return key.full_path in value
        return key in value
    if isinstance(value, (list, tuple, Stream)):
        items = value.items if isinstance(value, Stream) else value
        idx = _as_int(key, name="has", arg=2)
        return 0 <= idx < len(items)
    if value is None:
        return False
    raise BuiltinError(f"has: cannot test membership on {_type_name(value)}")


@_register(
    "in",
    summary="True when the input is a key of the given object (or a valid index of the given array).",
    signatures=(
        "in(key: string, value: object) -> boolean",
        "in(index: integer, value: list) -> boolean",
    ),
    details="""
    Matches jq's ``in``: the inverse of ``has``.  The input is the key
    being tested; the argument is the container.  Reads naturally with
    the implicit-receiver form:
    ``"snatpool" | in(.ltm.virtual.web_vs)``.

    Related: ``has`` (inverse), ``keys``.
    """,
    examples=(
        '"name" | in({name: "x", pool: "y"})       # -> true',
        ".ltm.virtual[].name | select(in($wanted)) # keep VSes named in $wanted",
    ),
    category="value",
    min_args=2,
    max_args=2,
)
def _builtin_in(key: Any, value: Any) -> bool:
    return _builtin_has(value, key)


# ---------------------------------------------------------------------------
# Additional string builtins (ltrimstr / rtrimstr / tonumber / tostring /
# explode / implode / test / scan / capture / splits / ascii_*)
# ---------------------------------------------------------------------------


@_register(
    "ltrimstr",
    summary="Strip *prefix* from the start of a string if present; otherwise return unchanged.",
    signatures=("ltrimstr(value: string, prefix: string) -> string",),
    details="""
    Matches jq's ``ltrimstr``: if *value* starts with *prefix*, drops
    that prefix and returns the rest; otherwise returns *value*
    unchanged.  Accepts :class:`PathRef` on either side (coerced
    through ``full_path``).

    Idiomatic for normalising names:
    ``.ltm.virtual[].name | ltrimstr("vs_")``.

    Related: ``rtrimstr``, ``startswith``, ``sub``.
    """,
    examples=(
        'ltrimstr("vs_prod_web", "vs_")            # -> "prod_web"',
        'ltrimstr("api_vs", "vs_")                 # -> "api_vs" (unchanged)',
        '.ltm.virtual[].name | ltrimstr(., "vs_")',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_ltrimstr(value: Any, prefix: Any) -> str:
    s = _as_str(value, name="ltrimstr", arg=1)
    p = _as_str(prefix, name="ltrimstr", arg=2)
    return s[len(p) :] if s.startswith(p) else s


@_register(
    "rtrimstr",
    summary="Strip *suffix* from the end of a string if present; otherwise return unchanged.",
    signatures=("rtrimstr(value: string, suffix: string) -> string",),
    details="""
    Matches jq's ``rtrimstr``: if *value* ends with *suffix*, drops
    that suffix and returns the rest; otherwise returns *value*
    unchanged.

    Pairs with ``ltrimstr`` for symmetric stripping:
    ``.name | ltrimstr("vs_") | rtrimstr("_pool")``.

    Related: ``ltrimstr``, ``endswith``, ``sub``.
    """,
    examples=(
        'rtrimstr("web_pool", "_pool")            # -> "web"',
        'rtrimstr("web_pool", "_xxx")             # -> "web_pool" (unchanged)',
    ),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_rtrimstr(value: Any, suffix: Any) -> str:
    s = _as_str(value, name="rtrimstr", arg=1)
    p = _as_str(suffix, name="rtrimstr", arg=2)
    return s[: -len(p)] if p and s.endswith(p) else s


@_register(
    "tonumber",
    summary="Parse a string as a number; return numbers unchanged.",
    signatures=("tonumber(value: string | number) -> number",),
    details="""
    Matches jq's ``tonumber``: numeric input passes through; a string
    is parsed as an integer when possible, otherwise as a float.
    Leading / trailing whitespace is tolerated.

    Booleans are rejected — they are not numbers in jq.  ``null`` and
    non-numeric strings raise ``BuiltinError`` (jq raises too).

    Related: ``tostring``, ``str``, ``floor``, ``ceil``.
    """,
    examples=(
        'tonumber("42")                           # -> 42',
        'tonumber("3.14")                         # -> 3.14',
        "tonumber(.ltm.pool[].monitor.interval)",
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_tonumber(value: Any) -> int | float:
    if isinstance(value, bool):
        raise BuiltinError("tonumber: cannot convert bool to number")
    if isinstance(value, (int, float)):
        return value
    if value is None:
        raise BuiltinError("tonumber: cannot convert null to number")
    if isinstance(value, PathRef):
        text = value.full_path
    elif isinstance(value, str):
        text = value
    else:
        raise BuiltinError(f"tonumber: cannot convert {_type_name(value)} to number")
    stripped = text.strip()
    try:
        return int(stripped)
    except ValueError:
        pass
    try:
        return float(stripped)
    except ValueError as exc:
        raise BuiltinError(f"tonumber: cannot parse {text!r} as a number") from exc


@_register(
    "tostring",
    summary="Convert any value to its string form — jq parity alias of ``str``.",
    signatures=("tostring(value: any) -> string",),
    details="""
    Matches jq's ``tostring``:

    - **string** / :class:`PathRef`: returned as-is (PathRef →
      ``full_path``).
    - **integers** and **floats**: decimal form.
    - **booleans**: ``"true"`` / ``"false"``.
    - **null**: ``"null"``.
    - **lists** / **objects**: JSON-style encoding (jq parity —
      objects emit as JSON, not as TMSH stanzas).

    Use ``str`` for the scalar-only form that refuses to stringify
    aggregates; use ``tostring`` when you want the round-trip JSON
    spelling.

    Related: ``str``, ``tonumber``, ``join``.
    """,
    examples=(
        'tostring(42)                             # -> "42"',
        'tostring([1, 2, 3])                      # -> "[1,2,3]"',
        'tostring({a: 1})                         # -> "{\\"a\\":1}"',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_tostring(value: Any) -> str:
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
    if isinstance(value, (list, tuple, Stream, dict, ObjectRef)):
        import json

        return json.dumps(_to_jsonable(value), separators=(",", ":"), sort_keys=True)
    raise BuiltinError(f"tostring: cannot stringify {_type_name(value)}")


def _to_jsonable(value: Any) -> Any:
    """Coerce a DSL value into a JSON-serialisable Python object."""
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    if isinstance(value, PathRef):
        return value.full_path
    if isinstance(value, (list, tuple)):
        return [_to_jsonable(v) for v in value]
    if isinstance(value, Stream):
        return [_to_jsonable(v) for v in value.items]
    if isinstance(value, dict):
        return {str(k): _to_jsonable(v) for k, v in value.items()}
    if isinstance(value, ObjectRef):
        return {
            k: _to_jsonable(resolve_lazy_field(value, k, value.fields[k]))
            for k in sorted(value.fields)
        }
    return str(value)


@_register(
    "explode",
    summary="String to list of Unicode codepoints.",
    signatures=("explode(value: string) -> list[integer]",),
    details="""
    Matches jq's ``explode``: returns the input string as a list of
    integer codepoints.  Useful for character-level manipulation
    (case folding by codepoint table, ROT-N ciphers, codepoint
    arithmetic) before reassembling with ``implode``.

    :class:`PathRef` is accepted and exploded as its ``full_path``.

    Related: ``implode``, ``length``, ``split``.
    """,
    examples=(
        'explode("abc")                           # -> [97, 98, 99]',
        "explode(.name) | length                  # codepoint count",
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_explode(value: Any) -> list[int]:
    s = _as_str(value, name="explode", arg=1)
    return [ord(ch) for ch in s]


@_register(
    "implode",
    summary="List of Unicode codepoints back to a string.",
    signatures=("implode(value: list[integer]) -> string",),
    details="""
    Matches jq's ``implode``: inverse of ``explode``.  Each item must
    be a non-negative integer that names a valid Unicode codepoint.

    Related: ``explode``, ``join``.
    """,
    examples=(
        'implode([97, 98, 99])                    # -> "abc"',
        "explode(.name) | implode                  # round-trip identity",
    ),
    category="string",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_implode(value: Any) -> str:
    items = _as_sequence(value, name="implode", arg=1)
    parts: list[str] = []
    for i, item in enumerate(items):
        if isinstance(item, bool) or not isinstance(item, int):
            raise BuiltinError(
                f"implode: item {i} must be an integer codepoint, got {_type_name(item)}"
            )
        if item < 0 or item > 0x10FFFF:
            raise BuiltinError(f"implode: item {i} = {item} is not a valid Unicode codepoint")
        parts.append(chr(item))
    return "".join(parts)


@_register(
    "test",
    summary="Regex test — true when the pattern matches anywhere in the string (jq's ``test``).",
    signatures=(
        "test(value: string, pattern: string) -> boolean",
        "test(value: string, pattern: string, flags: string) -> boolean",
    ),
    details="""
    Matches jq's ``test``: a Boolean predicate that returns ``true``
    when *pattern* matches anywhere in *value*.  Same engine as
    ``match`` / ``sub`` / ``gsub`` — see those for the trust-boundary
    notes (length cap, refusal of catastrophic-backtracking shapes).

    Flags string is a subset of jq's: ``i`` for case-insensitive,
    ``x`` for free-spacing, ``s`` for dot-matches-newline, ``m`` for
    multi-line.  Unknown flags raise.

    This is the jq name; ``match`` is the legacy DSL name and remains
    available as the same Boolean predicate.  Pick whichever reads
    more naturally.

    Related: ``match``, ``scan``, ``capture``, ``sub``, ``gsub``.
    """,
    examples=(
        'test(.name, "^vs_")',
        'test(.name, "^VS_", "i")                 # case-insensitive',
    ),
    category="string",
    min_args=2,
    max_args=3,
)
def _builtin_test(value: Any, pattern: Any, flags: Any = "") -> bool:
    s = _as_str(value, name="test", arg=1)
    p = _as_str(pattern, name="test", arg=2)
    f = _as_str(flags, name="test", arg=3) if flags != "" else ""
    rx = _safe_regex_compile(p, name="test", flags=_jq_regex_flags(f, name="test"))
    return rx.search(s) is not None


@_register(
    "scan",
    summary="Stream of every regex match in a string.",
    signatures=(
        "scan(value: string, pattern: string) -> list",
        "scan(value: string, pattern: string, flags: string) -> list",
    ),
    details="""
    Matches jq's ``scan``: walks *value* finding every non-overlapping
    match of *pattern* and returns them as a list.

    - When *pattern* has **no capture groups**, each element is the
      matched substring.
    - When *pattern* has **one or more capture groups**, each element
      is a list of capture values (matching jq's array-per-match
      shape; the full match is **not** included).

    Empty matches at advancing positions are skipped to avoid infinite
    loops — Python's ``finditer`` already does this.

    Related: ``match`` / ``test`` (predicate), ``capture`` (named
    groups), ``splits``.
    """,
    examples=(
        'scan("a1 b22 c333", "[0-9]+")            # -> ["1", "22", "333"]',
        'scan("a=1 b=2 c=3", "([a-z])=([0-9])")   # -> [["a","1"], ["b","2"], ["c","3"]]',
    ),
    category="string",
    min_args=2,
    max_args=3,
)
def _builtin_scan(value: Any, pattern: Any, flags: Any = "") -> list[Any]:
    s = _as_str(value, name="scan", arg=1)
    p = _as_str(pattern, name="scan", arg=2)
    f = _as_str(flags, name="scan", arg=3) if flags != "" else ""
    rx = _safe_regex_compile(p, name="scan", flags=_jq_regex_flags(f, name="scan"))
    if rx.groups == 0:
        return [m.group(0) for m in rx.finditer(s)]
    return [list(m.groups()) for m in rx.finditer(s)]


@_register(
    "capture",
    summary="Named-group regex match — returns an object of capture names → captured text.",
    signatures=(
        "capture(value: string, pattern: string) -> object",
        "capture(value: string, pattern: string, flags: string) -> object",
    ),
    details="""
    Matches jq's ``capture``: runs *pattern* against *value* and
    returns an object mapping each **named** capture group to its
    matched text.  Use ``(?P<name>...)`` syntax for named groups (jq
    uses ``(?<name>...)``; both forms are accepted by Python's ``re``
    when ``(?P<name>...)`` is used, and ``capture`` rewrites jq-style
    ``(?<name>...)`` to the Python spelling so jq snippets paste
    through).

    Returns an empty object when the pattern has no named groups but
    matches; raises ``BuiltinError`` when the pattern doesn't match
    anywhere (jq parity — jq raises a no-match error too).

    Related: ``match``, ``scan``, ``sub``, ``gsub``.
    """,
    examples=(
        'capture(.destination, "(?<addr>[0-9.]+):(?<port>[0-9]+)")',
        'capture("vs_prod_web", "(?<env>prod|dev|qa)_(?<app>.+)")',
    ),
    category="string",
    min_args=2,
    max_args=3,
)
def _builtin_capture(value: Any, pattern: Any, flags: Any = "") -> dict[str, str]:
    s = _as_str(value, name="capture", arg=1)
    p = _as_str(pattern, name="capture", arg=2)
    f = _as_str(flags, name="capture", arg=3) if flags != "" else ""
    # jq accepts ``(?<name>...)``; Python's ``re`` wants ``(?P<name>...)``.
    # Rewrite the former into the latter so jq snippets paste through.
    p_python = re.sub(r"\(\?<([A-Za-z_][A-Za-z0-9_]*)>", r"(?P<\1>", p)
    rx = _safe_regex_compile(p_python, name="capture", flags=_jq_regex_flags(f, name="capture"))
    m = rx.search(s)
    if m is None:
        raise BuiltinError(f"capture: pattern {p!r} did not match")
    return {name: text for name, text in m.groupdict().items() if text is not None}


@_register(
    "splits",
    summary="Regex-based split — returns the substrings between matches.",
    signatures=(
        "splits(value: string, pattern: string) -> list[string]",
        "splits(value: string, pattern: string, flags: string) -> list[string]",
    ),
    details="""
    Matches jq's ``splits``: splits *value* on every (possibly empty)
    match of *pattern* and returns the substrings between matches as
    a list.

    Unlike ``split``, which takes a literal separator, ``splits``
    interprets its second argument as a regex.  Useful when the
    separator is irregular: variable whitespace, optional punctuation,
    multi-character alternatives.

    Related: ``split`` (literal), ``scan``, ``join``.
    """,
    examples=(
        'splits("a, b ,c,  d", " *, *")           # -> ["a", "b", "c", "d"]',
        'splits("v1.2.3-rc4", "[.-]")             # -> ["v1", "2", "3", "rc4"]',
    ),
    category="string",
    min_args=2,
    max_args=3,
)
def _builtin_splits(value: Any, pattern: Any, flags: Any = "") -> list[str]:
    s = _as_str(value, name="splits", arg=1)
    p = _as_str(pattern, name="splits", arg=2)
    f = _as_str(flags, name="splits", arg=3) if flags != "" else ""
    rx = _safe_regex_compile(p, name="splits", flags=_jq_regex_flags(f, name="splits"))
    return rx.split(s)


@_register(
    "ascii_upcase",
    summary="ASCII-only uppercase — jq parity alias of ``upcase``.",
    signatures=("ascii_upcase(value: string) -> string",),
    details="""
    Matches jq's ``ascii_upcase``: returns *value* with every ASCII
    letter (a-z) converted to uppercase, leaving non-ASCII letters
    untouched.  Identical to ``upcase`` in this DSL — both are
    ASCII-only.

    Provided for jq compatibility so snippets paste through.

    Related: ``upcase``, ``ascii_downcase``.
    """,
    examples=('ascii_upcase("vs_prod")                  # -> "VS_PROD"',),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_ascii_upcase(value: Any) -> str:
    return _as_str(value, name="ascii_upcase", arg=1).upper()


@_register(
    "ascii_downcase",
    summary="ASCII-only lowercase — jq parity alias of ``downcase``.",
    signatures=("ascii_downcase(value: string) -> string",),
    details="""
    Matches jq's ``ascii_downcase``.  Identical to ``downcase`` in
    this DSL.  Provided for jq compatibility.

    Related: ``downcase``, ``ascii_upcase``.
    """,
    examples=('ascii_downcase("VS_PROD")                # -> "vs_prod"',),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_ascii_downcase(value: Any) -> str:
    return _as_str(value, name="ascii_downcase", arg=1).lower()


def _jq_regex_flags(flags: str, *, name: str) -> int:
    """Translate jq-style regex flag letters to Python ``re`` flag bits.

    jq's regex flag string is a subset of PCRE.  This DSL accepts:

    - ``i`` — case-insensitive (``re.IGNORECASE``).
    - ``x`` — extended / free-spacing (``re.VERBOSE``).
    - ``s`` — single-line / dot matches newline (``re.DOTALL``).
    - ``m`` — multi-line (``re.MULTILINE``).

    Anything else raises so a typo in a copied jq pattern doesn't
    silently lose its case-insensitive flag.
    """
    out = 0
    for ch in flags:
        if ch == "i":
            out |= re.IGNORECASE
        elif ch == "x":
            out |= re.VERBOSE
        elif ch == "s":
            out |= re.DOTALL
        elif ch == "m":
            out |= re.MULTILINE
        else:
            raise BuiltinError(f"{name}: unsupported regex flag {ch!r} (supported: i, x, s, m)")
    return out


# ---------------------------------------------------------------------------
# Math builtins (floor / ceil / round / abs / sqrt / pow / exp / log / ...)
# ---------------------------------------------------------------------------


def _as_number(value: Any, *, name: str, arg: int) -> int | float:
    """Coerce *value* to a Python number, raising ``BuiltinError`` otherwise."""
    if isinstance(value, bool):
        raise BuiltinError(f"{name}: argument {arg} must be a number, got bool")
    if isinstance(value, (int, float)):
        return value
    raise BuiltinError(f"{name}: argument {arg} must be a number, got {_type_name(value)}")


@_register(
    "floor",
    summary="Round a number down to the nearest integer.",
    signatures=("floor(value: number) -> integer",),
    details="""
    Matches jq's ``floor``: returns the largest integer ``<= value``.
    Integers pass through unchanged.

    Related: ``ceil``, ``round``, ``abs``.
    """,
    examples=(
        "floor(3.7)                               # -> 3",
        "floor(-3.2)                              # -> -4",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_floor(value: Any) -> int:
    return math.floor(_as_number(value, name="floor", arg=1))


@_register(
    "ceil",
    summary="Round a number up to the nearest integer.",
    signatures=("ceil(value: number) -> integer",),
    details="""
    Matches jq's ``ceil``: returns the smallest integer ``>= value``.
    Integers pass through unchanged.

    Related: ``floor``, ``round``.
    """,
    examples=(
        "ceil(3.2)                                # -> 4",
        "ceil(-3.7)                               # -> -3",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_ceil(value: Any) -> int:
    return math.ceil(_as_number(value, name="ceil", arg=1))


@_register(
    "round",
    summary="Round a number to the nearest integer (ties away from zero — jq parity).",
    signatures=("round(value: number) -> integer",),
    details="""
    Matches jq's ``round`` (which calls C's ``round``, rounding ties
    away from zero — **not** Python's banker's rounding).
    ``round(0.5)`` → 1, ``round(-0.5)`` → -1, ``round(2.5)`` → 3.

    Related: ``floor``, ``ceil``, ``abs``.
    """,
    examples=(
        "round(2.5)                               # -> 3",
        "round(-2.5)                              # -> -3",
        "round(2.49)                              # -> 2",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_round(value: Any) -> int:
    n = _as_number(value, name="round", arg=1)
    # C ``round`` semantics: ties away from zero.  Python's built-in
    # ``round`` uses banker's rounding; we want jq parity.
    if n >= 0:
        return int(math.floor(n + 0.5))
    return -int(math.floor(-n + 0.5))


@_register(
    "abs",
    summary="Absolute value of a number.",
    signatures=("abs(value: number) -> number",),
    details="""
    Matches jq 1.7+'s ``abs``: returns the magnitude of a number.
    Integer in → integer out; float in → float out.

    Related: ``fabs`` (always float), ``floor``, ``ceil``.
    """,
    examples=(
        "abs(-5)                                  # -> 5",
        "abs(3.14)                                # -> 3.14",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_abs(value: Any) -> int | float:
    return abs(_as_number(value, name="abs", arg=1))


@_register(
    "fabs",
    summary="Absolute value as a float.",
    signatures=("fabs(value: number) -> number",),
    details="""
    Matches jq's ``fabs``: ``math.fabs`` from C — like ``abs`` but
    always returns a float, even for integer input.

    Related: ``abs``, ``floor``.
    """,
    examples=("fabs(-5)                                 # -> 5.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_fabs(value: Any) -> float:
    return math.fabs(_as_number(value, name="fabs", arg=1))


@_register(
    "sqrt",
    summary="Square root of a non-negative number.",
    signatures=("sqrt(value: number) -> number",),
    details="""
    Matches jq's ``sqrt``: returns ``math.sqrt(value)``.  Negative
    input raises ``BuiltinError`` (jq returns NaN; we prefer a
    visible error).

    Related: ``pow``, ``exp``.
    """,
    examples=("sqrt(16)                                 # -> 4.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_sqrt(value: Any) -> float:
    n = _as_number(value, name="sqrt", arg=1)
    if n < 0:
        raise BuiltinError(f"sqrt: negative input {n!r}")
    return math.sqrt(n)


@_register(
    "pow",
    summary="Raise *base* to the *exponent* power.",
    signatures=("pow(base: number, exponent: number) -> number",),
    details="""
    Matches jq's ``pow``: ``pow(x; y)`` = x^y.  Returns a float.

    Related: ``sqrt``, ``exp``, ``log``.
    """,
    examples=(
        "pow(2, 10)                               # -> 1024.0",
        "pow(2, 0.5)                              # -> sqrt(2)",
    ),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_pow(base: Any, exponent: Any) -> float:
    b = _as_number(base, name="pow", arg=1)
    e = _as_number(exponent, name="pow", arg=2)
    return math.pow(b, e)


@_register(
    "exp",
    summary="``e`` raised to the *value* power.",
    signatures=("exp(value: number) -> number",),
    details="""
    Matches jq's ``exp``: returns ``e^value``.

    Related: ``log``, ``pow``.
    """,
    examples=("exp(1)                                   # -> 2.718...",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_exp(value: Any) -> float:
    return math.exp(_as_number(value, name="exp", arg=1))


@_register(
    "log",
    summary="Natural logarithm (base e) of a positive number.",
    signatures=("log(value: number) -> number",),
    details="""
    Matches jq's ``log``: returns ``ln(value)``.  Non-positive input
    raises ``BuiltinError``.

    Related: ``log10``, ``log2``, ``exp``.
    """,
    examples=("log(2.71828)                             # -> ~1.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_log(value: Any) -> float:
    n = _as_number(value, name="log", arg=1)
    if n <= 0:
        raise BuiltinError(f"log: non-positive input {n!r}")
    return math.log(n)


@_register(
    "log10",
    summary="Base-10 logarithm of a positive number.",
    signatures=("log10(value: number) -> number",),
    details="""
    Matches jq's ``log10``.  Non-positive input raises.

    Related: ``log``, ``log2``.
    """,
    examples=("log10(1000)                              # -> 3.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_log10(value: Any) -> float:
    n = _as_number(value, name="log10", arg=1)
    if n <= 0:
        raise BuiltinError(f"log10: non-positive input {n!r}")
    return math.log10(n)


@_register(
    "log2",
    summary="Base-2 logarithm of a positive number.",
    signatures=("log2(value: number) -> number",),
    details="""
    Matches jq's ``log2``.  Non-positive input raises.

    Related: ``log``, ``log10``.
    """,
    examples=("log2(1024)                               # -> 10.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_log2(value: Any) -> float:
    n = _as_number(value, name="log2", arg=1)
    if n <= 0:
        raise BuiltinError(f"log2: non-positive input {n!r}")
    return math.log2(n)


@_register(
    "nan",
    summary="The not-a-number floating-point value.",
    signatures=("nan() -> number",),
    details="""
    Matches jq's ``nan``: returns the IEEE-754 NaN value.  Useful as
    a sentinel when arithmetic over partial data needs to propagate
    "not measured" through pipelines.

    Related: ``infinite``, ``isnan``.
    """,
    examples=("nan                                      # -> NaN",),
    category="math",
    min_args=0,
    max_args=0,
)
def _builtin_nan() -> float:
    return float("nan")


@_register(
    "infinite",
    summary="Positive infinity floating-point value.",
    signatures=("infinite() -> number",),
    details="""
    Matches jq's ``infinite``: returns positive ``inf``.  Negate with
    the unary ``-`` operator for negative infinity.

    Related: ``nan``, ``isinfinite``.
    """,
    examples=("infinite                                 # -> Infinity",),
    category="math",
    min_args=0,
    max_args=0,
)
def _builtin_infinite() -> float:
    return float("inf")


@_register(
    "isnan",
    summary="True when the value is the IEEE-754 NaN.",
    signatures=("isnan(value: number) -> boolean",),
    details="""
    Matches jq's ``isnan``.  Returns ``false`` for non-number input
    (which differs from jq's "is the input not a number" interpretation
    — that's a rarely-useful question, and a misuse on non-numbers is
    almost always a typo).

    Related: ``nan``, ``isinfinite``, ``isnormal``.
    """,
    examples=(
        "isnan(nan)                               # -> true",
        "isnan(1.0)                               # -> false",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_isnan(value: Any) -> bool:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    return math.isnan(value) if isinstance(value, float) else False


@_register(
    "isinfinite",
    summary="True when the value is positive or negative infinity.",
    signatures=("isinfinite(value: number) -> boolean",),
    details="""
    Matches jq's ``isinfinite``.  Returns ``false`` for non-number
    input.

    Related: ``infinite``, ``isnan``, ``isnormal``.
    """,
    examples=(
        "isinfinite(infinite)                     # -> true",
        "isinfinite(1.0)                          # -> false",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_isinfinite(value: Any) -> bool:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    return math.isinf(value) if isinstance(value, float) else False


@_register(
    "isnormal",
    summary="True when the value is a finite, non-zero, non-subnormal number.",
    signatures=("isnormal(value: number) -> boolean",),
    details="""
    Matches jq's ``isnormal``: true when the value is a finite,
    non-zero, non-subnormal number.  Integers count as normal when
    they are non-zero.

    Returns ``false`` for non-number input.

    Related: ``isnan``, ``isinfinite``.
    """,
    examples=(
        "isnormal(1.0)                            # -> true",
        "isnormal(0)                              # -> false",
        "isnormal(nan)                            # -> false",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_isnormal(value: Any) -> bool:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    if isinstance(value, int):
        return value != 0
    if not math.isfinite(value) or value == 0.0:
        return False
    # Subnormal: smallest normal positive float is 2**-1022.
    return abs(value) >= 2.2250738585072014e-308


# ---------------------------------------------------------------------------
# Path / tree manipulation (jq paths / getpath / setpath / del / delpaths)
# ---------------------------------------------------------------------------


def _value_children(value: Any) -> list[tuple[Any, Any]]:
    """Return ``(key, child)`` pairs for a composite value, or ``[]`` for leaves.

    Used by ``paths``, ``leaf_paths``, ``walk``, and ``recurse`` to walk
    the tree of any value uniformly.  Dict-like values yield string keys
    in sorted order; list-like values yield integer indices.
    """
    if isinstance(value, ObjectRef):
        return [(k, resolve_lazy_field(value, k, value.fields[k])) for k in sorted(value.fields)]
    if isinstance(value, dict):
        return [(k, value[k]) for k in sorted(value)]
    if isinstance(value, (list, tuple)):
        return list(enumerate(value))
    if isinstance(value, Stream):
        return list(enumerate(value.items))
    return []


def _is_container(value: Any) -> bool:
    return isinstance(value, (dict, list, tuple, Stream, ObjectRef))


def _coerce_path_key(key: Any, *, name: str, idx: int) -> str | int:
    """Coerce a single path element to a string or int."""
    if isinstance(key, bool):
        raise BuiltinError(f"{name}: path[{idx}] cannot be bool")
    if isinstance(key, (int, str)):
        return key
    if isinstance(key, PathRef):
        return key.full_path
    raise BuiltinError(f"{name}: path[{idx}] must be a string or integer, got {_type_name(key)}")


def _coerce_path_list(path: Any, *, name: str, arg: int) -> list[str | int]:
    """Coerce *path* to a list of string/int keys."""
    seq = _as_sequence(path, name=name, arg=arg)
    return [_coerce_path_key(k, name=name, idx=i) for i, k in enumerate(seq)]


def _value_at_path(value: Any, path: list[str | int]) -> Any:
    """Walk *value* following *path*; return ``None`` for missing keys."""
    cur = value
    for key in path:
        if cur is None:
            return None
        if isinstance(cur, ObjectRef):
            if not isinstance(key, str):
                return None
            fields = cur.fields
            if key not in fields:
                return None
            cur = resolve_lazy_field(cur, key, fields[key])
        elif isinstance(cur, dict):
            if key not in cur:
                return None
            cur = cur[key]
        elif isinstance(cur, (list, tuple)):
            if not isinstance(key, int):
                return None
            if not (-len(cur) <= key < len(cur)):
                return None
            cur = cur[key]
        elif isinstance(cur, Stream):
            if not isinstance(key, int):
                return None
            if not (-len(cur.items) <= key < len(cur.items)):
                return None
            cur = cur.items[key]
        else:
            return None
    return cur


def _set_at_path(value: Any, path: list[str | int], new_value: Any) -> Any:
    """Return a copy of *value* with *new_value* placed at *path*.

    Containers are copied along the path; values not on the path are
    shared.  ``ObjectRef`` and ``Stream`` are coerced to dict / list
    forms so the assignment is well-defined on plain Python types
    (jq parity — ``setpath`` returns a fresh value).
    """
    if not path:
        return new_value
    head = path[0]
    rest = path[1:]
    if value is None:
        # Autocreate the missing container based on the next key.
        value = {} if isinstance(head, str) else []
    if isinstance(value, ObjectRef):
        materialised = {k: resolve_lazy_field(value, k, value.fields[k]) for k in value.fields}
        return _set_at_path(materialised, path, new_value)
    if isinstance(value, dict):
        out_dict = dict(value)
        sub = out_dict.get(head)
        out_dict[head if isinstance(head, str) else str(head)] = _set_at_path(sub, rest, new_value)
        return out_dict
    if isinstance(value, (list, tuple, Stream)):
        items = list(value.items) if isinstance(value, Stream) else list(value)
        if not isinstance(head, int):
            raise BuiltinError(
                f"setpath: cannot index list-like value with {_type_name(head)} key {head!r}"
            )
        if head < 0:
            head = len(items) + head
        while head >= len(items):
            items.append(None)
        items[head] = _set_at_path(items[head], rest, new_value)
        return items
    raise BuiltinError(f"setpath: cannot descend into {_type_name(value)} at key {head!r}")


def _delete_at_path(value: Any, path: list[str | int]) -> Any:
    """Return a copy of *value* with *path* removed.  Missing path is a no-op."""
    if not path:
        return None
    head = path[0]
    rest = path[1:]
    if isinstance(value, ObjectRef):
        materialised = {k: resolve_lazy_field(value, k, value.fields[k]) for k in value.fields}
        return _delete_at_path(materialised, path)
    if isinstance(value, dict):
        if head not in value:
            return dict(value)
        out_dict = dict(value)
        if not rest:
            del out_dict[head]
        else:
            out_dict[head] = _delete_at_path(out_dict[head], rest)
        return out_dict
    if isinstance(value, (list, tuple, Stream)):
        items = list(value.items) if isinstance(value, Stream) else list(value)
        if not isinstance(head, int):
            return items
        if head < 0:
            head = len(items) + head
        if not (0 <= head < len(items)):
            return items
        if not rest:
            del items[head]
        else:
            items[head] = _delete_at_path(items[head], rest)
        return items
    return value


def _all_paths(value: Any, *, only_leaves: bool) -> list[list[str | int]]:
    """Return every path through *value*.

    When *only_leaves* is true, returns only paths to non-container
    values (matching jq's ``leaf_paths``).  Otherwise emits every
    non-root path (matching jq's ``paths``).
    """
    out: list[list[str | int]] = []

    def walk(cur: Any, prefix: list[str | int]) -> None:
        if prefix:
            if only_leaves:
                if not _is_container(cur):
                    out.append(list(prefix))
            else:
                out.append(list(prefix))
        for key, child in _value_children(cur):
            prefix.append(key)
            walk(child, prefix)
            prefix.pop()

    walk(value, [])
    return out


@_register(
    "paths",
    summary="Every path through the value as a stream of lists.",
    signatures=(
        "paths() -> stream[list]",
        "paths(filter) -> stream[list]",
    ),
    details="""
    **Special form.**  Matches jq's ``paths``: emits a stream where
    each element is a path (list of strings / integers) into the
    current value.  Every reachable composite **and** leaf position
    is enumerated except the empty root path.

    With a *filter* argument, only emits paths whose value satisfies
    *filter*: ``paths(type == "string")`` yields every path to a
    string leaf.

    Useful for introspection, audit, and as input to ``getpath`` /
    ``setpath`` / ``delpaths``.

    Related: ``leaf_paths``, ``getpath``, ``setpath``, ``del``,
    ``delpaths``, ``walk``.
    """,
    examples=(
        "{a: 1, b: [2, 3]} | paths",
        'paths(type == "number")',
    ),
    category="value",
    min_args=0,
    max_args=1,
    special_form=True,
)
def _builtin_paths(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("paths must be evaluated through the evaluator")


@_register(
    "leaf_paths",
    summary="Every path through the value that ends at a non-container leaf.",
    signatures=("leaf_paths() -> stream[list]",),
    details="""
    Matches jq's ``leaf_paths``: emits the path to every leaf value
    (string, number, bool, null) inside the current value.  Internal
    composite positions are not included.

    Equivalent to ``paths(. | type != "object" and . | type != "array")``.

    Related: ``paths``, ``getpath``.
    """,
    examples=(
        "{a: 1, b: [2, 3]} | leaf_paths",
        "[.ltm.virtual.web_vs] | leaf_paths      # every leaf inside one VS",
    ),
    category="value",
    min_args=0,
    max_args=0,
    special_form=True,
)
def _builtin_leaf_paths(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("leaf_paths must be evaluated through the evaluator")


@_register(
    "getpath",
    summary="Read the value at the given path (list of string / integer keys).",
    signatures=("getpath(path: list) -> any",),
    details="""
    Matches jq's ``getpath``: walks the current value following each
    element of *path* (strings index objects, integers index arrays /
    streams).  Returns ``null`` when any element is missing or
    out-of-range.

    Composes naturally with ``paths``: ``[paths] | map(getpath(.))``
    enumerates every reachable value.

    Related: ``setpath``, ``del``, ``delpaths``, ``paths``.
    """,
    examples=(
        'getpath(["a", "b"])                     # walk .a.b safely',
        '{a: {b: 42}} | getpath(["a", "b"])      # -> 42',
    ),
    category="value",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_getpath(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("getpath must be evaluated through the evaluator")


@_register(
    "setpath",
    summary="Return the current value with *path* set to *new_value*.",
    signatures=("setpath(path: list, new_value: any) -> any",),
    details="""
    Matches jq's ``setpath``: creates a copy of the current value with
    the slot at *path* set to *new_value*.  Missing intermediate
    containers are auto-created based on the next key type: a string
    key creates a dict, an integer key creates a list.

    Returns a fresh Python value — this is a functional update, not
    an in-place mutation.  For BIG-IP edit-pipeline writes, use the
    ``=`` / ``|=`` assignment operators instead.

    Related: ``getpath``, ``del``, ``delpaths``.
    """,
    examples=(
        '{a: 1} | setpath(["b", "c"], 9)         # -> {a:1, b:{c:9}}',
        '{a: [1, 2, 3]} | setpath(["a", 1], 99)',
    ),
    category="value",
    min_args=2,
    max_args=2,
    special_form=True,
)
def _builtin_setpath(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("setpath must be evaluated through the evaluator")


@_register(
    "del",
    summary="Return the current value with the slot at *path* removed.",
    signatures=("del(path: list) -> any",),
    details="""
    Matches jq's ``del`` for the single-path form.  Removes the slot
    at *path* and returns a fresh copy of the current value.  Missing
    paths are silently no-ops (jq parity).

    jq's ``del`` accepts a path **expression** (``del(.foo)``); this
    DSL takes a path **list** instead (``del(["foo"])``) so we don't
    have to introduce a third evaluation mode.  Use ``delpaths`` for
    deleting multiple paths in one call.

    Related: ``delpaths``, ``setpath``, ``getpath``.
    """,
    examples=(
        '{a: 1, b: 2} | del(["a"])               # -> {b: 2}',
        '{a: [1, 2, 3]} | del(["a", 1])           # -> {a: [1, 3]}',
    ),
    category="value",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_del(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("del must be evaluated through the evaluator")


@_register(
    "delpaths",
    summary="Return the current value with every slot in *paths* removed.",
    signatures=("delpaths(paths: list[list]) -> any",),
    details="""
    Matches jq's ``delpaths``: takes a list of path-lists and returns
    a copy of the current value with every slot deleted.  Paths must
    be sorted long-to-short (jq's expectation) — this DSL sorts them
    internally so callers don't have to worry about deletion order
    invalidating later paths.

    Related: ``del``, ``setpath``, ``paths``.
    """,
    examples=('{a: 1, b: 2, c: 3} | delpaths([["a"], ["c"]])  # -> {b: 2}',),
    category="value",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_delpaths(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("delpaths must be evaluated through the evaluator")


@_register(
    "walk",
    summary="Apply *body* to every value bottom-up, returning the rebuilt structure.",
    signatures=("walk(body) -> any",),
    details="""
    **Special form.**  Matches jq's ``walk``: traverses the current
    value's tree bottom-up.  For each composite, ``walk`` first
    recurses into its children, then evaluates *body* with ``.``
    re-bound to the (now-transformed) composite.  Leaves are passed
    directly to *body*.

    Classic uses:

    - **String normalisation across a whole config**:
      ``walk(if type == "string" then ascii_downcase else . end)``.
    - **Field renames**: ``walk(if type == "object" then
      with_entries(...) else . end)``.
    - **Pruning**: ``walk(if type == "array" then map(select(.))
      else . end)`` drops empty entries from every array.

    Related: ``recurse``, ``map``, ``with_entries``.
    """,
    examples=(
        'walk(if type == "string" then ascii_downcase else . end)',
        'walk(if type == "number" then . + 1 else . end)',
    ),
    category="value",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_walk(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("walk must be evaluated through the evaluator")


@_register(
    "recurse",
    summary="Emit every value reachable from the current input, optionally driven by *body*.",
    signatures=(
        "recurse() -> stream",
        "recurse(body) -> stream",
        "recurse(body, cond) -> stream",
    ),
    details="""
    **Special form.**  Matches jq's ``recurse`` family.

    - **Zero args** (``recurse``) — emits the current value, then
      every reachable value (every dict value, every array element,
      every nested composite, recursively).
    - **One arg** (``recurse(body)``) — emits the current value, then
      ``body`` applied once, then ``body`` applied to that, etc.
      Stops on null.
    - **Two args** (``recurse(body, cond)``) — same as the one-arg
      form but stops when ``cond`` is false.

    To prevent runaway loops, this DSL caps total emissions at
    100,000 — pipelines that legitimately need more should narrow
    *body* or pre-collect.

    Related: ``walk``, ``map``, ``paths``.
    """,
    examples=(
        "{a: 1, b: {c: 2}} | recurse",
        "1 | recurse(. + 1, . < 5)              # -> 1, 2, 3, 4",
    ),
    category="value",
    min_args=0,
    max_args=2,
    special_form=True,
)
def _builtin_recurse(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("recurse must be evaluated through the evaluator")


@_register(
    "until",
    summary="Iterate *update* against the current value until *cond* becomes true.",
    signatures=("until(cond, update) -> any",),
    details="""
    **Special form.**  Matches jq's ``until``: starting from the
    current value, repeatedly applies *update* (with ``.`` re-bound
    to the running value) and tests *cond* against the result.
    Returns the first value for which *cond* is truthy.

    Capped at 100,000 iterations to prevent runaway loops — pipelines
    that legitimately need more should restructure.

    Related: ``recurse``, ``repeat``.
    """,
    examples=(
        "1 | until(. >= 100, . * 2)              # -> 128",
        "0 | until(. > 5, . + 1)                  # -> 6",
    ),
    category="value",
    min_args=2,
    max_args=2,
    special_form=True,
)
def _builtin_until(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("until must be evaluated through the evaluator")


@_register(
    "repeat",
    summary="Emit ``f(.), f(f(.)), …`` capped at 100,000 iterations.",
    signatures=("repeat(body) -> stream",),
    details="""
    **Special form.**  Matches jq's ``repeat`` modulo a safety cap.
    Emits an infinite-by-design stream of successive applications of
    *body* to the current value.  jq pairs this with ``limit(n; ...)``
    to bound the result; this DSL also enforces an absolute cap of
    100,000 emissions so a forgetful pipeline can't wedge the
    evaluator.

    Common pattern: ``[limit(5, repeat(. + 1))]`` (using jq-style
    semantics) — though in this DSL ``[range(5)] | map(. + 1)`` is
    usually shorter.

    Related: ``until``, ``recurse``, ``range``.
    """,
    examples=("1 | [limit(repeat(. + 1), 5)]            # capped to 5",),
    category="value",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_repeat(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("repeat must be evaluated through the evaluator")


# ---------------------------------------------------------------------------
# Flow / control (empty, error, not, inside, IN, INDEX, combinations)
# ---------------------------------------------------------------------------


@_register(
    "empty",
    summary="Emit no values — the zero element of jq's stream algebra.",
    signatures=("empty() -> stream",),
    details="""
    Matches jq's ``empty``: produces a stream with zero items.  Used
    inside ``if ... then ... else empty end`` to silently drop a
    branch, or inside ``map(...)`` to filter without ``select``.

    Related: ``select`` (drops one item), ``error`` (raises instead).
    """,
    examples=(
        "if .pool then .name else empty end",
        "map(if .x > 0 then .x else empty end)",
    ),
    category="stream",
    min_args=0,
    max_args=0,
)
def _builtin_empty() -> Stream:
    return Stream(items=[])


@_register(
    "error",
    summary="Raise a query error with a custom message.",
    signatures=(
        "error() -> never",
        "error(msg: string) -> never",
    ),
    details="""
    Matches jq's ``error``: aborts the query with an error.  With no
    argument, raises with the current value's string form; with a
    message, raises with that message.

    Useful for fail-fast validation inside ``if`` / ``select``
    pipelines: ``if .destination == null then error("VS has no dest")
    else . end``.

    Related: ``select``, ``empty``.
    """,
    examples=('if .pool == null then error("no pool") else . end',),
    category="stream",
    min_args=0,
    max_args=1,
)
def _builtin_error(*args: Any) -> Any:
    if not args:
        raise BuiltinError("error: query halted by error()")
    msg = args[0]
    if isinstance(msg, str):
        text = msg
    elif isinstance(msg, PathRef):
        text = msg.full_path
    else:
        text = repr(msg)
    raise BuiltinError(text)


@_register(
    "not",
    summary="Invert the truthiness of the input — jq's postfix ``not``.",
    signatures=("not() -> boolean",),
    details="""
    Matches jq's ``not``: returns ``true`` when the current value is
    falsy and ``false`` otherwise.  Truthiness follows the DSL's
    usual rules (null / false / empty string / empty list are falsy).

    The DSL also exposes ``not`` as a unary prefix operator — both
    forms exist for jq snippet compatibility.

    Related: ``select``, ``any``, ``all``.
    """,
    examples=(
        ".pool | not                             # true when pool is unset",
        ".ltm.virtual[] | select(.snatpool | not)",
    ),
    category="stream",
    min_args=0,
    max_args=0,
    special_form=True,
)
def _builtin_not(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("not must be evaluated through the evaluator")


@_register(
    "inside",
    summary="Inverse of ``contains`` — current is *inside* the given container.",
    signatures=("inside(needle: any, container: any) -> boolean",),
    details="""
    Matches jq's ``inside``: ``a | inside(b)`` is equivalent to
    ``b | contains(a)``.  String substring test, list element test,
    dict submap test.  Use the implicit-receiver form for the
    natural reading: ``"bar" | inside("foobar")``.

    Related: ``contains``, ``has``, ``in``.
    """,
    examples=(
        '"bar" | inside("foobar")                 # -> true',
        '"/Common/log_rule" | inside(.rules)      # rule in attached set?',
    ),
    category="stream",
    min_args=2,
    max_args=2,
)
def _builtin_inside(needle: Any, container: Any) -> bool:
    return _builtin_contains(container, needle)


@_register(
    "combinations",
    summary="Cartesian product of a list of lists.",
    signatures=(
        "combinations() -> stream[list]",
        "combinations(n: integer) -> stream[list]",
    ),
    details="""
    **Special form.**  Matches jq's ``combinations``: with no
    argument, returns every combination drawn from a list of lists —
    one element from each sub-list.  With an integer ``n``, returns
    every n-length combination of the current list's elements
    (repeats allowed).

    The result is a stream of lists.  For empty input or empty
    sub-lists, the stream is empty.

    Operates on the current input — call via pipe
    (``[X] | combinations``) or directly (``combinations``).

    Related: ``map``, ``range``, ``flatten``.
    """,
    examples=(
        "[[1, 2], [3, 4]] | combinations          # -> [1,3], [1,4], [2,3], [2,4]",
        "[1, 2, 3] | combinations(2)              # -> [1,1], [1,2], ..., [3,3]",
    ),
    category="stream",
    min_args=0,
    max_args=1,
    special_form=True,
)
def _builtin_combinations(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("combinations must be evaluated through the evaluator")


def _combinations_impl(value: Any, n: int | None) -> Stream:
    items = _as_sequence(value, name="combinations", arg=1)
    if n is not None:
        if n < 0:
            raise BuiltinError(f"combinations: n must be non-negative, got {n}")
        sub_lists = [items] * n
    else:
        sub_lists = []
        for i, sub in enumerate(items):
            if not isinstance(sub, (list, tuple, Stream)):
                raise BuiltinError(f"combinations: item {i} must be a list, got {_type_name(sub)}")
            sub_lists.append(list(sub.items) if isinstance(sub, Stream) else list(sub))

    def _cart(prefix: list[Any], remaining: list[list[Any]]) -> list[list[Any]]:
        if not remaining:
            return [prefix]
        head, *tail = remaining
        out: list[list[Any]] = []
        for item in head:
            out.extend(_cart([*prefix, item], tail))
        return out

    return Stream(items=_cart([], sub_lists))


@_register(
    "IN",
    summary="True when the current value equals any of the candidate arguments.",
    signatures=("IN(...candidates) -> boolean",),
    details="""
    **Special form.**  Matches jq's ``IN(s)``: returns ``true`` when
    the current value compares equal to **any** of the values in the
    argument stream.

    Our DSL accepts the candidates as ordinary function arguments —
    ``IN("a", "b", "c")`` — because comma inside a call is the arg
    separator, not jq's stream-concat operator.  Each argument is
    evaluated against the current input; stream-valued arguments
    expand element-wise so the jq pattern
    ``.ltm.virtual[].name | IN([candidates] | .[])`` works the same
    way.

    Short-circuits on the first match.

    Related: ``INDEX``, ``contains``, ``in``, ``any``.
    """,
    examples=(
        '.name | IN("a", "b", "c")',
        '.ltm.virtual[].name | select(IN("web_vs", "api_vs"))',
    ),
    category="stream",
    min_args=1,
    max_args=None,
    special_form=True,
)
def _builtin_IN(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("IN must be evaluated through the evaluator")


@_register(
    "INDEX",
    summary="Build an object keyed by *idx_expr* evaluated against each item.",
    signatures=(
        "INDEX(idx_expr) -> object",
        "INDEX(source, idx_expr) -> object",
    ),
    details="""
    **Special form.**  Matches jq's ``INDEX``: with one argument,
    indexes the current list / stream by the value of *idx_expr*
    per item.  With two arguments, indexes the stream produced by
    *source* the same way.

    Duplicate keys: last write wins (jq parity).  Keys are coerced
    to strings.

    Related: ``group_by``, ``unique_by``, ``to_entries``.
    """,
    examples=(
        "[.ltm.virtual[]] | INDEX(.name)",
        "INDEX(.ltm.virtual[]; .name)             # jq's two-arg form",
    ),
    category="stream",
    min_args=1,
    max_args=2,
    special_form=True,
)
def _builtin_INDEX(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("INDEX must be evaluated through the evaluator")


# ---------------------------------------------------------------------------
# JSON encoding / decoding (tojson / fromjson) and @-prefix format helpers
# ---------------------------------------------------------------------------


@_register(
    "tojson",
    summary="Encode the current value as a JSON string (jq's ``tojson`` / ``@json``).",
    signatures=("tojson(value: any) -> string",),
    details="""
    Matches jq's ``tojson`` and the ``@json`` format string: returns
    a JSON encoding of *value*.  Numbers, booleans, and ``null``
    serialise as JSON literals; strings and PathRef values quote;
    lists and dicts recurse.

    Identical in output to ``tostring`` for aggregates, but always
    returns valid JSON (``tostring`` of a string returns the string
    unchanged — ``tojson`` quotes it).

    Related: ``fromjson``, ``tostring``, ``str``.
    """,
    examples=(
        'tojson(42)                               # -> "42"',
        'tojson("hi")                            # -> \'"hi"\'',
        'tojson({a: 1, b: [2, 3]})                # -> \'{"a":1,"b":[2,3]}\'',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_tojson(value: Any) -> str:
    import json

    return json.dumps(_to_jsonable(value), separators=(",", ":"), sort_keys=True)


@_register(
    "fromjson",
    summary="Parse a JSON-encoded string into a value (jq's ``fromjson``).",
    signatures=("fromjson(value: string) -> any",),
    details="""
    Matches jq's ``fromjson``: parses *value* as JSON and returns
    the resulting Python value.  Numbers become int / float, strings
    quote, booleans pass through, ``null`` becomes Python ``None``,
    arrays become lists, objects become dicts.

    Raises ``BuiltinError`` on malformed input.

    Related: ``tojson``, ``json_load``, ``json_parse``.
    """,
    examples=(
        'fromjson("42")                         # -> 42',
        'fromjson("{\\"a\\": 1}")                  # -> {a: 1}',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_fromjson(value: Any) -> Any:
    import json

    s = _as_str(value, name="fromjson", arg=1)
    try:
        return json.loads(s)
    except json.JSONDecodeError as exc:
        raise BuiltinError(f"fromjson: invalid JSON: {exc}") from exc


@_register(
    "uri",
    summary="URL-encode a string — equivalent of jq's ``@uri`` format string.",
    signatures=("uri(value: string) -> string",),
    details="""
    Matches jq's ``@uri``: percent-encodes characters that need
    escaping in URL components (everything outside the unreserved
    set ``A-Z a-z 0-9 - _ . ~``).

    Related: ``base64``, ``html``, ``sh``.
    """,
    examples=(
        'uri("hello world")                       # -> "hello%20world"',
        'uri("/Common/web_vs?x=1&y=2")',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_uri(value: Any) -> str:
    return _urlparse.quote(_as_str(value, name="uri", arg=1), safe="")


@_register(
    "base64",
    summary="Base64-encode a string — equivalent of jq's ``@base64`` format string.",
    signatures=("base64(value: string) -> string",),
    details="""
    Matches jq's ``@base64``: Base64-encodes the UTF-8 bytes of
    *value* and returns the standard-alphabet result.

    Related: ``base64d``, ``uri``.
    """,
    examples=('base64("hello")                          # -> "aGVsbG8="',),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_base64(value: Any) -> str:
    s = _as_str(value, name="base64", arg=1)
    return _base64.b64encode(s.encode("utf-8")).decode("ascii")


@_register(
    "base64d",
    summary="Base64-decode a string — equivalent of jq's ``@base64d`` format string.",
    signatures=("base64d(value: string) -> string",),
    details="""
    Matches jq's ``@base64d``: decodes a Base64-encoded ASCII string
    into the original UTF-8 text.  Malformed input raises
    ``BuiltinError``.

    Related: ``base64``.
    """,
    examples=('base64d("aGVsbG8=")                      # -> "hello"',),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_base64d(value: Any) -> str:
    s = _as_str(value, name="base64d", arg=1)
    try:
        return _base64.b64decode(s, validate=True).decode("utf-8")
    except (ValueError, UnicodeDecodeError) as exc:
        raise BuiltinError(f"base64d: invalid Base64 input: {exc}") from exc


@_register(
    "html",
    summary="HTML-escape a string — equivalent of jq's ``@html`` format string.",
    signatures=("html(value: string) -> string",),
    details="""
    Matches jq's ``@html``: replaces ``< > & ' "`` with their HTML
    entity equivalents so the result is safe to embed in HTML text
    or attribute values.

    Related: ``uri``, ``sh``.
    """,
    examples=('html("<a href=\\"x\\">&copy;</a>")',),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_html(value: Any) -> str:
    return _html.escape(_as_str(value, name="html", arg=1), quote=True)


@_register(
    "sh",
    summary="POSIX-shell-quote a string or list of strings — jq's ``@sh``.",
    signatures=(
        "sh(value: string) -> string",
        "sh(value: list[string]) -> string",
    ),
    details="""
    Matches jq's ``@sh``: returns a representation safe to interpolate
    into a POSIX shell command.  Strings become single-quoted with
    embedded ``'`` escaped; lists become space-separated quoted
    fields.

    Related: ``uri``, ``base64``, ``join``.
    """,
    examples=(
        'sh("hello world")                       # -> "\'hello world\'"',
        'sh(["a", "b c"])                          # -> "\'a\' \'b c\'"',
    ),
    category="string",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_sh(value: Any) -> str:
    if isinstance(value, (list, tuple, Stream)):
        items = value.items if isinstance(value, Stream) else value
        return " ".join(_shlex.quote(_as_str(item, name="sh", arg=1)) for item in items)
    return _shlex.quote(_as_str(value, name="sh", arg=1))


# ---------------------------------------------------------------------------
# String extensions (utf8bytelength, ascii)
# ---------------------------------------------------------------------------


@_register(
    "utf8bytelength",
    summary="Number of UTF-8 bytes the string encodes to.",
    signatures=("utf8bytelength(value: string) -> integer",),
    details="""
    Matches jq's ``utf8bytelength``: returns the byte count of
    *value*'s UTF-8 encoding.  Differs from ``length`` (which
    counts codepoints) whenever the string contains non-ASCII.

    Related: ``length``, ``explode``.
    """,
    examples=(
        'utf8bytelength("hello")                  # -> 5',
        'utf8bytelength("héllo")                  # -> 6  (é is 2 bytes)',
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_utf8bytelength(value: Any) -> int:
    s = _as_str(value, name="utf8bytelength", arg=1)
    return len(s.encode("utf-8"))


@_register(
    "ascii",
    summary="Codepoint integer to its single-character string form.",
    signatures=("ascii(value: integer) -> string",),
    details="""
    Sugar for ``[value] | implode``: returns the single-character
    string for a Unicode codepoint integer.  Useful inside arithmetic
    pipelines that compute characters by codepoint offset.

    Related: ``explode``, ``implode``.
    """,
    examples=(
        'ascii(65)                                # -> "A"',
        "65 | ascii                               # same via implicit receiver",
    ),
    category="string",
    min_args=1,
    max_args=1,
)
def _builtin_ascii(value: Any) -> str:
    n = _as_int(value, name="ascii", arg=1)
    if n < 0 or n > 0x10FFFF:
        raise BuiltinError(f"ascii: {n} is not a valid Unicode codepoint")
    return chr(n)


# ---------------------------------------------------------------------------
# Math (trig, hyperbolic, advanced C-math)
# ---------------------------------------------------------------------------


def _math_unary(fn, name: str):
    def impl(value: Any) -> float:
        return fn(_as_number(value, name=name, arg=1))

    impl.__name__ = f"_builtin_{name}"
    return impl


_TRIG_DOCS = {
    "sin": "Sine of a radian angle.",
    "cos": "Cosine of a radian angle.",
    "tan": "Tangent of a radian angle.",
    "asin": "Inverse sine in radians.",
    "acos": "Inverse cosine in radians.",
    "atan": "Inverse tangent in radians.",
    "sinh": "Hyperbolic sine.",
    "cosh": "Hyperbolic cosine.",
    "tanh": "Hyperbolic tangent.",
    "asinh": "Inverse hyperbolic sine.",
    "acosh": "Inverse hyperbolic cosine.",
    "atanh": "Inverse hyperbolic tangent.",
}


for _trig_name, _trig_summary in _TRIG_DOCS.items():
    _register(
        _trig_name,
        summary=_trig_summary + " Matches jq's namesake C-math function.",
        signatures=(f"{_trig_name}(value: number) -> number",),
        details=(
            f"    Matches jq's ``{_trig_name}``: thin wrapper over Python's\n"
            f"    ``math.{_trig_name}``.  Domain errors (``acos(2)`` etc.)\n"
            "    raise ``BuiltinError`` rather than returning NaN so the\n"
            "    failure shows in query output."
        ),
        examples=(f"{_trig_name}(0)", f"{_trig_name}(.angle)"),
        category="math",
        min_args=1,
        max_args=1,
    )(_math_unary(getattr(math, _trig_name), _trig_name))


@_register(
    "atan2",
    summary="Two-argument inverse tangent — ``atan2(y, x)``.",
    signatures=("atan2(y: number, x: number) -> number",),
    details="""
    Matches jq's ``atan2``: returns the angle in radians between the
    positive x-axis and the point (x, y), with the correct quadrant.

    Related: ``atan``, ``sin``, ``cos``.
    """,
    examples=("atan2(1, 1)                              # -> pi/4",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_atan2(y: Any, x: Any) -> float:
    return math.atan2(
        _as_number(y, name="atan2", arg=1),
        _as_number(x, name="atan2", arg=2),
    )


@_register(
    "cbrt",
    summary="Cube root of a number.",
    signatures=("cbrt(value: number) -> number",),
    details="""
    Matches jq's ``cbrt``: returns the real cube root of *value*.
    Handles negative inputs (``cbrt(-8) == -2``) — distinct from
    ``pow(., 1.0/3)`` which is undefined for negative bases.

    Related: ``sqrt``, ``pow``.
    """,
    examples=("cbrt(27)                                 # -> 3.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_cbrt(value: Any) -> float:
    n = _as_number(value, name="cbrt", arg=1)
    if n < 0:
        return -((-n) ** (1 / 3))
    return n ** (1 / 3)


@_register(
    "exp2",
    summary="``2`` raised to the *value* power.",
    signatures=("exp2(value: number) -> number",),
    details="""
    Matches jq's ``exp2``: returns ``2^value``.

    Related: ``exp``, ``exp10``, ``pow``, ``log2``.
    """,
    examples=("exp2(10)                                 # -> 1024.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_exp2(value: Any) -> float:
    return math.pow(2.0, _as_number(value, name="exp2", arg=1))


@_register(
    "exp10",
    summary="``10`` raised to the *value* power.",
    signatures=("exp10(value: number) -> number",),
    details="""
    Matches jq's ``exp10``: returns ``10^value``.

    Related: ``exp``, ``exp2``, ``pow``, ``log10``.
    """,
    examples=("exp10(3)                                 # -> 1000.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_exp10(value: Any) -> float:
    return math.pow(10.0, _as_number(value, name="exp10", arg=1))


@_register(
    "logb",
    summary="Integer binary exponent of *value* (``floor(log2(|value|))``).",
    signatures=("logb(value: number) -> number",),
    details="""
    Matches jq's ``logb`` / C's ``logb``: returns the exponent of
    *value*'s base-2 representation as a floating-point number.  For
    a non-zero finite *value* this is the integer part of
    ``log2(|value|)``.

    Returns ``-inf`` for zero and ``+inf`` for infinity (jq parity).

    Related: ``log2``, ``frexp``.
    """,
    examples=(
        "logb(8)                                  # -> 3.0",
        "logb(0.25)                               # -> -2.0",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_logb(value: Any) -> float:
    n = _as_number(value, name="logb", arg=1)
    if n == 0:
        return -math.inf
    if math.isinf(n):
        return math.inf
    return float(math.floor(math.log2(abs(n))))


@_register(
    "trunc",
    summary="Truncate toward zero — drop the fractional part.",
    signatures=("trunc(value: number) -> integer",),
    details="""
    Matches jq's ``trunc``: returns *value* with its fractional part
    removed, rounding toward zero.

    Related: ``floor``, ``ceil``, ``round``.
    """,
    examples=(
        "trunc(3.9)                               # -> 3",
        "trunc(-3.9)                              # -> -3",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_trunc(value: Any) -> int:
    return math.trunc(_as_number(value, name="trunc", arg=1))


@_register(
    "rint",
    summary="Round to the nearest integer, ties to even (banker's rounding).",
    signatures=("rint(value: number) -> integer",),
    details="""
    Matches jq's ``rint``: rounds to the nearest integer with
    ties-to-even semantics — distinct from ``round`` which uses
    ties-away-from-zero.

    Related: ``round``, ``floor``, ``ceil``.
    """,
    examples=(
        "rint(2.5)                                # -> 2  (banker's)",
        "rint(3.5)                                # -> 4",
    ),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_rint(value: Any) -> int:
    return int(round(_as_number(value, name="rint", arg=1)))


@_register(
    "copysign",
    summary="Magnitude of *x* with the sign of *y*.",
    signatures=("copysign(x: number, y: number) -> number",),
    details="""
    Matches jq's ``copysign``: ``copysign(x; y)`` returns a value with
    the magnitude of ``x`` and the sign of ``y``.

    Related: ``abs``, ``fabs``.
    """,
    examples=("copysign(3, -1)                          # -> -3.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_copysign(x: Any, y: Any) -> float:
    return math.copysign(
        _as_number(x, name="copysign", arg=1),
        _as_number(y, name="copysign", arg=2),
    )


@_register(
    "fdim",
    summary="Positive difference — ``max(x - y, 0)``.",
    signatures=("fdim(x: number, y: number) -> number",),
    details="""
    Matches jq's ``fdim``: returns ``max(x - y, 0)``.

    Related: ``min``, ``max``, ``abs``.
    """,
    examples=("fdim(5, 3)                               # -> 2.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_fdim(x: Any, y: Any) -> float:
    nx = _as_number(x, name="fdim", arg=1)
    ny = _as_number(y, name="fdim", arg=2)
    return max(float(nx - ny), 0.0)


@_register(
    "frexp",
    summary="Decompose a number into mantissa and exponent — returns ``[m, e]``.",
    signatures=("frexp(value: number) -> list",),
    details="""
    Matches jq's ``frexp``: returns ``[mantissa, exponent]`` such
    that ``value == mantissa * 2**exponent`` and ``0.5 <= |mantissa|
    < 1`` (or both parts zero when ``value`` is zero).

    jq returns a 2-element array; this DSL returns a Python list of
    the same shape.

    Related: ``ldexp``, ``modf``, ``logb``.
    """,
    examples=("frexp(12)                                # -> [0.75, 4]",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_frexp(value: Any) -> list[Any]:
    m, e = math.frexp(_as_number(value, name="frexp", arg=1))
    return [m, e]


@_register(
    "ldexp",
    summary="``mantissa * 2**exponent`` — inverse of ``frexp``.",
    signatures=("ldexp(mantissa: number, exponent: integer) -> number",),
    details="""
    Matches jq's ``ldexp``: returns ``mantissa * 2**exponent``.

    Related: ``frexp``, ``exp2``.
    """,
    examples=("ldexp(0.75, 4)                           # -> 12.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_ldexp(mantissa: Any, exponent: Any) -> float:
    return math.ldexp(
        _as_number(mantissa, name="ldexp", arg=1),
        _as_int(exponent, name="ldexp", arg=2),
    )


@_register(
    "modf",
    summary="Split a number into fractional and integer parts — returns ``[frac, int]``.",
    signatures=("modf(value: number) -> list",),
    details="""
    Matches jq's ``modf``: returns ``[frac, int]`` where ``frac`` is
    the fractional part of *value* and ``int`` is the integer part,
    both with the same sign as *value*.

    Related: ``trunc``, ``floor``.
    """,
    examples=("modf(3.75)                               # -> [0.75, 3.0]",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_modf(value: Any) -> list[Any]:
    frac, integ = math.modf(_as_number(value, name="modf", arg=1))
    return [frac, integ]


@_register(
    "gamma",
    summary="Gamma function — ``tgamma`` (jq alias).",
    signatures=("gamma(value: number) -> number",),
    details="""
    Matches jq's ``gamma`` / ``tgamma``: returns the gamma function
    of *value*.  Domain errors raise ``BuiltinError``.

    Related: ``lgamma``, ``tgamma``.
    """,
    examples=("gamma(5)                                 # -> 24.0  (4!)",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_gamma(value: Any) -> float:
    n = _as_number(value, name="gamma", arg=1)
    try:
        return math.gamma(n)
    except ValueError as exc:
        raise BuiltinError(f"gamma: {exc}") from exc


@_register(
    "tgamma",
    summary="Gamma function (alias of ``gamma``).",
    signatures=("tgamma(value: number) -> number",),
    details="""
    Matches jq's ``tgamma`` (C's ``tgamma``).  Same result as
    ``gamma``.

    Related: ``gamma``, ``lgamma``.
    """,
    examples=("tgamma(5)                                # -> 24.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_tgamma(value: Any) -> float:
    return _builtin_gamma(value)


@_register(
    "lgamma",
    summary="Natural log of the absolute value of the gamma function.",
    signatures=("lgamma(value: number) -> number",),
    details="""
    Matches jq's ``lgamma`` / ``lgamma_r``: returns ``log(|gamma(x)|)``
    — useful for combinatoric calculations where the gamma value
    would overflow.

    Related: ``gamma``, ``log``.
    """,
    examples=("lgamma(5)                                # -> log(24)",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_lgamma(value: Any) -> float:
    return math.lgamma(_as_number(value, name="lgamma", arg=1))


@_register(
    "significand",
    summary="Significand (mantissa with exponent normalised to zero).",
    signatures=("significand(value: number) -> number",),
    details="""
    Matches jq's ``significand``: returns the value scaled to the
    range ``[1, 2)`` (or ``[-2, -1]`` for negatives), i.e. divided by
    ``2 ** logb(value)``.  Returns the input unchanged for zero,
    infinities, and NaN.

    Related: ``frexp``, ``logb``.
    """,
    examples=("significand(12)                          # -> 1.5",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_significand(value: Any) -> float:
    n = _as_number(value, name="significand", arg=1)
    if n == 0 or math.isinf(n) or math.isnan(n):
        return float(n)
    e = math.floor(math.log2(abs(n)))
    return float(n) / math.pow(2.0, e)


@_register(
    "j0",
    summary="Bessel function of the first kind, order 0 — series approximation.",
    signatures=("j0(value: number) -> number",),
    details="""
    Matches jq's ``j0``: returns ``J_0(value)``, the Bessel function
    of the first kind at order 0.  Implemented via a polynomial /
    asymptotic approximation (Abramowitz & Stegun 9.4.1 / 9.2.5)
    accurate to ~1e-7 — adequate for the ad-hoc audit queries this
    DSL targets.  For research-grade precision use SciPy.

    Related: ``j1``, ``y0``, ``y1``.
    """,
    examples=("j0(0)                                    # -> 1.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_j0(value: Any) -> float:
    return _bessel_j0(_as_number(value, name="j0", arg=1))


@_register(
    "j1",
    summary="Bessel function of the first kind, order 1.",
    signatures=("j1(value: number) -> number",),
    details="""
    Matches jq's ``j1``: ``J_1(value)``, accuracy ~1e-7.  See
    ``j0`` for the approximation notes.

    Related: ``j0``, ``y0``, ``y1``.
    """,
    examples=("j1(0)                                    # -> 0.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_j1(value: Any) -> float:
    return _bessel_j1(_as_number(value, name="j1", arg=1))


@_register(
    "y0",
    summary="Bessel function of the second kind, order 0.",
    signatures=("y0(value: number) -> number",),
    details="""
    Matches jq's ``y0``: ``Y_0(value)``, defined for ``value > 0``.
    Series / asymptotic approximation accurate to ~1e-7.

    Related: ``y1``, ``j0``, ``j1``.
    """,
    examples=("y0(1)",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_y0(value: Any) -> float:
    n = _as_number(value, name="y0", arg=1)
    if n <= 0:
        raise BuiltinError(f"y0: argument must be positive, got {n}")
    return _bessel_y0(n)


@_register(
    "y1",
    summary="Bessel function of the second kind, order 1.",
    signatures=("y1(value: number) -> number",),
    details="""
    Matches jq's ``y1``: ``Y_1(value)``, defined for ``value > 0``.
    Series / asymptotic approximation accurate to ~1e-7.

    Related: ``y0``, ``j0``, ``j1``.
    """,
    examples=("y1(1)",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_y1(value: Any) -> float:
    n = _as_number(value, name="y1", arg=1)
    if n <= 0:
        raise BuiltinError(f"y1: argument must be positive, got {n}")
    return _bessel_y1(n)


def _bessel_j0(x: float) -> float:
    """Abramowitz & Stegun 9.4.1 / 9.4.3 approximation for J_0."""
    ax = abs(x)
    if ax < 8.0:
        y = x * x
        num = 57568490574.0 + y * (
            -13362590354.0
            + y * (651619640.7 + y * (-11214424.18 + y * (77392.33017 + y * (-184.9052456))))
        )
        den = 57568490411.0 + y * (
            1029532985.0 + y * (9494680.718 + y * (59272.64853 + y * (267.8532712 + y)))
        )
        return num / den
    z = 8.0 / ax
    y = z * z
    p0 = 1.0 + y * (
        -0.1098628627e-2 + y * (0.2734510407e-4 + y * (-0.2073370639e-5 + y * 0.2093887211e-6))
    )
    q0 = -0.1562499995e-1 + y * (
        0.1430488765e-3 + y * (-0.6911147651e-5 + y * (0.7621095161e-6 + y * (-0.934935152e-7)))
    )
    return math.sqrt(0.636619772 / ax) * (
        math.cos(ax - math.pi / 4) * p0 - z * math.sin(ax - math.pi / 4) * q0
    )


def _bessel_j1(x: float) -> float:
    """Abramowitz & Stegun 9.4.4 / 9.4.6 approximation for J_1."""
    ax = abs(x)
    if ax < 8.0:
        y = x * x
        num = x * (
            72362614232.0
            + y
            * (
                -7895059235.0
                + y * (242396853.1 + y * (-2972611.439 + y * (15704.48260 + y * (-30.16036606))))
            )
        )
        den = 144725228442.0 + y * (
            2300535178.0 + y * (18583304.74 + y * (99447.43394 + y * (376.9991397 + y)))
        )
        return num / den
    z = 8.0 / ax
    y = z * z
    p1 = 1.0 + y * (
        0.183105e-2 + y * (-0.3516396496e-4 + y * (0.2457520174e-5 + y * (-0.240337019e-6)))
    )
    q1 = 0.04687499995 + y * (
        -0.2002690873e-3 + y * (0.8449199096e-5 + y * (-0.88228987e-6 + y * 0.105787412e-6))
    )
    result = math.sqrt(0.636619772 / ax) * (
        math.cos(ax - 3 * math.pi / 4) * p1 - z * math.sin(ax - 3 * math.pi / 4) * q1
    )
    return -result if x < 0 else result


def _bessel_y0(x: float) -> float:
    """Abramowitz & Stegun approximation for Y_0(x), x > 0."""
    if x < 8.0:
        y = x * x
        num = -2957821389.0 + y * (
            7062834065.0
            + y * (-512359803.6 + y * (10879881.29 + y * (-86327.92757 + y * 228.4622733)))
        )
        den = 40076544269.0 + y * (
            745249964.8 + y * (7189466.438 + y * (47447.26470 + y * (226.1030244 + y)))
        )
        return num / den + 0.636619772 * _bessel_j0(x) * math.log(x)
    z = 8.0 / x
    y = z * z
    p0 = 1.0 + y * (
        -0.1098628627e-2 + y * (0.2734510407e-4 + y * (-0.2073370639e-5 + y * 0.2093887211e-6))
    )
    q0 = -0.1562499995e-1 + y * (
        0.1430488765e-3 + y * (-0.6911147651e-5 + y * (0.7621095161e-6 + y * (-0.934935152e-7)))
    )
    return math.sqrt(0.636619772 / x) * (
        math.sin(x - math.pi / 4) * p0 + z * math.cos(x - math.pi / 4) * q0
    )


def _bessel_y1(x: float) -> float:
    """Abramowitz & Stegun approximation for Y_1(x), x > 0."""
    if x < 8.0:
        y = x * x
        num = x * (
            -0.4900604943e13
            + y
            * (
                0.1275274390e13
                + y
                * (
                    -0.5153438139e11
                    + y * (0.7349264551e9 + y * (-0.4237922726e7 + y * 0.8511937935e4))
                )
            )
        )
        den = 0.2499580570e14 + y * (
            0.4244419664e12
            + y
            * (
                0.3733650367e10
                + y * (0.2245904002e8 + y * (0.1020426050e6 + y * (0.3549632885e3 + y)))
            )
        )
        return num / den + 0.636619772 * (_bessel_j1(x) * math.log(x) - 1.0 / x)
    z = 8.0 / x
    y = z * z
    p1 = 1.0 + y * (
        0.183105e-2 + y * (-0.3516396496e-4 + y * (0.2457520174e-5 + y * (-0.240337019e-6)))
    )
    q1 = 0.04687499995 + y * (
        -0.2002690873e-3 + y * (0.8449199096e-5 + y * (-0.88228987e-6 + y * 0.105787412e-6))
    )
    return math.sqrt(0.636619772 / x) * (
        math.sin(x - 3 * math.pi / 4) * p1 + z * math.cos(x - 3 * math.pi / 4) * q1
    )


# ---------------------------------------------------------------------------
# Time (now / fromdate / todate / mktime / gmtime / localtime / strftime / strptime)
# ---------------------------------------------------------------------------


def _broken_down_from_unix(unix_seconds: float, *, utc: bool) -> list[Any]:
    """Return the jq-shaped broken-down time array for *unix_seconds*."""
    if utc:
        tm = _time.gmtime(unix_seconds)
    else:
        tm = _time.localtime(unix_seconds)
    # jq's broken-down array order: [year-1900, month-0-indexed, day,
    # hour, minute, second, day-of-week (0=Sun), day-of-year-1].
    return [
        tm.tm_year - 1900,
        tm.tm_mon - 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        tm.tm_wday + 1 if tm.tm_wday < 6 else 0,  # 0=Sun..6=Sat (C convention)
        tm.tm_yday - 1,
    ]


def _broken_down_to_struct(arr: list[Any], *, name: str) -> _time.struct_time:
    """Inverse of :func:`_broken_down_from_unix` — to a ``struct_time``."""
    if not isinstance(arr, (list, tuple)) or len(arr) < 6:
        raise BuiltinError(
            f"{name}: broken-down time must be a list of at least 6 ints, got {_type_name(arr)}"
        )
    pieces = list(arr) + [0] * (9 - len(arr))
    year, month, day, hour, minute, second = (
        pieces[0] + 1900,
        pieces[1] + 1,
        pieces[2],
        pieces[3],
        pieces[4],
        pieces[5],
    )
    return _time.struct_time((year, month, day, hour, minute, int(second), 0, 1, -1))


@_register(
    "now",
    summary="Current Unix epoch time as a float.",
    signatures=("now() -> number",),
    details="""
    Matches jq's ``now``: returns the current time in seconds since
    the Unix epoch (UTC) as a float with sub-second resolution.

    Related: ``todate``, ``fromdate``, ``strftime``.
    """,
    examples=("now",),
    category="time",
    min_args=0,
    max_args=0,
)
def _builtin_now() -> float:
    return _time.time()


@_register(
    "todate",
    summary="Unix epoch seconds → ISO-8601 UTC string.",
    signatures=("todate(value: number) -> string",),
    details="""
    Matches jq's ``todate`` / ``todateiso8601``: formats a Unix
    epoch-seconds value as an ISO-8601 string in UTC, second
    precision (``YYYY-MM-DDTHH:MM:SSZ``).

    Related: ``fromdate``, ``strftime``, ``now``.
    """,
    examples=(
        'todate(0)                                # -> "1970-01-01T00:00:00Z"',
        "now | todate",
    ),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_todate(value: Any) -> str:
    n = _as_number(value, name="todate", arg=1)
    dt = _datetime.fromtimestamp(n, tz=_timezone.utc)
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")


@_register(
    "todateiso8601",
    summary="Alias of ``todate`` — Unix epoch → ISO-8601 UTC string.",
    signatures=("todateiso8601(value: number) -> string",),
    details="""
    Matches jq's ``todateiso8601`` (same as ``todate``).  Both names
    available for jq snippet portability.

    Related: ``todate``, ``fromdateiso8601``.
    """,
    examples=("todateiso8601(0)",),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_todateiso8601(value: Any) -> str:
    return _builtin_todate(value)


@_register(
    "fromdate",
    summary="ISO-8601 UTC string → Unix epoch seconds.",
    signatures=("fromdate(value: string) -> number",),
    details="""
    Matches jq's ``fromdate`` / ``fromdateiso8601``: parses an
    ISO-8601 UTC timestamp (``YYYY-MM-DDTHH:MM:SS[.fff]Z`` or with
    ``+00:00`` offset) and returns the corresponding Unix epoch
    seconds.

    Related: ``todate``, ``strptime``, ``now``.
    """,
    examples=(
        'fromdate("1970-01-01T00:00:00Z")          # -> 0',
        'fromdate("2025-01-01T00:00:00Z")',
    ),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_fromdate(value: Any) -> float:
    s = _as_str(value, name="fromdate", arg=1)
    try:
        # ``datetime.fromisoformat`` accepts ``Z`` from Python 3.11+;
        # normalise it for portability across the supported range.
        text = s.replace("Z", "+00:00") if s.endswith("Z") else s
        dt = _datetime.fromisoformat(text)
    except ValueError as exc:
        raise BuiltinError(f"fromdate: cannot parse {s!r}: {exc}") from exc
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=_timezone.utc)
    return dt.timestamp()


@_register(
    "fromdateiso8601",
    summary="Alias of ``fromdate``.",
    signatures=("fromdateiso8601(value: string) -> number",),
    details="""
    Matches jq's ``fromdateiso8601``.

    Related: ``fromdate``, ``todateiso8601``.
    """,
    examples=('fromdateiso8601("1970-01-01T00:00:00Z")',),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_fromdateiso8601(value: Any) -> float:
    return _builtin_fromdate(value)


@_register(
    "date",
    summary="Alias of ``todate`` for jq snippet compatibility.",
    signatures=("date(value: number) -> string",),
    details="""
    Matches jq's ``date`` (jq 1.7+): same as ``todate``.

    Related: ``todate``, ``fromdate``.
    """,
    examples=("date(0)",),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_date(value: Any) -> str:
    return _builtin_todate(value)


@_register(
    "gmtime",
    summary="Unix epoch seconds → broken-down UTC time array.",
    signatures=("gmtime(value: number) -> list",),
    details="""
    Matches jq's ``gmtime``: returns a length-8 list of integers in
    jq's order — ``[year - 1900, month, day, hour, minute, second,
    weekday, yearday]`` (months 0..11, weekday 0=Sunday, yearday
    0..365).

    Related: ``localtime``, ``mktime``, ``strftime``.
    """,
    examples=("gmtime(0)                                # -> [70, 0, 1, 0, 0, 0, 4, 0]",),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_gmtime(value: Any) -> list[Any]:
    return _broken_down_from_unix(_as_number(value, name="gmtime", arg=1), utc=True)


@_register(
    "localtime",
    summary="Unix epoch seconds → broken-down local time array.",
    signatures=("localtime(value: number) -> list",),
    details="""
    Matches jq's ``localtime``: same shape as ``gmtime`` but in the
    process's local timezone.

    Related: ``gmtime``, ``mktime``.
    """,
    examples=("localtime(0)",),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_localtime(value: Any) -> list[Any]:
    return _broken_down_from_unix(_as_number(value, name="localtime", arg=1), utc=False)


@_register(
    "mktime",
    summary="Broken-down UTC time array → Unix epoch seconds.",
    signatures=("mktime(value: list) -> number",),
    details="""
    Matches jq's ``mktime``: inverse of ``gmtime``.  Accepts the
    broken-down array ``[year - 1900, month, day, hour, minute,
    second, ...]`` and returns the corresponding Unix epoch seconds,
    interpreting the input as UTC.

    Related: ``gmtime``, ``fromdate``.
    """,
    examples=("mktime([70, 0, 1, 0, 0, 0])              # -> 0",),
    category="time",
    min_args=1,
    max_args=1,
)
def _builtin_mktime(value: Any) -> float:
    tm = _broken_down_to_struct(value, name="mktime")
    return float(_calendar.timegm(tm))


@_register(
    "strftime",
    summary="Format a Unix epoch-seconds value using ``strftime``.",
    signatures=("strftime(value: number, fmt: string) -> string",),
    details="""
    Matches jq's ``strftime``: formats a Unix epoch-seconds value
    (UTC) using a strftime-style format string.

    jq's two-arg form is ``strftime(fmt)`` applied to the broken-down
    time as input; this DSL flattens to the more common case
    ``strftime(unix_seconds, fmt)``.  Use the implicit-receiver form
    for the jq feel: ``now | strftime("%Y-%m-%d")``.

    Related: ``strptime``, ``todate``, ``now``.
    """,
    examples=(
        'strftime(0, "%Y-%m-%d")                   # -> "1970-01-01"',
        'now | strftime("%Y-%m-%d %H:%M:%S")',
    ),
    category="time",
    min_args=2,
    max_args=2,
)
def _builtin_strftime(value: Any, fmt: Any) -> str:
    n = _as_number(value, name="strftime", arg=1)
    f = _as_str(fmt, name="strftime", arg=2)
    return _time.strftime(f, _time.gmtime(n))


@_register(
    "strptime",
    summary="Parse a timestamp string into a broken-down UTC time array.",
    signatures=("strptime(value: string, fmt: string) -> list",),
    details="""
    Matches jq's ``strptime``: parses *value* using *fmt* and returns
    the broken-down array compatible with ``mktime`` /
    ``gmtime`` (jq's order).

    Related: ``strftime``, ``mktime``, ``fromdate``.
    """,
    examples=('strptime("2025-01-01", "%Y-%m-%d") | mktime',),
    category="time",
    min_args=2,
    max_args=2,
)
def _builtin_strptime(value: Any, fmt: Any) -> list[Any]:
    s = _as_str(value, name="strptime", arg=1)
    f = _as_str(fmt, name="strptime", arg=2)
    try:
        tm = _time.strptime(s, f)
    except ValueError as exc:
        raise BuiltinError(f"strptime: cannot parse {s!r} with {f!r}: {exc}") from exc
    return [
        tm.tm_year - 1900,
        tm.tm_mon - 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        tm.tm_wday + 1 if tm.tm_wday < 6 else 0,
        tm.tm_yday - 1,
    ]


@_register(
    "dateadd",
    summary="Add a number of seconds to a Unix epoch value.",
    signatures=("dateadd(value: number, seconds: number) -> number",),
    details="""
    Matches jq's ``dateadd`` (jq 1.7+): ``dateadd(t; s)`` is
    ``t + s``.  Provided for jq-snippet portability — plain
    arithmetic works too.

    Related: ``datesub``, ``now``.
    """,
    examples=('fromdate("2025-01-01T00:00:00Z") | dateadd(., 86400) | todate',),
    category="time",
    min_args=2,
    max_args=2,
)
def _builtin_dateadd(value: Any, seconds: Any) -> float:
    return _as_number(value, name="dateadd", arg=1) + _as_number(seconds, name="dateadd", arg=2)


@_register(
    "datesub",
    summary="Subtract a number of seconds from a Unix epoch value.",
    signatures=("datesub(value: number, seconds: number) -> number",),
    details="""
    Matches jq's ``datesub`` (jq 1.7+): ``datesub(t; s)`` is
    ``t - s``.

    Related: ``dateadd``, ``now``.
    """,
    examples=("now | datesub(., 3600) | todate           # one hour ago",),
    category="time",
    min_args=2,
    max_args=2,
)
def _builtin_datesub(value: Any, seconds: Any) -> float:
    return _as_number(value, name="datesub", arg=1) - _as_number(seconds, name="datesub", arg=2)


# ---------------------------------------------------------------------------
# Remaining jq surface — object / stream / debug / halt / env / misc math
# ---------------------------------------------------------------------------


@_register(
    "keys_unsorted",
    summary="Field names of an object in insertion order (jq's ``keys_unsorted``).",
    signatures=("keys_unsorted(value: object) -> list[string]",),
    details="""
    Matches jq's ``keys_unsorted``: like ``keys`` but does **not**
    sort the result.  Returns the field names in the order they were
    first seen on the object.  For lists / streams, returns indices
    ``0..n-1`` (matching jq).

    Use ``keys`` for the sorted form.

    Related: ``keys``, ``values``, ``to_entries``.
    """,
    examples=("{b: 1, a: 2} | keys_unsorted",),
    category="stream",
    min_args=1,
    max_args=1,
    stream_aware=True,
)
def _builtin_keys_unsorted(value: Any) -> list[Any]:
    if isinstance(value, ObjectRef):
        return list(value.fields)
    if isinstance(value, dict):
        return list(value)
    if isinstance(value, Iterable) and not isinstance(value, (str, bytes, bytearray, dict)):
        return list(range(len(list(value))))
    raise BuiltinError(
        f"keys_unsorted: argument 1 must be an object or list, got {_type_name(value)}"
    )


@_register(
    "map_values",
    summary="Apply *body* to every value of an object / array, keeping shape.",
    signatures=("map_values(body) -> any",),
    details="""
    **Special form.**  Matches jq's ``map_values``: equivalent to
    ``with_entries(.value |= body)`` for objects and ``map(body)``
    for arrays.  Preserves the input's shape — an object stays an
    object with the same keys, an array stays an array.

    Returning the ``select`` drop sentinel removes the value's slot.

    Related: ``map``, ``with_entries``, ``select``.
    """,
    examples=(
        "{a: 1, b: 2} | map_values(. * 10)        # -> {a: 10, b: 20}",
        "[1, 2, 3] | map_values(. * 10)            # -> [10, 20, 30]",
    ),
    category="stream",
    min_args=1,
    max_args=1,
    special_form=True,
)
def _builtin_map_values(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("map_values must be evaluated through the evaluator")


@_register(
    "pick",
    summary="Keep only the slots named by *path_exprs*; everything else is dropped.",
    signatures=("pick(...path_exprs) -> any",),
    details="""
    **Special form.**  Matches jq 1.7's ``pick``: enumerates one or
    more path expressions against the current value, then returns
    a value containing only those paths (with intermediate
    containers reconstructed).

    Each argument should be a static path projection (``.foo``,
    ``.bar.baz``, ``.list[0]``).  Multiple paths may be passed either
    as separate function arguments (``pick(.a, .c)``) or as a single
    comma-stream argument (``pick((.a, .c))``) — both work.  Missing
    slots are silently ignored.

    Related: ``getpath``, ``setpath``, ``del``, ``paths``.
    """,
    examples=(
        "{a: 1, b: 2, c: 3} | pick(.a, .c)         # -> {a: 1, c: 3}",
        "{a: {b: 1, c: 2}, d: 3} | pick(.a.b, .d)",
    ),
    category="value",
    min_args=1,
    max_args=None,
    special_form=True,
)
def _builtin_pick(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("pick must be evaluated through the evaluator")


@_register(
    "recurse_down",
    summary="Alias of ``recurse`` with no arguments — emit every reachable value.",
    signatures=("recurse_down() -> stream",),
    details="""
    Matches jq's ``recurse_down`` (deprecated in jq itself, kept for
    backward compatibility).  Same as ``recurse`` with no arguments.

    Related: ``recurse``, ``walk``.
    """,
    examples=("{a: 1, b: {c: 2}} | [recurse_down]",),
    category="value",
    min_args=0,
    max_args=0,
    special_form=True,
)
def _builtin_recurse_down(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("recurse_down must be evaluated through the evaluator")


@_register(
    "min_max",
    summary="Return ``[min, max]`` of a list, keyed by *body*.",
    signatures=(
        "min_max() -> list",
        "min_max(body) -> list",
    ),
    details="""
    **Special form.**  Matches jq 1.7's ``min_max``: returns a
    two-element list ``[minimum, maximum]``.  Without *body*, items
    compare under jq's cross-type ordering; with *body*, each item's
    key is the result of *body* applied to it (like ``min_by`` /
    ``max_by``).

    Empty input returns ``[null, null]`` (jq parity).

    Related: ``min``, ``max``, ``min_by``, ``max_by``, ``max_min``.
    """,
    examples=(
        "[5, 2, 8, 1] | min_max",
        "[.ltm.pool[]] | min_max(.members | length)",
    ),
    category="stream",
    min_args=0,
    max_args=1,
    special_form=True,
)
def _builtin_min_max(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("min_max must be evaluated through the evaluator")


@_register(
    "max_min",
    summary="Return ``[max, min]`` of a list, keyed by *body*.",
    signatures=(
        "max_min() -> list",
        "max_min(body) -> list",
    ),
    details="""
    **Special form.**  Matches jq 1.7's ``max_min``: like ``min_max``
    but in the opposite order — ``[maximum, minimum]``.

    Related: ``min_max``, ``min``, ``max``, ``min_by``, ``max_by``.
    """,
    examples=("[5, 2, 8, 1] | max_min",),
    category="stream",
    min_args=0,
    max_args=1,
    special_form=True,
)
def _builtin_max_min(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("max_min must be evaluated through the evaluator")


@_register(
    "halt",
    summary="Halt the query silently — no further output.",
    signatures=("halt() -> never",),
    details="""
    Matches jq's ``halt``: terminates query evaluation without
    emitting an error message.  In this DSL, ``halt`` raises a
    distinct ``BuiltinError`` flagged so the runner exits with status
    0 (versus ``halt_error`` which exits non-zero).

    Useful for "I've found what I wanted, stop early" pipelines.

    Related: ``halt_error``, ``error``, ``empty``.
    """,
    examples=('.ltm.virtual[] | select(.name == "web_vs") | halt',),
    category="stream",
    min_args=0,
    max_args=0,
)
def _builtin_halt() -> Any:
    raise BuiltinError("halt: query halted by halt()")


@_register(
    "halt_error",
    summary="Halt the query with an error message and exit code.",
    signatures=(
        "halt_error() -> never",
        "halt_error(exit_code: integer) -> never",
    ),
    details="""
    Matches jq's ``halt_error``: terminates evaluation and signals
    a non-zero exit.  With an optional integer argument, jq sets
    that exit code; this DSL preserves the code on the error so the
    CLI can map it to a process exit.

    Related: ``halt``, ``error``.
    """,
    examples=(".ltm.virtual[] | select(.pool == null) | halt_error(5)",),
    category="stream",
    min_args=0,
    max_args=1,
)
def _builtin_halt_error(*args: Any) -> Any:
    exit_code = 5
    if args:
        exit_code = _as_int(args[0], name="halt_error", arg=1)
    raise BuiltinError(f"halt_error: query halted (exit_code={exit_code})")


@_register(
    "debug",
    summary="Pass-through that logs the current value to stderr.",
    signatures=(
        "debug() -> any",
        "debug(label: string) -> any",
    ),
    details="""
    **Special form.**  Matches jq's ``debug``: returns the current
    value unchanged but writes a debug line to stderr
    (``["DEBUG:", value]`` in jq's one-arg form, ``["DEBUG:", label,
    value]`` with a label).  Useful for tracing complex pipelines
    without adding extra pipeline stages.

    Related: ``stderr``, ``error``.
    """,
    examples=(
        ".ltm.virtual[] | debug | .name",
        '.ltm.virtual[] | debug("vs") | .name',
    ),
    category="stream",
    min_args=0,
    max_args=1,
    special_form=True,
)
def _builtin_debug(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("debug must be evaluated through the evaluator")


@_register(
    "stderr",
    summary="Pass-through that writes the current value to stderr as JSON.",
    signatures=("stderr() -> any",),
    details="""
    **Special form.**  Matches jq's ``stderr``: returns the current
    value unchanged but writes its JSON encoding to stderr.  Same
    chokepoint as ``debug`` without the ``["DEBUG:", ...]`` wrapping.

    Related: ``debug``, ``error``.
    """,
    examples=(".ltm.virtual[] | stderr | .name",),
    category="stream",
    min_args=0,
    max_args=0,
    special_form=True,
)
def _builtin_stderr(*_args):  # pragma: no cover - dispatched specially
    raise RuntimeError("stderr must be evaluated through the evaluator")


@_register(
    "env",
    summary="The process environment as a dict — jq's ``env``.",
    signatures=("env() -> object",),
    details="""
    Matches jq's ``env`` builtin: returns the process environment
    variables as an object.  Read-only — modifications don't
    propagate back.

    Related: ``$ENV`` (jq's variable form, not exposed here).
    """,
    examples=(
        "env | .HOME",
        'env | with_entries(select(.key | startswith("F5_")))',
    ),
    category="value",
    min_args=0,
    max_args=0,
)
def _builtin_env() -> dict[str, str]:
    import os as _os

    return dict(_os.environ)


# ---------------------------------------------------------------------------
# Additional math (expm1, log1p, hypot, fma*, pow10, nearbyint, remainder,
# Bessel-arbitrary-order jn/yn, lgamma_r)
# ---------------------------------------------------------------------------


@_register(
    "expm1",
    summary="``exp(value) - 1`` with high precision near zero.",
    signatures=("expm1(value: number) -> number",),
    details="""
    Matches jq's ``expm1``: thin wrapper over Python's ``math.expm1``.
    Avoids the loss of precision that ``exp(x) - 1`` suffers when
    ``x`` is small.

    Related: ``exp``, ``log1p``.
    """,
    examples=("expm1(1e-10)                             # -> 1e-10 (high precision)",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_expm1(value: Any) -> float:
    return math.expm1(_as_number(value, name="expm1", arg=1))


@_register(
    "log1p",
    summary="``log(1 + value)`` with high precision near zero.",
    signatures=("log1p(value: number) -> number",),
    details="""
    Matches jq's ``log1p``: thin wrapper over Python's ``math.log1p``.
    Avoids the loss of precision that ``log(1 + x)`` suffers when
    ``x`` is small.

    Related: ``log``, ``expm1``.
    """,
    examples=("log1p(1e-10)                             # -> 1e-10 (high precision)",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_log1p(value: Any) -> float:
    return math.log1p(_as_number(value, name="log1p", arg=1))


@_register(
    "pow10",
    summary="Alias of ``exp10`` — ``10 ** value``.",
    signatures=("pow10(value: number) -> number",),
    details="""
    Matches jq's ``pow10``: ``10**value``.  Identical to ``exp10``.

    Related: ``exp10``, ``pow``, ``log10``.
    """,
    examples=("pow10(3)                                 # -> 1000.0",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_pow10(value: Any) -> float:
    return math.pow(10.0, _as_number(value, name="pow10", arg=1))


@_register(
    "nearbyint",
    summary="Round to nearest integer using current rounding mode (banker's).",
    signatures=("nearbyint(value: number) -> integer",),
    details="""
    Matches jq's ``nearbyint``: same result as ``rint`` (Python's
    ``round`` uses banker's rounding).  C distinguishes them by
    whether they raise the inexact flag; Python doesn't expose
    that, so the two are aliases here.

    Related: ``rint``, ``round``, ``trunc``.
    """,
    examples=("nearbyint(2.5)                           # -> 2",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_nearbyint(value: Any) -> int:
    return int(round(_as_number(value, name="nearbyint", arg=1)))


@_register(
    "hypot",
    summary="``sqrt(x*x + y*y)`` without intermediate overflow.",
    signatures=("hypot(x: number, y: number) -> number",),
    details="""
    Matches jq's ``hypot``: the Euclidean distance.  Uses Python's
    ``math.hypot``, which avoids overflow for large operands.

    Related: ``sqrt``, ``pow``.
    """,
    examples=("hypot(3, 4)                              # -> 5.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_hypot(x: Any, y: Any) -> float:
    return math.hypot(
        _as_number(x, name="hypot", arg=1),
        _as_number(y, name="hypot", arg=2),
    )


@_register(
    "fma",
    summary="Fused multiply-add — ``x * y + z`` in one rounding.",
    signatures=("fma(x: number, y: number, z: number) -> number",),
    details="""
    Matches jq's ``fma``: returns ``x*y + z``.  Python doesn't expose
    hardware FMA in older versions; this implementation computes the
    naive expression, which agrees with FMA to within one ULP.

    Related: ``pow``, ``hypot``.
    """,
    examples=("fma(2, 3, 1)                             # -> 7.0",),
    category="math",
    min_args=3,
    max_args=3,
)
def _builtin_fma(x: Any, y: Any, z: Any) -> float:
    nx = _as_number(x, name="fma", arg=1)
    ny = _as_number(y, name="fma", arg=2)
    nz = _as_number(z, name="fma", arg=3)
    return float(nx) * float(ny) + float(nz)


@_register(
    "fmax",
    summary="Larger of two numbers — ``max(x, y)`` (jq parity).",
    signatures=("fmax(x: number, y: number) -> number",),
    details="""
    Matches jq's ``fmax``: two-argument numeric maximum.  Unlike
    ``max`` (which is stream-aware), ``fmax`` takes exactly two
    scalar numbers.

    Related: ``max``, ``fmin``, ``fdim``.
    """,
    examples=("fmax(3, 7)                               # -> 7",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_fmax(x: Any, y: Any) -> float:
    return max(
        _as_number(x, name="fmax", arg=1),
        _as_number(y, name="fmax", arg=2),
    )


@_register(
    "fmin",
    summary="Smaller of two numbers — ``min(x, y)`` (jq parity).",
    signatures=("fmin(x: number, y: number) -> number",),
    details="""
    Matches jq's ``fmin``: two-argument numeric minimum.

    Related: ``min``, ``fmax``.
    """,
    examples=("fmin(3, 7)                               # -> 3",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_fmin(x: Any, y: Any) -> float:
    return min(
        _as_number(x, name="fmin", arg=1),
        _as_number(y, name="fmin", arg=2),
    )


@_register(
    "fmod",
    summary="Floating-point modulo — ``x - y * trunc(x / y)``.",
    signatures=("fmod(x: number, y: number) -> number",),
    details="""
    Matches jq's ``fmod`` (C's ``fmod``): the floating-point remainder
    truncated toward zero.  Result has the sign of *x*.

    Related: ``remainder``, ``drem``, ``trunc``.
    """,
    examples=("fmod(7, 3)                               # -> 1.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_fmod(x: Any, y: Any) -> float:
    return math.fmod(
        _as_number(x, name="fmod", arg=1),
        _as_number(y, name="fmod", arg=2),
    )


@_register(
    "remainder",
    summary="IEEE-754 remainder of ``x / y`` (rounded to nearest).",
    signatures=("remainder(x: number, y: number) -> number",),
    details="""
    Matches jq's ``remainder`` / ``drem``: the IEEE-754 remainder,
    which rounds the quotient to the nearest integer (ties to even)
    rather than truncating.  Result is in ``[-y/2, y/2]``.

    Related: ``fmod``, ``drem``.
    """,
    examples=("remainder(7, 3)                          # -> 1.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_remainder(x: Any, y: Any) -> float:
    return math.remainder(
        _as_number(x, name="remainder", arg=1),
        _as_number(y, name="remainder", arg=2),
    )


@_register(
    "drem",
    summary="Alias of ``remainder`` (legacy BSD name).",
    signatures=("drem(x: number, y: number) -> number",),
    details="""
    Matches jq's ``drem``: alias of ``remainder``.

    Related: ``remainder``, ``fmod``.
    """,
    examples=("drem(7, 3)                               # -> 1.0",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_drem(x: Any, y: Any) -> float:
    return _builtin_remainder(x, y)


@_register(
    "lgamma_r",
    summary="lgamma with the sign of gamma — returns ``[lgamma, sign]``.",
    signatures=("lgamma_r(value: number) -> list",),
    details="""
    Matches jq's ``lgamma_r``: returns a 2-element list
    ``[log(|gamma(x)|), sign]`` where *sign* is +1 or -1.

    Related: ``lgamma``, ``gamma``.
    """,
    examples=("lgamma_r(5)                              # -> [log(24), 1]",),
    category="math",
    min_args=1,
    max_args=1,
)
def _builtin_lgamma_r(value: Any) -> list[Any]:
    n = _as_number(value, name="lgamma_r", arg=1)
    lg = math.lgamma(n)
    # Sign of gamma(n): negative for n in (-(2k+1), -2k) for k >= 0.
    try:
        gv = math.gamma(n)
        sign = 1 if gv >= 0 else -1
    except ValueError:
        sign = 1
    return [lg, sign]


@_register(
    "jn",
    summary="Bessel J_n(x) by upward recurrence — accurate for small ``n``.",
    signatures=("jn(n: integer, x: number) -> number",),
    details="""
    Matches jq's ``jn``: ``J_n(x)`` for integer order *n*.  Uses the
    standard upward recurrence ``J_{n+1}(x) = (2n/x) J_n(x) -
    J_{n-1}(x)`` seeded by ``j0`` / ``j1``.  Stable for ``n <= |x|``;
    callers asking for ``n`` far above ``|x|`` may see precision loss
    — for research-grade results use SciPy.

    Related: ``j0``, ``j1``, ``yn``.
    """,
    examples=("jn(2, 5)",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_jn(n: Any, x: Any) -> float:
    order = _as_int(n, name="jn", arg=1)
    val = _as_number(x, name="jn", arg=2)
    if order == 0:
        return _bessel_j0(float(val))
    if order == 1:
        return _bessel_j1(float(val))
    if order < 0:
        sign = -1 if (order & 1) else 1
        return sign * _builtin_jn(-order, val)
    if val == 0:
        return 0.0
    jm1 = _bessel_j0(float(val))
    j = _bessel_j1(float(val))
    for k in range(1, order):
        jp1 = (2.0 * k / float(val)) * j - jm1
        jm1 = j
        j = jp1
    return j


@_register(
    "yn",
    summary="Bessel Y_n(x) by upward recurrence (``x > 0``).",
    signatures=("yn(n: integer, x: number) -> number",),
    details="""
    Matches jq's ``yn``: ``Y_n(x)`` for integer order *n*, defined
    for positive *x*.  Upward recurrence seeded by ``y0`` / ``y1``;
    upward recurrence is stable for Y_n at all orders.

    Related: ``y0``, ``y1``, ``jn``.
    """,
    examples=("yn(2, 5)",),
    category="math",
    min_args=2,
    max_args=2,
)
def _builtin_yn(n: Any, x: Any) -> float:
    order = _as_int(n, name="yn", arg=1)
    val = _as_number(x, name="yn", arg=2)
    if val <= 0:
        raise BuiltinError(f"yn: argument must be positive, got {val}")
    if order == 0:
        return _bessel_y0(float(val))
    if order == 1:
        return _bessel_y1(float(val))
    if order < 0:
        sign = -1 if (order & 1) else 1
        return sign * _builtin_yn(-order, val)
    ym1 = _bessel_y0(float(val))
    y = _bessel_y1(float(val))
    for k in range(1, order):
        yp1 = (2.0 * k / float(val)) * y - ym1
        ym1 = y
        y = yp1
    return y


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
