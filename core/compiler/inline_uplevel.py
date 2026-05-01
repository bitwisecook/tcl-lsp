"""Whole-callee ``uplevel``-passthrough inlining.

A Tcl idiom for building small helpers that mutate the caller's state
looks like::

    proc reset_counter {} {
        uplevel 1 {set counter 0}
    }

    proc my_action {} {
        set counter 10
        reset_counter
        # counter is now 0
    }

At compile time, ``reset_counter``'s body is known to be a single
:class:`IRUpFrame` with ``frame_shift == 1`` — its *only* work is to
evaluate a body in the caller's frame.  A call to ``reset_counter``
therefore pushes a proc frame, stashes ``frame_depth`` back down by
one to unwind it, evaluates the body, restores, and pops the frame —
four frame operations for no semantic benefit.

This pass recognises that pattern and rewrites every callsite of a
passthrough callee to inline the callee's body as an :class:`IRBlock`
in the caller's scope.  Because the body already ran at the caller's
scope semantically, the rewrite is a pure call-overhead elimination:
same observable behaviour, fewer frame operations.

### Gate — static (zero-param) shape

A proc is a static passthrough candidate when **all** hold:

1. Zero parameters.
2. Body is exactly one :class:`IRUpFrame` statement.  No prologue, no
   epilogue, no ``return``.
3. ``frame_shift == 1``.  ``#0`` (absolute global) or deeper shifts
   cannot be expressed as a same-frame inline, because the caller's
   scope is not the target scope.
4. The inner body (``IRUpFrame.body``) contains no nested
   ``uplevel`` / ``upvar`` / frame-inspecting ``info``.  Those can be
   re-inlined by a future pass; for the first wave we conservatively
   bail.

### Gate — single-body-param shape

Covers the canonical "pass my caller's script body up a frame" helper::

    proc dispatcher {body} {
        uplevel 1 $body
    }

where the entire proc body is an :class:`IRBarrier` of the form
``uplevel 1 $<param>`` referencing the sole proc parameter.  The
``uplevel`` cannot be statically relaxed at lowering time (the body
token is ``$var`` against an unknown parameter value), but every
call site that passes a braced-literal argument **does** know the
body, so we inline per-call.

A proc is a param-body passthrough candidate when **all** hold:

1. Exactly one parameter named *P*.
2. Body is exactly one :class:`IRBarrier` whose ``command ==
   "uplevel"`` and whose ``args`` shape is ``("$P",)`` (implicit
   level 1) or ``("1", "$P")`` (explicit level 1).
3. The body-arg token is a :class:`TokenType.VAR` whose ``text == P``.

At each call site ``IRCall(command=<candidate>, args=(arg,))`` whose
single argument is a ``TokenType.STR`` braced literal, the call is
rewritten to an :class:`IRBlock` whose body is the parsed literal —
identical to what the static case produces, but with the body coming
from the call site instead of the callee.

Non-static call shapes (``[proc_name]``, ``eval`` with a dynamic body)
are unaffected because the pass rewrites only :class:`IRCall` nodes
whose ``command`` resolves to a candidate callee's qualified name.

### Outcome

The rewritten call site is an :class:`IRBlock` that splices the inlined
body into the caller's IR.  Downstream:

* The WASM codegen's ``IRBlock`` handler emits the statements inline
  in the caller's frame — zero call overhead, zero frame_depth_stash.
* Every analysis pass that treats ``IRUpFrame`` as a barrier (var_escape,
  memory_ssa, interprocedural) sees a plain ``IRBlock`` instead, so
  the caller's frame-adjacent analysis runs with full visibility into
  what the body mutates.

### Non-goals

* Multi-parameter substitution — only the sole-param-is-body shape
  is recognised.  Helpers like tcltest's ``Eval {script {ignoreOutput
  1}}`` stay on the barrier path because their bodies have prologue
  / epilogue beyond the ``uplevel``.
* Inlining non-passthrough procs.  ``set x 1; uplevel 1 {…}`` is NOT a
  candidate because the ``set`` mutates the callee's frame, not the
  caller's.  Extending to "trivial prologue + uplevel" is a follow-up.
* Deleting now-unused candidate procs.  The existing unused-procs pass
  can reclaim them in a separate step.
"""

# canonicalisation: audited #246

