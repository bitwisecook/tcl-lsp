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

### Gate

A proc is a passthrough candidate when **all** hold:

1. Zero parameters.  Non-zero params would require substituting
   actual-argument IR into the body, which is deferred to a follow-up.
2. Body is exactly one :class:`IRUpFrame` statement.  No prologue, no
   epilogue, no ``return``.
3. ``frame_shift == 1``.  ``#0`` (absolute global) or deeper shifts
   cannot be expressed as a same-frame inline, because the caller's
   scope is not the target scope.
4. The inner body (``IRUpFrame.body``) contains no nested
   ``uplevel`` / ``upvar`` / frame-inspecting ``info``.  Those can be
   re-inlined by a future pass pass; for the first wave we conservatively
   bail.

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

* Parameter substitution — zero-param only for now.
* Inlining non-passthrough procs.  ``set x 1; uplevel 1 {…}`` is NOT a
  candidate because the ``set`` mutates the callee's frame, not the
  caller's.  Extending to "trivial prologue + uplevel" is a follow-up.
* Deleting now-unused candidate procs.  The existing unused-procs pass
  can reclaim them in a separate step.
"""

from __future__ import annotations

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


def _collect_candidates(module: IRModule) -> dict[str, IRScript]:
    """Return ``{qname: inlined_body}`` for every passthrough proc in the module."""
    out: dict[str, IRScript] = {}
    for qname, proc in module.procedures.items():
        if qname in module.redefined_procedures:
            # Runtime redefinition may swap the implementation — safer
            # not to inline.
            continue
        body = _passthrough_body(proc)
        if body is not None:
            out[qname] = body
    return out


def _passthrough_body(proc: IRProcedure) -> IRScript | None:
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
        if isinstance(stmt, IRBarrier) and stmt.command in ("uplevel", "upvar"):
            return True
        if isinstance(stmt, IRCall) and stmt.command in ("upvar",):
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
    script: IRScript, candidates: dict[str, IRScript], *, namespace: str
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
    stmt: IRStatement, candidates: dict[str, IRScript], *, namespace: str
) -> IRStatement:
    if isinstance(stmt, IRCall):
        # Only rewrite zero-argument calls: a passthrough callee takes
        # no params, so any args at the call site mean a different
        # callee or a user arity error we should leave alone.
        if stmt.args:
            return stmt
        if not stmt.command:
            return stmt
        target = _resolve_call_target(stmt.command, namespace)
        body = candidates.get(target) or candidates.get(f"::{stmt.command}")
        if body is None:
            return stmt
        # Splice the callee's body in as an IRBlock at the call site.
        # The namespace of the inlined block is the caller's namespace
        # so unqualified commands inside resolve correctly at codegen.
        return IRBlock(
            range=stmt.range,
            body=body,
            namespace=namespace,
            source_args=(),
            source_tokens=stmt.tokens,
        )

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
