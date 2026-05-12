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

from dataclasses import dataclass, field
from typing import Any

from . import builtins as _builtins
from .ast import (
    Assignment,
    BinOp,
    Call,
    Expr,
    Field,
    Identity,
    ListLiteral,
    Literal,
    PathExpr,
    Pipe,
    Program,
    Subscript,
    UnaryOp,
)
from .builtins import _truthy
from .edit_plan import EditOp, EditPlan
from .errors import EvalError
from .projection import Container, root_container
from .values import ObjectRef, PathRef, Root, Stream


@dataclass
class EvalContext:
    root: Root
    edits: EditPlan = field(default_factory=EditPlan)


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
    if isinstance(node, ListLiteral):
        if node.inner is None:
            return []
        return _flatten(_eval(node.inner, current, ctx))
    if isinstance(node, PathExpr):
        return _eval_path(node, current, ctx)
    if isinstance(node, Pipe):
        lhs = _eval(node.lhs, current, ctx)
        return _pipe_through(lhs, node.rhs, ctx)
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
    running ``length`` on each PathRef.  Use ``.[]`` (or ``collect``)
    to convert between forms explicitly.
    """
    if isinstance(values, Stream):
        out: list[Any] = []
        for item in values.items:
            out.extend(_flatten(_eval(rhs, item, ctx)))
        return Stream(items=out)
    return _eval(rhs, values, ctx)


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------


def _eval_path(node: PathExpr, current: Any, ctx: EvalContext) -> Any:
    values: list[Any] = [current]
    is_stream = False
    for step in node.steps:
        next_values: list[Any] = []
        for value in values:
            for produced, produced_is_stream in _step(value, step, ctx):
                next_values.append(produced)
                if produced_is_stream:
                    is_stream = True
        values = next_values
    if is_stream or len(values) != 1:
        return Stream(items=values)
    return values[0]


def _step(value: Any, step, ctx: EvalContext):
    if isinstance(step, Field):
        for produced in _field_step(value, step, ctx):
            yield produced, False
        return
    if isinstance(step, Subscript):
        if step.stream:
            for item in _stream_items(_subscript_root(value, ctx)):
                yield item, True
            return
        if step.regex is not None:
            for item in _regex_subscript(value, step.regex, ctx):
                yield item, True
            return
        idx_val = _eval(step.index, value, ctx)
        yield _subscript_step(value, idx_val, ctx), False
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
    raise EvalError(f"cannot read field {name!r} on {_describe(value)}")


def _subscript_root(value: Any, ctx: EvalContext) -> Any:
    if isinstance(value, PathRef):
        target = _resolve_pathref(value, ctx)
        if target is None:
            return Stream(items=[])
        return _subscript_root(target, ctx)
    if isinstance(value, Container):
        return list(value.entries().values())
    if isinstance(value, (list, Stream)):
        return value
    if isinstance(value, ObjectRef):
        return list(value.fields.values())
    raise EvalError(f"cannot iterate {_describe(value)}")


def _regex_subscript(value: Any, pattern: str, ctx: EvalContext):
    if isinstance(value, PathRef):
        target = _resolve_pathref(value, ctx)
        if target is None:
            return ()
        return _regex_subscript(target, pattern, ctx)
    if isinstance(value, Container):
        keys = value.regex_keys(pattern)
        return [value.entries()[k] for k in keys]
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

    container = root_container(root)
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
    if spec.with_ctx:
        return spec.impl(*args, ctx=ctx)
    return spec.impl(*args)


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

    lhs = _coerce_scalar(_eval(node.lhs, current, ctx))
    rhs = _coerce_scalar(_eval(node.rhs, current, ctx))
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
    raise EvalError(f"cannot add {_describe(lhs)} and {_describe(rhs)}")


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
    targets = _resolve_assignment_targets(node.target, current, ctx)
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
    prefix = PathExpr(steps=path.steps[:-1], offset=path.offset)
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
    return EditOp(
        source_uri=ctx.root.uri,
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
