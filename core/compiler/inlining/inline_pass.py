"""S4.2 — IR-level inlining.

The catalogue stage (:mod:`core.compiler.inlining.decision`) tags
each :class:`~core.compiler.ir.IRProcedure` with an
:class:`~core.compiler.ir.InlineDecision`.  This module consumes
those tags and rewrites the IR module so the marked calls
disappear before codegen.

The pass handles four progressively-larger shapes of statement-
position :class:`~core.compiler.ir.IRCall`:

**v0 — empty-body splice.**  Proc with zero body statements; the
call vanishes entirely.

**v1 — single-call wrapper splice.**  Zero-parameter proc whose
body is exactly one splice-safe :class:`IRCall` with no ``defs``.

**v2 — multi-statement wrapper splice.**  Zero-parameter proc
whose body is a sequence of splice-safe :class:`IRCall`s.  Each
body statement propagates verbatim to the call site.

**v3 — parameterised inlining.**  Procs with parameters and / or
local writes.  The pass:

1. Generates a fresh mangled name for every parameter and every
   variable written in the body (``__inline_<id>__<name>``).
2. Rewrites the entire body — including value strings, expression
   ASTs, ``defs`` / ``reads``, foreach iterator vars, catch /
   try exception bindings — through the rename map.
3. Prepends an ``IRAssignValue`` for each parameter that binds
   the call-site arg string to the mangled parameter name.
4. Drops a trailing :class:`IRReturn` (statement-position calls
   discard the return value).

v3 declines on procs with non-trailing :class:`IRReturn`,
variadic ``args`` parameters, parameter defaults consumed by
the caller (arity mismatch), array-element writes, or any of
the existing splice-safety gates (frame-observing commands,
``[cmd]`` substitutions in args).  Those keep the proc-call
boundary; future revisions can lift the restriction once the
IR grows label / break primitives for early-return handling.

After inlining, procs whose ``static_call_count`` reached zero
through every reachable call being spliced are dropped from
``module.procedures``.  The module-init epilogue's
``proc_register_compiled`` calls likewise vanish, shrinking
the WASM binary.

The pass returns a new :class:`~core.compiler.ir.IRModule`; the
input module is left untouched (every IR node is frozen, so
rebuilding via :func:`dataclasses.replace` is the only option).
"""

from __future__ import annotations

import re
from dataclasses import dataclass, replace
from enum import Enum

from ..expr_ast import ExprLiteral, vars_in_expr_node
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBlock,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRIncr,
    IRModule,
    IRProcedure,
    IRReturn,
    IRScript,
    IRSwitch,
    IRSwitchArm,
    IRTry,
    IRTryHandler,
    IRUpFrame,
    IRWhile,
    InlineDecision,
)
from ..var_escape import ProcEscapeSummary
from ..var_escape._interprocedural import _resolve_callee
from ._rename import rewrite_script as _rewrite_with_rename


class _InlineKind(Enum):
    """Internal kind tag for the per-callee inline plan."""

    EMPTY = "empty"  # v0: drop the call
    VERBATIM = "verbatim"  # v1 / v2: splice body unchanged
    PARAMETERISED = "parameterised"  # v3: bind args, α-rename, splice


@dataclass(frozen=True, slots=True)
class _InlineSpec:
    """Per-callee inlining plan.

    For ``EMPTY`` and ``VERBATIM`` modes ``body`` carries the
    pre-computed splice content.  For ``PARAMETERISED`` mode the
    rewrite happens at each call site (since args differ per call)
    using ``proc`` as the source of params and the rename target.
    """

    kind: _InlineKind
    body: tuple = ()
    proc: IRProcedure | None = None


def inline_module(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
) -> IRModule:
    """Return a new module with eligible calls inlined.

    The ``module`` must already carry the S4.1 catalogue (run
    :func:`core.compiler.inlining.decision.apply_inline_catalogue`
    first).  ``summaries`` is the interprocedural var-escape result
    used to resolve unqualified call words to their canonical
    qualified target.

    The pass is idempotent: re-running on a module that has no
    remaining inlinable sites returns an equivalent module.
    Procedures whose static-call-count drops to zero post-inlining
    are NOT yet pruned — keeping them around is harmless and lets
    the catalogue tag survive across re-runs.  A future revision
    can add dead-proc DCE once the inliner handles the wider set
    of cases that make it material.
    """

    inlinable = _build_inlinable_map(module, summaries)
    if not inlinable:
        return module

    # Mutable counter threaded through the recursion so each
    # parameterised inline generates a unique mangled-name suffix.
    counter = [0]

    # Top-level and proc-body walks both pass ``parent_is_terminal=True``
    # so the LAST statement of each body is recognised as terminal
    # — that's where keeping a callee's trailing IRReturn is sound.
    new_top = _rewrite_script(
        module.top_level, "::", inlinable, summaries, counter,
        parent_is_terminal=True,
    )
    new_procs: dict[str, IRProcedure] = {}
    for qname, proc in module.procedures.items():
        new_body = _rewrite_script(
            proc.body, qname, inlinable, summaries, counter,
            parent_is_terminal=True,
        )
        if new_body is proc.body:
            new_procs[qname] = proc
        else:
            new_procs[qname] = replace(proc, body=new_body)
    rebuilt = replace(module, top_level=new_top, procedures=new_procs)

    # S4.2 dead-proc elimination: drop procs whose only static
    # references were spliced away.  We restrict the candidate
    # set to procs the catalogue marked inlinable — those are
    # already proved pure_leaf and were therefore guaranteed
    # safe to inline.  Truly opaque procs that might be reached
    # through dynamic dispatch (eval / unknown) stay regardless
    # of static call count.
    return _drop_dead_procs(rebuilt, summaries, frozenset(inlinable.keys()))


