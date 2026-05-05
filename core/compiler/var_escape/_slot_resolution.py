"""Phase 7 of the var/frame architecture refactor — compile-time slot
resolution for proc-locals.

The runtime side (``frame_local_at`` / ``frame_local_set_at`` /
``frame_locals_array``) is already in place in ``runtime/zig/interp/
tcl_frames.zig``; this pass adds the codegen consumer.  For every
proc whose body satisfies a small set of safety checks we assign
indices ``0..LOCALS_ARRAY_CAP-1`` to its scalar literal locals, and
the WASM emitter uses those indices to swap the name-keyed
``tcl_local_set`` / ``tcl_local_get`` calls for the indexed
accessors.

Eligibility — every check must pass for the proc to receive a slot
mapping at all:

* ``not summary.dynamic_barrier`` — pessimistic procs already use
  the WASM-local + sync path; the indexed slots stay unused.
* ``not summary.has_fallback`` and ``not summary.has_call_fallback``
  — any path that reaches the interpreter would need a name-keyed
  bucket on the frame; an indexed slot wouldn't satisfy
  ``var_resolve``'s linear scan.
* ``not summary.upvar_source_names`` and ``not summary.
  unbounded_upvar_source`` — a callee's ``upvar`` reaches into the
  caller frame's hash store; redirecting to indexed storage would
  break the alias.
* No ``upvar`` / ``global`` / ``variable`` / ``info exists`` /
  ``info vars`` / ``info locals`` / ``trace add variable`` /
  ``trace remove variable`` / ``trace info variable`` against any
  local in the body (these all need the hash-keyed bucket).
* No ``eval`` / ``uplevel`` / ``apply`` whose body could name the
  local at runtime.
* No dynamic name access (``set $varname value`` / ``[set $name]``)
  that would route through the runtime by-name path.

Within an eligible proc, every literal scalar local that survives
the per-name checks gets an index in registration order (first
write or first read wins).  Capacity is the runtime-defined
``LOCALS_ARRAY_CAP`` (16); names beyond that fall back to the hash
store transparently — :func:`assign_local_slots` simply omits them
from the returned mapping.

The pass returns the slot map; callers use
``ProcEscapeSummary.with_local_slots`` to fold it into the summary.
"""

from __future__ import annotations

from typing import Iterable

from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRExprEval,
    IRIncr,
    IRReturn,
    IRScript,
    IRStatement,
)
from ._types import ProcEscapeSummary

# Mirror of :data:`runtime.zig.interp.tcl_frames.LOCALS_ARRAY_CAP`.
# Bumping this on the runtime side should also bump it here.
LOCALS_ARRAY_CAP: int = 16

# Commands whose call shape forces the entire proc out of slot
# resolution — the runtime side of these resolves variable names
# at a level (or in a way) the codegen can't reliably mirror.
# ``upvar`` / ``global`` / ``variable`` install aliases on the
# frame's hash bucket; ``info exists`` / ``info vars`` /
# ``info locals`` walk the bucket; ``trace add variable`` registers
# against the bucket.  Pessimising the whole proc is conservative
# but keeps the implementation simple — a proc that needs any of
# these typically needs the hash store anyway.
_FRAME_HASH_BUILTINS = frozenset(
    {
        "::upvar",
        "::global",
        "::variable",
        "::lassign",  # writes vars by name
        "::lset",  # reads + writes via name
        "::regexp",  # binds capture vars by name (with -all etc.)
        "::regsub",  # similar
        "::scan",  # binds parsed values into named vars
        "::binary",  # ``binary scan`` binds vars
        "::vwait",
        "::tkwait",
    }
)

# ``info`` subcommands that introspect by name — any literal-named
# local that appears as their argument must stay in the hash store.
_INFO_INTROSPECTING_SUBCMDS = frozenset(
    {
        "exists",
        "vars",
        "locals",
        "args",
        "default",
    }
)

# ``trace`` subcommands that target a variable by name.
_TRACE_NAME_TARGETING = frozenset({"add", "remove", "info", "variable", "vdelete", "vinfo"})

