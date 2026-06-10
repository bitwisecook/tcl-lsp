"""Variable canonicalisation / resolution — syntactic ref → canonical Place.

The variable analogue of :func:`compiler.lowering._Lowerer._canonicalise_command`:
given a variable reference as written (``x``, ``a(k)``, ``$x``, ``${tok}(k)``,
``$state($whom)``) plus the per-frame scope context (current namespace, the
``global`` / ``variable`` / ``upvar`` declarations in effect, active traces),
produce a canonical :class:`compiler.place.Place`.

Reuses the existing resolution substrate — ``split_array_name`` /
``normalise_qualified_name`` (`shared/naming.py`) and the variable-reference
scanner (`compiler/var_refs.py`) — rather than re-deriving it.  Anything that
cannot be pinned down statically (dynamic variable name, computed array name,
``upvar`` to a dynamic target) resolves to ``UNKNOWN`` / ``dynamic`` so the
overlap relation stays sound (over-approximating).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from shared.naming import normalise_qualified_name, split_array_name

from . import place as _p
from .place import Index, Place
from .var_refs import VarReferenceScanner, VarScanOptions

# Scanner used to recover the variable reads inside a *dynamic* index / name
# (``$i`` in ``a($i)``, ``X`` in ``set $X``).  Read-roles + cmd-sub recursion so
# ``a([idx])`` and ``a($ns::i)`` are covered.
_INNER_SCANNER = VarReferenceScanner(
    VarScanOptions(include_var_read_roles=True, recurse_cmd_substitutions=True)
)


@dataclass(frozen=True, slots=True)
class ResolveContext:
    """Per-frame scope context for resolving a variable reference.

    Built once per proc/method/namespace scope and reused (mirrors how the
    lowerer reuses its command-canonicalisation maps).
    """

    namespace: str = "::"  # current FQ namespace
    globals: frozenset[str] = frozenset()  # names under a `global` declaration
    ns_vars: frozenset[str] = frozenset()  # names under a `variable` declaration
    upvar_aliases: dict[str, str] = field(
        default_factory=dict
    )  # local alias -> caller target ("" = dynamic)
    traced: frozenset[str] = frozenset()  # names with an active `trace`
    instance_vars: frozenset[str] = frozenset()  # TclOO instance vars in scope
    instance_owner: str = ""  # owning class/object FQ name for instance_vars


_EMPTY_CTX = ResolveContext()


def _has_subst(text: str) -> bool:
    return "$" in text or "[" in text


def _inner_read_places(text: str, ctx: ResolveContext) -> tuple[Place, ...]:
    """Resolve the variables read inside a dynamic index / name *text*."""
    names = _INNER_SCANNER.scan_word(text)
    return tuple(sorted((_bind_scalar(n, ctx) for n in names), key=lambda pl: (pl.ns, pl.name)))


def _classify_index(elem: str, ctx: ResolveContext) -> Index:
    """Turn an array index / dict key text into an :class:`Index`."""
    if _has_subst(elem):
        return _p.dynamic_index(_inner_read_places(elem, ctx))
    return _p.literal_index(elem)


def _bind_scalar(base: str, ctx: ResolveContext, *, observed: bool | None = None) -> Place:
    """Bind a (sigil-stripped, scalar) base name to a canonical Place using the
    scope declarations in *ctx*."""
    obs = (base in ctx.traced) if observed is None else observed
    if base in ctx.upvar_aliases:
        target = ctx.upvar_aliases[base]
        return _p.upvar_alias(base, target, dynamic=not target)
    if base in ctx.instance_vars:
        return _p.instance_var(base, ctx.instance_owner, observed=obs)
    if base in ctx.globals:
        return _p.scalar(base, ns="::", observed=obs)
    if base in ctx.ns_vars:
        return _p.scalar(base, ns=ctx.namespace, observed=obs)
    if base.startswith("::"):
        fq = normalise_qualified_name(base)
        ns, _, tail = fq.rpartition("::")
        return _p.scalar(tail, ns=(ns or "::"), observed=obs)
    return _p.scalar(base, ns=_p.LOCAL_NS, observed=obs)


def resolve_place(
    ref: str, ctx: ResolveContext = _EMPTY_CTX, *, whole_array: bool = False
) -> Place:
    """Resolve a variable reference *ref* (as written) to a canonical Place.

    *whole_array* marks a reference the command touches as an entire array
    (``array get a`` / ``array set a`` / a bare ``$a`` passed to such), so it
    becomes ``ARRAY_WHOLE`` rather than a scalar.
    """
    if not ref:
        return _p.UNKNOWN

    # A target whose *name* is value-substituted — ``set $X v`` (IR name
    # ``${X}``), ``set ${tok}(k) v`` — denotes the variable named by the
    # substitution's value, i.e. a place that cannot be pinned down.  But it
    # *reads* the name variable(s) (``X`` / ``tok``): the genuine read the
    # dead-store/unused checks need.  (Read references arrive de-sigilled from
    # the scanner — ``state($whom)`` not ``$state($whom)`` — so a leading ``$``
    # here always marks a write-target dynamic name.)
    if ref.startswith("$"):
        return _p.unknown(name_reads=_inner_read_places(ref, ctx))

    base, elem = split_array_name(ref)

    if not base or _has_subst(base):
        return _p.unknown(name_reads=_inner_read_places(ref, ctx))

    if elem is not None:
        index = _classify_index(elem, ctx)
        bound = _bind_scalar(base, ctx)
        # Carry namespace/observed from the bound scalar onto the element.
        return Place(
            _p.PlaceKind.ARRAY_ELEM,
            ns=bound.ns,
            name=bound.name,
            index=index,
            owner=bound.owner,
            observed=bound.observed,
            dynamic=bound.dynamic,
        )

    if whole_array:
        bound = _bind_scalar(base, ctx)
        return Place(
            _p.PlaceKind.ARRAY_WHOLE,
            ns=bound.ns,
            name=bound.name,
            owner=bound.owner,
            observed=bound.observed,
            dynamic=bound.dynamic,
        )

    return _bind_scalar(base, ctx)


def resolve_dict_path(root_ref: str, keys: list[str], ctx: ResolveContext = _EMPTY_CTX) -> Place:
    """Resolve a ``dict get/set $d k1 k2 …`` access to a ``DICT_PATH`` Place."""
    base, _ = split_array_name(root_ref)
    if not base or _has_subst(base):
        return _p.unknown(name_reads=_inner_read_places(root_ref, ctx))
    bound = _bind_scalar(base, ctx)
    if bound.kind is not _p.PlaceKind.SCALAR:
        # global/ns/upvar dict root — keep its binding but mark a dict path.
        ns, name = bound.ns, bound.name
    else:
        ns, name = bound.ns, bound.name
    return _p.dict_path(name, tuple(_classify_index(k, ctx) for k in keys), ns=ns)


__all__ = ["ResolveContext", "resolve_place", "resolve_dict_path"]