def _build_inlinable_map(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
) -> dict[str, _InlineSpec]:
    """Return a map from inlinable qname → :class:`_InlineSpec`.

    ``IF_SINGLE_CALL`` is **not** eligible because its profitability
    depends on the post-inline proc-pruning interaction — inlining
    without pruning would leave a redundant copy of the body in
    the module.  Only ``ALWAYS``-tagged procs qualify.

    The catalogue runs three checks per proc, picking the most
    permissive shape that fits:

    * v0 (empty body) — the call vanishes.
    * v1/v2 (verbatim) — zero params, every body statement
      splice-safe, body propagates unchanged.
    * v3 (parameterised) — params + body locals OK, gated by
      :func:`_v3_eligible` which screens for non-trailing
      ``IRReturn`` / variadic ``args`` / array writes / nested
      control-flow not yet supported.
    """

    eligible: dict[str, _InlineSpec] = {}
    for qname, proc in module.procedures.items():
        if proc.inline_decision is not InlineDecision.ALWAYS:
            continue

        # v0 — empty body
        if not proc.body.statements:
            eligible[qname] = _InlineSpec(kind=_InlineKind.EMPTY)
            continue

        # v1 / v2 — zero-param wrapper around N pure-side-effect
        # IRCalls.  Every body statement must be splice-eligible.
        # Multi-statement bodies propagate every body statement
        # verbatim to the call site.
        if not proc.params:
            body_stmts = proc.body.statements
            verbatim_safe = all(
                _stmt_is_splice_eligible(s, qname, summaries) for s in body_stmts
            )
            if verbatim_safe:
                eligible[qname] = _InlineSpec(
                    kind=_InlineKind.VERBATIM,
                    body=tuple(body_stmts),
                )
                continue
            # Zero params but verbatim path declines — fall through
            # to v3 in case rename + binding lifts a constraint.

        # v3 — parameterised inline with α-renaming.  Generates a
        # rebuilt body per call site, so eligibility just records
        # the proc reference and lets the call-site rewriter
        # produce the spliced statements.
        if _v3_eligible(proc, qname, summaries):
            eligible[qname] = _InlineSpec(
                kind=_InlineKind.PARAMETERISED,
                proc=proc,
            )
            continue

    return eligible


def _v3_eligible(
    proc: IRProcedure,
    qname: str,
    summaries: dict[str, ProcEscapeSummary],
) -> bool:
    """Return True iff ``proc`` qualifies for parameterised inlining.

    Accepts:

    * **Variadic ``args``** — packed into a list literal at the
      call site.
    * **Parameter defaults** — parsed from ``params_raw``.
    * **Nested control flow** — every IR shape walked by the
      α-rename rewriter.
    * **Array element writes** (``arr(idx)``) — the array base
      name is renamed consistently across writes and
      ``$arr(idx)`` reads.
    * **Trailing :class:`IRReturn`** at body top level — kept
      verbatim by the splice when the call site is in terminal
      position; otherwise the for/break wrap below activates.
    * **Non-trailing :class:`IRReturn`** — translated to
      ``set __result <value>; break`` and the body wrapped in a
      ``for {} {1} {} { … }`` loop, EXCEPT when an IRReturn
      sits inside a context whose own ``break`` semantics would
      trap our break: loop bodies (``IRFor`` / ``IRWhile`` /
      ``IRForeach``), exception traps (``IRCatch`` / ``IRTry``),
      and frame-shifted blocks (``IRUpFrame``).  IRReturn inside
      transparent groupings (``IRBlock`` / ``IRIf`` / ``IRSwitch``
      arms) IS handled because their ``break`` propagates to the
      enclosing scope — our wrapper.

    Still declines:

    * ``IRReturn`` inside loop / catch / try / upframe bodies.
    """
    if _has_irreturn_in_unsafe_scope(proc.body, inside_unsafe=False):
        return False
    body_stmts = proc.body.statements
    for stmt in body_stmts:
        if isinstance(stmt, IRReturn):
            continue  # IRReturn at top level is fine — handled by wrap or kept
        if not _v3_stmt_eligible(stmt, qname, summaries):
            return False
    return True


def _has_irreturn_in_unsafe_scope(
    script: IRScript | None,
    *,
    inside_unsafe: bool,
) -> bool:
    """Walk ``script`` looking for any :class:`IRReturn` inside a
    construct whose ``break`` semantics would trap our wrapper's
    break.

    Loop bodies (``IRFor`` / ``IRWhile`` / ``IRForeach``) catch
    ``break``; exception traps (``IRCatch`` / ``IRTry``) catch
    return-codes including break; ``IRUpFrame`` frame-shifts
    around its body so ``break`` inside it is unclear.  All of
    these flag ``inside_unsafe=True`` for their bodies.

    Transparent groupings (``IRBlock``, ``IRIf`` clauses,
    ``IRSwitch`` arms) propagate the parent's ``inside_unsafe``
    flag — ``break`` inside them goes through to whatever scope
    encloses them.
    """
    if script is None:
        return False
    for stmt in script.statements:
        if isinstance(stmt, IRReturn):
            if inside_unsafe:
                return True
            continue
        if isinstance(stmt, IRFor):
            if _has_irreturn_in_unsafe_scope(stmt.init, inside_unsafe=inside_unsafe):
                return True
            if _has_irreturn_in_unsafe_scope(stmt.next, inside_unsafe=inside_unsafe):
                return True
            if _has_irreturn_in_unsafe_scope(stmt.body, inside_unsafe=True):
                return True
        elif isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
            if _has_irreturn_in_unsafe_scope(stmt.body, inside_unsafe=True):
                return True
        elif isinstance(stmt, IRTry):
            if _has_irreturn_in_unsafe_scope(stmt.body, inside_unsafe=True):
                return True
            for h in stmt.handlers:
                if _has_irreturn_in_unsafe_scope(h.body, inside_unsafe=True):
                    return True
            if _has_irreturn_in_unsafe_scope(stmt.finally_body, inside_unsafe=True):
                return True
        elif isinstance(stmt, IRBlock):
            if _has_irreturn_in_unsafe_scope(stmt.body, inside_unsafe=inside_unsafe):
                return True
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                if _has_irreturn_in_unsafe_scope(
                    clause.body, inside_unsafe=inside_unsafe
                ):
                    return True
            if _has_irreturn_in_unsafe_scope(
                stmt.else_body, inside_unsafe=inside_unsafe
            ):
                return True
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if _has_irreturn_in_unsafe_scope(
                    arm.body, inside_unsafe=inside_unsafe
                ):
                    return True
            if _has_irreturn_in_unsafe_scope(
                stmt.default_body, inside_unsafe=inside_unsafe
            ):
                return True
    return False