from __future__ import annotations

from dataclasses import dataclass

from ..parsing.lexer import TclParseError
from ..parsing.tokens import TokenType
from .ir import (
    IRBarrier,
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
    IRStatement,
    IRSwitch,
    IRSwitchArm,
    IRTry,
    IRTryHandler,
    IRUpFrame,
    IRWhile,
)

_FRAME_ONE_SHIFT = 1


@dataclass(frozen=True, slots=True)
class _Candidate:
    """Recognised passthrough proc shape used by the rewriter.

    ``kind == "static"`` — body is pre-known; ``body`` holds the IR to
    splice in directly (zero-param shape).

    ``kind == "param_body"`` — body is the call site's single braced
    literal arg; the rewriter parses the literal and splices it in
    (single-body-param shape).  ``body`` is unused in this mode.
    """

    kind: str  # "static" | "param_body"
    body: IRScript | None = None


def _lower_literal_script(literal: str, *, namespace: str) -> IRScript | None:
    """Lower a braced-literal script *literal* to an :class:`IRScript`.

    Used by the param-body rewrite path to synthesize the call site's
    inlined body from the single STR argument.  Returns ``None`` when
    the literal cannot be parsed — callers keep the original call.
    """
    from .lowering import _Lowerer  # local import to avoid module-level cycle

    lowerer = _Lowerer()
    try:
        return lowerer._lower_script(literal, namespace=namespace)
    except TclParseError:
        return None


def inline_uplevel_passthrough(module: IRModule) -> None:
    """Mutate *module* in place, inlining all passthrough call sites.

    Safe to run multiple times: the rewrite replaces :class:`IRCall`
    nodes with :class:`IRBlock` nodes, and the candidate detection
    re-scans every proc on each invocation.  Calling twice is a no-op
    once all callsites have been rewritten.
    """
    candidates = _collect_candidates(module)
    if not candidates:
        return

    module.top_level = _rewrite_script(module.top_level, candidates, namespace="::")
    for qname, proc in list(module.procedures.items()):
        caller_ns = _namespace_of(qname)
        new_body = _rewrite_script(proc.body, candidates, namespace=caller_ns)
        if new_body is not proc.body:
            module.procedures[qname] = IRProcedure(
                name=proc.name,
                qualified_name=proc.qualified_name,
                params=proc.params,
                range=proc.range,
                body=new_body,
                params_raw=proc.params_raw,
                body_source=proc.body_source,
                namespace_scoped=proc.namespace_scoped,
                base_priority=proc.base_priority,
            )


def _collect_candidates(module: IRModule) -> dict[str, _Candidate]:
    """Return ``{qname: candidate}`` for every passthrough proc in the module."""
    out: dict[str, _Candidate] = {}
    for qname, proc in module.procedures.items():
        if qname in module.redefined_procedures:
            # Runtime redefinition may swap the implementation — safer
            # not to inline.
            continue
        cand = _passthrough_candidate(proc)
        if cand is not None:
            out[qname] = cand
    return out


def _passthrough_candidate(proc: IRProcedure) -> _Candidate | None:
    """Classify *proc* as a passthrough candidate, or return ``None``."""
    # Static (zero-param) shape.
    static = _static_passthrough_body(proc)
    if static is not None:
        return _Candidate(kind="static", body=static)
    # Single-body-param shape.
    if _is_param_body_passthrough(proc):
        return _Candidate(kind="param_body", body=None)
    return None


def _static_passthrough_body(proc: IRProcedure) -> IRScript | None:
    """Return the inner body if *proc* is a zero-param uplevel-1 passthrough."""
    if proc.params:
        return None
    stmts = proc.body.statements
    if len(stmts) != 1:
        return None
    stmt = stmts[0]
    if not isinstance(stmt, IRUpFrame):
        return None
    if stmt.frame_shift != _FRAME_ONE_SHIFT:
        return None
    if _body_has_frame_reach(stmt.body):
        return None
    return stmt.body


