"""Walk the query AST against a :class:`BigipConfig`.

The evaluator is a small set of recursive functions that translate
each AST node into a Python value.  Streams are flattened lazily: an
expression that produces a :class:`.values.Stream` continues to flow
through pipes one value at a time.

Assignments are not applied during evaluation.  Each :class:`Assignment`
node emits an :class:`.edit_plan.EditOp` (or a sequence of them, when
the LHS resolves to multiple targets) into the supplied
:class:`EvalContext`.  The runner applies the collected ops once
evaluation has finished — that keeps an edit's view of the world
stable across the whole query.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass, field, fields, is_dataclass
from typing import Any

from . import builtins as _builtins
from .ast import (
    Assignment,
    BinOp,
    Call,
    Expr,
    Field,
    Identity,
    LetBinding,
    ListLiteral,
    Literal,
    ObjectLiteral,
    PathExpr,
    Pipe,
    Program,
    Subscript,
    UnaryOp,
    Variable,
)
from .builtins import _truthy
from .edit_plan import EditOp, EditPlan
from .errors import EvalError
from .projection import Container, root_container
from .values import ObjectRef, PathRef, QueryFieldProvider, Root, Stream


@dataclass
class EvalContext:
    root: Root
    edits: EditPlan = field(default_factory=EditPlan)
    # When the verb was invoked with more than one config, *named_roots*
    # maps each source's user-facing name (auto-derived from the
    # filename stem, or set explicitly via ``--name N=PATH``) to its
    # :class:`Root`.  The ``$name`` DSL form consults this table; when
    # the table is empty (single-input run) ``$name`` raises.
    named_roots: dict[str, Root] = field(default_factory=dict)
    # ``--merge`` was passed: graph builtins (``refs`` /
    # ``referenced_by``) should walk references across every loaded
    # source, not just the one the object came from.
    merge_mode: bool = False
    # Lexical bindings introduced by ``expr as $name | body`` —
    # ``$name`` lookup checks here first and falls back to
    # ``named_roots``.  The let-binding evaluator pushes a new frame
    # (a shallow-copied dict) per iteration so concurrent stream
    # evaluations stay isolated; the previous frame is restored when
    # the body finishes.
    bindings: dict[str, Any] = field(default_factory=dict)

    def resolve_pathref(self, ref: PathRef) -> ObjectRef | None:
        """Resolve a query PathRef without exposing evaluator internals."""
        return _resolve_pathref(ref, self)


# ---------------------------------------------------------------------------
# Top-level entry point
# ---------------------------------------------------------------------------


def evaluate_statement(stmt: Expr, ctx: EvalContext) -> list[Any]:
    """Evaluate a single statement against ``ctx.root``.

    Returns the flattened list of values produced.  Side-effects (edits)
    are recorded on ``ctx.edits`` for the caller to apply.
    """
    root_input = root_container(ctx.root)
    return _flatten(_eval(stmt, root_input, ctx))


def evaluate(program: Program, ctx: EvalContext) -> list[Any]:
    """Evaluate every statement against the root, returning the last result.

    This entry point is convenient for read-only queries.  Multi-
    statement *mutating* queries should drive ``evaluate_statement``
    one statement at a time and apply each statement's edits before
    moving to the next — the runner does exactly that so a
    ``rename_partition(...) ; .ltm.virtual[].destination = ...``
    chain works the way users expect.
    """
    if not program.statements:
        return []
    last_values: list[Any] = []
    for index, stmt in enumerate(program.statements):
        results = evaluate_statement(stmt, ctx)
        if index == len(program.statements) - 1:
            last_values = results
    return last_values


# ---------------------------------------------------------------------------
# Core dispatch
# ---------------------------------------------------------------------------


def _eval(node: Expr, current: Any, ctx: EvalContext) -> Any:
    if isinstance(node, Literal):
        return node.value
    if isinstance(node, Identity):
        return current
    if isinstance(node, Variable):
        return _eval_variable(node, ctx)
    if isinstance(node, ListLiteral):
        if node.inner is None:
            return []
        return _flatten(_eval(node.inner, current, ctx))
    if isinstance(node, ObjectLiteral):
        return _eval_object_literal(node, current, ctx)
    if isinstance(node, PathExpr):
        return _eval_path(node, current, ctx)
    if isinstance(node, Pipe):
        lhs = _eval(node.lhs, current, ctx)
        return _pipe_through(lhs, node.rhs, ctx)
    if isinstance(node, LetBinding):
        return _eval_let_binding(node, current, ctx)
    if isinstance(node, Call):
        return _eval_call(node, current, ctx)
    if isinstance(node, BinOp):
        return _eval_binop(node, current, ctx)
    if isinstance(node, UnaryOp):
        return _eval_unop(node, current, ctx)
    if isinstance(node, Assignment):
        return _eval_assignment(node, current, ctx)
    raise EvalError(f"unsupported AST node: {type(node).__name__}")


def _pipe_through(values: Any, rhs: Expr, ctx: EvalContext) -> Any:
    """Apply *rhs* to each value flowing out of the LHS.

    Only :class:`Stream` values iterate through the pipe one item at
    a time — plain Python lists are passed to *rhs* as a single value.
    This matches jq's "arrays don't iterate, generators do" rule and
    lets ``.rules | length`` measure the rules list rather than
    running ``length`` on each PathRef.  Use ``.[]`` to iterate a list
    explicitly, or wrap a streaming pipeline in ``[ ... ]`` to collect
    its items back into a single list.

    The LHS may also be the :data:`_DROP` sentinel returned by
    ``select`` when its predicate fails on a non-stream value.  Treat
    that as an empty stream — same semantics as jq's "empty output
    short-circuits the rest of the pipe" — so ``select(false) | .x``
    silently produces nothing instead of erroring with "cannot read
    field".  Without this, the sentinel would leak into the RHS and
    blow up the next stage; the ``Stream`` branch already handles the
    sentinel transparently because ``_flatten`` drops it.
    """
    if isinstance(values, _Drop):
        return Stream(items=[])
    if isinstance(values, Stream):
        out: list[Any] = []
        for item in values.items:
            if isinstance(item, _Drop):
                continue
            out.extend(_flatten(_eval(rhs, item, ctx)))
        return Stream(items=out)
    return _eval(rhs, values, ctx)


# ---------------------------------------------------------------------------
# Object literals
# ---------------------------------------------------------------------------


def _eval_object_literal(node: ObjectLiteral, current: Any, ctx: EvalContext) -> Any:
    """Evaluate ``{key: expr, ...}`` against the current input.

    Each value expression is evaluated with ``.`` bound to *current*
    (same shape as :func:`select` / :func:`map` bodies).  When any
    value evaluates to a :class:`Stream`, the object literal
    broadcasts element-wise: a stream of dicts comes back, one per
    item of the longest stream value (every other stream value must
    share that length; scalars replicate).  This matches the
    behaviour of the call dispatcher and the arithmetic operators, so
    ``.ltm.virtual[] | {name, dest: .destination, members: [.pool.members[].address]}``
    builds a list-per-row consistently.
    """
    if not node.entries:
        return {}
    resolved: list[tuple[str, Any]] = []
    for key, value_expr in node.entries:
        resolved.append((key, _eval(value_expr, current, ctx)))

    # Identify stream-valued slots; if any, broadcast.
    stream_indices = [i for i, (_, v) in enumerate(resolved) if isinstance(v, Stream)]
    if not stream_indices:
        return {k: _scalarise_dict_value(v) for k, v in resolved}
    lengths = {len(resolved[i][1].items) for i in stream_indices}
    if len(lengths) > 1:
        raise EvalError(
            "object literal: cannot broadcast streams of differing "
            f"lengths ({sorted(lengths)!r}); collect one side with "
            "[...] first to keep each field a single list value"
        )
    (length,) = lengths
    rows: list[dict[str, Any]] = []
    for idx in range(length):
        row: dict[str, Any] = {}
        for i, (k, v) in enumerate(resolved):
            row[k] = _scalarise_dict_value(v.items[idx] if i in stream_indices else v)
        rows.append(row)
    return Stream(items=rows)


def _scalarise_dict_value(value: Any) -> Any:
    """Render a value for placement inside a dict.

    ``PathRef`` → its full-path string (consistent with how scalar
    rendering treats PathRefs everywhere else), ``ObjectRef`` →
    full-path (objects are too big to inline as a sub-tree; if the
    user wants the projected fields they can spell them out as
    individual keys).  Plain lists pass through so a user-built
    ``[.pool.members[].address]`` lands as a JSON array.
    """
    if isinstance(value, PathRef):
        return value.full_path
    if isinstance(value, ObjectRef):
        return value.full_path
    return value


# ---------------------------------------------------------------------------
# Variables
# ---------------------------------------------------------------------------


def _eval_variable(node: Variable, ctx: EvalContext) -> Any:
    """Resolve ``$name`` to a bound value or the named source's root.

    Lexical ``as``-bindings (introduced by
    ``expr as $name | body``) win first; if no lexical binding is in
    scope, the named-source table (auto-named from the filename stem
    or set explicitly via ``--name N=PATH``) is consulted.  Unknown
    names raise an :class:`EvalError` that lists what's actually
    bound — guessing silently against an arbitrary source / value
    would mask typos.
    """
    if node.name in ctx.bindings:
        return ctx.bindings[node.name]
    root = ctx.named_roots.get(node.name)
    if root is None:
        bound_names = sorted(set(ctx.named_roots) | set(ctx.bindings)) or ["<none>"]
        if not bound_names or bound_names == ["<none>"]:
            raise EvalError(
                f"${node.name}: no variables are bound — introduce one "
                "with ``expr as $name | ...`` or load several configs "
                "to auto-bind one per filename stem."
            )
        known = ", ".join(bound_names)
        raise EvalError(f"${node.name}: unknown variable (known: {known})")
    return root_container(root)


def _eval_let_binding(node: LetBinding, current: Any, ctx: EvalContext) -> Any:
    """Evaluate ``expr as $name | body``.

    The source expression is evaluated once against the current
    input.  Only :class:`Stream` source values iterate — one body
    call per item, matching jq's generator semantics:

        .ltm.virtual[] as $vs | $vs.pool.members[] | $vs.name + ...

    Plain Python lists (the result of an explicit ``[ ... ]`` array
    constructor) bind as a single value, matching jq's array
    constructor + binding model: ``[.ltm.virtual[].name] as $names |
    $names`` binds the whole collected list to ``$names`` and runs
    the body once.  Other scalar values (numbers, strings,
    :class:`ObjectRef`, :class:`PathRef`, dicts, ...) also bind once.

    The current input (``.``) is *not* rebound — the body still sees
    the outer ``.``, so let-bindings compose with the surrounding
    pipeline.  The binding frame is shallow-copied per iteration so
    nested let-bindings of the same name shadow correctly.
    """
    source_value = _eval(node.source, current, ctx)
    # Only Streams iterate; lists/scalars bind once.  Without this
    # split, ``[.x[].name] as $names | $names`` would explode the
    # collected list back into N body calls and produce a stream of
    # names rather than binding the list itself, contradicting jq's
    # array constructor semantics.
    if isinstance(source_value, Stream):
        items = list(source_value.items)
        is_iterating = True
    else:
        items = [source_value]
        is_iterating = False

    out: list[Any] = []
    for item in items:
        prior = ctx.bindings.get(node.name, _UNBOUND)
        ctx.bindings[node.name] = item
        try:
            result = _eval(node.body, current, ctx)
        finally:
            if prior is _UNBOUND:
                ctx.bindings.pop(node.name, None)
            else:
                ctx.bindings[node.name] = prior
        out.extend(_flatten(result))
    if is_iterating or len(items) != 1:
        return Stream(items=out)
    if not out:
        return Stream(items=[])
    if len(out) == 1:
        return out[0]
    return Stream(items=out)


class _Unbound:
    """Sentinel marking a binding that did not exist before push."""

    __slots__ = ()


_UNBOUND = _Unbound()


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------


def _eval_path(node: PathExpr, current: Any, ctx: EvalContext) -> Any:
    # ``head`` is the optional rooting expression: ``$x.foo``,
    # ``f(.).y``, ``"text"[0]``.  When set the walk starts from its
    # value; otherwise the bare ``.`` is the starting input.  The
    # outer ``current`` stays available as ``path_input`` so a
    # subscript index expression (``$x[.field]``) keeps the same
    # ``.`` as the surrounding pipeline — matches jq.
    is_stream = False
    if node.head is not None:
        head_value = _eval(node.head, current, ctx)
        if isinstance(head_value, Stream):
            values = list(head_value.items)
            is_stream = True
        else:
            values = [head_value]
    else:
        values = [current]
    path_input = current
    for step in node.steps:
        next_values: list[Any] = []
        for value in values:
            for produced, produced_is_stream in _step(value, step, path_input, ctx):
                next_values.append(produced)
                if produced_is_stream:
                    is_stream = True
        values = next_values
    if is_stream or len(values) != 1:
        return Stream(items=values)
    return values[0]


def _step(value: Any, step, path_input: Any, ctx: EvalContext):
    if isinstance(step, Field):
        try:
            produced_iter = list(_field_step(value, step, ctx))
        except EvalError:
            if getattr(step, "optional", False):
                return
            raise
        for produced in produced_iter:
            yield produced, False
        return
    if isinstance(step, Subscript):
        optional = getattr(step, "optional", False)
        if step.stream:
            try:
                root = _subscript_root(value, ctx)
            except EvalError:
                if optional:
                    return
                raise
            for item in _stream_items(root):
                yield item, True
            return
        if step.regex is not None:
            try:
                regex_items = list(_regex_subscript(value, step.regex, ctx))
            except EvalError:
                if optional:
                    return
                raise
            for item in regex_items:
                yield item, True
            return
        # Subscript index expressions evaluate against the *outer*
        # pipe input (``path_input``), matching jq: in ``.a | $x[.b]``
        # the ``.b`` resolves against ``.a``, not against ``$x``.
        idx_val = _eval(step.index, path_input, ctx)
        try:
            yield _subscript_step(value, idx_val, ctx), False
        except EvalError:
            if optional:
                return
            raise
        return
    raise EvalError(f"unknown path step: {type(step).__name__}")  # pragma: no cover


def _field_step(value: Any, step: Field, ctx: EvalContext):
    name = step.name
    # PathRefs auto-deref: looking up a field on a path goes to the
    # referenced object's fields.
    if isinstance(value, PathRef):
        target = _resolve_pathref(value, ctx)
        if target is None:
            return ()
        return _field_step(target, step, ctx)
    if isinstance(value, Container):
        return (value.lookup(name),)
    if isinstance(value, ObjectRef):
        if name not in value.fields:
            raise EvalError(f"{value.kind}: no field {name!r}")
        return (value.fields[name],)
    if isinstance(value, dict):
        if name not in value:
            raise EvalError(f"no field {name!r}")
        return (value[name],)
    typed_value = _typed_query_field(value, name)
    if typed_value is not _MISSING:
        return (typed_value,)
    raise EvalError(f"cannot read field {name!r} on {_describe(value)}")


class _Missing:
    __slots__ = ()


_MISSING = _Missing()


def _typed_query_field(value: object, name: str) -> object:
    """Return one explicit typed-value field or ``_MISSING``.

    This keeps query field access broad enough for typed BIG-IP values
    while avoiding the old arbitrary ``hasattr/getattr`` fallback.  A
    value can opt in with ``query_fields()``; dataclass fields and class
    properties are also exposed because the typed value layer is built
    from frozen dataclasses with deliberate computed properties such as
    ``full_path`` and ``name``.
    """
    if name.startswith("_"):
        return _MISSING
    attr = name.replace("-", "_")
    if attr.startswith("_"):
        return _MISSING
    if isinstance(value, QueryFieldProvider):
        query_fields = value.query_fields()
        if name in query_fields:
            return query_fields[name]
        if attr in query_fields:
            return query_fields[attr]
    if is_dataclass(value) and not isinstance(value, type):
        for dataclass_field in fields(value):
            if dataclass_field.name == attr:
                return getattr(value, attr)
    descriptor = getattr(type(value), attr, None)
    if isinstance(descriptor, property):
        return descriptor.__get__(value, type(value))
    return _MISSING


def _subscript_root(value: Any, ctx: EvalContext) -> Any:
    if isinstance(value, PathRef):
        target = _resolve_pathref(value, ctx)
        if target is None:
            return Stream(items=[])
        return _subscript_root(target, ctx)
    if isinstance(value, Container):
        entries = list(value.entries().values())
        # ``.pem[]`` / ``.ltm[]`` / ``.[]`` etc. iterate a container
        # whose direct entries are *also* containers (module → kind
        # → object).  Flatten until we hit non-``Container`` items
        # so the user gets a stream of addressable objects across
        # every kind (or every module) instead of opaque
        # ``Container(kind=...)`` repr that the renderer can't
        # handle as a scalar.  Bounded by Python's recursion limit
        # via the explicit loop — two real levels at root (module →
        # kind → object), one level elsewhere.
        while entries and all(isinstance(e, Container) for e in entries):
            flat: list[Any] = []
            for sub in entries:
                flat.extend(sub.entries().values())
            entries = flat
        return entries
    if isinstance(value, (list, Stream)):
        return value
    if isinstance(value, dict):
        return list(value.values())
    if isinstance(value, ObjectRef):
        return list(value.fields.values())
    # Iterable typed values (BigipList / MonitorExpression / …)
    # implement ``__iter__`` directly; honour it so DSL ``[]``
    # subscripts work without registering every concrete type
    # here.  Strings are excluded because the DSL never iterates
    # character-by-character.
    if isinstance(value, Iterable) and not isinstance(value, (str, bytes)):
        return list(iter(value))
    raise EvalError(f"cannot iterate {_describe(value)}")


def _regex_subscript(value: Any, pattern: str, ctx: EvalContext):
    from .builtins import BuiltinError, _safe_regex_compile

    if isinstance(value, PathRef):
        target = _resolve_pathref(value, ctx)
        if target is None:
            return ()
        return _regex_subscript(target, pattern, ctx)
    if isinstance(value, Container):
        keys = value.regex_keys(pattern)
        return [value.entries()[k] for k in keys]
    if isinstance(value, dict):
        try:
            rx = _safe_regex_compile(pattern, name="regex subscript")
        except BuiltinError as exc:
            raise EvalError(str(exc)) from exc
        return [item for key, item in value.items() if rx.search(str(key))]
    if isinstance(value, ObjectRef):
        # Cross-kind regex subscript: ``.ltm[]["~app3"]`` flattens the
        # module container through every kind into a stream of
        # ObjectRefs, then this branch matches each one by full-path
        # against the regex.  Returns a single-element list when the
        # object matches and an empty list otherwise so the caller's
        # for-each loop produces the expected stream of matches.  The
        # pattern is anchored via :func:`re.search` (not ``fullmatch``)
        # so substring-style searches (``"~app3"``) work as users
        # would expect from the kind-level subscript form
        # (``.ltm.pool["~app3"]`` is also a substring search).  Route
        # through the shared length / catastrophic-backtracking guards
        # so MCP / chat surfaces can't feed a pathological pattern
        # into this branch and hang the engine.
        try:
            rx = _safe_regex_compile(pattern, name="regex subscript")
        except BuiltinError as exc:
            raise EvalError(str(exc)) from exc
        return [value] if rx.search(value.full_path) else []
    raise EvalError(f"regex subscript not supported on {_describe(value)}")


def _subscript_step(value: Any, index: Any, ctx: EvalContext) -> Any:
    if isinstance(value, PathRef):
        target = _resolve_pathref(value, ctx)
        if target is None:
            raise EvalError(f"cannot subscript unresolved path {value.full_path!r}")
        return _subscript_step(target, index, ctx)
    if isinstance(value, Container):
        if isinstance(index, str):
            return value.lookup(index)
        if isinstance(index, int):
            keys = list(value.entries())
            if not -len(keys) <= index < len(keys):
                raise EvalError(f"{value.kind}: index {index} out of range")
            return value.entries()[keys[index]]
    if isinstance(value, list):
        if not isinstance(index, int):
            raise EvalError(f"list subscript must be an integer, got {_describe(index)}")
        if not -len(value) <= index < len(value):
            raise EvalError(f"list index {index} out of range")
        return value[index]
    if isinstance(value, dict):
        if isinstance(index, str):
            if index in value:
                return value[index]
            raise EvalError(f"no field {index!r}")
        if isinstance(index, int):
            items = list(value.values())
            if not -len(items) <= index < len(items):
                raise EvalError(f"object index {index} out of range")
            return items[index]
    if isinstance(value, ObjectRef):
        if isinstance(index, str):
            if index in value.fields:
                return value.fields[index]
            raise EvalError(f"{value.kind}: no field {index!r}")
    raise EvalError(f"cannot subscript {_describe(value)} with {_describe(index)}")


def _resolve_pathref(ref: PathRef, ctx: EvalContext) -> ObjectRef | None:
    """Look up the :class:`ObjectRef` that *ref* points to.

    Forces the relevant container's entries to be built (which caches
    every ObjectRef on the root) and then returns the cached entry.
    """
    if not ref.full_path or ref.root is None:
        return None
    root = ref.root
    # Fast path: try the cache first using the expected kind (or the
    # ``ltm pool``/etc. recorded on the :class:`PathRef`).
    if ref.expected_kind:
        cached = root._object_cache.get((ref.expected_kind, ref.full_path))
        if cached is not None:
            return cached
    from .projection import MODULE_KINDS

    # PathRef resolution only runs against BIG-IP roots — JSON
    # roots never produce PathRefs.  Narrow the type so ``ty``
    # accepts the ``.lookup()`` calls below.
    container = root_container(root)
    if not isinstance(container, Container):
        return None
    # Targeted scope: if we know the kind we're looking for, only
    # build that one container's entries.  Keeps lazy projection cost
    # proportional to the kinds the query actually touches.
    if ref.expected_kind:
        for module, kinds in MODULE_KINDS.items():
            for label, (_, kind) in kinds.items():
                if kind != ref.expected_kind:
                    continue
                try:
                    mod_container = container.lookup(module)
                    mod_container.entries()[label].entries()
                except (KeyError, EvalError):
                    return None
                return root._object_cache.get((ref.expected_kind, ref.full_path))
        return None
    # Fallback: no expected kind — walk every container under every
    # module until a cache entry surfaces.  Only reached for
    # hand-built PathRefs without an ``expected_kind``.
    for module in MODULE_KINDS:
        try:
            mod_container = container.lookup(module)
        except EvalError:
            continue
        for label in mod_container.entries():
            try:
                mod_container.entries()[label].entries()
            except EvalError:
                continue
            for cache_key, cached in root._object_cache.items():
                if cache_key[1] == ref.full_path:
                    return cached
    return None


# ---------------------------------------------------------------------------
# Calls
# ---------------------------------------------------------------------------


def _eval_call(node: Call, current: Any, ctx: EvalContext) -> Any:
    spec = _builtins.lookup(node.name)
    if spec is None:
        raise EvalError(f"unknown function {node.name!r}")
    arity = len(node.args)

    # jq-style implicit ``.``: when a single-argument builtin is
    # called with no parentheses (``.rules | count`` instead of
    # ``.rules | count(.)``), the current input is passed as the one
    # argument.  Only applies to non-special-form builtins whose
    # arity is exactly one — special forms (``select`` / ``map``)
    # already require a body, and multi-arg builtins need explicit
    # arguments to be unambiguous.
    if arity == 0 and not spec.special_form and spec.min_args == 1 and spec.max_args == 1:
        if spec.with_ctx:
            return spec.impl(current, ctx=ctx)
        return spec.impl(current)

    # jq-style implicit-receiver for multi-arg builtins: ``.x |
    # contains("foo")`` is equivalent to ``contains(.x, "foo")``.
    # When a non-special-form builtin is called with exactly one
    # fewer argument than its minimum, prepend the current value as
    # the receiver.  Skips ``with_ctx`` builtins (they reach into
    # evaluator state and are usually free-standing) and stream-
    # aware builtins (their broadcast rules clash with implicit
    # prepending).
    if (
        not spec.special_form
        and not spec.with_ctx
        and not spec.stream_aware
        and spec.min_args >= 2
        and arity == spec.min_args - 1
        and (spec.max_args is None or arity + 1 <= spec.max_args)
    ):
        prepended_args = [_eval(a, current, ctx) for a in node.args]
        return spec.impl(current, *prepended_args)

    if arity < spec.min_args or (spec.max_args is not None and arity > spec.max_args):
        if spec.min_args == spec.max_args:
            expected = f"{spec.min_args}"
        elif spec.max_args is None:
            expected = f">= {spec.min_args}"
        else:
            expected = f"{spec.min_args}..{spec.max_args}"
        raise EvalError(f"{node.name}: expected {expected} argument(s), got {arity}")

    if spec.special_form:
        return _eval_special_form(node, current, ctx)

    args = [_eval(a, current, ctx) for a in node.args]
    # Scalar builtins (the default) broadcast each stream argument
    # element-wise so idioms like
    # ``is_fqdn(.pool.members[].address)`` produce a stream of
    # booleans rather than rejecting the stream input.  When several
    # arguments are streams they zip pairwise (lengths must match);
    # scalar arguments mixed with stream arguments replicate across
    # each iteration.  Stream-aware builtins (``count`` / ``sort`` /
    # ``any`` / ``all`` / ...) opt out via ``stream_aware=True`` so
    # they receive the whole stream as a single argument; ``with_ctx``
    # builtins also skip broadcast because they reach into evaluator
    # state and rarely match the element-wise shape.
    if not spec.stream_aware and not spec.with_ctx:
        broadcast = _broadcast_call_args(node.name, args)
        if broadcast is not None:
            return Stream(items=[spec.impl(*tup) for tup in broadcast])
    if spec.with_ctx:
        return spec.impl(*args, ctx=ctx)
    return spec.impl(*args)


def _broadcast_call_args(name: str, args: list[Any]) -> list[tuple[Any, ...]] | None:
    """Compute the broadcast iteration plan for a scalar builtin's args.

    Returns ``None`` when no argument is a :class:`Stream` (so the
    dispatcher takes the regular call path).  Otherwise returns a list
    of arg-tuples, one per element of the broadcast.  All stream
    arguments must share a length; scalars replicate.  Raises
    :class:`EvalError` on length mismatch.
    """
    stream_indices = [i for i, a in enumerate(args) if isinstance(a, Stream)]
    if not stream_indices:
        return None
    lengths = {len(args[i].items) for i in stream_indices}
    if len(lengths) > 1:
        raise EvalError(
            f"{name}: cannot broadcast streams of differing lengths "
            f"({sorted(lengths)!r}); collect one side with [...] first"
        )
    (length,) = lengths
    plan: list[tuple[Any, ...]] = []
    for idx in range(length):
        row = []
        for i, a in enumerate(args):
            if i in stream_indices:
                row.append(a.items[idx])
            else:
                row.append(a)
        plan.append(tuple(row))
    return plan


def _eval_special_form(node: Call, current: Any, ctx: EvalContext) -> Any:
    if node.name == "select":
        body = node.args[0]
        result = _eval(body, current, ctx)
        # Streams: drop falsy entries one by one.
        if isinstance(result, Stream):
            return current if any(_truthy(v) for v in result.items) else _DROP
        return current if _truthy(result) else _DROP
    if node.name == "map":
        body = node.args[0]
        items = _stream_items(current)
        out: list[Any] = []
        for item in items:
            value = _eval(body, item, ctx)
            out.extend(_flatten(value))
        return out
    raise EvalError(f"unsupported special form: {node.name}")


# Sentinel used by ``select`` to indicate "drop the current value".
class _Drop:
    __slots__ = ()


_DROP = _Drop()


# ---------------------------------------------------------------------------
# Operators
# ---------------------------------------------------------------------------


def _eval_binop(node: BinOp, current: Any, ctx: EvalContext) -> Any:
    if node.op == "and":
        lhs = _eval(node.lhs, current, ctx)
        if not _truthy(lhs):
            return False
        return _truthy(_eval(node.rhs, current, ctx))
    if node.op == "or":
        lhs = _eval(node.lhs, current, ctx)
        if _truthy(lhs):
            return True
        return _truthy(_eval(node.rhs, current, ctx))

    lhs_raw = _eval(node.lhs, current, ctx)
    rhs_raw = _eval(node.rhs, current, ctx)
    # Stream broadcast: when either side of an arithmetic / comparison
    # operator is a :class:`Stream`, broadcast the scalar across each
    # item.  Without this, the natural row-builder shape
    # ``.ltm.virtual[].name + "\t" + .pool.members[].address`` would
    # error out with "cannot add str and stream"; broadcasting yields
    # one row per stream item which is what the user almost always
    # wanted.  Both-sides-streams iterate pairwise when the lengths
    # match and error otherwise (consistent with how jq's ``,`` /
    # stream-pair semantics work).
    if isinstance(lhs_raw, Stream) or isinstance(rhs_raw, Stream):
        return _broadcast_binop(node.op, lhs_raw, rhs_raw)
    lhs = _coerce_scalar(lhs_raw)
    rhs = _coerce_scalar(rhs_raw)
    if node.op == "==":
        return _eq(lhs, rhs)
    if node.op == "!=":
        return not _eq(lhs, rhs)
    if node.op in {"<", "<=", ">", ">="}:
        return _cmp(lhs, rhs, node.op)
    if node.op == "+":
        return _add(lhs, rhs)
    if node.op == "-":
        return _sub(lhs, rhs)
    if node.op == "*":
        return _mul(lhs, rhs)
    if node.op == "/":
        return _div(lhs, rhs)
    raise EvalError(f"unsupported operator {node.op!r}")


def _apply_scalar_binop(op: str, lhs: Any, rhs: Any) -> Any:
    """Run a scalar-only binop after operand coercion.

    The same dispatch table as :func:`_eval_binop` but applied to
    already-evaluated values; used by :func:`_broadcast_binop` so the
    stream-broadcast path doesn't have to recurse through the AST.
    """
    lhs = _coerce_scalar(lhs)
    rhs = _coerce_scalar(rhs)
    if op == "==":
        return _eq(lhs, rhs)
    if op == "!=":
        return not _eq(lhs, rhs)
    if op in {"<", "<=", ">", ">="}:
        return _cmp(lhs, rhs, op)
    if op == "+":
        return _add(lhs, rhs)
    if op == "-":
        return _sub(lhs, rhs)
    if op == "*":
        return _mul(lhs, rhs)
    if op == "/":
        return _div(lhs, rhs)
    raise EvalError(f"unsupported operator {op!r}")


def _broadcast_binop(op: str, lhs: Any, rhs: Any) -> Stream:
    """Apply *op* across a stream operand, broadcasting the scalar side.

    - ``stream op scalar``  → ``Stream(items=[op(x, scalar) for x in stream])``
    - ``scalar op stream``  → ``Stream(items=[op(scalar, x) for x in stream])``
    - ``stream op stream``  → pairwise when lengths match; raises when
      they don't (the user almost certainly meant to widen with
      ``[...]`` first if they wanted a cartesian).
    """
    if isinstance(lhs, Stream) and isinstance(rhs, Stream):
        if len(lhs.items) != len(rhs.items):
            raise EvalError(
                f"cannot {op!r} two streams of differing lengths "
                f"({len(lhs.items)} vs {len(rhs.items)}) — collect one "
                "side with [...] first"
            )
        return Stream(items=[_apply_scalar_binop(op, a, b) for a, b in zip(lhs.items, rhs.items)])
    if isinstance(lhs, Stream):
        return Stream(items=[_apply_scalar_binop(op, item, rhs) for item in lhs.items])
    assert isinstance(rhs, Stream)
    return Stream(items=[_apply_scalar_binop(op, lhs, item) for item in rhs.items])


def _eval_unop(node: UnaryOp, current: Any, ctx: EvalContext) -> Any:
    val = _eval(node.operand, current, ctx)
    if node.op == "not":
        return not _truthy(val)
    if node.op == "-":
        if isinstance(val, bool):
            raise EvalError("cannot negate a boolean")
        if isinstance(val, (int, float)):
            return -val
        raise EvalError(f"cannot negate {_describe(val)}")
    raise EvalError(f"unsupported unary operator {node.op!r}")


def _coerce_scalar(value: Any) -> Any:
    if isinstance(value, PathRef):
        return value.full_path
    return value


def _eq(lhs: Any, rhs: Any) -> bool:
    return lhs == rhs


def _cmp(lhs: Any, rhs: Any, op: str) -> bool:
    if type(lhs) is not type(rhs) and not (
        isinstance(lhs, (int, float)) and isinstance(rhs, (int, float))
    ):
        raise EvalError(f"cannot compare {_describe(lhs)} with {_describe(rhs)}")
    if op == "<":
        return lhs < rhs
    if op == "<=":
        return lhs <= rhs
    if op == ">":
        return lhs > rhs
    return lhs >= rhs  # ">="


def _add(lhs: Any, rhs: Any) -> Any:
    if isinstance(lhs, str) and isinstance(rhs, str):
        return lhs + rhs
    if isinstance(lhs, (int, float)) and isinstance(rhs, (int, float)):
        return lhs + rhs
    if isinstance(lhs, list):
        # ``rules += "/Common/log"`` — appending a scalar to a list is
        # an ergonomic divergence from jq; it makes the cookbook
        # "ensure this iRule is attached" idiom one statement.
        if isinstance(rhs, list):
            return lhs + rhs
        return lhs + [rhs]
    # str + scalar / scalar + str: coerce the non-string side to
    # its scalar string form so report-style queries like
    # ``.name + ": " + count(.members)`` Just Work without an
    # explicit ``str()`` cast.  Bools render as ``true`` / ``false``
    # to match the rest of the DSL's scalar rendering; ``None``
    # renders as ``null``.
    if isinstance(lhs, str) and _is_concat_scalar(rhs):
        return lhs + _scalar_to_string(rhs)
    if isinstance(rhs, str) and _is_concat_scalar(lhs):
        return _scalar_to_string(lhs) + rhs
    raise EvalError(f"cannot add {_describe(lhs)} and {_describe(rhs)}")


def _is_concat_scalar(value: Any) -> bool:
    return isinstance(value, (int, float, bool, PathRef)) or value is None


def _scalar_to_string(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, PathRef):
        return value.full_path
    return str(value)


def _sub(lhs: Any, rhs: Any) -> Any:
    if isinstance(lhs, (int, float)) and isinstance(rhs, (int, float)):
        return lhs - rhs
    if isinstance(lhs, list):
        if isinstance(rhs, list):
            return [item for item in lhs if item not in rhs]
        return [item for item in lhs if item != rhs]
    raise EvalError(f"cannot subtract {_describe(rhs)} from {_describe(lhs)}")


def _mul(lhs: Any, rhs: Any) -> Any:
    if isinstance(lhs, (int, float)) and isinstance(rhs, (int, float)):
        return lhs * rhs
    raise EvalError(f"cannot multiply {_describe(lhs)} and {_describe(rhs)}")


def _div(lhs: Any, rhs: Any) -> Any:
    if isinstance(lhs, (int, float)) and isinstance(rhs, (int, float)):
        if rhs == 0:
            raise EvalError("division by zero")
        return lhs / rhs
    raise EvalError(f"cannot divide {_describe(lhs)} by {_describe(rhs)}")


# ---------------------------------------------------------------------------
# Assignments — collect into the edit plan, return the (post-edit) value.
# ---------------------------------------------------------------------------


def _eval_assignment(node: Assignment, current: Any, ctx: EvalContext) -> Any:
    # ``$name.field = ...`` walks the path against the named root's
    # container rather than the outer input.  This is the only place
    # the AST exposes a Variable directly on the LHS; the parser sets
    # ``node.source`` for that form and leaves it ``None`` for the
    # normal ``.field = ...`` case.
    path_input = current
    if node.source is not None:
        path_input = _eval_variable(node.source, ctx)
    targets = _resolve_assignment_targets(node.target, path_input, ctx)
    produced: list[Any] = []
    for target in targets:
        new_value = _compute_assignment_value(node, target, current, ctx)
        op = _build_edit_op(target, node.op, new_value, ctx)
        ctx.edits.add(op)
        produced.append(new_value)
    if not produced:
        return Stream(items=[])
    if len(produced) == 1:
        return produced[0]
    return Stream(items=produced)


@dataclass
class _AssignTarget:
    """A single resolved LHS for an assignment."""

    obj: ObjectRef
    field_name: str  # TMSH-spelt
    current_value: Any


def _resolve_assignment_targets(
    path: PathExpr,
    current: Any,
    ctx: EvalContext,
) -> list[_AssignTarget]:
    """Walk *path* but stop one step before the final field access.

    The penultimate value must be an :class:`ObjectRef`, and the final
    step must be a :class:`Field`.  This keeps the writable surface
    small and the error messages crisp.
    """
    if not path.steps:
        raise EvalError("cannot assign to the identity ('.')")
    final = path.steps[-1]
    if not isinstance(final, Field):
        raise EvalError(
            "assignment LHS must end in a field access (e.g. '.foo'); "
            "subscript-only paths are not writable"
        )
    prefix = PathExpr(steps=path.steps[:-1], offset=path.offset, head=path.head)
    prefix_value = _eval_path(prefix, current, ctx)
    targets: list[_AssignTarget] = []
    for obj in _flatten(prefix_value):
        if isinstance(obj, PathRef):
            resolved = _resolve_pathref(obj, ctx)
            if resolved is None:
                raise EvalError(f"cannot assign through unresolved path {obj.full_path!r}")
            obj = resolved
        if not isinstance(obj, ObjectRef):
            raise EvalError(
                f"cannot assign to {final.name!r} on {_describe(obj)} "
                "(LHS must resolve to a BIG-IP object)"
            )
        if final.name not in obj.fields:
            raise EvalError(f"{obj.kind}: no field {final.name!r}")
        targets.append(
            _AssignTarget(obj=obj, field_name=final.name, current_value=obj.fields[final.name])
        )
    return targets


def _compute_assignment_value(
    node: Assignment,
    target: _AssignTarget,
    outer: Any,
    ctx: EvalContext,
) -> Any:
    if node.op == "=":
        return _flatten_one(_eval(node.rhs, outer, ctx))
    if node.op == "|=":
        return _flatten_one(_eval(node.rhs, target.current_value, ctx))
    if node.op == "+=":
        rhs_val = _flatten_one(_eval(node.rhs, outer, ctx))
        return _add(target.current_value, rhs_val)
    if node.op == "-=":
        rhs_val = _flatten_one(_eval(node.rhs, outer, ctx))
        return _sub(target.current_value, rhs_val)
    raise EvalError(f"unknown assignment operator: {node.op}")


def _build_edit_op(
    target: _AssignTarget,
    op: str,
    new_value: Any,
    ctx: EvalContext,
) -> EditOp:
    # ``target.obj.config_uri`` is the URI of the source that defined
    # the object — it may differ from ``ctx.root.uri`` when the LHS
    # was ``$name.path`` and the named root is a different config.
    # Routing by the target's URI is what makes cross-file edits land
    # in the right file.  Falls back to the eval context's root when
    # the object came from a synthetic / un-attributed source.
    return EditOp(
        source_uri=target.obj.config_uri or ctx.root.uri,
        object_path=target.obj.full_path,
        object_kind=target.obj.kind,
        field_name=target.field_name,
        operator=op,
        old_value=target.current_value,
        new_value=new_value,
        field_slot=target.obj.field_slots.get(target.field_name),
        stanza_slot=target.obj.stanza_slot,
    )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _flatten(value: Any) -> list[Any]:
    if isinstance(value, _Drop):
        return []
    if isinstance(value, Stream):
        out: list[Any] = []
        for item in value.items:
            if isinstance(item, _Drop):
                continue
            out.append(item)
        return out
    return [value]


def _flatten_one(value: Any) -> Any:
    flat = _flatten(value)
    if not flat:
        raise EvalError("empty stream cannot be assigned")
    if len(flat) > 1:
        raise EvalError("assignment RHS produced multiple values")
    return flat[0]


def _stream_items(value: Any) -> list[Any]:
    if isinstance(value, Stream):
        return list(value.items)
    if isinstance(value, list):
        return list(value)
    return [value]


def _describe(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, Container):
        return f"container({value.kind})"
    if isinstance(value, ObjectRef):
        return f"object({value.kind})"
    if isinstance(value, PathRef):
        return f"path-ref({value.full_path or '<empty>'})"
    if isinstance(value, Stream):
        return f"stream(len={len(value)})"
    if isinstance(value, list):
        return f"list(len={len(value)})"
    return type(value).__name__