# Commands whose body is executed by the interpreter, opening a
# name-resolution channel back into our local frame.  Any local
# name used inside these bodies is observable, so we pessimise
# the entire proc when it contains one.  ``catch`` is OK because
# its body still compiles; the issue is only commands that take
# the body as a SCRIPT to be re-evaluated dynamically.
_DYNAMIC_EVAL_COMMANDS = frozenset(
    {
        "::eval",
        "::uplevel",
        "::apply",
        "::source",
        "::namespace",  # ``namespace eval`` runs the body via interpreter
        "::interp",  # ``interp eval`` similarly
    }
)


def _is_dynamic_value(value: str) -> bool:
    """True when *value* is a Tcl value with substitutions — anything
    starting with ``$`` (variable substitution) or ``[`` (command
    substitution).  Names that pass this check are not literals and
    must NOT be slot-indexed (the runtime resolves them dynamically).
    """
    if not value:
        return False
    if value.startswith(("$", "[")):
        return True
    # ``${foo}`` style — also dynamic.  ``$`` later in the string
    # implies an interpolation; rule those out too.
    if "$" in value or "[" in value:
        return True
    return False


def _walk_statements(script: IRScript) -> Iterable[IRStatement]:
    """Yield every IRStatement reachable from *script*, recursing
    through compound bodies.  Order is depth-first source order so
    a slot-assignment loop sees names in the same order codegen
    emits them.
    """
    for stmt in script.statements:
        yield stmt
        for attr in ("body", "init", "next", "else_body", "finally_body"):
            sub = getattr(stmt, attr, None)
            if isinstance(sub, IRScript):
                yield from _walk_statements(sub)
        arms = getattr(stmt, "arms", None)
        if arms is not None:
            for arm in arms:
                arm_body = getattr(arm, "body", None)
                if isinstance(arm_body, IRScript):
                    yield from _walk_statements(arm_body)
        clauses = getattr(stmt, "clauses", None)
        if clauses is not None:
            for clause in clauses:
                clause_body = getattr(clause, "body", None)
                if isinstance(clause_body, IRScript):
                    yield from _walk_statements(clause_body)


def _proc_is_slot_eligible(summary: ProcEscapeSummary) -> bool:
    """Pre-screen on the escape summary alone.  Cheap path — bails
    before walking the IR for procs that the analysis already
    flagged as needing a runtime frame.
    """
    if summary.dynamic_barrier:
        return False
    if summary.has_fallback or summary.has_call_fallback:
        return False
    if summary.upvar_source_names or summary.unbounded_upvar_source:
        return False
    return True


def _stmt_disables_slots(stmt: IRStatement, ineligible_names: set[str]) -> bool:
    """Inspect *stmt* for patterns that force a particular local —
    or every local — to the hash store.  Updates *ineligible_names*
    in place with any literal-named locals the statement targets via
    a by-name builtin or introspection sub.

    Returns True when the statement's shape forces the entire proc
    to skip slot resolution (e.g. dynamic ``eval`` body, dynamic
    name in a write).
    """
    if isinstance(stmt, IRBarrier):
        # Barrier statements are pessimisation flags; treat as a
        # full disable.  The escape pre-screen already filters most
        # of these out via ``has_fallback``.
        return True
    if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
        if not isinstance(stmt.name, str):
            return True
        # ``set $varname value`` lowers to an IRCall to ``::set``
        # with a dynamic first arg.  IRAssign* always carry literal
        # names so we don't need to check here.
        return False
    if isinstance(stmt, IRCall):
        canonical = stmt.canonical_command
        args = stmt.args
        if canonical in _DYNAMIC_EVAL_COMMANDS:
            return True
        if canonical == "::set" and args:
            # ``set $varname …`` — dynamic name, can target any local.
            if _is_dynamic_value(args[0]):
                return True
        if canonical == "::info" and args:
            sub = args[0]
            if sub in _INFO_INTROSPECTING_SUBCMDS:
                # Mark any literal name argument as ineligible;
                # ``info exists $name`` (dynamic sub name) disables
                # the whole proc.
                if len(args) >= 2:
                    target = args[1]
                    if _is_dynamic_value(target):
                        return True
                    ineligible_names.add(target)
                else:
                    return True
            if sub == "level" or sub == "frame":
                # ``info level`` / ``info frame`` reach into the
                # frame and could observe locals via argv.  Pessimise.
                return True
        if canonical == "::trace" and args:
            sub = args[0]
            if sub in _TRACE_NAME_TARGETING:
                # ``trace add variable NAME …`` etc. — second arg
                # (after ``add``) is the kind, third is the name.
                if sub in ("add", "remove", "info") and len(args) >= 3:
                    if args[1] == "variable":
                        target = args[2]
                        if _is_dynamic_value(target):
                            return True
                        ineligible_names.add(target)
                elif sub in ("variable", "vdelete", "vinfo") and len(args) >= 2:
                    target = args[1]
                    if _is_dynamic_value(target):
                        return True
                    ineligible_names.add(target)
        if canonical in _FRAME_HASH_BUILTINS:
            # Conservative: any reference to one of these commands
            # disables slot resolution for the whole proc.  The
            # commands' name-resolution semantics depend on the hash
            # bucket (upvar aliases, regexp capture binding, etc.);
            # routing affected locals to the indexed array would
            # silently lose writes the runtime expects to see.
            return True
        # Dynamic command (``$cmd``) bypasses our static analysis —
        # bail out conservatively.
        if _is_dynamic_value(stmt.command):
            return True
    if isinstance(stmt, (IRExprEval, IRReturn)):
        # Expr / return values are evaluated and don't *write* into
        # locals by name — nothing to disable on top of the per-name
        # checks the assign-side already covers.
        return False
    return False


