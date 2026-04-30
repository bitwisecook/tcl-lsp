"""_WasmEmitterCmdMixin: Tcl command emitters."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .....commands.registry import EmitContext, WasmRuntimeImport
from .....parsing.tokens import TokenType
from ....ir import (
    CommandTokens,
)
from .._imports import (
    runtime_import_for,
)
from .._ir import (
    WasmOp,
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

        if command == "apply":
            # ``apply LAMBDA ?arg ...?`` — pack tail args into a Tcl
            # list (see ``cmds/apply_.py`` for the rationale).
            self._emit_value(args[0] if args else "")
            self._emit_args_list(tuple(args[1:]))
        else:
            for i in range(min(param_count, len(args))):
                self._emit_value(args[i])
            for _ in range(param_count - len(args)):
                self._emit_i32_const(0)

        self._emit_call(func_idx)
        self._runtime_call_end(rimp, defs, context)

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
        self._emit_call(func_idx)

        # This method is only called from statement context (_emit_call_stmt).
        # The CFG builder populates `defs` with upvar-tracked variables (variables
        # that may be mutated via upvar inside the callee) for SSA purposes, NOT
        # as assignment targets.  Always drop the return value here; the caller
        # handles assignment separately via IRAssignValue / _emit_var_write_obj.
        self._emit(WasmOp.DROP)