def _v3_stmt_eligible(
    stmt: object,
    callee_qname: str,
    summaries: dict[str, ProcEscapeSummary],
) -> bool:
    """Per-statement eligibility for v3 inlining.

    Accepts pure value computation (IRAssignConst / IRAssignValue
    / IRAssignExpr / IRIncr / IRExprEval), splice-safe
    :class:`IRCall`s, and nested control flow whose bodies
    contain only the same allowed shapes (recursively).
    Rejects any :class:`IRReturn` at non-trailing positions.
    """
    if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
        # Array writes (``arr(idx)``) qualify: the α-renamer
        # extracts the array base name (``arr``) and applies the
        # rename consistently across writes and ``$arr(idx)``
        # reads, so the whole-array binding stays coherent.
        # ``::``-qualified names (globals) still decline because
        # the rename map only covers locals.
        name = getattr(stmt, "name", "")
        if "::" in name:
            return False
        return True
    if isinstance(stmt, IRCall):
        return _stmt_is_splice_eligible(stmt, callee_qname, summaries)
    if isinstance(stmt, IRExprEval):
        return True
    if isinstance(stmt, IRBlock):
        return _v3_script_eligible(stmt.body, callee_qname, summaries)
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            if not _v3_script_eligible(clause.body, callee_qname, summaries):
                return False
        if stmt.else_body is not None:
            if not _v3_script_eligible(stmt.else_body, callee_qname, summaries):
                return False
        return True
    if isinstance(stmt, IRFor):
        return (
            _v3_script_eligible(stmt.init, callee_qname, summaries)
            and _v3_script_eligible(stmt.next, callee_qname, summaries)
            and _v3_script_eligible(stmt.body, callee_qname, summaries)
        )
    if isinstance(stmt, IRWhile):
        return _v3_script_eligible(stmt.body, callee_qname, summaries)
    if isinstance(stmt, IRForeach):
        return _v3_script_eligible(stmt.body, callee_qname, summaries)
    if isinstance(stmt, IRCatch):
        return _v3_script_eligible(stmt.body, callee_qname, summaries)
    if isinstance(stmt, IRTry):
        if not _v3_script_eligible(stmt.body, callee_qname, summaries):
            return False
        for handler in stmt.handlers:
            if not _v3_script_eligible(handler.body, callee_qname, summaries):
                return False
        if stmt.finally_body is not None:
            if not _v3_script_eligible(stmt.finally_body, callee_qname, summaries):
                return False
        return True
    if isinstance(stmt, IRSwitch):
        for arm in stmt.arms:
            if arm.body is not None:
                if not _v3_script_eligible(arm.body, callee_qname, summaries):
                    return False
        if stmt.default_body is not None:
            if not _v3_script_eligible(stmt.default_body, callee_qname, summaries):
                return False
        return True
    if isinstance(stmt, IRUpFrame):
        return _v3_script_eligible(stmt.body, callee_qname, summaries)
    return False


def _v3_script_eligible(
    script: IRScript | None,
    callee_qname: str,
    summaries: dict[str, ProcEscapeSummary],
) -> bool:
    """Recurse into a nested script and check per-statement
    eligibility.  IRReturn inside a nested script IS allowed
    (the for/break wrap routes it correctly) — the unsafe-scope
    check at the body root has already verified no IRReturn
    sits inside a loop / catch / try / upframe.
    """
    if script is None:
        return True
    for stmt in script.statements:
        if isinstance(stmt, IRReturn):
            # IRReturn inside a nested transparent grouping is
            # accepted; the for/break wrap at inline time
            # rewrites it.  Soundness depends on
            # ``_has_irreturn_in_unsafe_scope`` having vetted
            # the surrounding context.
            continue
        if not _v3_stmt_eligible(stmt, callee_qname, summaries):
            return False
    return True


def _stmt_is_splice_eligible(
    stmt: object,
    callee_qname: str,
    summaries: dict[str, ProcEscapeSummary],
) -> bool:
    """Return True iff ``stmt`` can be lifted out of the wrapper and
    spliced into a caller's body without changing observable
    behaviour.

    Only :class:`IRCall` qualifies today.  The call must have no
    ``defs`` (would mutate the caller's scope), resolve to a
    namespace-invariant command (``::``-qualified or unresolved
    runtime builtin), be in the splice-safe allow-list
    (frame-independent commands only — no ``info ...``,
    ``uplevel``, ``upvar``, no ``return`` / ``break`` /
    ``continue``), and carry no arg containing a ``[cmd]``
    substitution.
    """
    if not isinstance(stmt, IRCall):
        return False
    if stmt.defs:
        return False
    if not _command_is_namespace_invariant(stmt.command, callee_qname, summaries):
        return False
    if not _command_is_splice_safe(stmt.command):
        return False
    if any(_arg_has_command_subst(a) for a in stmt.args):
        return False
    return True


# Commands whose semantics are independent of the calling frame.
# A wrapped IRCall whose command is in this set can be spliced into
# any caller's frame without changing its observable behaviour.
# Frame-observing commands (``info ...``, ``uplevel``, ``upvar``)
# and frame-affecting control flow (``return``, ``break``,
# ``continue``) are deliberately omitted: a wrapper around any of
# them must keep the proc-call boundary so the command sees the
# right frame / terminates the right scope.
_SPLICE_SAFE_COMMANDS: frozenset[str] = frozenset(
    {
        # List primitives — pure value computation.
        "list",
        "lindex",
        "lrange",
        "linsert",
        "llength",
        "lsort",
        "lsearch",
        "lreverse",
        "lreplace",
        "lrepeat",
        "concat",
        # String primitives — pure value computation.
        "split",
        "join",
        "string",
        # Arithmetic — pure value computation.
        "expr",
        # I/O — observable side effect, but doesn't depend on the
        # caller's frame structure.
        "puts",
    }
)


def _command_is_splice_safe(command: str) -> bool:
    """True iff ``command`` is safe to splice across a proc boundary.

    Strips ``::`` qualification before checking the allow-list so
    both bare and qualified forms (``puts``, ``::puts``) match.
    """
    bare = command[2:] if command.startswith("::") else command
    return bare in _SPLICE_SAFE_COMMANDS


def _arg_has_command_subst(arg: str) -> bool:
    """True iff ``arg`` contains a ``[cmd …]`` command substitution.

    A bare ``[`` opens a command substitution in Tcl; a literal
    bracket would be backslash-escaped or quoted.  We use the
    coarse "any ``[`` outside braces" approximation — false
    positives just decline a hoist, never silently inline an
    unsafe one.
    """
    depth = 0
    i = 0
    while i < len(arg):
        c = arg[i]
        if c == "\\" and i + 1 < len(arg):
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            if depth > 0:
                depth -= 1
        elif c == "[" and depth == 0:
            return True
        i += 1
    return False


