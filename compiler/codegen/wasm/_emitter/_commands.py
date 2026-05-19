"""_WasmEmitterCmdMixin: Tcl command emitters."""

# canonicalisation: audited #246

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from compiler.registry import EmitContext, WasmRuntimeImport
from shared.tokens import TokenType

from ....ir import (
    CommandTokens,
)
from .._imports import (
    runtime_import_for,
)
from .._ir import (
    _BLOCK_VOID,
    ValType,
    WasmOp,
)

# Commands whose runtime import is a fixed-arity helper (one-list-in,
# one-list-out shape) but whose surface accepts trailing option /
# index / value arguments.  When the call has more args than the
# helper's signature, the generic ``_emit_runtime_call`` would
# silently drop the extras and call the helper with only the leading
# args — producing the wrong answer.  This allowlist routes overflow
# calls through ``_emit_eval_fallback`` so the runtime dispatcher
# (``eval_lsort`` / ``eval_lsearch``) sees every argument and parses
# the options correctly.  Add a command here when its runtime helper
# signature can't represent the multi-arg surface.
_VARIADIC_OVERFLOW_TO_EVAL = frozenset(
    {
        "lsort",
        "::lsort",
        "lsearch",
        "::lsearch",
    }
)


class _WasmEmitterCmdMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_args_list(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_prepare_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_push_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unsupported_trap(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_diag_site(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_frame_sync(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_interp_boundary(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_frame_readback(self, *a: Any, **kw: Any) -> Any: ...

    def _runtime_prep(
        self,
        command: str,
        args: tuple[str, ...],
    ) -> tuple[int, WasmRuntimeImport] | None:
        """Resolve ``command`` to a ``(func_idx, WasmRuntimeImport)`` pair.

        Returns ``None`` when the command has no runtime-import mapping
        (unknown command) or the import wasn't registered (the scan
        phase didn't emit it).  On success, also records a diag site
        unless the import is marked ``nontrapping``.  Shared by the
        specialised hooks under ``cmds/`` — each one would otherwise
        duplicate this five-line prologue.
        """
        rimp = runtime_import_for(command)
        if rimp is None:
            return None
        func_idx = self._shared_imports.get(rimp.import_key)
        if func_idx is None:
            return None
        if not rimp.nontrapping:
            self._emit_diag_site(command, args=args, kind="runtime")
        return func_idx, rimp

    def _runtime_call_end(
        self,
        rimp: WasmRuntimeImport,
        defs: tuple[str, ...],
        context: EmitContext,
    ) -> None:
        """Finish a runtime-dispatched call — store, drop, or keep on stack.

        Reads ``rimp.results`` to know whether the import returned a
        value.  Behaviour per context:

        * ``STATEMENT`` + result + ``defs`` → store into ``defs[0]``
        * ``STATEMENT`` + result + no ``defs`` → drop
        * ``VALUE`` + result → leave on stack
        * ``VALUE`` + no result → push ``i32.const 0`` (null TclObj) to
          fill the slot the caller expects
        """
        has_result = bool(rimp.results)
        if context is EmitContext.VALUE:
            if not has_result:
                self._emit_i32_const(0)
            return
        if has_result:
            if defs:
                def_idx = self._intern_local(defs[0])
                self._emit_local_set(def_idx)
            else:
                self._emit(WasmOp.DROP)

    def _emit_cmd_runtime(
        self,
        command: str,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        context: EmitContext = EmitContext.STATEMENT,
    ) -> None:
        """Emit a generic call to an imported runtime function.

        Pushes the first ``param_count`` args (padding with null
        TclObj for any missing), calls the import, and settles the
        result via :meth:`_runtime_call_end` — store into ``defs[0]``,
        drop, or keep on stack depending on context.

        Command-specific arg shaping (variadic packing, option-string
        assembly, multi-value chains, variable mutators, trailing
        positionals past ``-switches``) lives in dedicated hooks
        under ``_emitter/cmds/<cmd>_.py`` that register via
        ``REGISTRY.register_wasm_emitter`` and win over this generic
        path.  See ``cmds/runtime_.py`` for the auto-registration
        skip logic.
        """
        rimp = runtime_import_for(command)
        if rimp is None:
            # Fall through to the interpreter regardless of context.
            # Statement-context commands (``update``, ``vwait``,
            # ``after`` standalone) without a direct runtime import
            # still need to execute via :func:`tcl_eval`; trapping
            # would defeat the purpose of declaring them in the
            # registry without a wasm_runtime_import.
            self._emit_eval_fallback(command, args)
            if context is EmitContext.STATEMENT:
                # Eval fallback leaves a TclObj on the stack; STATEMENT
                # context has no consumer, so drop it.
                self._emit(WasmOp.DROP)
            return
        func_idx = self._shared_imports.get(rimp.import_key)
        if func_idx is None:
            self._emit_eval_fallback(command, args)
            if context is EmitContext.STATEMENT:
                self._emit(WasmOp.DROP)
            return

        # Record a diag site for commands whose stubs can trap
        # (``unsupported command: X`` from I/O / FS / event / coroutine
        # stubs, ``regexp`` on bad patterns, dict / clock error paths,
        # etc.) so stderr's ``tcl trap: site=<id>`` line resolves to
        # the right source location.  ``nontrapping`` imports (``puts``,
        # ``append``) skip the per-call ``tcl_diag_set`` preamble.
        if not rimp.nontrapping:
            self._emit_diag_site(command, args=args, kind="runtime")

        param_count = len(rimp.params)

        # Probe the active call's parsed tokens (stashed by
        # ``_emit_call_stmt`` before invoking the hook) to find
        # which positional args originated from a ``{…}`` STR
        # token.  Braced words pass through with backslashes
        # preserved literally — without the probe, ``string
        # match {\?*} cmd`` would have its IR ``\?*`` value run
        # through Tcl backslash substitution at emit time, turning
        # the pattern into ``?*`` and matching anything starting
        # with any single char (so ``OptIsOpt cmd`` returns 1
        # instead of 0 — the root cause of opt-10.5/.6/.7/.8/.9/
        # .10 / opt-3.1 silently succeeding when they should
        # raise ``no value given for parameter "cmd"``).
        tokens = getattr(self, "_current_call_tokens", None)

        def _was_braced(call_arg_idx: int) -> bool:
            if tokens is None or tokens.argv is None:
                return False
            tok_idx = call_arg_idx + 1
            if tok_idx >= len(tokens.argv):
                return False
            return tokens.argv[tok_idx].type == TokenType.STR

        if command == "::apply":
            # ``apply LAMBDA ?arg ...?`` — pack tail args into a Tcl
            # list (see ``cmds/apply_.py`` for the rationale).
            self._emit_value(args[0] if args else "", was_braced=_was_braced(0))
            self._emit_args_list(tuple(args[1:]))
        elif len(args) > param_count and command in _VARIADIC_OVERFLOW_TO_EVAL:
            # Variadic call with more args than the fixed runtime
            # signature accepts — route through the eval fallback so
            # the runtime dispatcher sees every argument.  The list
            # of commands in :data:`_VARIADIC_OVERFLOW_TO_EVAL` is
            # the explicit allowlist: only commands whose
            # multi-arg form needs interpreter dispatch (lsort with
            # options, lsearch with options) get this treatment.
            # Plain commands like ``lindex`` keep the runtime-import
            # fast path because their multi-arg surface is handled
            # by the helper itself.
            self._emit_eval_fallback(command, args)
            if context is EmitContext.STATEMENT:
                self._emit(WasmOp.DROP)
            return
        else:
            for i in range(min(param_count, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            for _ in range(param_count - len(args)):
                self._emit_i32_const(0)

        self._emit_call(func_idx)
        self._runtime_call_end(rimp, defs, context)

    def _emit_compiled_call_with_bridge(
        self,
        func_idx: int,
        *,
        defs: tuple[str, ...] = (),
    ) -> None:
        """Emit a compiled-proc ``call`` with an optional frame
        sync/readback bridge.

        When the caller's escape summary is ``dynamic_barrier=True``
        we cannot prove the callee won't reach into the caller's
        frame via ``upvar``/``uplevel``.  Mirror every Tcl-visible
        WASM local into the runtime frame immediately before the
        call (so ``var_resolve`` finds the right value) and reload
        from the frame after the call (so writes the callee made
        through an upvar/uplevel land back in the caller's WASM
        slots).  Eval-fallback paths already wrap their own
        sync/readback so this helper is only relevant for direct
        compiled-to-compiled dispatch (statement context via
        :meth:`_emit_cmd_proc_call`, value context via
        ``_emit_command_subst_value``).

        *defs* are caller-side names the CFG builder marked as
        upvar-back targets — pre-interned so they appear in
        ``_tcl_var_locals`` and the readback covers them.

        Stack on entry: callee args + pending-argv0 already pushed
        and stashed; ``call func_idx`` is the only consume here.
        Stack on exit: the callee's return value (i32 TclObj).
        """
        summary = self._escape_summary
        needs_frame_bridge = (
            summary is not None
            and summary.dynamic_barrier
            and self._is_proc
            and "tcl_local_set" in self._shared_imports
        )
        if needs_frame_bridge:
            # Pre-intern any caller-side names the CFG builder
            # marked as upvar-back ``defs``.  Without this, a name
            # first introduced via the callee's upvar (e.g.
            # ``OptLengths $desc nl tl dl`` writing back into a
            # fresh ``nl``) isn't in ``_tcl_var_locals`` yet, so
            # the readback below silently skips it and the post-
            # call ``$nl`` read raises ``can't read "nl": no such
            # variable`` despite the runtime frame having the
            # value.
            for _def_var in defs:
                self._intern_local(_def_var)
            self._emit_interp_boundary("call")
        self._emit_call(func_idx)
        if needs_frame_bridge:
            # Stash the call's result before the readback rebinds
            # WASM locals — readback runs through ``tcl_local_get``
            # (lenient) for every Tcl-visible local, so the result
            # must be parked in a scratch local until we're done.
            result_tmp = self._add_extra_local(prefix="_pcall_res", val_type=ValType.I32)
            self._emit_local_set(result_tmp)
            self._emit_frame_readback()
            self._emit_local_get(result_tmp)
        # Error-flag propagation: if the callee raised an error
        # (set ``error_flag``), the caller must stop executing
        # subsequent statements and unwind back to the nearest
        # enclosing ``catch``.  Without this, ``proc Outer {} {
        # Inner; puts after-Inner }`` would still print "after-
        # Inner" when ``Inner`` errors — the WASM call returns
        # normally and the next statement runs regardless.
        self._emit_error_flag_check_and_return()

    def _emit_error_flag_check_and_return(self) -> None:
        """Emit a signal-flag check after a compiled-proc call.

        Reads ``catch_has_error`` and ``flow_check_return`` from
        the runtime — both globals are written by ``error`` /
        ``return`` (and every other path that ultimately routes
        through them).  Any signal set by the callee aborts the
        current compiled function:

        * ``error_flag`` set (and not in a compile-time catch) →
          discard the call result, push null, ``WASM.RETURN``.
          The flag stays live so an enclosing catch (or ``::top``
          trap) sees it.  Inside a catch the surrounding
          ``_emit_catch`` per-statement ``catch_has_error`` check
          handles the unwind, and an early WASM ``return`` here
          would skip ``catch_leave`` and break the catch.
        * ``return_flag`` set → ALWAYS absorb at the proc-dispatch
          boundary regardless of catch nesting, by calling
          ``flow_take_return`` (which clears the flag and yields
          ``return_val``) and treating that value as the call
          result.  Mirrors ``eval_proc_call``'s post-body
          absorption for interpreted callees.  In real Tcl,
          ``return X`` inside a proc body unwinds to the proc
          dispatch (TCL_OK to caller), so a surrounding ``catch``
          observes a normal call result — never code 2.  Skipping
          the absorption inside a compile-time catch made
          ``set y [catch {Callee} m]`` see TCL_RETURN (2)
          instead of TCL_OK (0) for any callee that ended its
          body via ``return`` through the eval-fallback path.

        At top level (``::top``) the WASM ``return`` exit is
        skipped because there's nothing to return from, but the
        ``return_flag`` absorption still runs — a callee that
        ended its body with ``return X`` (via the eval-fallback
        path) leaves the flag set, and a top-level ``catch
        {Callee}`` would otherwise see TCL_RETURN instead of
        TCL_OK.
        """
        check_ret_idx = self._shared_imports.get("tcl_flow_check_return")
        take_ret_idx = self._shared_imports.get("tcl_flow_take_return")
        has_err_idx = self._shared_imports.get("tcl_catch_has_error")
        if has_err_idx is None and check_ret_idx is None:
            return
        # Stash the call's i32 TclObj result — both checks may
        # need to either discard it (error path) or replace it
        # (return path) before handing back to the caller.
        result_tmp = self._add_extra_local(prefix="_errchk_res", val_type=ValType.I32)
        self._emit_local_set(result_tmp)
        # Return absorption runs at every callsite — proc-dispatch
        # semantics don't change inside a catch, and they also
        # apply at top level.  Leaking ``return_flag`` past the
        # dispatch causes the surrounding catch to see TCL_RETURN
        # where TCL_OK is the correct code.
        if check_ret_idx is not None and take_ret_idx is not None:
            self._emit_call(check_ret_idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_call(take_ret_idx)
            self._emit_local_set(result_tmp)
            # ``return -code return`` / ``-level N`` propagates past
            # the immediate callee: ``flow_take_return`` decrements
            # the extra-level counter and leaves ``return_flag``
            # set so the caller (the proc emitting this code) also
            # exits.  Re-check the flag after the take and ``return``
            # from the surrounding function so the next caller up
            # observes the unwind too.  Skipped at top level (no
            # enclosing function) and inside a catch (the
            # surrounding ``_emit_catch`` per-statement check
            # handles unwind via the catch absorption path).
            if self._is_proc and self._catch_depth == 0:
                self._emit_call(check_ret_idx)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                self._emit_local_get(result_tmp)
                self._emit(WasmOp.RETURN)
                self._emit(WasmOp.END)
            self._emit(WasmOp.END)
        # ``return -code break`` / ``return -code continue`` from a
        # compiled callee leaves a signal flag set.  Two cases:
        #
        # * Inside a compiled while/foreach: fall through naturally
        #   to the loop's body-end ``flow_consume_break/continue``
        #   probes, which catch ``break_flag`` / ``continue_flag``
        #   (both of which were paired with the signal flag).  No
        #   explicit branch needed — the flags stay live.  We just
        #   clear the *signal* side-channel so the next dispatch
        #   level doesn't try to translate it.
        #
        # * Outside any loop in this proc: ``return`` from the WASM
        #   function so the proc dispatcher (compiled-proc path in
        #   ``eval_proc_call_bucket`` or the matching post-dispatch
        #   stamp) translates the signal into the caller's
        #   ``break_flag`` / ``continue_flag``.  Skipped inside a
        #   catch — the surrounding ``_emit_catch`` per-statement
        #   probe handles unwind via the catch absorption path.
        sig_loop_idx = self._shared_imports.get("tcl_flow_check_signal_loop")
        if (
            sig_loop_idx is not None
            and self._is_proc
            and self._catch_depth == 0
            and self._loop_depth == 0
        ):
            self._emit_call(sig_loop_idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_local_get(result_tmp)
            self._emit(WasmOp.RETURN)
            self._emit(WasmOp.END)
        # Error propagation only applies inside a compiled proc
        # AND outside a compile-time catch — top level has no
        # function to return from, and inside a catch the
        # surrounding ``_emit_catch`` per-stmt check ``br_if`` s
        # out of the body block before ``catch_leave`` clears
        # ``error_flag``.
        if has_err_idx is not None and self._is_proc and self._catch_depth == 0:
            self._emit_call(has_err_idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            # Error path — discard result, return null.  Flag
            # stays set so the enclosing catch / outer compiled
            # frame's check fires next.
            self._emit_i32_const(0)
            self._emit(WasmOp.RETURN)
            self._emit(WasmOp.END)
        self._emit_local_get(result_tmp)

    def _emit_signal_check_and_return(self) -> None:
        """Emit a signal check after an eval-fallback's ``tcl_eval``.

        Same flag inspection as :meth:`_emit_error_flag_check_and_return`,
        but without the proc-dispatch absorption — the eval
        body lives *inside* the current proc's call, so a
        ``return`` raised by the body must propagate to the
        caller (``return`` from inside ``proc P { … set x [eval
        $body] }`` should make ``P`` return, not just terminate
        the assignment).  The eval result is left on the stack
        so the body's final value becomes the implicit ``return``
        argument, matching Tcl's ``proc P { … return $x }``
        shape.

        Skipped at top level — ``::top`` doesn't have a function
        to return from, and the runtime's outer driver handles
        flag dispatch directly.

        Skipped *inside a catch body* — the surrounding
        ``_emit_catch`` already emits a per-statement
        ``catch_has_error`` check that ``br_if`` s out of the
        body block while leaving ``catch_leave`` to clear the
        flag.  An early WASM ``return`` here would skip
        ``catch_leave`` entirely and break the catch's absorption
        contract (the next ``catch`` would see a stale
        ``error_flag`` on entry, and ``return $msg`` after a
        caught error would leak the inner error code).
        """
        if not self._is_proc:
            return
        if self._catch_depth > 0:
            return
        has_err_idx = self._shared_imports.get("tcl_catch_has_error")
        check_ret_idx = self._shared_imports.get("tcl_flow_check_return")
        if has_err_idx is None and check_ret_idx is None:
            return
        # Stash the eval result so we can return it on the signal
        # path or hand it back to the caller on the cheap path.
        result_tmp = self._add_extra_local(prefix="_evalsig_res", val_type=ValType.I32)
        self._emit_local_set(result_tmp)
        if has_err_idx is not None:
            self._emit_call(has_err_idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_i32_const(0)
            self._emit(WasmOp.RETURN)
            self._emit(WasmOp.END)
        if check_ret_idx is not None:
            self._emit_call(check_ret_idx)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            # Hand the eval result back as the proc's result —
            # ``return $x`` semantics.  Leave the flag set so the
            # caller's bridge can absorb at the proc-dispatch
            # boundary; if there's no caller (e.g. ``::top``),
            # the runtime's outer driver will see the flag.
            self._emit_local_get(result_tmp)
            self._emit(WasmOp.RETURN)
            self._emit(WasmOp.END)
        self._emit_local_get(result_tmp)

    def _emit_cmd_proc_call(
        self,
        func_idx: int,
        n_params: int,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        qname: str | None = None,
        tokens: "CommandTokens | None" = None,
        invoked_name: str | None = None,
    ) -> None:
        """Emit a direct call to a compiled procedure.

        Enforces WASM arity: pushes exactly *n_params* i32 TclObj
        pointers onto the stack — padding with the proc's declared
        defaults (via ``_proc_defaults``) for missing args, or
        boxed zero when a param has no default.  Surplus args are
        dropped (matches Tcl's behaviour for procs without ``args``
        catchall).

        When the callee has an ``args`` variadic tail (its qname is in
        ``_proc_args_tail``), all call-site args beyond the fixed
        positional count are packed into a single list TclObj and
        passed as the last argument.

        *tokens* provides the original parsed tokens for each call-site
        word, letting us distinguish braced (``{…}``) args from
        unbraced ones.  Braced args must skip backslash / interpolation
        substitution at emit time (Tcl semantics: braces suppress all
        substitution).

        *invoked_name* is the exact word the source used for the
        call (``::foo::bar``, ``bar``, ``renamed_bar``, …).  Stashed
        in the runtime's pending-argv0 slot immediately before the
        ``call`` so the callee's prologue can report it via
        ``info level 0``.  When ``None`` — callers that don't have
        the source word handy — the callee's prologue falls back
        to its qname tail.
        """
        # The callee's prologue (see :meth:`generate`) applies
        # declared defaults on null-TclObj slots, so callers just
        # pad with ``i32.const 0`` via :meth:`_emit_default_arg`
        # for missing args — no per-call-site default lookup
        # needed.
        has_args_tail = qname is not None and qname in self._proc_args_tail
        argv0_local = (
            self._emit_prepare_pending_argv0(invoked_name) if invoked_name is not None else None
        )

        def _was_braced(call_arg_idx: int) -> bool:
            """Return True if the call-site word at *call_arg_idx* was braced.

            *call_arg_idx* is 0-based into *args* (the CALLSITE args tuple),
            so ``tokens.argv`` is indexed at ``call_arg_idx + 1`` to skip
            the command-name token.
            """
            if tokens is None or tokens.argv is None:
                return False
            tok_idx = call_arg_idx + 1
            if tok_idx >= len(tokens.argv):
                return False
            return tokens.argv[tok_idx].type == TokenType.STR

        if has_args_tail and n_params > 0:
            # Fixed positional slots: first n_params-1
            fixed = n_params - 1
            for i in range(min(fixed, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            # Pad missing fixed slots with defaults / null
            for _slot in range(len(args), fixed):
                # Pad with null TclObj; the compiled-proc prologue
                # substitutes the declared default (if any) and
                # the unsubstituted null lets ``frame_set_argv``
                # report an accurate argv for ``info level 0``.
                self._emit_i32_const(0)
            # Last slot: list of all remaining call args.  Pass a
            # per-tail-arg ``was_braced`` probe so braced tokens in
            # the args tail keep their exact source bytes (no
            # backslash subst) when packed into the list.
            tail_args = args[fixed:]
            self._emit_args_list(
                tail_args,
                was_braced_fn=lambda i, base=fixed: _was_braced(base + i),
            )
        else:
            # Push arguments up to the callee's parameter count
            for i in range(min(n_params, len(args))):
                self._emit_value(args[i], was_braced=_was_braced(i))
            # Pad missing args with the declared default or boxed zero.
            # Defaults are LITERAL values from the param spec — ``{$y}``
            # and ``{[clock seconds]}`` must reach the callee unchanged,
            # *not* be substituted at call time.  Emit as an obj literal
            # so no ``$var`` / ``[cmd]`` interpolation happens.
            for _slot in range(len(args), n_params):
                # Pad with null TclObj; the compiled-proc prologue
                # substitutes the declared default (if any) and
                # the unsubstituted null lets ``frame_set_argv``
                # report an accurate argv for ``info level 0``.
                self._emit_i32_const(0)

        # Publish the caller's invoked word to the pending-argv0
        # slot right before the call — the callee's prologue
        # consumes it on entry.
        self._emit_push_pending_argv0(argv0_local)
        self._emit_compiled_call_with_bridge(func_idx, defs=defs)

        # This method is only called from statement context (_emit_call_stmt).
        # The CFG builder populates `defs` with upvar-tracked variables (variables
        # that may be mutated via upvar inside the callee) for SSA purposes, NOT
        # as assignment targets.  Always drop the return value here; the caller
        # handles assignment separately via IRAssignValue / _emit_var_write_obj.
        self._emit(WasmOp.DROP)