def _is_param_body_passthrough(proc: IRProcedure) -> bool:
    """True if *proc* is ``proc NAME {P} { uplevel ?1? $P }``.

    Recognises both the bare ``uplevel $body`` (implicit level 1) and
    explicit ``uplevel 1 $body`` forms.  The body must be a single
    :class:`IRBarrier` — the VM hasn't relaxed it to :class:`IRUpFrame`
    because the body token is ``$var`` rather than a static literal.
    """
    if len(proc.params) != 1:
        return False
    param = proc.params[0]
    stmts = proc.body.statements
    if len(stmts) != 1:
        return False
    stmt = stmts[0]
    if not isinstance(stmt, IRBarrier):
        return False
    if stmt.canonical_command != "::uplevel":
        return False
    tokens = stmt.tokens
    if tokens is None:
        return False
    # argv is the parsed token stream starting with "uplevel".  The
    # body token is the last non-separator arg.  Recognise both
    # single-arg (implicit level 1) and "1 $body" forms.
    arg_tokens = tokens.argv[1:]
    arg_texts = tokens.argv_texts[1:]
    if len(arg_tokens) == 1:
        level_explicit = False
        body_tok = arg_tokens[0]
    elif len(arg_tokens) == 2:
        level_explicit = True
        level_tok = arg_tokens[0]
        if level_tok.type is not TokenType.ESC or arg_texts[0] != "1":
            return False
        body_tok = arg_tokens[1]
    else:
        return False
    _ = level_explicit  # kept for future frame-shift widening
    if body_tok.type is not TokenType.VAR:
        return False
    if body_tok.text != param:
        return False
    return True


def _body_has_frame_reach(script: IRScript) -> bool:
    """True if *script* contains an ``uplevel`` / ``upvar`` / frame-inspecting
    command that would reach outside the inlined scope.

    After inlining, the body runs in the caller's frame.  If the body
    itself does ``uplevel 1 {...}``, that now references the caller's
    *caller* — a frame the original author may not have anticipated.
    Reject conservatively: a future pass can decide per-callsite
    whether the semantics survive.
    """
    for stmt in script.statements:
        if isinstance(stmt, IRUpFrame):
            return True
        if isinstance(stmt, IRBarrier) and stmt.canonical_command in ("::uplevel", "::upvar"):
            return True
        if isinstance(stmt, IRCall) and stmt.canonical_command == "::upvar":
            return True
        if _has_frame_reach_in_children(stmt):
            return True
    return False


def _has_frame_reach_in_children(stmt: IRStatement) -> bool:
    """Recurse into structured IR (if / while / for / etc.) checking for frame reach."""
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            if _body_has_frame_reach(clause.body):
                return True
        if stmt.else_body is not None and _body_has_frame_reach(stmt.else_body):
            return True
    elif isinstance(stmt, IRFor):
        if (
            _body_has_frame_reach(stmt.init)
            or _body_has_frame_reach(stmt.next)
            or _body_has_frame_reach(stmt.body)
        ):
            return True
    elif isinstance(stmt, IRWhile):
        if _body_has_frame_reach(stmt.body):
            return True
    elif isinstance(stmt, IRForeach):
        if _body_has_frame_reach(stmt.body):
            return True
    elif isinstance(stmt, IRCatch):
        if _body_has_frame_reach(stmt.body):
            return True
    elif isinstance(stmt, IRTry):
        if _body_has_frame_reach(stmt.body):
            return True
        for handler in stmt.handlers:
            if _body_has_frame_reach(handler.body):
                return True
        if stmt.finally_body is not None and _body_has_frame_reach(stmt.finally_body):
            return True
    elif isinstance(stmt, IRSwitch):
        for arm in stmt.arms:
            if arm.body is not None and _body_has_frame_reach(arm.body):
                return True
        if stmt.default_body is not None and _body_has_frame_reach(stmt.default_body):
            return True
    elif isinstance(stmt, IRBlock):
        if _body_has_frame_reach(stmt.body):
            return True
    return False


def _namespace_of(qname: str) -> str:
    """Return the namespace component of a fully qualified proc name."""
    idx = qname.rfind("::")
    if idx <= 0:
        return "::"
    return qname[:idx] if qname[:idx].startswith("::") else f"::{qname[:idx]}"


def _resolve_call_target(command: str, caller_ns: str) -> str:
    """Resolve *command* (as written at the call site) to a qualified name.

    Mirrors the namespace-scoped lookup ``resolve_internal_call`` does
    but simplified for our inliner: ``::``-prefixed names are absolute,
    otherwise we qualify against the caller's namespace.  The caller
    checks both the qualified form and a ``::<bare>`` form because
    the bare form is how top-level procs are keyed in the module.
    """
    if command.startswith("::"):
        return command
    if caller_ns == "::":
        return f"::{command}"
    return f"{caller_ns}::{command}"