def _command_is_namespace_invariant(
    command: str,
    callee_qname: str,
    summaries: dict[str, ProcEscapeSummary],
) -> bool:
    """Return ``True`` when ``command`` resolves the same way from any
    caller's namespace as it does from the callee's.

    Two cases qualify:

    1. ``command`` is fully qualified (starts with ``::``).  The
       resolution is namespace-independent by construction.
    2. ``command`` is unqualified and does not resolve to any
       tracked proc from the callee's namespace.  Since the
       interprocedural pass already walked the namespace chain,
       a non-resolution means the word is a runtime builtin
       (``puts``, ``incr``, …) which lives in the global namespace
       and is reachable from every caller.

    A bare unqualified command that DOES resolve to a tracked
    proc fails the test — splicing it into a different namespace
    could re-bind the call.
    """
    if command.startswith("::"):
        return True
    return _resolve_callee(command, callee_qname, summaries) is None


def _rewrite_script(
    script: IRScript,
    caller_qname: str,
    inlinable: dict[str, _InlineSpec],
    summaries: dict[str, ProcEscapeSummary],
    counter: list[int],
    parent_is_terminal: bool = False,
) -> IRScript:
    """Return ``script`` with eligible top-level calls dropped.

    ``parent_is_terminal`` is True only when this script itself
    sits in terminal position of its own enclosing structure (a
    proc body, the top-level script, or recursively the trailing
    branch of a script in a caller already known to be terminal).
    The flag propagates to the LAST statement of this script so
    a v3 inline of a proc with a trailing IRReturn can keep the
    return intact when (and only when) its execution would
    naturally have produced the proc's return value.

    Recurses into every nested control-flow body so calls inside
    ``if`` / ``for`` / ``foreach`` / ``while`` / ``catch`` / ``try``
    / ``switch`` arms are also subject to the splice.  Returns the
    original instance unchanged when no rewrites are needed.
    """

    new_stmts: list = []
    changed = False
    n = len(script.statements)
    for i, stmt in enumerate(script.statements):
        is_last = (i == n - 1)
        replacements = _rewrite_stmt(
            stmt,
            caller_qname,
            inlinable,
            summaries,
            counter,
            is_terminal=parent_is_terminal and is_last,
        )
        if replacements is None:
            new_stmts.append(stmt)
            continue
        changed = True
        new_stmts.extend(replacements)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _rewrite_stmt(
    stmt: object,
    caller_qname: str,
    inlinable: dict[str, _InlineSpec],
    summaries: dict[str, ProcEscapeSummary],
    counter: list[int],
    is_terminal: bool = False,
) -> list | None:
    """Return a replacement statement list, or ``None`` to keep ``stmt``.

    A return value of ``[]`` drops the statement entirely (v0
    empty-body case); a non-empty list substitutes the inlined
    body for the call site (v1 / v2 / v3 cases).
    """

    if isinstance(stmt, IRCall):
        target = _resolve_callee(stmt.command, caller_qname, summaries)
        if target is not None and target in inlinable:
            spec = inlinable[target]
            return _splice_call_site(stmt, spec, counter, is_terminal)
        return None

    if isinstance(stmt, IRBlock):
        new_body = _rewrite_script(
            stmt.body, caller_qname, inlinable, summaries, counter
        )
        if new_body is stmt.body:
            return None
        return [replace(stmt, body=new_body)]

    if isinstance(stmt, IRIf):
        new_clauses, clauses_changed = _rewrite_if_clauses(
            stmt.clauses, caller_qname, inlinable, summaries, counter
        )
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _rewrite_script(
                stmt.else_body, caller_qname, inlinable, summaries, counter
            )
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return None
        return [replace(stmt, clauses=new_clauses, else_body=new_else)]

    if isinstance(stmt, IRFor):
        new_init = _rewrite_script(
            stmt.init, caller_qname, inlinable, summaries, counter
        )
        new_next = _rewrite_script(
            stmt.next, caller_qname, inlinable, summaries, counter
        )
        new_body = _rewrite_script(
            stmt.body, caller_qname, inlinable, summaries, counter
        )
        if (
            new_init is stmt.init
            and new_next is stmt.next
            and new_body is stmt.body
        ):
            return None
        return [replace(stmt, init=new_init, next=new_next, body=new_body)]

    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
        new_body = _rewrite_script(
            stmt.body, caller_qname, inlinable, summaries, counter
        )
        if new_body is stmt.body:
            return None
        return [replace(stmt, body=new_body)]

    if isinstance(stmt, IRTry):
        new_body = _rewrite_script(
            stmt.body, caller_qname, inlinable, summaries, counter
        )
        new_handlers, handlers_changed = _rewrite_try_handlers(
            stmt.handlers, caller_qname, inlinable, summaries, counter
        )
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = _rewrite_script(
                stmt.finally_body, caller_qname, inlinable, summaries, counter
            )
            finally_changed = new_finally is not stmt.finally_body
        if (
            new_body is stmt.body
            and not handlers_changed
            and not finally_changed
        ):
            return None
        return [
            replace(
                stmt,
                body=new_body,
                handlers=new_handlers,
                finally_body=new_finally,
            )
        ]

    if isinstance(stmt, IRSwitch):
        new_arms, arms_changed = _rewrite_switch_arms(
            stmt.arms, caller_qname, inlinable, summaries, counter
        )
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _rewrite_script(
                stmt.default_body, caller_qname, inlinable, summaries, counter
            )
            default_changed = new_default is not stmt.default_body
        if not arms_changed and not default_changed:
            return None
        return [replace(stmt, arms=new_arms, default_body=new_default)]

    return None


