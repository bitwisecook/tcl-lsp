"""S4.2 — IR-level inlining.

The catalogue stage (:mod:`core.compiler.inlining.decision`) tags
each :class:`~core.compiler.ir.IRProcedure` with an
:class:`~core.compiler.ir.InlineDecision`.  This module consumes
those tags and rewrites the IR module so the marked calls
disappear before codegen.

The pass handles two shapes of statement-position
:class:`~core.compiler.ir.IRCall` whose resolved target is an
:data:`~core.compiler.ir.InlineDecision.ALWAYS`-tagged proc:

**v0 — empty-body splice.**  A proc with zero body statements;
the call vanishes entirely.

**v1 — single-call wrapper splice.**  A zero-parameter proc whose
body is exactly one :class:`IRCall` with no ``defs`` (writes no
variable in the caller's scope).  The wrapper's wrapped call is
substituted for the original call site.  The substitution is
sound only when the wrapped call's command word resolves to the
same target from the caller's namespace as it did from the
callee's, so v1 declines unless the wrapped command is
``::``-qualified or refers to an unknown (i.e. runtime-builtin)
command — both classes are namespace-invariant.  The call site
itself must have no arguments (zero params on the callee already
forbids them in well-formed Tcl) so we never drop arg-evaluation
side effects.

Larger-body inlining (multiple statements, parameter binding,
return-value capture, α-renaming) is left for a future revision
that grows the necessary infrastructure on the IR side.

The pass returns a new :class:`~core.compiler.ir.IRModule`; the
input module is left untouched (every IR node is frozen, so
rebuilding via :func:`dataclasses.replace` is the only option).
"""

from __future__ import annotations

from dataclasses import replace

from ..ir import (
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRModule,
    IRProcedure,
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

    new_top = _rewrite_script(module.top_level, "::", inlinable, summaries)
    new_procs: dict[str, IRProcedure] = {}
    for qname, proc in module.procedures.items():
        new_body = _rewrite_script(proc.body, qname, inlinable, summaries)
        if new_body is proc.body:
            new_procs[qname] = proc
        else:
            new_procs[qname] = replace(proc, body=new_body)
    return replace(module, top_level=new_top, procedures=new_procs)


def _build_inlinable_map(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
) -> dict[str, tuple]:
    """Return a map from inlinable qname → tuple of statements to splice.

    An empty tuple means the call vanishes entirely (v0 empty-body
    case).  A single-element tuple of :class:`IRCall` means the
    call site is replaced by that wrapped call (v1 wrapper case).

    ``IF_SINGLE_CALL`` is **not** eligible because its profitability
    depends on the post-inline proc-pruning that isn't implemented
    yet — inlining without pruning would leave a redundant copy of
    the body in the module.
    """

    eligible: dict[str, tuple] = {}
    for qname, proc in module.procedures.items():
        if proc.inline_decision is not InlineDecision.ALWAYS:
            continue

        # v0 — empty body
        if not proc.body.statements:
            eligible[qname] = ()
            continue

        # v1 — zero-param wrapper around a single IRCall
        if proc.params:
            continue
        if len(proc.body.statements) != 1:
            continue
        only = proc.body.statements[0]
        if not isinstance(only, IRCall):
            continue
        if only.defs:
            # Writing a variable in the splice would mutate the
            # caller's scope — decline.
            continue
        if not _command_is_namespace_invariant(only.command, qname, summaries):
            continue
        if not _command_is_splice_safe(only.command):
            # Some frameless commands observe the caller's frame
            # (``info level``, ``uplevel``, ``upvar``) or transfer
            # control out of it (``return``, ``break``, ``continue``).
            # Splicing them into a different frame would silently
            # change semantics — decline.
            continue
        if any(_arg_has_command_subst(a) for a in only.args):
            # Args containing a ``[cmd]`` substitution evaluate
            # ``cmd`` in whatever frame the caller's IRCall lives
            # in.  Splicing the wrapper into a different frame
            # would silently change ``info level`` / ``upvar`` /
            # ``info frame`` semantics that the inner ``cmd``
            # might depend on.  ``$var`` substitutions are also
            # frame-scoped but the v1 wrapper has no params (so
            # the wrapped IRCall can't reach the wrapper's own
            # locals — any ``$x`` it carries refers to a global,
            # which is frame-independent).
            continue
        eligible[qname] = (only,)

    return eligible


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
    inlinable: dict[str, tuple],
    summaries: dict[str, ProcEscapeSummary],
) -> IRScript:
    """Return ``script`` with eligible top-level calls dropped.

    Recurses into every nested control-flow body so calls inside
    ``if`` / ``for`` / ``foreach`` / ``while`` / ``catch`` / ``try``
    / ``switch`` arms are also subject to the splice.  Returns the
    original instance unchanged when no rewrites are needed (cheap
    structural sharing for non-affected scripts).
    """

    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        replacements = _rewrite_stmt(stmt, caller_qname, inlinable, summaries)
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
    inlinable: dict[str, tuple],
    summaries: dict[str, ProcEscapeSummary],
) -> list | None:
    """Return a replacement statement list, or ``None`` to keep ``stmt``.

    A return value of ``[]`` drops the statement entirely (v0
    empty-body case); a non-empty list substitutes the inlined
    body for the call site (v1 wrapper case).
    """

    if isinstance(stmt, IRCall):
        target = _resolve_callee(stmt.command, caller_qname, summaries)
        if target is not None and target in inlinable:
            # The call site itself must have no arguments to
            # qualify — otherwise we'd silently drop arg
            # evaluation side effects.  Zero-param callees match
            # well-formed call sites with zero args.  Empty-body
            # callees take the same gate but it's vacuously
            # satisfied for them.
            if stmt.args:
                return None
            replacement_statements = inlinable[target]
            if not replacement_statements:
                return []
            # v1: substitute the wrapped call, propagating the
            # call site's range so diagnostic attribution stays
            # at the user-visible source location.
            new_stmts = []
            for inner in replacement_statements:
                if isinstance(inner, IRCall):
                    new_stmts.append(replace(inner, range=stmt.range))
                else:
                    new_stmts.append(inner)
            return new_stmts
        return None

    if isinstance(stmt, IRBlock):
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        if new_body is stmt.body:
            return None
        return [replace(stmt, body=new_body)]

    if isinstance(stmt, IRIf):
        new_clauses, clauses_changed = _rewrite_if_clauses(
            stmt.clauses, caller_qname, inlinable, summaries
        )
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _rewrite_script(stmt.else_body, caller_qname, inlinable, summaries)
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return None
        return [replace(stmt, clauses=new_clauses, else_body=new_else)]

    if isinstance(stmt, IRFor):
        new_init = _rewrite_script(stmt.init, caller_qname, inlinable, summaries)
        new_next = _rewrite_script(stmt.next, caller_qname, inlinable, summaries)
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        if (
            new_init is stmt.init
            and new_next is stmt.next
            and new_body is stmt.body
        ):
            return None
        return [replace(stmt, init=new_init, next=new_next, body=new_body)]

    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        if new_body is stmt.body:
            return None
        return [replace(stmt, body=new_body)]

    if isinstance(stmt, IRTry):
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        new_handlers, handlers_changed = _rewrite_try_handlers(
            stmt.handlers, caller_qname, inlinable, summaries
        )
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = _rewrite_script(
                stmt.finally_body, caller_qname, inlinable, summaries
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
            stmt.arms, caller_qname, inlinable, summaries
        )
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _rewrite_script(
                stmt.default_body, caller_qname, inlinable, summaries
            )
            default_changed = new_default is not stmt.default_body
        if not arms_changed and not default_changed:
            return None
        return [replace(stmt, arms=new_arms, default_body=new_default)]

    return None


