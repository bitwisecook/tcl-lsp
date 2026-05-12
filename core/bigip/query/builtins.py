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
            special_form=special_form,
            with_ctx=with_ctx,
            min_args=min_args,
            max_args=max_args,
        )
        return fn

    return decorator


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

    When *name* is given, render only that builtin's entry (so users can
    drill down with ``--help-builtins ip``).  Otherwise render every
    builtin grouped by category.
    """
    if name is not None:
        spec = lookup(name)
        if spec is None:
            return f"no such builtin: {name}\n"
        return _render_one(spec) + "\n"

    out: list[str] = ["BUILTIN FUNCTIONS", ""]
    last_cat: str | None = None
    for spec in list_builtins():
        if spec.category != last_cat:
            out.append(f"  [{spec.category}]")
            last_cat = spec.category
        out.append(_render_one(spec, indent="    "))
    return "\n".join(out) + "\n"


def _render_one(spec: BuiltinSpec, indent: str = "") -> str:
    lines = [f"{indent}{spec.name}"]
    for sig in spec.signatures:
        lines.append(f"{indent}    {sig}")
    lines.append(f"{indent}    {spec.summary}")
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


def _as_sequence(value: object, *, name: str, arg: int) -> list[Any]:
    if isinstance(value, Stream):
        return list(value.items)
    if isinstance(value, list):
        return list(value)
    if isinstance(value, tuple):
        return list(value)
    raise BuiltinError(f"{name}: argument {arg} must be a list or stream, got {_type_name(value)}")


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

    ``partition_prefix`` includes the surrounding slashes (``"/Common/"``)
    or is empty.  Each of the other parts is the bare value (no ``%``,
    no ``:``) or empty when absent.
    """
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
    examples=(
        'ip("10.0.0.1")',
        'ip("192.168.9.0/24", .destination)   # rebase keeping host bits',
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
    examples=(
        'net("192.168.9.0/24")',
        'net("192.168.9.42/24")   # -> "192.168.9.0/24"',
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
        "any partition prefix and ``:port`` suffix."
    ),
    signatures=("host(value: string) -> string",),
    examples=(
        "host(.destination)",
        'host("/Common/192.168.1.1:80")   # -> "192.168.1.1"',
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
    examples=("port(.destination)", 'port("192.168.1.1:80")   # -> 80'),
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
    examples=('partition("/Common/web_pool")   # -> "Common"',),
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
    examples=('basename("/Common/web_pool")   # -> "web_pool"',),
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
    examples=('with_partition("/Common/web_pool", "Tenant_A")',),
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
    examples=(
        'in_cidr("10.0.0.5", "10.0.0.0/8")',
        'select(.destination | in_cidr("10.0.0.0/8"))',
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
    examples=(
        "route_domain(.destination)",
        'route_domain("10.0.0.1%5:80")   # -> "5"',
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
    examples=(
        "with_route_domain(.destination, 5)",
        'with_route_domain("/Common/10.0.0.1%5:80", "")   # strip rd',
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
# Rename — single-object and whole-partition cascade
# ---------------------------------------------------------------------------


@_register(
    "rename",
    summary=(
        "Rename a BIG-IP object full-path and update every reference to it.  "
        "Routes through the same engine ``f5 rename`` uses (token-bounded "
        "regex substitution across the whole source, covering iRule body "
        "references and pool-member identifiers).  Unlike ``.<kind>[old].name "
        "= new``, the kind is not specified — useful when ``old`` is "
        "user-supplied and the caller doesn't know which kind owns it.  A "
        "zero-occurrence outcome returns 0 rather than raising, so the CLI "
        "verb can surface ``warning: no occurrences of <old> found`` with "
        "exit code 1 instead of treating it as an error."
    ),
    signatures=("rename(old: string, new: string) -> integer",),
    examples=(
        'rename("/Common/old_pool", "/Common/new_pool")',
        'rename("/Common/log_rule", "/Common/audit_rule")',
    ),
    category="rename",
    min_args=2,
    max_args=2,
    with_ctx=True,
)
def _builtin_rename(old: Any, new: Any, *, ctx: Any) -> int:
    from .edit_plan import EditOp

    old_s = _as_str(old, name="rename", arg=1).strip()
    new_s = _as_str(new, name="rename", arg=2).strip()
    if not old_s:
        raise BuiltinError("rename: old name must not be empty")
    if not new_s:
        raise BuiltinError("rename: new name must not be empty")
    if old_s == new_s:
        return 0
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
        "of the ``/<old>/`` prefix across the whole source.  The match "
        "is token-bounded — neighbouring identifiers (``/<old>Ext/...``) "
        "are not touched — and applies to object headers, references in "
        "config properties, destination address prefixes, pool-member "
        "identifiers, and iRule body literals.  The bare "
        "``auth partition <old>`` stanza header is renamed too.  Pair "
        "with --in-place to persist or with the default dry-run diff to "
        "preview.  Returns the textual-match count."
    ),
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
        )
    )
    ctx.edits.add_prefix(
        PrefixRewrite(
            source_uri=ctx.root.uri,
            label=f"auth partition {old_name}",
            pattern=header_pattern,
            replacement=rf"\g<1>{new_name}",
        )
    )

    # Return a useful count for the user: the textual matches the
    # prefix rewrite will land on.  The CLI also surfaces this via the
    # stderr summary the planner emits after applying.
    return len(prefix_pattern.findall(ctx.root.source))


# ---------------------------------------------------------------------------
# String helpers
# ---------------------------------------------------------------------------


@_register(
    "length",
    summary="Length of a string, list, stream, or object's field map.",
    signatures=("length(value: any) -> integer",),
    examples=("length(.rules)", ".rules | length"),
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
    "startswith",
    summary="Test whether a string starts with a prefix.",
    signatures=("startswith(value: string, prefix: string) -> boolean",),
    examples=('startswith(.name, "vs_prod_")',),
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
    examples=('endswith(.name, "_pool")',),
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
    examples=('contains(.destination, ":443")', 'contains(.rules, "/Common/log")'),
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
    examples=('match(.name, "^vs_prod_.*")',),
    category="string",
    min_args=2,
    max_args=2,
)
def _builtin_match(value: Any, pattern: Any) -> bool:
    s = _as_str(value, name="match", arg=1)
    p = _as_str(pattern, name="match", arg=2)
    try:
        return re.search(p, s) is not None
    except re.error as exc:
        raise BuiltinError(f"match: invalid pattern {p!r}: {exc}") from exc


@_register(
    "sub",
    summary="Replace the first regex match in a string.",
    signatures=("sub(value: string, pattern: string, replacement: string) -> string",),
    examples=('sub(.name, "^vs_dev_", "vs_qa_")',),
    category="string",
    min_args=3,
    max_args=3,
)
def _builtin_sub(value: Any, pattern: Any, repl: Any) -> str:
    s = _as_str(value, name="sub", arg=1)
    p = _as_str(pattern, name="sub", arg=2)
    r = _as_str(repl, name="sub", arg=3)
    try:
        return re.sub(p, r, s, count=1)
    except re.error as exc:
        raise BuiltinError(f"sub: invalid pattern {p!r}: {exc}") from exc


@_register(
    "gsub",
    summary="Replace every regex match in a string.",
    signatures=("gsub(value: string, pattern: string, replacement: string) -> string",),
    examples=('gsub(.body, "/Common/old_", "/Common/new_")',),
    category="string",
    min_args=3,
    max_args=3,
)
def _builtin_gsub(value: Any, pattern: Any, repl: Any) -> str:
    s = _as_str(value, name="gsub", arg=1)
    p = _as_str(pattern, name="gsub", arg=2)
    r = _as_str(repl, name="gsub", arg=3)
    try:
        return re.sub(p, r, s)
    except re.error as exc:
        raise BuiltinError(f"gsub: invalid pattern {p!r}: {exc}") from exc


@_register(
    "split",
    summary="Split a string on a separator.  Returns a list.",
    signatures=("split(value: string, separator: string) -> list[string]",),
    examples=('split(.destination, ":")',),
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
    examples=('join(.rules, ", ")',),
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
    examples=("upcase(.name)",),
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
    examples=("downcase(.name)",),
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
    examples=("keys(.ltm.virtual)",),
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
    examples=("values(.ltm.virtual.web_vs)",),
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
    examples=("first(.rules)",),
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
    examples=("last(.rules)",),
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
    examples=(".ltm.virtual[] | count",),
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
    examples=(".ltm.virtual[].pool | unique",),
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
    examples=(".ltm.virtual[].name | sort",),
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
    examples=('any(.rules | map(. == "/Common/log"))',),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_any(value: Any) -> bool:
    return any(_truthy(v) for v in _as_sequence(value, name="any", arg=1))


@_register(
    "all",
    summary="True when every item of a list or stream is truthy.",
    signatures=("all(value: list | stream) -> boolean",),
    examples=('all(.rules | map(startswith(., "/Common/")))',),
    category="stream",
    min_args=1,
    max_args=1,
)
def _builtin_all(value: Any) -> bool:
    return all(_truthy(v) for v in _as_sequence(value, name="all", arg=1))


# ``select`` and ``map`` are special forms — they need to evaluate their
# argument once per input value, with ``.`` re-bound to that value.  The
# evaluator handles the actual binding loop; we just declare the spec
# here so the dispatch table and the help text stay symmetric.


@_register(
    "select",
    summary="Drop the current value unless the body evaluates to a truthy result.",
    signatures=("select(body) -> any | drop",),
    examples=(
        '.ltm.virtual[] | select(.pool != "")',
        '.ltm.virtual[] | select(startswith(.name, "vs_prod_"))',
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
    examples=(".rules | map(basename(.))",),
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
    examples=("kind(.ltm.virtual.web_vs)",),
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
    examples=("path(.ltm.virtual.web_vs)",),
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
    examples=("select(defined(.pool))",),
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
    examples=("type(.pool)",),
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
    examples=("refs(.ltm.virtual.web_vs)",),
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
    examples=("referenced_by(.ltm.pool.web_pool)",),
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