def _splice_call_site(
    call: IRCall,
    spec: _InlineSpec,
    counter: list[int],
    is_terminal: bool,
) -> list | None:
    """Produce the inlined statement list for a single call site.

    ``is_terminal`` is True when the call appears at the very last
    position of its enclosing script AND the chain of enclosing
    statements is itself terminal — i.e., the call's value would
    naturally become the proc's return value.  Used by v3 to
    decide whether a callee's trailing ``IRReturn`` may be kept
    intact.
    """
    kind = spec.kind
    if kind is _InlineKind.EMPTY:
        if call.args:
            return None
        return []
    if kind is _InlineKind.VERBATIM:
        if call.args:
            return None
        out: list = []
        for inner in spec.body:
            if isinstance(inner, IRCall):
                # Re-attribute the splice to the call site for
                # diagnostics.
                out.append(replace(inner, range=call.range))
            else:
                out.append(inner)
        return out
    if kind is _InlineKind.PARAMETERISED:
        proc = spec.proc
        if proc is None:
            return None

        # Bind call-site arg strings to mangled parameter slots,
        # accounting for parameter defaults (missing trailing
        # args filled from ``params_raw``) and variadic ``args``
        # (extra trailing call args packed into a list literal).
        bindings_or_decline = _build_param_bindings(call, proc, counter)
        if bindings_or_decline is None:
            return None
        cid, rename, bindings = bindings_or_decline

        # Mangle every locally-written variable too — without this
        # the body's own ``set y …`` would resolve to the caller's
        # ``y`` slot.
        for name in _collect_local_names(proc.body):
            if name not in rename:
                rename[name] = f"__inline_{cid}__{name}"

        # Determine the IRReturn-handling strategy.
        body_stmts = list(proc.body.statements)
        body_has_trailing_return = (
            body_stmts and isinstance(body_stmts[-1], IRReturn)
        )
        body_has_non_trailing_return = any(
            isinstance(s, IRReturn) for s in body_stmts[:-1]
        ) or _has_nested_irreturn(proc.body)

        renamed_body = _rewrite_with_rename(proc.body, rename)

        if body_has_non_trailing_return:
            # Non-trailing IRReturn — wrap the inlined body in a
            # ``for {} {1} {} { ... }`` loop and rewrite each
            # IRReturn to ``set __result <value>; break``.  The
            # final statement of the inlined body gets an
            # explicit ``break`` so paths without IRReturn also
            # exit the wrapper.  After the loop, the result is
            # in ``__result``; if the call is terminal we emit a
            # caller-level ``return $__result`` to forward it.
            result_var = f"__inline_{cid}__RESULT"
            wrapped = _wrap_with_irreturn_loop(
                renamed_body,
                result_var=result_var,
                call=call,
            )
            if is_terminal:
                wrapped.append(
                    IRReturn(
                        range=call.range,
                        value="$" + result_var,
                    )
                )
            return bindings + wrapped

        if body_has_trailing_return and not is_terminal:
            # Trailing IRReturn but call site isn't terminal —
            # keeping the return would short-circuit the rest of
            # the caller's body.  Same problem the wrap path
            # solves; reuse the wrap.
            result_var = f"__inline_{cid}__RESULT"
            wrapped = _wrap_with_irreturn_loop(
                renamed_body,
                result_var=result_var,
                call=call,
            )
            return bindings + wrapped

        # No IRReturn, or trailing IRReturn at a terminal call
        # site.  Flat splice — leaves the trailing IRReturn (if
        # any) intact so it exits the caller proc with the
        # renamed value.
        return bindings + list(renamed_body.statements)
    return None


def _has_nested_irreturn(script: IRScript | None) -> bool:
    """Return True if any IRReturn appears inside a nested
    control-flow body of ``script`` (i.e., not at ``script``'s
    own top level)."""
    if script is None:
        return False
    for stmt in script.statements:
        # Skip top-level IRReturn — caller examines that
        # separately.
        if isinstance(stmt, IRReturn):
            continue
        if _scan_for_irreturn(stmt):
            return True
    return False


def _scan_for_irreturn(stmt: object) -> bool:
    """Recursively scan ``stmt``'s sub-bodies for any IRReturn."""
    if isinstance(stmt, IRReturn):
        return True
    if isinstance(stmt, IRBlock):
        return _has_any_irreturn(stmt.body)
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            if _has_any_irreturn(clause.body):
                return True
        return _has_any_irreturn(stmt.else_body)
    if isinstance(stmt, IRFor):
        return (
            _has_any_irreturn(stmt.init)
            or _has_any_irreturn(stmt.next)
            or _has_any_irreturn(stmt.body)
        )
    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
        return _has_any_irreturn(stmt.body)
    if isinstance(stmt, IRTry):
        if _has_any_irreturn(stmt.body):
            return True
        for h in stmt.handlers:
            if _has_any_irreturn(h.body):
                return True
        return _has_any_irreturn(stmt.finally_body)
    if isinstance(stmt, IRSwitch):
        for arm in stmt.arms:
            if _has_any_irreturn(arm.body):
                return True
        return _has_any_irreturn(stmt.default_body)
    return False


def _has_any_irreturn(script: IRScript | None) -> bool:
    if script is None:
        return False
    for stmt in script.statements:
        if isinstance(stmt, IRReturn):
            return True
        if _scan_for_irreturn(stmt):
            return True
    return False


def _wrap_with_irreturn_loop(
    renamed_body: IRScript,
    *,
    result_var: str,
    call: IRCall,
) -> list:
    """Wrap ``renamed_body`` in a one-shot ``for`` loop that
    routes IRReturn-as-break.

    Generates::

        set __result ""
        for {} {1} {} {
            <body with each IRReturn → set __result <v>; break>
            break  # explicit fall-through exit
        }

    Returns the synthesised statement list — the caller appends
    a ``return $__result`` after the loop iff the call site is
    terminal.
    """
    # Step 1: rewrite IRReturn → set result_var <value>; break.
    rewritten = _substitute_irreturn(renamed_body, result_var, call.range)

    # Step 2: append a final break to ensure the loop exits even
    # when no IRReturn fires.
    body_with_break = list(rewritten.statements) + [
        IRCall(
            range=call.range,
            command="break",
            args=(),
        )
    ]

    # Step 3: build the wrap.  Init the result var to empty, then
    # loop with a constant-true condition.  We use ``IRWhile``
    # rather than ``IRFor`` because ``IRFor`` with empty init /
    # next emits ``<empty_clause>`` placeholder IRCalls that the
    # WASM codegen routes through the eval fallback (and traps
    # because ``<empty_clause>`` isn't a real Tcl command).
    # ``IRWhile`` has no init / next clauses to worry about.
    out: list = []
    out.append(
        IRAssignConst(
            range=call.range,
            name=result_var,
            value="",
        )
    )
    out.append(
        IRWhile(
            range=call.range,
            condition=ExprLiteral(text="1", start=0, end=1),
            condition_range=call.range,
            body=IRScript(statements=tuple(body_with_break)),
            body_range=call.range,
        )
    )
    return out