def assign_local_slots(
    script: IRScript,
    summary: ProcEscapeSummary,
    *,
    params: tuple[str, ...] = (),
) -> dict[str, int]:
    """Return a ``{local_name: slot_index}`` mapping for *script*'s
    proc-locals that pass the slot-resolution eligibility check.

    *params* is the proc's parameter list; eligible parameters get
    the lowest-numbered slots so the prologue's param-binding
    routes through the indexed setter.  Locals introduced by the
    body claim slots in registration order.

    Returns an empty dict when the proc isn't slot-resolvable at
    all (dynamic eval, has_fallback, etc.).  Caller folds the
    result into the summary via ``with_local_slots``.
    """
    if not _proc_is_slot_eligible(summary):
        return {}

    # First pass: identify any per-name disqualifications and detect
    # whole-proc disables.  We collect the set of literal local names
    # that appear in eligible writes / reads as we go.
    ineligible: set[str] = set()
    ordered_names: list[str] = []
    seen: set[str] = set()
    # Reserve slots for parameters first — codegen's prologue binds
    # them via ``tcl_local_set`` per Tcl semantics, and we want
    # ``info args`` consumers to see them in declaration order.
    for pname in params:
        if pname and pname not in seen:
            ordered_names.append(pname)
            seen.add(pname)

    for stmt in _walk_statements(script):
        if _stmt_disables_slots(stmt, ineligible):
            return {}
        # Record assign target names in source order so the slot
        # assignment is deterministic.
        target = getattr(stmt, "name", None)
        if isinstance(target, str) and target and target not in seen:
            # Skip names that include array-element notation —
            # those route through the array directory regardless.
            if "(" not in target and not target.startswith("::"):
                ordered_names.append(target)
                seen.add(target)

    # Filter out names that any ineligibility hit caught.
    eligible_names = [n for n in ordered_names if n not in ineligible]
    # Cap at LOCALS_ARRAY_CAP — names beyond that fall back to the
    # hash store automatically (the codegen consumes this dict and
    # routes any name without an entry through the legacy path).
    return {name: idx for idx, name in enumerate(eligible_names[:LOCALS_ARRAY_CAP])}


def populate_local_slots(
    summaries: dict[str, ProcEscapeSummary],
    ir_module: "IRModule | None",  # noqa: F821 — forward import below
) -> dict[str, ProcEscapeSummary]:
    """Walk every proc in *ir_module*, run :func:`assign_local_slots`,
    and return a new ``summaries`` dict with the slot mapping folded
    into each summary via :func:`ProcEscapeSummary.with_local_slots`.

    Pure transformation — never mutates the input dict.  Procs that
    aren't slot-eligible round-trip unchanged.
    """
    if ir_module is None:
        return summaries
    out: dict[str, ProcEscapeSummary] = {}
    for qname, summary in summaries.items():
        proc = ir_module.procedures.get(qname)
        if proc is None:
            out[qname] = summary
            continue
        slots = assign_local_slots(proc.body, summary, params=tuple(proc.params or ()))
        out[qname] = summary.with_local_slots(slots) if slots else summary
    return out