def _rewrite_if_clauses(
    clauses: tuple[IRIfClause, ...],
    caller_qname: str,
    inlinable: dict[str, tuple],
    summaries: dict[str, ProcEscapeSummary],
) -> tuple[tuple[IRIfClause, ...], bool]:
    new_clauses = []
    changed = False
    for clause in clauses:
        new_body = _rewrite_script(clause.body, caller_qname, inlinable, summaries)
        if new_body is clause.body:
            new_clauses.append(clause)
        else:
            new_clauses.append(replace(clause, body=new_body))
            changed = True
    return tuple(new_clauses), changed


def _rewrite_try_handlers(
    handlers: tuple[IRTryHandler, ...],
    caller_qname: str,
    inlinable: dict[str, tuple],
    summaries: dict[str, ProcEscapeSummary],
) -> tuple[tuple[IRTryHandler, ...], bool]:
    new_handlers = []
    changed = False
    for handler in handlers:
        new_body = _rewrite_script(handler.body, caller_qname, inlinable, summaries)
        if new_body is handler.body:
            new_handlers.append(handler)
        else:
            new_handlers.append(replace(handler, body=new_body))
            changed = True
    return tuple(new_handlers), changed


def _rewrite_switch_arms(
    arms: tuple[IRSwitchArm, ...],
    caller_qname: str,
    inlinable: dict[str, tuple],
    summaries: dict[str, ProcEscapeSummary],
) -> tuple[tuple[IRSwitchArm, ...], bool]:
    new_arms = []
    changed = False
    for arm in arms:
        if arm.body is None:
            new_arms.append(arm)
            continue
        new_body = _rewrite_script(arm.body, caller_qname, inlinable, summaries)
        if new_body is arm.body:
            new_arms.append(arm)
        else:
            new_arms.append(replace(arm, body=new_body))
            changed = True
    return tuple(new_arms), changed