def _substitute_irreturn(
    script: IRScript,
    result_var: str,
    range_,
) -> IRScript:
    """Replace each :class:`IRReturn` reachable from ``script``
    with the two-statement sequence ``set <result_var> <value>;
    break``.  Recurses into transparent nested bodies (IRBlock,
    IRIf, IRSwitch); does NOT recurse into loop / catch / try /
    upframe bodies because the eligibility gate guarantees no
    IRReturn is nested inside them.
    """
    if script is None:
        return script
    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        if isinstance(stmt, IRReturn):
            value = stmt.value if stmt.value is not None else ""
            new_stmts.append(
                IRAssignValue(
                    range=stmt.range,
                    name=result_var,
                    value=value,
                )
            )
            new_stmts.append(
                IRCall(
                    range=stmt.range,
                    command="break",
                    args=(),
                )
            )
            changed = True
            continue
        substituted = _substitute_irreturn_stmt(stmt, result_var, range_)
        if substituted is stmt:
            new_stmts.append(stmt)
        else:
            new_stmts.append(substituted)
            changed = True
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _substitute_irreturn_stmt(stmt, result_var: str, range_):
    """Recurse into transparent-grouping containers and substitute
    their bodies' IRReturn nodes.  Does NOT touch loop / catch /
    try / upframe bodies — break inside those wouldn't reach our
    wrapper, and the eligibility gate has already verified no
    IRReturn sits there."""
    if isinstance(stmt, IRBlock):
        new_body = _substitute_irreturn(stmt.body, result_var, range_)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRIf):
        new_clauses = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_body = _substitute_irreturn(clause.body, result_var, range_)
            if new_body is clause.body:
                new_clauses.append(clause)
            else:
                new_clauses.append(replace(clause, body=new_body))
                clauses_changed = True
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _substitute_irreturn(stmt.else_body, result_var, range_)
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return stmt
        return replace(stmt, clauses=tuple(new_clauses), else_body=new_else)
    if isinstance(stmt, IRSwitch):
        new_arms = []
        arms_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            new_body = _substitute_irreturn(arm.body, result_var, range_)
            if new_body is arm.body:
                new_arms.append(arm)
            else:
                new_arms.append(replace(arm, body=new_body))
                arms_changed = True
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _substitute_irreturn(stmt.default_body, result_var, range_)
            default_changed = new_default is not stmt.default_body
        if not arms_changed and not default_changed:
            return stmt
        return replace(stmt, arms=tuple(new_arms), default_body=new_default)
    return stmt


def _build_param_bindings(
    call: IRCall,
    proc: IRProcedure,
    counter: list[int],
) -> tuple[int, dict[str, str], list] | None:
    """Build the parameter-binding statement list for a v3 call.

    Returns ``(cid, rename, bindings)`` on success, or ``None`` to
    decline (arity mismatch with no defaults, or other case the
    binder doesn't yet handle).

    Behaviour:

    * **Variadic ``args``** — the trailing parameter named
      ``args`` is bound to a list literal containing the surplus
      call words.  Zero surplus words bind it to ``""``.
    * **Defaults** — when the call passes fewer args than the
      proc has positional parameters, missing slots are filled
      from ``params_raw``-parsed defaults.  A missing slot
      without a default declines.
    """
    counter[0] += 1
    cid = counter[0]
    rename: dict[str, str] = {}
    for p in proc.params:
        rename[p] = f"__inline_{cid}__{p}"

    params = proc.params
    args = call.args
    has_variadic = bool(params) and params[-1] == "args"

    # Resolve each parameter's bound value.  ``bound[i]`` is the
    # source string for the IRAssignValue going into
    # ``__inline_<cid>__<params[i]>``.
    bound: list[str] = []
    if has_variadic:
        positional_count = len(params) - 1
        if len(args) < positional_count:
            # Variadic doesn't help when caller is short on
            # required positionals — fall through to default
            # handling below as if the proc had ``positional_count``
            # required params.
            return _build_with_defaults(
                cid, rename, proc, args, positional_count, has_variadic
            )
        # First ``positional_count`` args bind to required params.
        bound.extend(args[:positional_count])
        # Remaining args pack into the ``args`` list literal.
        # ``list a b c`` is a clean Tcl idiom that produces a
        # well-formed list regardless of element content.
        extras = args[positional_count:]
        if not extras:
            bound.append("")
        else:
            # Build ``[list <e1> <e2> …]``.  Each extra is
            # already a call-site arg string (possibly with $-
            # subst, [cmd] subst, or literal); ``list`` is
            # responsible for quoting at runtime.
            list_call = "[list " + " ".join(extras) + "]"
            bound.append(list_call)
    else:
        if len(args) > len(params):
            # Too many args — Tcl error normally; we decline so
            # the runtime surfaces the error from the call site.
            return None
        if len(args) < len(params):
            return _build_with_defaults(
                cid, rename, proc, args, len(params), False
            )
        bound.extend(args)

    bindings: list = [
        IRAssignValue(
            range=call.range,
            name=rename[p],
            value=v,
        )
        for p, v in zip(params, bound)
    ]
    return cid, rename, bindings


def _build_with_defaults(
    cid: int,
    rename: dict[str, str],
    proc: IRProcedure,
    args: tuple[str, ...],
    positional_count: int,
    has_variadic: bool,
) -> tuple[int, dict[str, str], list] | None:
    """Try to fill missing positional args from declared defaults.

    Returns ``None`` if any missing slot lacks a default.  When
    ``has_variadic`` is True, the final ``args`` parameter is
    always bound to ``""`` (no trailing call words to pack).
    """
    if not proc.params_raw:
        return None
    from ..lowering import _parse_params_with_defaults

    parsed = _parse_params_with_defaults(proc.params_raw)
    if len(parsed) != len(proc.params):
        return None  # parse drift — bail safely

    bound: list[str] = []
    for i in range(positional_count):
        if i < len(args):
            bound.append(args[i])
            continue
        # Missing slot — need a default.
        _, default = parsed[i]
        if default is None:
            return None
        bound.append(default)

    if has_variadic:
        bound.append("")  # empty trailing args

    bindings: list = [
        IRAssignValue(
            range=proc.range,
            name=rename[p],
            value=v,
        )
        for p, v in zip(proc.params, bound)
    ]
    return cid, rename, bindings


