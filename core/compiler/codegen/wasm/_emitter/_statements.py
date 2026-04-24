"""_WasmEmitterStmtMixin: statement dispatch and proc calls."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .....commands.registry import REGISTRY as _REGISTRY
from .....parsing.substitution import backslash_subst as _tcl_backslash_subst
from .....parsing.tokens import TokenType
from ....ir import (
    CommandTokens,
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRReturn,
    IRStatement,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)
from .._encoding import (
    _tcl_list_quote,
)
from .._imports import (
    _CMD_RUNTIME,
    _DICT_SUBCMD_IMPORT,
    _RUNTIME_IMPORTS,
    _SCOPE_NOP_COMMANDS,
    _STRING_IS_IMPORT,
    _STRING_SUBCMD_IMPORT,
)
from .._ir import (
    DiagSite,
    ValType,
    WasmOp,
)
from .._parsing import (
    _parse_array_ref,
)


class _WasmEmitterStmtMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_prepare_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_push_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _intern_string(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj_keep(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_frame_sync(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_frame_readback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_namespace_eval_bridge(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterExprMixin
        def _emit_expr(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_expr_obj(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCtrlMixin
        def _emit_if(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_for(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_while(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_foreach(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_switch(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_catch(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_try(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_catch_from_args(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCmdMixin
        def _emit_cmd_return(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_proc_call(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_runtime(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_uplevel(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_upvar(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_variable(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_lassign(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_info_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_clock_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_array_subcmd_value(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_stmt(self, stmt: IRStatement) -> None:
        """Emit WASM for a single IR statement."""
        # Record this statement's source location so any nested trap
        # emission (eval fallback, unsupported-command trap) can stamp
        # a sidecar diag site with a meaningful file:line:col.
        self._record_stmt_context(stmt)
        match stmt:
            case IRAssignConst(name=name, value=value):
                self._emit_obj_literal(value)
                self._emit_var_write_obj(name)
                if self._optimise and name not in self._aliases:
                    # Aliased writes go to a global under a possibly-dynamic
                    # name — constant-tracking the local Tcl name would
                    # short-circuit reads that ought to see cross-proc
                    # updates to the global.
                    try:
                        self._const_map[name] = int(value)
                    except ValueError:
                        self._const_map.pop(name, None)

            case IRAssignExpr(name=name, expr=expr):
                self._emit_expr_obj(expr)
                self._emit_var_write_obj(name)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRAssignValue(name=name, value=value):
                # value_needs_backsubst is already handled by _emit_value's
                # general backslash substitution path for non-braced literals.
                self._emit_value(value)
                self._emit_var_write_obj(name)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRIncr(name=name, amount=amount):
                self._emit_var_read_obj(name)
                self._emit_unbox_int()
                amt = 1
                if amount is not None:
                    try:
                        amt = int(amount)
                    except ValueError:
                        # Variable increment — unbox the amount variable
                        self._emit_value(amount)
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        self._emit_var_write_obj(name)
                        if self._optimise:
                            self._const_map.pop(name, None)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                self._emit_var_write_obj(name)
                if self._optimise:
                    self._const_map.pop(name, None)

            case IRExprEval(expr=expr):
                self._emit_expr(expr)
                self._emit(WasmOp.DROP)

            case IRCall(command=command, args=args, defs=defs, tokens=tokens):
                if (
                    tokens is not None
                    and tokens.expand_word is not None
                    and any(tokens.expand_word)
                    and tokens.argv_texts
                ):
                    # ``{*}`` argument expansion — reconstruct the
                    # original command text with ``{*}`` prefixes in
                    # front of each expanded word so the runtime can
                    # handle the expansion.  Using argv_texts directly
                    # without the prefix would miss the expansion.
                    ew = tokens.expand_word
                    parts = [
                        (f"{{*}}{t}" if (i < len(ew) and ew[i]) else t)
                        for i, t in enumerate(tokens.argv_texts)
                    ]
                    script = " ".join(parts)
                    self._emit_eval_fallback(command, args, script_override=script)
                    self._emit(WasmOp.DROP)
                    if self._optimise:
                        self._const_map.clear()
                else:
                    self._emit_call_stmt(command, args, defs, tokens=tokens)

            case IRReturn(value=value, expr=expr):
                if expr is not None:
                    self._emit_expr_obj(expr)
                elif value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i32_const(0)
                self._emit(WasmOp.RETURN)

            case IRBarrier(command=barrier_cmd, args=barrier_args, reason=reason):
                # Barriers are dynamic commands (eval, uplevel, trace,
                # etc.) that defeat static *analysis* but may still
                # have concrete runtime implementations we can call
                # directly.  Dispatch in priority order:
                #   1. ``uplevel`` has a dedicated emitter that shifts
                #      the frame-depth around a tcl_eval call so the
                #      body runs at a caller's scope.
                #   2. Commands in ``_CMD_RUNTIME`` have real (or stub-
                #      trapping) runtime fns — dispatch through that
                #      table so ``trace add variable …`` becomes a
                #      direct ``tcl_trace`` call with precise diag
                #      attribution instead of a generic tcl_eval
                #      fallback that would lose arity info.
                #   3. Anything else really is a black-box barrier:
                #      fall back to the interpreter.
                if barrier_cmd == "uplevel" and barrier_args:
                    self._emit_cmd_uplevel(barrier_args)
                    self._emit(WasmOp.DROP)
                elif (
                    barrier_cmd == "return"
                    and barrier_args
                    and len(barrier_args) == 3
                    and barrier_args[0] == "-code"
                    and barrier_args[1] == "error"
                ):
                    # ``return -code error <msg>`` — evaluate the
                    # message value inline so embedded ``$var`` /
                    # ``[cmd]`` substitutions resolve against the
                    # current frame, then call ``tcl_cmd_error``.
                    # Going through the eval fallback would
                    # brace-wrap the message and block the
                    # substitutions the error text needs (tcltest's
                    # error strings embed ``$option``/``$values``
                    # everywhere).  Other ``return -code …`` forms
                    # (dynamic code, break/continue, numeric) reach
                    # the fallback below where quoting is safe.
                    self._emit_cmd_return(barrier_args)
                elif barrier_cmd and barrier_cmd in _CMD_RUNTIME:
                    self._emit_cmd_runtime(barrier_cmd, barrier_args, ())
                elif barrier_cmd:
                    self._emit_eval_fallback(barrier_cmd, barrier_args)
                    self._emit(WasmOp.DROP)
                else:
                    self._emit_eval_fallback(reason)
                    self._emit(WasmOp.DROP)
                if self._optimise:
                    self._const_map.clear()

            case IRBlock(body=body, namespace=block_ns):
                # ``namespace eval`` body inlined into the enclosing
                # script.  Procs inside were already lifted to
                # ``module.procedures`` with qualified names; the
                # remaining statements run as plain code.  We stash
                # the block's namespace so ``_emit_call_stmt`` can
                # resolve unqualified command names (``Option …`` →
                # ``::tcltest::Option``) for the duration of the body.
                #
                # We also push the block namespace onto the runtime
                # ``ns_current`` via ``tcl_ns_set`` so that any
                # eval-fallback inside the body (``tcl_eval("bar")``)
                # performs its ``proc_lookup`` with the correct
                # context namespace.  Without this, under
                # ``full_flush`` (``interp create``/``eval``/``delete``)
                # every unqualified call inside ``namespace eval ::foo
                # { … }`` routes through ``tcl_eval`` with
                # ``ns_current() == ::`` and fails to find
                # ``::foo::bar`` — the concrete symptom is the
                # ``ArrayDefault`` trap when sourcing tcltest.tcl.
                prev_ns = self._block_namespace
                self._block_namespace = block_ns
                ns_saved_local: int | None = None
                ns_set_idx = self._shared_imports.get("tcl_ns_set")
                ns_restore_idx = self._shared_imports.get("tcl_ns_restore")
                should_wrap_runtime_ns = (
                    ns_set_idx is not None
                    and ns_restore_idx is not None
                    and block_ns is not None
                    and block_ns != "::"
                )
                if should_wrap_runtime_ns:
                    ns_saved_local = self._add_extra_local(
                        prefix="_block_ns_saved", val_type=ValType.I64
                    )
                    ns_literal = block_ns
                    offset = self._intern_string(ns_literal)
                    encoded = ns_literal.encode("utf-8", errors="surrogatepass")
                    # _intern_string stores a 4-byte length prefix
                    # before the bytes at ``offset`` — the
                    # pointer the runtime wants is ``offset + 4``.
                    assert ns_set_idx is not None  # for type-checker
                    self._emit_i32_const(offset + 4)
                    self._emit_i32_const(len(encoded))
                    self._emit_call(ns_set_idx)
                    self._emit_local_set(ns_saved_local)
                try:
                    for stmt in body.statements:
                        self._emit_stmt(stmt)
                finally:
                    self._block_namespace = prev_ns
                if should_wrap_runtime_ns and ns_saved_local is not None:
                    assert ns_restore_idx is not None  # for type-checker
                    self._emit_local_get(ns_saved_local)
                    self._emit_call(ns_restore_idx)
                if self._optimise:
                    self._const_map.clear()

            case IRUpFrame(frame_shift=shift, body=_body, source_tokens=src_toks):
                # ``uplevel ?level? {static-body}`` — barrier-relaxed
                # form.  At runtime we still route the body through
                # ``tcl_eval`` with the frame-depth stash/restore
                # wrapper, matching ``_emit_cmd_uplevel``'s semantics.
                # Inline-IR execution in the compiled callee's frame
                # is not yet correct because the WASM compile-to-
                # compile call chain does not push runtime frames —
                # caller WASM-locals are invisible to the runtime
                # frame table regardless of what shift we apply.  A
                # follow-up pass (whole-callee ``uplevel``-passthrough
                # inlining) eliminates the frame boundary entirely by
                # splicing the callee body into the caller's IR, at
                # which point this codegen case can emit ``body.statements``
                # inline.  Until then, IRUpFrame is a semantically-
                # richer IRBarrier: optimiser passes can inspect the
                # parsed body even though codegen still defers to the
                # interpreter.
                stash_idx = self._shared_imports.get("tcl_frame_depth_stash")
                restore_idx = self._shared_imports.get("tcl_frame_depth_restore")
                eval_idx = self._shared_imports.get("tcl_eval")
                body_text = ""
                if src_toks is not None and src_toks.argv_texts:
                    # argv[0] is ``uplevel``; the trailing word is the
                    # braced body.  Use the raw source text so the
                    # interpreter parses it exactly as the user wrote.
                    body_text = src_toks.argv_texts[-1]
                if eval_idx is None:
                    # No interpreter linked (standalone test compile
                    # without the runtime) — emit nothing.  Callers
                    # that check for this case handle the absence.
                    pass
                elif stash_idx is None or restore_idx is None:
                    # Frame helpers missing — run the body without a
                    # shift.  Same degraded behaviour as the existing
                    # ``_emit_cmd_uplevel`` fallback.
                    self._emit_obj_literal(body_text)
                    self._emit_call(eval_idx)
                    self._emit(WasmOp.DROP)
                else:
                    saved_local = self._add_extra_local(
                        prefix="_upframe_saved", val_type=ValType.I32
                    )
                    self._emit_i32_const(shift)
                    self._emit_call(stash_idx)
                    self._emit_local_set(saved_local)
                    self._emit_obj_literal(body_text)
                    self._emit_call(eval_idx)
                    self._emit(WasmOp.DROP)
                    self._emit_local_get(saved_local)
                    self._emit_call(restore_idx)
                if self._optimise:
                    self._const_map.clear()

            case IRIf(clauses=clauses, else_body=else_body):
                self._emit_if(clauses, else_body)

            case IRFor(init=init, condition=condition, next=next_script, body=body):
                self._emit_for(init, condition, next_script, body)

            case IRWhile(condition=condition, body=body):
                self._emit_while(condition, body)

            case IRForeach(iterators=iterators, body=body):
                self._emit_foreach(iterators, body)

            case IRSwitch(subject=subject, arms=arms, default_body=default_body, mode=mode):
                self._emit_switch(subject, arms, default_body, mode=mode)

            case IRCatch(body=body, result_var=result_var):
                self._emit_catch(body, result_var)

            case IRTry(body=body, handlers=handlers, finally_body=finally_body):
                self._emit_try(body, handlers, finally_body)

    def _resolve_import(self, command: str) -> str | None:
        """Resolve an unqualified ``command`` via ``namespace import``.

        Consults the resolved import table built by the codegen
        driver: for each candidate context namespace (the active
        ``namespace eval`` block, then the enclosing proc's namespace,
        then global), look up the short name and return the
        fully-qualified target if it points at a known proc.

        Imports recorded in a specific namespace only apply when
        we're compiling inside that namespace, matching Tcl's
        lexical resolution.  The global import table is always
        consulted last so top-level ``namespace import ::tcltest::*``
        still lets a bare ``test`` call resolve when compiling a
        top-level statement.
        """
        if not self._proc_imports:
            return None
        # Probe the most-specific context first, then widen.
        contexts: list[str] = []
        if self._block_namespace and self._block_namespace != "::":
            contexts.append(self._block_namespace)
        if self._proc_namespace and self._proc_namespace != "::":
            contexts.append(self._proc_namespace)
        contexts.append("::")
        seen: set[str] = set()
        for ctx in contexts:
            if ctx in seen:
                continue
            seen.add(ctx)
            table = self._proc_imports.get(ctx)
            if table is None:
                continue
            qn = table.get(command)
            if qn is not None and qn in self._proc_index:
                return qn
        return None

    def _resolve_proc_qname(self, command: str) -> str | None:
        """Resolve ``command`` to the qualified proc name if it matches."""
        if command.startswith("::"):
            return command if command in self._proc_index else None
        if self._block_namespace and self._block_namespace != "::":
            qn = f"{self._block_namespace}::{command}"
            if qn in self._proc_index:
                return qn
        if self._proc_namespace and self._proc_namespace != "::":
            qn = f"{self._proc_namespace}::{command}"
            if qn in self._proc_index:
                return qn
        if f"::{command}" in self._proc_index:
            return f"::{command}"
        if command in self._proc_index:
            return command
        # Final step: consult ``namespace import`` mappings so a bare
        # call like ``test …`` resolves to ``::tcltest::test`` when
        # the caller executed ``namespace import ::tcltest::*``.
        return self._resolve_import(command)

    def _resolve_proc(self, command: str) -> tuple[int, int] | None:
        """Look up a user-defined proc by name, returning (func_idx, n_params) or None.

        Resolution order for an *unqualified* name (Tcl namespace-path
        semantics, simplified):
          1. ``<active block namespace>::<name>`` — inside a
             ``namespace eval ::ns { … }`` body.
          2. ``<enclosing proc namespace>::<name>`` — when compiling
             ``::tcltest::workingDirectory``'s body, a bare call to
             ``AcceptAbsolutePath`` must first try
             ``::tcltest::AcceptAbsolutePath``.
          3. ``::<name>`` — the global namespace.
          4. Bare ``<name>`` — defensive fallback for non-standard
             proc-index entries.
        """
        if command.startswith("::"):
            return self._proc_index.get(command)
        # Inside a namespace-eval block, try the block's namespace first.
        if self._block_namespace and self._block_namespace != "::":
            ns_qname = f"{self._block_namespace}::{command}"
            hit = self._proc_index.get(ns_qname)
            if hit is not None:
                return hit
        # Try the enclosing proc's own namespace — captured in
        # ``_proc_namespace`` at emitter construction time.
        if self._proc_namespace and self._proc_namespace != "::":
            ns_qname = f"{self._proc_namespace}::{command}"
            hit = self._proc_index.get(ns_qname)
            if hit is not None:
                return hit
        direct = self._proc_index.get(f"::{command}") or self._proc_index.get(command)
        if direct is not None:
            return direct
        # Final step: consult ``namespace import`` mappings so bare
        # calls after ``namespace import ::tcltest::*`` (etc.)
        # dispatch directly instead of falling back to ``tcl_eval``.
        imported = self._resolve_import(command)
        if imported is not None:
            return self._proc_index.get(imported)
        return None

    def _emit_call_stmt(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...] = (),
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Emit a command invocation.

        Dispatches known commands to inline WASM or imported runtime
        functions.  Unknown commands emit NOP.

        Proc calls are resolved first so that a user-defined proc
        named ``puts`` shadows the built-in runtime import, matching
        Tcl's command resolution semantics.

        *tokens* — original parsed tokens for the command invocation.
        Threaded through to user-proc calls so they can distinguish
        braced (``{…}``) args from unbraced ones (braced args must
        skip backslash / interpolation substitution, per Tcl
        semantics).
        """
        # <upvar-invalidate> — synthetic IRCall emitted by the CFG builder
        # to invalidate caller-side SSA defs around a call that modifies
        # variables via upvar.  No code to emit at the WASM level.
        if command == "<upvar-invalidate>":
            return

        # <cond> — synthetic IRCall emitted by the CFG builder in front
        # of an ``if`` dispatch whose condition contains a command
        # substitution that defines variables (``[catch { ... } result]``
        # is the canonical example).  The actual condition evaluation
        # happens via the block's branch terminator — this placeholder
        # only exists to carry the ``defs`` list for SSA reasoning, and
        # emits no code.
        if command == "<cond>":
            return

        # global varName — register variable as global-scoped
        if command == "global":
            for var_name in args:
                self._globals.add(var_name)
                # Pre-load the global value into the local
                gget_idx = self._shared_imports.get("tcl_global_get")
                if gget_idx is not None:
                    local_idx = self._intern_local(var_name)
                    self._emit_obj_literal(var_name)
                    self._emit_call(gget_idx)
                    self._emit_local_set(local_idx)
            return

        # upvar ?level? otherVar myVar ?otherVar myVar ...? — register
        # a local alias so subsequent reads/writes route through the
        # target scope.  Only ``#0`` (global alias) is currently supported.
        if command == "upvar":
            self._emit_cmd_upvar(args)
            return

        # variable name ?value? ?name value ...? — in a namespace proc,
        # aliases local ``name`` to ``::ns::name`` in the enclosing
        # namespace and optionally initialises it.
        if command == "variable":
            self._emit_cmd_variable(args)
            return

        # Scope declarations are NOPs in the WASM model — EXCEPT when
        # the proc / namespace command has a dynamic name that must be
        # resolved at runtime (tcltest's ``Option`` does ``proc
        # $varName body`` to install accessor procs, and the compile-
        # time registry never learns about them).  Route those through
        # the eval fallback so the interpreter's ``proc`` handler
        # registers under the current namespace.
        if command in _SCOPE_NOP_COMMANDS:
            if command == "proc" and args and (args[0].startswith("$") or args[0].startswith("[")):
                self._emit_eval_fallback(command, args)
                self._emit(WasmOp.DROP)
                return
            # ``namespace eval ns arg1 arg2 ...`` in statement context
            # with dynamic script args: build the script at WASM level
            # (so compiled-frame aliases like $arr($key) are resolved
            # correctly) and call tcl_eval for side effects.
            if command == "namespace" and args and args[0] == "eval" and len(args) > 2:
                # Bridge drops the result since we're in statement context.
                # If imports are missing the bridge returns False and we
                # silently skip (statement context has no stack commitments).
                self._emit_namespace_eval_bridge(args[2:], drop_result=True)
                return
            # ``namespace import`` / ``namespace export`` / ``namespace
            # forget`` — record the side effect at runtime so the
            # interpreter's ``ns_import`` / ``ns_export`` / ``ns_forget``
            # creates real redirects / export patterns.  The compile-time
            # resolver (``module.namespace_imports`` / ``namespace_exports``)
            # still shortens specialised calls when the proc index is live,
            # but under full flush (``interp create`` / ``eval`` /
            # ``delete``) that table is cleared and every call routes
            # through ``tcl_eval`` — the runtime then needs the import
            # redirect to resolve ``testConstraint`` → ``::tcltest::testConstraint``.
            #
            # Only emit for subcommands with real runtime side effects.
            # ``namespace eval`` is handled above; ``namespace current``,
            # ``namespace which`` etc. are lookups with no side effect
            # and remain NOPs at codegen.
            if command == "namespace" and args and args[0] in ("import", "export", "forget"):
                self._emit_eval_fallback(command, args)
                self._emit(WasmOp.DROP)
                return
            return

        # break/continue — emit WASM br to exit/restart the enclosing loop.
        # Loop structure: block{ loop{ block{ <body> }; <next>; br 0 } }
        # _loop_ctrl_depths records ctrl_depth at the inner (continue) block.
        # From inside the body at ctrl_depth D, with loop_ctrl C:
        #   continue: br(D - C) exits the continue block → runs <next>
        #   break:    br(D - C + 2) exits continue block + loop + outer block
        if command == "break" and self._loop_ctrl_depths:
            loop_ctrl = self._loop_ctrl_depths[-1]
            br_depth = self._ctrl_depth - loop_ctrl + 2
            self._emit_br(br_depth)
            return
        if command == "continue" and self._loop_ctrl_depths:
            loop_ctrl = self._loop_ctrl_depths[-1]
            br_depth = self._ctrl_depth - loop_ctrl
            self._emit_br(br_depth)
            return

        # catch {body} ?resultVar? — re-parse body text and emit with
        # error-flag semantics.  The CFG builder converts IRCatch into
        # IRCall("catch", (body_text, ...)) with defs listing modified vars.
        if command == "catch" and args:
            self._emit_catch_from_args(args, defs)
            return

        # User-defined proc call takes priority over built-ins
        proc_info = self._resolve_proc(command)
        if proc_info is not None:
            qname = self._resolve_proc_qname(command)
            self._emit_cmd_proc_call(
                proc_info[0],
                proc_info[1],
                args,
                defs,
                qname=qname,
                tokens=tokens,
                invoked_name=command,
            )
            return

        # Registry-driven dispatch — covers set, incr, return, string,
        # dict, info, lassign, lset, clock, uplevel, array, unset, list,
        # and all runtime-import commands.  Each hook returns True when
        # handled and False to fall through (e.g. unset with no array elems).
        # Uses get_wasm_hook (not get_any) to scan all specs: dialect packs
        # loaded after the emitter was first imported add new specs without
        # hooks, but the hook is still on an earlier spec.
        hook = _REGISTRY.get_wasm_hook(command)
        if hook is not None:
            if hook(self, args, defs):
                return

        # Unknown command — fall back to interpreter.
        # Pre-intern defs so _emit_frame_readback (inside the fallback)
        # can reload them.  Without this, a first-time defs variable like
        # ``scan ... v`` is not yet in _tcl_var_locals and the readback
        # silently skips it, leaving the WASM local at 0.
        for _def_var in defs:
            self._intern_local(_def_var)
        self._emit_eval_fallback(command, args, tokens=tokens)
        self._emit(WasmOp.DROP)  # statement context — discard result

    def _emit_diag_site(
        self,
        command: str,
        *,
        args: tuple[str, ...] = (),
        kind: str = "fallback",
    ) -> None:
        """Register a diagnostic site and emit a ``tcl_diag_set`` call.

        The site ID is a monotonic counter scoped to the module's
        :class:`DiagMap`; ID 0 is reserved for "unset", so the first
        emitted site is ID 1.  The site stores the current statement's
        source range (populated by ``_emit_stmt``) and the supplied
        command + kind so a sidecar resolver can print a useful
        location on trap.

        When no ``diag_map`` is attached to this emitter (standalone
        tests that don't care about source mapping) the call is a
        no-op — the ``tcl_diag_set`` import is still registered, but
        we simply don't emit a call for it.
        """
        if self._diag_map is None:
            return
        diag_idx = self._shared_imports.get("tcl_diag_set")
        if diag_idx is None:
            return
        rng = self._current_range
        if rng is None:
            # Happens when a trap is emitted outside a statement context
            # (rare — e.g. the upvar preamble pre-scan).  Skip rather
            # than pointing at an unrelated site.
            return
        site_id = len(self._diag_map.sites) + 1
        site = DiagSite(
            id=site_id,
            file=self._diag_map.filename,
            line=rng.start.line + 1,  # IR is 0-based; sidecar is 1-based
            col=rng.start.character + 1,
            end_line=rng.end.line + 1,
            end_col=rng.end.character + 1,
            command=command,
            args=args,
            kind=kind,
            proc=self._proc_qname,
        )
        self._diag_map.add_site(site)
        self._emit_i32_const(site_id)
        self._emit_call(diag_idx)

    def _emit_unsupported_trap(self, command: str) -> None:
        """Emit hard trap for commands that cannot work in WASM at all.

        Used for exec, socket, coroutine, etc. — these can't be
        handled by the interpreter either.  Inside catch, error()
        sets the flag without trapping.
        """
        self._emit_diag_site(command, kind="unsupported")
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            msg = f"unsupported in WASM: {command}"
            self._emit_obj_literal(msg)
            self._emit_call(fidx)
        if self._catch_depth == 0:
            self._emit(WasmOp.UNREACHABLE)

    def _emit_eval_fallback(
        self,
        command: str,
        args: tuple[str, ...] | list[str] = (),
        *,
        script_override: str | None = None,
        tokens: "CommandTokens | None" = None,
    ) -> None:
        """Fall back to the Zig interpreter for an uncompiled command.

        Builds a command string from *command* and *args* and calls
        ``tcl_eval(script)``.  The result (i32 TclObj) is left on
        the WASM stack — caller must drop or use it as needed.

        When *script_override* is given it is used verbatim as the
        eval script instead of reconstructing it from *command* / *args*.
        Use this when the original source text must be preserved (e.g.
        for ``{*}`` argument expansion whose syntax cannot be round-tripped
        through the normal quoting path).

        *tokens* carries the original parsed word tokens when available.
        Used to distinguish braced (``{…}``, STR) arguments — whose IR
        value is the literal content — from plain / double-quoted
        (ESC) arguments whose IR value still contains raw backslash
        sequences that must be substituted before list-quoting.
        Without this distinction ``"a\\\\{b"`` (source → IR value
        ``a\\{b``) would round-trip through list-quote as though the
        two backslashes were the *final* value, producing ``a\\{b``
        instead of ``a\\{b`` after the interpreter re-parses the word.
        """
        self._emit_diag_site(command, args=tuple(args), kind="fallback")
        eval_idx = self._shared_imports.get("tcl_eval")
        if eval_idx is not None:
            if script_override is not None:
                script = script_override
            else:
                # Build command string: "command arg1 arg2 ..."
                # For literal args, concatenate them. For $var refs,
                # include the dollar sign so the interpreter can resolve them.
                def _arg_was_braced(i: int) -> bool:
                    """Return True if call-site arg *i* came from a ``{…}`` token.

                    *i* is 0-based into *args*, so ``tokens.argv[i + 1]``
                    is the corresponding parsed word (argv[0] is the
                    command name).  Requires a single-token word —
                    a concatenated word like ``{a}b`` is not purely
                    braced and needs normal processing.
                    """
                    if tokens is None or tokens.argv is None:
                        return False
                    tok_idx = i + 1
                    if tok_idx >= len(tokens.argv):
                        return False
                    if tokens.single_token_word is not None:
                        if tok_idx >= len(tokens.single_token_word):
                            return False
                        if not tokens.single_token_word[tok_idx]:
                            return False
                    return tokens.argv[tok_idx].type == TokenType.STR

                parts = [command]
                for i, a in enumerate(args):
                    if _arg_was_braced(i):
                        # Braced token — IR holds the literal content
                        # with outer ``{}`` stripped.  Re-wrap in braces
                        # so the interpreter sees the exact same word
                        # without applying any substitution.  Fall back
                        # to list-quote when the value contains an
                        # unbalanced brace (rare — ``{a{b}`` style).
                        if "{" in a or "}" in a:
                            # Use list-quote to get balanced/escaped form.
                            # Args are never at command-start so a leading
                            # ``#`` does not need quoting.
                            parts.append(_tcl_list_quote(a, first=False))
                        else:
                            parts.append("{" + a + "}")
                    elif a.startswith("$") or a.startswith("["):
                        # Substitution words pass through unquoted so
                        # the interpreter can resolve them at eval time.
                        parts.append(a)
                    else:
                        # Literal IR value from an ESC token (plain or
                        # double-quoted word).  The IR stores the RAW
                        # text: source-level ``\\{`` is still two bytes
                        # ``\`` + ``{``.  Apply backslash substitution
                        # so the value we embed in the script reflects
                        # what the original word would have evaluated
                        # to.  _tcl_list_quote then encodes it as a
                        # safe Tcl word that round-trips through the
                        # interpreter's word parser.
                        #
                        # Guard: if compile-time substitution would
                        # produce a Python string with isolated
                        # surrogates (``\uD83D\uDE02`` in a Tcl 9 test)
                        # we can't safely UTF-8-encode it into the WASM
                        # data segment.  Fall back to embedding the raw
                        # value verbatim so the interpreter itself
                        # handles the escape sequences at eval time.
                        prepped: str | None = None
                        if "\\" in a:
                            candidate = _tcl_backslash_subst(a)
                            try:
                                candidate.encode("utf-8")
                            except UnicodeEncodeError:
                                prepped = None  # signal: use raw path
                            else:
                                prepped = candidate
                        else:
                            prepped = a
                        # Script args never sit at command-start — pass
                        # ``first=False`` so a leading ``#`` is left alone.
                        if prepped is None:
                            parts.append(_tcl_list_quote(a, first=False))
                        else:
                            parts.append(_tcl_list_quote(prepped, first=False))
                script = " ".join(parts)
            # Sync all live proc-locals into the frame so the
            # interpreter can see them via var_resolve.  We sync
            # conservatively (every local, not just ``$name`` refs
            # in the script) because Tcl commands like ``info exists
            # x`` / ``unset x`` / ``upvar 1 x y`` take BARE identifiers
            # that we can't reliably distinguish from literal strings
            # by a simple scan.  Narrowing is a future optimisation;
            # requires a per-command argument-kind analysis.
            self._emit_frame_sync()
            # Stamp the current namespace into the interpreter's ns
            # register so dynamic ``proc $name body`` / ``variable
            # $name`` inside the fallback qualify into the enclosing
            # namespace instead of falling through to ``::``.  Only
            # applies inside compiled procs that have a namespace
            # other than global.
            ns_saved_idx: int | None = None
            if self._is_proc and self._proc_namespace and self._proc_namespace != "::":
                ns_set_idx = self._shared_imports.get("tcl_ns_set")
                if ns_set_idx is not None:
                    ns_saved_idx = self._add_extra_local(prefix="_ns_saved", val_type=ValType.I64)
                    # ns name without the leading ``::`` — the
                    # interpreter's qualify_name prepends ``::`` if
                    # needed; keep the full ``::ns`` form for
                    # consistency with how the compiler emits
                    # qualified names elsewhere.
                    ns_literal = self._proc_namespace
                    # Stash namespace bytes in the data section and
                    # push (ptr, len).  Reuse the string-constant
                    # pool so multiple fallbacks in the same proc
                    # share the bytes.
                    offset = self._intern_string(ns_literal)
                    encoded = ns_literal.encode("utf-8", errors="surrogatepass")
                    self._emit_i32_const(offset + 4)
                    self._emit_i32_const(len(encoded))
                    self._emit_call(ns_set_idx)
                    self._emit_local_set(ns_saved_idx)
            self._emit_obj_literal(script)
            self._emit_call(eval_idx)
            # Reload locals from frame — eval may have modified them (e.g.
            # ``set x 99``, writes through ``upvar`` aliases, ``unset``).
            self._emit_frame_readback()
            # Restore the caller's namespace context.
            if ns_saved_idx is not None:
                ns_restore_idx = self._shared_imports.get("tcl_ns_restore")
                if ns_restore_idx is not None:
                    # Stash eval result before calling ns_restore
                    # (which returns void) and put it back after.
                    result_tmp = self._add_extra_local(prefix="_eval_result", val_type=ValType.I32)
                    self._emit_local_set(result_tmp)
                    self._emit_local_get(ns_saved_idx)
                    self._emit_call(ns_restore_idx)
                    self._emit_local_get(result_tmp)
            return
        # No interpreter available — hard trap
        fidx = self._shared_imports.get("tcl_error")
        if fidx is not None:
            msg = f"unsupported in WASM: {command}"
            self._emit_obj_literal(msg)
            self._emit_call(fidx)
        self._emit(WasmOp.UNREACHABLE)

    def _emit_call_stmt_tail(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...] = (),
    ) -> None:
        """Emit a command invocation in tail position, keeping its i32 result on the stack.

        Used for implicit return: the last command's result becomes the
        proc's return value.  Dispatch order matches ``_emit_call_stmt``
        (proc calls first, then built-ins) to ensure consistent behaviour.
        """
        # Scope declarations produce no value — return null TclObj
        if command in _SCOPE_NOP_COMMANDS:
            # ``namespace eval ns arg1 arg2 ...`` in tail position with dynamic
            # script args: assemble the script at WASM level and call tcl_eval
            # so the result becomes the proc's return value.
            if command == "namespace" and args and args[0] == "eval" and len(args) > 2:
                if self._emit_namespace_eval_bridge(args[2:], drop_result=False):
                    return
                # Runtime imports missing — push null TclObj as fallback.
                self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # <upvar-invalidate> — synthetic CFG-builder invalidation node.
        # In tail position (rare, but possible if placed last by optimisation)
        # we still need to push a null TclObj.
        if command == "<upvar-invalidate>":
            self._emit_i32_const(0)
            return

        # upvar/variable in tail position — run the alias-setup side effects,
        # then push null (upvar returns empty string).
        if command == "upvar":
            self._emit_cmd_upvar(args)
            self._emit_i32_const(0)
            return
        if command == "variable":
            self._emit_cmd_variable(args)
            self._emit_i32_const(0)
            return

        # catch in tail position — emit body with error handling,
        # leave the catch return code (0 or 1) on the stack.
        if command == "catch" and args:
            self._emit_catch_from_args(args, defs, keep_on_stack=True)
            return

        # User-defined proc call — keep i32 result
        proc_info = self._resolve_proc(command)
        if proc_info is not None:
            func_idx, n_params = proc_info
            # Stash the invoked word for the callee's
            # ``info level 0`` argv0 — see
            # :meth:`_emit_prepare_pending_argv0`.
            argv0_local = self._emit_prepare_pending_argv0(command)
            # Push exactly n_params args (truncate surplus, pad missing)
            for i in range(min(n_params, len(args))):
                self._emit_value(args[i])
            for _ in range(n_params - len(args)):
                self._emit_i32_const(0)
            self._emit_push_pending_argv0(argv0_local)
            self._emit_call(func_idx)
            # i32 result stays on the stack
            return

        # set: inline local operations (returns the i32 TclObj)
        if command == "set" and 1 <= len(args) <= 2:
            var = args[0]
            # Inside ``namespace eval ::ns { ... }`` the
            # fast-local-path is wrong — unqualified writes must
            # land in ``::ns::<name>``.  Route through the full
            # write path (which consults ``_block_namespace``
            # and emits ``tcl_global_set``).
            in_ns_block = (
                not self._is_proc
                and self._block_namespace is not None
                and self._block_namespace != "::"
                and _parse_array_ref(var) is None
            )
            if var in self._aliases and len(args) >= 2:
                # Aliased write: route through global set, which returns the value.
                self._emit_value(args[1])
                self._emit_var_write_obj_keep(var)
                return
            if var in self._aliases:
                # Aliased read: tcl_global_get leaves value on stack.
                self._emit_var_read_obj(var)
                return
            if in_ns_block:
                if len(args) >= 2:
                    self._emit_value(args[1])
                    self._emit_var_write_obj_keep(var)
                else:
                    self._emit_var_read_obj(var)
                return
            idx = self._intern_local(var)
            if len(args) >= 2:
                self._emit_value(args[1])
                self._emit_local_tee(idx)
            else:
                self._emit_local_get(idx)
            return

        # incr: returns the new value as i32 TclObj (Tcl semantics)
        if command == "incr" and 1 <= len(args) <= 2:
            var = args[0]
            in_ns_block = (
                not self._is_proc
                and self._block_namespace is not None
                and self._block_namespace != "::"
                and _parse_array_ref(var) is None
            )
            _incr_array_ref = _parse_array_ref(var)
            _incr_base = _incr_array_ref[0] if _incr_array_ref else var
            if var in self._aliases or _incr_base in self._aliases or in_ns_block:
                # Aliased or namespace-scoped incr: load via the
                # global table, add, store back.
                self._emit_var_read_obj(var)
                self._emit_unbox_int()
                amt = 1
                if len(args) >= 2:
                    try:
                        amt = int(args[1])
                    except ValueError:
                        self._emit_value(args[1])
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        self._emit_var_write_obj_keep(var)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                self._emit_var_write_obj_keep(var)
                return
            idx = self._intern_local(var)
            self._emit_local_get(idx)
            self._emit_unbox_int()
            amt = 1
            if len(args) >= 2:
                try:
                    amt = int(args[1])
                except ValueError:
                    self._emit_value(args[1])
                    self._emit_unbox_int()
                    self._emit(WasmOp.I64_ADD)
                    self._emit_box_int()
                    self._emit_local_tee(idx)
                    return
            self._emit_i64_const(amt)
            self._emit(WasmOp.I64_ADD)
            self._emit_box_int()
            self._emit_local_tee(idx)
            return

        # return: emit WASM return (handled at call site, but can appear in tail)
        if command == "return":
            if args:
                self._emit_value(args[0])
            else:
                self._emit_i32_const(0)
            return

        # info sub-commands — keep result on stack
        if command == "info" and args:
            self._emit_info_value(args)
            return

        # lassign — keep leftover-list result on stack
        if command == "lassign" and args:
            self._emit_cmd_lassign(args, defs, keep_on_stack=True)
            return

        # clock — keep timer result on stack
        if command == "clock" and args:
            self._emit_clock_value(args)
            return

        # array subcommand in tail position — keep result on stack
        if command == "array" and args:
            self._emit_array_subcmd_value(args)
            return

        # uplevel in tail position — keep eval result on stack.
        if command == "uplevel" and args:
            self._emit_cmd_uplevel(args)
            return

        # string sub-commands — keep result on stack
        if command == "string" and args:
            subcmd = args[0]
            # Handle "string is <class> <value>" in tail position
            if subcmd == "is" and len(args) >= 3:
                is_key = _STRING_IS_IMPORT.get(args[1])
                if is_key is not None and is_key in self._shared_imports:
                    func_idx = self._shared_imports[is_key]
                    self._emit_value(args[-1])
                    self._emit_call(func_idx)
                    return
            import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not spec[3]:
                    self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # dict sub-commands — keep result on stack
        if command == "dict" and args:
            subcmd = args[0]
            import_key = _DICT_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in self._shared_imports:
                func_idx = self._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = args[1:]
                # For dict set, first arg is the variable name
                if subcmd == "set" and len(sub_args) >= 3:
                    var_idx = self._intern_local(sub_args[0])
                    self._emit_local_get(var_idx)  # dict value
                    self._emit_value(sub_args[1])  # key
                    self._emit_value(sub_args[2])  # value
                    self._emit_call(func_idx)
                    self._emit_local_tee(var_idx)
                else:
                    for i in range(min(param_count, len(sub_args))):
                        self._emit_value(sub_args[i])
                    for _ in range(param_count - len(sub_args)):
                        self._emit_i32_const(0)
                    self._emit_call(func_idx)
                    if not spec[3]:
                        self._emit_i32_const(0)
                return
            self._emit_i32_const(0)
            return

        # Runtime command — use the same dispatch logic as non-tail,
        # but keep the return value on the stack instead of dropping it.
        if command in _CMD_RUNTIME:
            import_key, _ = _CMD_RUNTIME[command]
            fidx = self._shared_imports.get(import_key)
            if fidx is not None:
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                mutates_var = command in ("append", "lappend")
                if mutates_var and len(args) >= 2:
                    var_name = args[0]
                    is_aliased = var_name in self._aliases or (
                        "(" in var_name and var_name.split("(")[0] in self._aliases
                    )
                    if is_aliased:
                        # Route through alias-aware global table; leave the
                        # final updated value on the stack for implicit return.
                        last = len(args) - 1
                        for i, value_arg in enumerate(args[1:], start=1):
                            self._emit_var_read_obj(var_name)
                            self._emit_value(value_arg)
                            self._emit_call(fidx)
                            if i == last:
                                self._emit_var_write_obj_keep(var_name)
                            else:
                                self._emit_var_write_obj(var_name)
                    else:
                        var_idx = self._intern_local(var_name)
                        # Loop: each value_arg gets concatenated / appended
                        # in order.  After the last one we tee — the final
                        # updated value is left on the stack for implicit
                        # return.
                        last = len(args) - 1
                        for i, value_arg in enumerate(args[1:], start=1):
                            self._emit_local_get(var_idx)
                            self._emit_value(value_arg)
                            self._emit_call(fidx)
                            if i == last:
                                self._emit_local_tee(var_idx)
                            else:
                                self._emit_local_set(var_idx)
                elif command == "puts":
                    if args:
                        self._emit_value(args[-1])
                    else:
                        self._emit_i32_const(0)
                    self._emit_call(fidx)
                    if not spec[3]:
                        self._emit_i32_const(0)
                else:
                    for i in range(min(param_count, len(args))):
                        self._emit_value(args[i])
                    for _ in range(param_count - len(args)):
                        self._emit_i32_const(0)
                    self._emit_call(fidx)
                    if not spec[3]:
                        self._emit_i32_const(0)
                return

        # Unknown command in tail position — fall back to interpreter,
        # leaving the result on the stack for implicit return.
        self._emit_eval_fallback(command, args)

    # -- Global variable write-through --