def _rewrite_script(
    script: IRScript, candidates: dict[str, _Candidate], *, namespace: str
) -> IRScript:
    new_stmts: list[IRStatement] = []
    changed = False
    for stmt in script.statements:
        rewritten = _rewrite_stmt(stmt, candidates, namespace=namespace)
        if rewritten is not stmt:
            changed = True
        new_stmts.append(rewritten)
    if changed:
        return IRScript(statements=tuple(new_stmts))
    return script


def _rewrite_stmt(
    stmt: IRStatement, candidates: dict[str, _Candidate], *, namespace: str
) -> IRStatement:
    if isinstance(stmt, IRCall):
        if not stmt.command:
            return stmt
        target = _resolve_call_target(stmt.command, namespace)
        cand = candidates.get(target) or candidates.get(f"::{stmt.command}")
        if cand is None:
            return stmt

        if cand.kind == "static":
            # Zero-param: only rewrite zero-argument calls.
            if stmt.args:
                return stmt
            assert cand.body is not None
            return IRBlock(
                range=stmt.range,
                body=cand.body,
                namespace=namespace,
                source_args=(),
                source_tokens=stmt.tokens,
            )

        if cand.kind == "param_body":
            # Single-body-param: rewrite calls whose sole argument is a
            # braced-literal STR token.  Dynamic args (``$var``, ``[cmd]``,
            # ESC / double-quoted) stay on the call path — the runtime
            # will still dispatch them to the proc and eval the body.
            if len(stmt.args) != 1:
                return stmt
            tokens = stmt.tokens
            if tokens is None or len(tokens.argv) < 2:
                return stmt
            arg_tok = tokens.argv[1]
            if arg_tok.type is not TokenType.STR:
                return stmt
            if tokens.expand_word and any(tokens.expand_word):
                return stmt
            literal = stmt.args[0]
            inlined = _lower_literal_script(literal, namespace=namespace)
            if inlined is None:
                return stmt
            if _body_has_frame_reach(inlined):
                return stmt
            return IRBlock(
                range=stmt.range,
                body=inlined,
                namespace=namespace,
                source_args=(),
                source_tokens=stmt.tokens,
            )

        return stmt

    # Recurse into structured IR so nested call sites get the same rewrite.
    if isinstance(stmt, IRIf):
        new_clauses: list[IRIfClause] = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_body = _rewrite_script(clause.body, candidates, namespace=namespace)
            if new_body is not clause.body:
                clauses_changed = True
                new_clauses.append(
                    IRIfClause(
                        condition=clause.condition,
                        condition_range=clause.condition_range,
                        body=new_body,
                        body_range=clause.body_range,
                    )
                )
            else:
                new_clauses.append(clause)
        new_else = stmt.else_body
        if stmt.else_body is not None:
            new_else = _rewrite_script(stmt.else_body, candidates, namespace=namespace)
            if new_else is not stmt.else_body:
                clauses_changed = True
        if clauses_changed:
            return IRIf(
                range=stmt.range,
                clauses=tuple(new_clauses),
                else_body=new_else,
                else_range=stmt.else_range,
            )
        return stmt
    if isinstance(stmt, IRFor):
        new_init = _rewrite_script(stmt.init, candidates, namespace=namespace)
        new_next = _rewrite_script(stmt.next, candidates, namespace=namespace)
        new_body = _rewrite_script(stmt.body, candidates, namespace=namespace)
        if new_init is not stmt.init or new_next is not stmt.next or new_body is not stmt.body:
            return IRFor(
                range=stmt.range,
                init=new_init,
                init_range=stmt.init_range,
                condition=stmt.condition,
                condition_range=stmt.condition_range,
                next=new_next,
                next_range=stmt.next_range,
                body=new_body,
                body_range=stmt.body_range,
                raw_args=stmt.raw_args,
            )
        return stmt
    if isinstance(stmt, IRWhile):
        new_body = _rewrite_script(stmt.body, candidates, namespace=namespace)
        if new_body is not stmt.body:
            return IRWhile(
                range=stmt.range,
                condition=stmt.condition,
                condition_range=stmt.condition_range,
                body=new_body,
                body_range=stmt.body_range,
                raw_args=stmt.raw_args,
            )
        return stmt
    if isinstance(stmt, IRForeach):
        new_body = _rewrite_script(stmt.body, candidates, namespace=namespace)
        if new_body is not stmt.body:
            return IRForeach(
                range=stmt.range,
                iterators=stmt.iterators,
                body=new_body,
                body_range=stmt.body_range,
                is_lmap=stmt.is_lmap,
                raw_args=stmt.raw_args,
                is_dict_iteration=stmt.is_dict_iteration,
            )
        return stmt
    if isinstance(stmt, IRCatch):
        new_body = _rewrite_script(stmt.body, candidates, namespace=namespace)
        if new_body is not stmt.body:
            return IRCatch(
                range=stmt.range,
                body=new_body,
                result_var=stmt.result_var,
                options_var=stmt.options_var,
                body_range=stmt.body_range,
                raw_args=stmt.raw_args,
            )
        return stmt
    if isinstance(stmt, IRTry):
        new_body = _rewrite_script(stmt.body, candidates, namespace=namespace)
        new_handlers: list[IRTryHandler] = []
        handlers_changed = False
        for handler in stmt.handlers:
            h_body = _rewrite_script(handler.body, candidates, namespace=namespace)
            if h_body is not handler.body:
                handlers_changed = True
                new_handlers.append(
                    IRTryHandler(
                        kind=handler.kind,
                        match_arg=handler.match_arg,
                        var_name=handler.var_name,
                        options_var=handler.options_var,
                        body=h_body,
                        body_range=handler.body_range,
                    )
                )
            else:
                new_handlers.append(handler)
        new_finally = stmt.finally_body
        if stmt.finally_body is not None:
            new_finally = _rewrite_script(stmt.finally_body, candidates, namespace=namespace)
        if new_body is not stmt.body or handlers_changed or new_finally is not stmt.finally_body:
            return IRTry(
                range=stmt.range,
                body=new_body,
                handlers=tuple(new_handlers),
                finally_body=new_finally,
                body_range=stmt.body_range,
                finally_range=stmt.finally_range,
            )
        return stmt
    if isinstance(stmt, IRSwitch):
        new_arms: list[IRSwitchArm] = []
        arms_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            a_body = _rewrite_script(arm.body, candidates, namespace=namespace)
            if a_body is not arm.body:
                arms_changed = True
                new_arms.append(
                    IRSwitchArm(
                        pattern=arm.pattern,
                        pattern_range=arm.pattern_range,
                        body=a_body,
                        body_range=arm.body_range,
                        fallthrough=arm.fallthrough,
                    )
                )
            else:
                new_arms.append(arm)
        new_default = stmt.default_body
        if stmt.default_body is not None:
            new_default = _rewrite_script(stmt.default_body, candidates, namespace=namespace)
        if arms_changed or new_default is not stmt.default_body:
            return IRSwitch(
                range=stmt.range,
                subject=stmt.subject,
                subject_range=stmt.subject_range,
                arms=tuple(new_arms),
                default_body=new_default,
                default_range=stmt.default_range,
                mode=stmt.mode,
                nocase=stmt.nocase,
                raw_args=stmt.raw_args,
            )
        return stmt
    if isinstance(stmt, IRBlock):
        # Don't descend into blocks produced by barrier relaxation —
        # those are already inlined from a different source.  But do
        # descend into ``namespace eval`` blocks because they contain
        # top-level-like statements that may call into candidates.
        if stmt.source_tokens is not None and stmt.source_tokens.argv_texts:
            cmd0 = stmt.source_tokens.argv_texts[0]
            if cmd0 == "eval":
                # An eval-shape IRBlock's body was already spliced in
                # at lowering — further inlining inside is fine.
                pass
        new_body = _rewrite_script(stmt.body, candidates, namespace=stmt.namespace or namespace)
        if new_body is not stmt.body:
            return IRBlock(
                range=stmt.range,
                body=new_body,
                namespace=stmt.namespace,
                source_args=stmt.source_args,
                source_tokens=stmt.source_tokens,
            )
        return stmt

    return stmt