def _collect_local_names(script: IRScript) -> set[str]:
    """Return the union of every local-variable name written inside
    ``script``.  Used by the v3 inliner to build a complete rename
    map so the body's own writes don't leak into the caller's slot
    namespace.  Array-element writes (``arr(idx)``) contribute the
    array base name (``arr``) — the rename map needs the whole
    array's binding to ride into the inlined scope.
    """
    names: set[str] = set()
    _walk_local_writes(script, names)
    return names


def _record_local_base(name: str, names: set[str]) -> None:
    """Add the local base of ``name`` to ``names``, skipping
    ``::``-qualified globals.  For ``arr(idx)``, only the ``arr``
    base is recorded — the rename map operates at array-name
    granularity, with the index expression preserved verbatim by
    the rename helper.
    """
    if "::" in name:
        return
    paren = name.find("(")
    base = name[:paren] if paren >= 0 else name
    if base:
        names.add(base)


def _walk_local_writes(script: IRScript | None, names: set[str]) -> None:
    if script is None:
        return
    for stmt in script.statements:
        if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
            _record_local_base(stmt.name, names)
        elif isinstance(stmt, IRCall):
            for d in stmt.defs:
                _record_local_base(d, names)
        elif isinstance(stmt, IRBlock):
            _walk_local_writes(stmt.body, names)
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _walk_local_writes(clause.body, names)
            _walk_local_writes(stmt.else_body, names)
        elif isinstance(stmt, IRFor):
            _walk_local_writes(stmt.init, names)
            _walk_local_writes(stmt.next, names)
            _walk_local_writes(stmt.body, names)
        elif isinstance(stmt, IRWhile):
            _walk_local_writes(stmt.body, names)
        elif isinstance(stmt, IRForeach):
            for var_list, _ in stmt.iterators:
                for v in var_list:
                    _record_local_base(v, names)
            _walk_local_writes(stmt.body, names)
        elif isinstance(stmt, IRCatch):
            _walk_local_writes(stmt.body, names)
            if stmt.result_var:
                names.add(stmt.result_var)
            if stmt.options_var:
                names.add(stmt.options_var)
        elif isinstance(stmt, IRTry):
            _walk_local_writes(stmt.body, names)
            for h in stmt.handlers:
                _walk_local_writes(h.body, names)
                if h.var_name:
                    names.add(h.var_name)
                if h.options_var:
                    names.add(h.options_var)
            _walk_local_writes(stmt.finally_body, names)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                _walk_local_writes(arm.body, names)
            _walk_local_writes(stmt.default_body, names)
        elif isinstance(stmt, IRUpFrame):
            _walk_local_writes(stmt.body, names)


def _drop_dead_procs(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
    candidates: frozenset[str],
) -> IRModule:
    """Remove from ``module.procedures`` every proc whose every
    static reference was just inlined away.

    Eligibility:

    * The proc must be in ``candidates`` (the catalogue's inlinable
      set — already proved pure_leaf, so no observable side-effect
      from removal).
    * Its pre-inline ``static_call_count`` must be **> 0** — that
      is, the catalogue saw at least one internal IRCall to it.
      Procs with zero internal call sites are typically reached
      externally (Python tests, embedding host calls,
      ``namespace import`` from other compilation units) and are
      conservatively kept regardless of post-inline reference
      count.
    * Post-inline reference set must not mention the proc by
      name — neither as a resolved IRCall command nor as a
      literal-text mention in any IRCall arg / IRAssign* value /
      IRSwitch subject / IRReturn value.  The textual scan
      catches dynamic-dispatch sites like
      ``rename once {}`` / ``info procs once`` /
      ``interp alias {} new {} once``: builtins that accept a
      proc name as a string argument and look it up at runtime.

    Truly opaque procs (non-pure_leaf, possibly reached through
    ``eval`` / ``unknown`` / dynamic ``[$cmd …]``) are never in
    ``candidates`` and stay regardless.
    """
    if not candidates:
        return module

    # Build the set of bare names we need to look for in args /
    # values.  Each candidate qname may be referenced by its
    # bare form (``once``) or its qualified form (``::once``);
    # checking both bare and qualified avoids false negatives.
    name_index: dict[str, set[str]] = {}
    for qname in candidates:
        bare = qname[2:] if qname.startswith("::") else qname
        name_index.setdefault(bare, set()).add(qname)
        name_index.setdefault(qname, set()).add(qname)

    referenced: set[str] = set()
    _collect_referenced_procs(
        module.top_level, "::", summaries, referenced, name_index
    )
    for qname, proc in module.procedures.items():
        _collect_referenced_procs(
            proc.body, qname, summaries, referenced, name_index
        )

    new_procs: dict[str, IRProcedure] = {}
    dropped: set[str] = set()
    for qname, proc in module.procedures.items():
        eligible = (
            qname in candidates
            and proc.static_call_count > 0
            and qname not in referenced
        )
        if eligible:
            dropped.add(qname)
            continue
        new_procs[qname] = proc
    if not dropped:
        return module

    # Also strip the top-level ``proc <name> …`` IRCall that
    # defines each dropped proc.  Without this the codegen still
    # tries to register the proc at module load time, but the
    # corresponding compiled function is gone — observed during
    # development as ``unknown command`` traps when the runtime
    # falls back to interpreted dispatch on a stub registration.
    new_top = _strip_proc_defs(module.top_level, dropped)
    return replace(module, top_level=new_top, procedures=new_procs)


def _strip_proc_defs(script: IRScript, dropped: set[str]) -> IRScript:
    """Remove every ``proc <name> …`` IRCall whose ``<name>``
    qualifies (after global-namespace prefix) to a dropped proc.

    Recurses into ``IRBlock`` nested at top level (``namespace
    eval`` lowering) so a proc defined inside a namespace block
    also gets its definition stripped.  Other control-flow
    containers (``IRIf``, loops) are not walked — proc defs
    inside conditional blocks would be runtime-dependent and
    aren't dropped by the catalogue anyway.
    """
    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        if isinstance(stmt, IRCall) and stmt.command == "proc" and stmt.args:
            proc_name = stmt.args[0]
            qname = (
                proc_name if proc_name.startswith("::") else "::" + proc_name
            )
            if qname in dropped:
                changed = True
                continue
        if isinstance(stmt, IRBlock):
            new_body = _strip_proc_defs(stmt.body, dropped)
            if new_body is not stmt.body:
                new_stmts.append(replace(stmt, body=new_body))
                changed = True
                continue
        new_stmts.append(stmt)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


