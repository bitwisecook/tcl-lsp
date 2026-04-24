"""_WasmEmitterCmdMixin: Tcl command emitters."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .....commands.registry import EmitContext
from .....parsing.tokens import TokenType
from ....ir import (
    CommandTokens,
)
from .._imports import (
    _RUNTIME_IMPORTS,
    runtime_import_for,
)
from .._ir import (
    WasmOp,
)
from ._ops import _is_end_relative_index


class _WasmEmitterCmdMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_args_list(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_prepare_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_push_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _has_embedded_subst(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unsupported_trap(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_diag_site(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_array_name_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _is_frame_only_var(self, *a: Any, **kw: Any) -> Any: ...

    def _runtime_call_end(
        self,
        spec: tuple,
        defs: tuple[str, ...],
        context: EmitContext,
    ) -> None:
        """Finish a runtime-dispatched call — store, drop, or keep on stack.

        ``spec`` is the ``_RUNTIME_IMPORTS`` entry for the import just
        called.  ``spec[3]`` is non-empty when the Zig export returns a
        value.  Behaviour per context:

        * ``STATEMENT`` + result + ``defs`` → store into ``defs[0]``
        * ``STATEMENT`` + result + no ``defs`` → drop
        * ``VALUE`` + result → leave on stack
        * ``VALUE`` + no result → push ``i32.const 0`` (null TclObj) to
          fill the slot the caller expects
        """
        has_result = bool(spec[3])
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
        """Emit a call to an imported runtime function for a known command.

        In ``STATEMENT`` context the result is stored into ``defs[0]`` (if
        present) or dropped.  In ``VALUE`` context the i32 TclObj result
        stays on the operand stack for an implicit-return / expression
        callers; void-result imports get ``i32.const 0`` pushed to fill
        the slot.  Mutator commands (``append``/``lappend``) write back
        with ``_emit_local_tee`` / ``_emit_var_write_obj_keep`` so the
        final updated value is the one left on the stack.
        """
        rimp = runtime_import_for(command)
        if rimp is None:
            if context is EmitContext.VALUE:
                # Fall through to the interpreter so the implicit-return
                # slot gets a real i32 TclObj — matches the pre-B.11
                # tail-context behaviour.
                self._emit_eval_fallback(command, args)
            else:
                self._emit_unsupported_trap(command)
            return
        import_key = rimp.import_key
        nontrapping = rimp.nontrapping

        func_idx = self._shared_imports.get(import_key)
        if func_idx is None:
            if context is EmitContext.VALUE:
                # Same reasoning as above — an unregistered import
                # in tail position falls back to eval so the stack
                # stays balanced for the caller.
                self._emit_eval_fallback(command, args)
            else:
                self._emit_unsupported_trap(command)
            return

        # Record a diag site for runtime-dispatched commands whose
        # stubs can trap (``unsupported command: X`` from I/O / FS /
        # event / coroutine stubs, ``regexp`` on bad patterns, dict /
        # clock error paths, etc.) so stderr's ``tcl trap: site=<id>``
        # line resolves to the right source location.  Commands flagged
        # ``nontrapping`` (currently ``puts`` and ``append``) are total
        # for every arg shape the codegen emits and never raise into
        # ``tcl_diag``, so the per-call ``tcl_diag_set`` preamble (~4
        # WASM bytes + one DiagSite record) is pure overhead for them.
        if not nontrapping:
            self._emit_diag_site(command, args=args, kind="runtime")

        spec = _RUNTIME_IMPORTS[import_key]
        param_count = len(spec[2])

        # For commands like append/lappend that mutate a variable,
        # the first arg is the variable name; the runtime receives the
        # current variable value + one new value and returns the
        # updated value.  ``append x a b c`` concatenates a, b, c onto
        # x in order; ``lappend x a b c`` appends each as a separate
        # element.  Both loop per-value and store the running result
        # back between each call.
        mutates_var = command in ("append", "lappend")
        if mutates_var and len(args) >= 2:
            var_name = args[0]
            is_aliased = var_name in self._aliases or (
                "(" in var_name and var_name.split("(")[0] in self._aliases
            )
            keep_last = context is EmitContext.VALUE
            last_index = len(args) - 1
            if is_aliased:
                # Variable (or its array base) is an upvar/variable alias —
                # reads and writes must go through the alias-aware global table
                # rather than a WASM local slot so that updates reach the
                # aliased storage.  In VALUE context, the last write keeps the
                # updated value on the stack for implicit return.
                for i, value_arg in enumerate(args[1:], start=1):
                    self._emit_var_read_obj(var_name)
                    self._emit_value(value_arg)
                    self._emit_call(func_idx)
                    if keep_last and i == last_index:
                        self._emit_var_write_obj_keep(var_name)
                    else:
                        self._emit_var_write_obj(var_name)
            else:
                var_idx = self._intern_local(var_name)
                for i, value_arg in enumerate(args[1:], start=1):
                    self._emit_local_get(var_idx)  # current value
                    self._emit_value(value_arg)  # value to append
                    self._emit_call(func_idx)
                    if keep_last and i == last_index:
                        self._emit_local_tee(var_idx)
                    else:
                        self._emit_local_set(var_idx)
            return

        # ``list`` handled outside _emit_cmd_runtime — see
        # ``_emit_list_value`` for the N-arg build.

        # ``format`` takes a format string + up to 3 arg TclObjs —
        # fewer args get zero-padded so the runtime sees empty slots.
        if command == "format":
            if not args:
                self._emit_i32_const(0)
                self._emit_i32_const(0)
                self._emit_i32_const(0)
                self._emit_i32_const(0)
            else:
                self._emit_value(args[0])
                for slot in range(1, 4):
                    if slot < len(args):
                        self._emit_value(args[slot])
                    else:
                        self._emit_i32_const(0)
            self._emit_call(func_idx)
            self._runtime_call_end(spec, defs, context)
            return

        # fconfigure takes a fd + a variable number of ``-option value``
        # pairs; the runtime fn has a 2-arg signature (fd, opts_obj),
        # so pack args[1:] into a single space-joined string literal
        # here and hand the resulting TclObj to the stub.  For
        # arguments that are variable references (``$value``) we
        # can't fold at compile time; in that case we emit a
        # ``tcl_concat`` chain at runtime.  Most fconfigure call
        # sites in the wild use only literal option names and
        # literal values, so the fast path is common.
        if command == "fconfigure":
            if not args:
                self._emit_i32_const(0)
                self._emit_i32_const(0)
            else:
                self._emit_value(args[0])
                rest = args[1:]
                if not rest:
                    self._emit_i32_const(0)
                elif all(not a.startswith("$") and not a.startswith("[") for a in rest):
                    self._emit_obj_literal(" ".join(rest))
                else:
                    # Mixed literals + refs — build via repeated
                    # ``tcl_concat(acc, " word")``.  tcl_concat is
                    # always imported via the lifecycle set.
                    concat_idx = self._shared_imports.get("tcl_concat")
                    if concat_idx is None:
                        self._emit_obj_literal(" ".join(rest))
                    else:
                        self._emit_obj_literal(rest[0])
                        for word in rest[1:]:
                            self._emit_obj_literal(" ")
                            self._emit_call(concat_idx)
                            self._emit_value(word)
                            self._emit_call(concat_idx)
            self._emit_call(func_idx)
            self._runtime_call_end(spec, defs, context)
            return

        # For puts, handle optional channel argument: puts ?-nonewline? ?channelId? string
        if command == "puts":
            # ``puts -nonewline <string>`` dispatches to a newline-
            # suppressing runtime helper.  Channel-id forms (e.g.
            # ``puts stdout foo``) still fall through to the default
            # tcl_cmd_puts call.  Both paths return the empty string
            # so VALUE context sees ``i32.const 0`` pushed by
            # ``_runtime_call_end`` (the Zig exports are marked void).
            nonewline = len(args) >= 2 and args[0] == "-nonewline"
            if nonewline:
                no_nl_idx = self._shared_imports.get("tcl_puts_nonewline")
                if no_nl_idx is not None:
                    self._emit_value(args[-1])
                    self._emit_call(no_nl_idx)
                    self._runtime_call_end(spec, defs, context)
                    return
            # Use the last argument as the string value
            if args:
                self._emit_value(args[-1])
            else:
                self._emit_i32_const(0)
            self._emit_call(func_idx)
            self._runtime_call_end(spec, defs, context)
            return

        # concat: variadic — Tcl concat trims whitespace from each arg and
        # joins non-empty results with a single space.
        if command == "concat":

            def _concat_is_lit(a: str) -> bool:
                return (
                    not a.startswith("$")
                    and not a.startswith("[")
                    and not self._has_embedded_subst(a)
                    and a not in self._aliases
                    and a not in self._local_index
                )

            all_lits = all(_concat_is_lit(a) for a in args)
            if not args:
                self._emit_obj_literal("")
            elif all_lits:
                # Compile-time: trim each IR value (already de-braced and
                # backslash-substituted by the lexer), drop empties, join.
                parts = [a.strip() for a in args]
                self._emit_obj_literal(" ".join(p for p in parts if p))
            else:
                # Mixed literals + runtime values: chain tcl_cmd_concat calls.
                # tcl_cmd_concat trims each arg's whitespace before joining.
                self._emit_value(args[0])
                for a in args[1:]:
                    self._emit_value(a)
                    self._emit_call(func_idx)
            self._runtime_call_end(spec, defs, context)
            return

        # ``linsert list index v1 ?v2 ...?`` with multiple values —
        # ``tcl_cmd_list_insert`` is a single-value export, so we chain
        # per-value inserts at the same index.  The iteration order
        # depends on how the index resolves against the growing list:
        #
        #   * Numeric indices stay pinned to the same position through
        #     each insert, so inserting in *reverse* value order
        #     produces the correct forward layout
        #     (``{before} v1 v2 … vN {after}``).
        #   * ``end`` / ``end-N`` indices re-resolve after every insert
        #     (``end`` moves as the list grows).  Here *forward* value
        #     order is correct because each iteration's index lands at
        #     ``end-N + (i-1)`` — right after the previously inserted
        #     value.
        #
        # The textual shape of the index tells us which strategy to
        # use at compile time; a ``$var`` index whose runtime value is
        # an ``end-N`` string would be mis-ordered, but that is an
        # uncommon shape we accept for now.
        if command == "linsert" and len(args) > param_count:
            list_arg = args[0]
            index_arg = args[1]
            values = args[2:]
            self._emit_value(list_arg)
            if _is_end_relative_index(index_arg):
                iter_values = values
            else:
                iter_values = tuple(reversed(values))
            for v in iter_values:
                self._emit_value(index_arg)
                self._emit_value(v)
                self._emit_call(func_idx)
            self._runtime_call_end(spec, defs, context)
            return

        # ``lreplace list first last v1 ?v2 ...?`` with multiple values —
        # emit one ``lreplace`` for one value (consumes the range
        # [first..last] and drops it in that slot), then chain inserts
        # for the remaining values.  Same index-type-dependent ordering
        # as linsert above: numeric ``first`` uses reverse value order
        # after the base replace; ``end``-family uses forward.
        if command == "lreplace" and len(args) > param_count:
            list_arg = args[0]
            first_arg = args[1]
            last_arg = args[2]
            values = args[3:]
            list_insert_idx = self._shared_imports.get("tcl_list_insert")
            if list_insert_idx is None or not values:
                self._emit_value(list_arg)
                self._emit_value(first_arg)
                self._emit_value(last_arg)
                self._emit_value(values[0] if values else "")
                self._emit_call(func_idx)
            elif _is_end_relative_index(first_arg):
                # ``end-N`` — replace the range with the *first* value,
                # then forward-insert the rest at ``first+1``.  Because
                # ``end-N`` grows with the list, each subsequent insert
                # lands immediately after the previous.  Note we use
                # ``first`` (not ``first+1`` as compile-time arithmetic)
                # because ``end-N`` has already moved forward by one
                # after the replace grew the list.
                self._emit_value(list_arg)
                self._emit_value(first_arg)
                self._emit_value(last_arg)
                self._emit_value(values[0])
                self._emit_call(func_idx)
                for v in values[1:]:
                    self._emit_value(first_arg)
                    self._emit_value(v)
                    self._emit_call(list_insert_idx)
            else:
                # Numeric index — replace with the *last* value, then
                # insert the earlier values in reverse at ``first``.
                self._emit_value(list_arg)
                self._emit_value(first_arg)
                self._emit_value(last_arg)
                self._emit_value(values[-1])
                self._emit_call(func_idx)
                for v in reversed(values[:-1]):
                    self._emit_value(first_arg)
                    self._emit_value(v)
                    self._emit_call(list_insert_idx)
            self._runtime_call_end(spec, defs, context)
            return

        # ``lsort``/``lsearch``/``lindex``-style commands accept a
        # trailing positional ``list`` preceded by optional ``-switches``.
        # The runtime export is the no-switch form, so when extra args
        # are present pick the trailing positional as the list rather
        # than the leading ``-option`` token — otherwise ``lsort
        # -integer {3 1 2}`` would dispatch with the list ``-integer``
        # and produce the single-element result ``-integer``.
        if command in ("lsort",) and len(args) > param_count:
            self._emit_value(args[-1])
            for _ in range(param_count - 1):
                self._emit_i32_const(0)
        else:
            # Generic: push args up to param_count (all i32 TclObj pointers)
            for i in range(min(param_count, len(args))):
                self._emit_value(args[i])
            # Pad missing args with null TclObj
            for _ in range(param_count - len(args)):
                self._emit_i32_const(0)

        self._emit_call(func_idx)
        self._runtime_call_end(spec, defs, context)

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