_PROC_NAME_WORD_RE = re.compile(r"(?<![A-Za-z0-9_:])(::[A-Za-z_][A-Za-z0-9_:]*|[A-Za-z_][A-Za-z0-9_]*)(?![A-Za-z0-9_:])")


# Commands whose args are NOT proc-name references — the
# textual scan would generate spurious matches (e.g. ``source
# helper.tcl`` would match ``helper`` inside the filename).
# Most of these wrap file paths, package names, or define-site
# bindings rather than dispatching to a named proc at runtime.
# ``rename`` / ``info procs`` / ``interp alias`` / ``namespace
# import`` are deliberately NOT in this set — those DO take
# proc-name args at runtime, so the scan must keep their
# referenced procs alive.
_ARGS_NOT_PROCS: frozenset[str] = frozenset(
    {
        "proc",  # defines, not uses (first arg is the proc being defined)
        "source",  # arg is a file path, not a command name
        "package",  # ``package require <name>`` — pkg name, not a proc
        "load",  # arg is a shared-lib path
        "puts",  # arg is the message text
    }
)


def _scan_text_for_procs(
    text: str | None,
    name_index: dict[str, set[str]],
    out: set[str],
) -> None:
    """Scan ``text`` for any whole-word match against ``name_index``
    and add the matching qualified procs to ``out``.

    Conservative: false positives (a string literal that happens
    to read like a proc name) just keep the proc alive.  False
    negatives — a real dynamic-dispatch site we miss — would
    silently drop a live proc.
    """
    if not text:
        return
    for m in _PROC_NAME_WORD_RE.finditer(text):
        word = m.group(1)
        targets = name_index.get(word)
        if targets:
            out.update(targets)


def _collect_referenced_procs(
    script: IRScript | None,
    caller_qname: str,
    summaries: dict[str, ProcEscapeSummary],
    out: set[str],
    name_index: dict[str, set[str]],
) -> None:
    if script is None:
        return
    for stmt in script.statements:
        if isinstance(stmt, IRCall):
            target = _resolve_callee(stmt.command, caller_qname, summaries)
            if target is not None:
                out.add(target)
            # Some commands' args aren't proc-name references and
            # would generate spurious matches in the textual scan
            # (``source helper.tcl`` would otherwise keep
            # ``::helper`` alive because the regex matches ``helper``
            # inside ``helper.tcl``).  Skip arg scanning for the
            # known command-arg-is-not-a-proc-name set.
            if stmt.command in _ARGS_NOT_PROCS:
                continue
            for arg in stmt.args:
                _scan_text_for_procs(arg, name_index, out)
        elif isinstance(stmt, IRAssignValue):
            _scan_text_for_procs(stmt.value, name_index, out)
        elif isinstance(stmt, IRAssignConst):
            _scan_text_for_procs(stmt.value, name_index, out)
        elif isinstance(stmt, IRReturn):
            if stmt.value is not None:
                _scan_text_for_procs(stmt.value, name_index, out)
        elif isinstance(stmt, IRSwitch):
            _scan_text_for_procs(stmt.subject, name_index, out)
        if isinstance(stmt, IRBlock):
            _collect_referenced_procs(
                stmt.body, caller_qname, summaries, out, name_index
            )
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _collect_referenced_procs(
                    clause.body, caller_qname, summaries, out, name_index
                )
            _collect_referenced_procs(
                stmt.else_body, caller_qname, summaries, out, name_index
            )
        elif isinstance(stmt, IRFor):
            _collect_referenced_procs(
                stmt.init, caller_qname, summaries, out, name_index
            )
            _collect_referenced_procs(
                stmt.next, caller_qname, summaries, out, name_index
            )
            _collect_referenced_procs(
                stmt.body, caller_qname, summaries, out, name_index
            )
        elif isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
            _collect_referenced_procs(
                stmt.body, caller_qname, summaries, out, name_index
            )
        elif isinstance(stmt, IRTry):
            _collect_referenced_procs(
                stmt.body, caller_qname, summaries, out, name_index
            )
            for h in stmt.handlers:
                _collect_referenced_procs(
                    h.body, caller_qname, summaries, out, name_index
                )
            _collect_referenced_procs(
                stmt.finally_body, caller_qname, summaries, out, name_index
            )
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                _collect_referenced_procs(
                    arm.body, caller_qname, summaries, out, name_index
                )
            _collect_referenced_procs(
                stmt.default_body, caller_qname, summaries, out, name_index
            )


def _rewrite_if_clauses(
    clauses: tuple[IRIfClause, ...],
    caller_qname: str,
    inlinable: dict[str, _InlineSpec],
    summaries: dict[str, ProcEscapeSummary],
    counter: list[int],
) -> tuple[tuple[IRIfClause, ...], bool]:
    new_clauses = []
    changed = False
    for clause in clauses:
        new_body = _rewrite_script(
            clause.body, caller_qname, inlinable, summaries, counter
        )
        if new_body is clause.body:
            new_clauses.append(clause)
        else:
            new_clauses.append(replace(clause, body=new_body))
            changed = True
    return tuple(new_clauses), changed


def _rewrite_try_handlers(
    handlers: tuple[IRTryHandler, ...],
    caller_qname: str,
    inlinable: dict[str, _InlineSpec],
    summaries: dict[str, ProcEscapeSummary],
    counter: list[int],
) -> tuple[tuple[IRTryHandler, ...], bool]:
    new_handlers = []
    changed = False
    for handler in handlers:
        new_body = _rewrite_script(
            handler.body, caller_qname, inlinable, summaries, counter
        )
        if new_body is handler.body:
            new_handlers.append(handler)
        else:
            new_handlers.append(replace(handler, body=new_body))
            changed = True
    return tuple(new_handlers), changed


def _rewrite_switch_arms(
    arms: tuple[IRSwitchArm, ...],
    caller_qname: str,
    inlinable: dict[str, _InlineSpec],
    summaries: dict[str, ProcEscapeSummary],
    counter: list[int],
) -> tuple[tuple[IRSwitchArm, ...], bool]:
    new_arms = []
    changed = False
    for arm in arms:
        if arm.body is None:
            new_arms.append(arm)
            continue
        new_body = _rewrite_script(
            arm.body, caller_qname, inlinable, summaries, counter
        )
        if new_body is arm.body:
            new_arms.append(arm)
        else:
            new_arms.append(replace(arm, body=new_body))
            changed = True
    return tuple(new_arms), changed
