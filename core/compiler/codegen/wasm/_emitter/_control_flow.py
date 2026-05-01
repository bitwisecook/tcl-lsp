"""_WasmEmitterCtrlMixin: if/for/while/foreach/switch/catch/try."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from ....expr_ast import (
    ExprNode,
)
from ....ir import (
    IRIfClause,
    IRScript,
    IRStatement,
)
from .._ir import (
    _BLOCK_VOID,
    ValType,
    WasmOp,
)
from .._ownership import Ownership


class _WasmEmitterCtrlMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterExprMixin
        def _emit_expr(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_expr_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_literal(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_stmt(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_call_stmt_tail(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unsupported_trap(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_static_parse_error_trap(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_read_obj_lenient(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj_keep(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_if(
        self,
        clauses: tuple[IRIfClause, ...],
        else_body: IRScript | None,
    ) -> None:
        """Emit an if/elseif/else chain."""
        if not clauses:
            return

        for i, clause in enumerate(clauses):
            self._emit_expr(clause.condition)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(
                WasmOp.IF,
                bytes([_BLOCK_VOID]),
                label="if" if i == 0 else "elseif",
            )
            self._emit_script(clause.body)

            is_last = i == len(clauses) - 1
            if not is_last or else_body:
                self._emit(WasmOp.ELSE)

        if else_body:
            self._emit_script(else_body)

        # Close all if/else blocks
        for _ in clauses:
            self._emit(WasmOp.END)

    def _emit_for(
        self,
        init: IRScript,
        condition: ExprNode,
        next_script: IRScript,
        body: IRScript,
    ) -> None:
        """Emit a for loop."""
        self._emit_script(init)

        # Structure: block{ loop{ <cond>; block{ <body> }; <next>; br 0 } }
        # continue → br to inner block end (skips rest of body, runs next)
        # break → br to outer block (exits loop entirely)
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]), label="for break")  # break target
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]), label="for")  # loop restart

        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)  # break if false

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._emit_script(body)
        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        self._emit_script(next_script)
        self._emit_br(0)  # back to loop

        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end break block

    def _emit_while(self, condition: ExprNode, body: IRScript) -> None:
        """Emit a while loop."""
        # For while, continue just restarts from the condition.
        # Structure: block{ loop{ <cond>; block{ <body> }; br 0 } }
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]), label="while break")
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]), label="while")

        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._emit_script(body)
        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        # Propagate ``break`` / ``continue`` from interpreter-side
        # bodies that ran through eval-fallback.  Compiled ``break``
        # already emits a wasm ``br`` directly, so these consume calls
        # only fire after a runtime-side flow-control signal (e.g.
        # ``while 1 { dict update d k v { break } }`` — the dict-
        # update body's ``break`` flips ``break_flag`` and we need to
        # see it here to exit the wasm loop).
        self._emit_flow_consume_after_loop_body(break_label_depth=1)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

    def _emit_flow_consume_after_loop_body(self, *, break_label_depth: int) -> None:
        """Emit the ``flow_consume_break`` / ``_continue`` probes.

        ``break_label_depth`` is the relative depth of the outer
        BLOCK that ``break`` should jump to (1 inside ``LOOP{}`` for
        ``while``; 2 inside ``LOOP{ BLOCK{} }`` for ``foreach``).
        ``continue`` is satisfied by simply not skipping the
        loop-restart ``br`` that follows, so we just consume the
        flag without branching.
        """
        bbreak = self._shared_imports.get("tcl_flow_consume_break")
        bcont = self._shared_imports.get("tcl_flow_consume_continue")
        if bbreak is not None:
            self._emit_call(bbreak)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_br(break_label_depth)
            self._emit(WasmOp.END)
        if bcont is not None:
            # Drain the flag so the next iteration starts clean.
            self._emit_call(bcont)
            self._emit(WasmOp.DROP)

    def _emit_foreach(
        self,
        iterators: tuple[tuple[tuple[str, ...], str], ...],
        body: IRScript,
    ) -> None:
        """Emit a foreach loop using real list element access.

        Calls ``list_length`` to get the element count, then iterates
        with a counter calling ``list_index`` on each iteration to
        extract each per-iteration element and assign it to the
        corresponding loop variable.  ``foreach {a b} {1 2 3 4} …``
        groups elements in pairs (or n-tuples when there are n vars);
        the counter advances by ``len(first_vars)`` each iteration.

        The body runs once more when the list length isn't a multiple
        of the variable count, with the trailing variable(s) bound to
        the empty string — matching reference Tcl semantics (see
        ``foreach`` manual page).

        Internal counter/limit are i64; the loop variable and list
        are i32 TclObj pointers.
        """
        counter = self._add_extra_local("_foreach_i")
        limit = self._add_extra_local("_foreach_n")
        list_local = self._add_extra_local("_foreach_list", ValType.I32)

        first_vars, first_list = iterators[0]
        step = max(len(first_vars), 1)

        # Store the list TclObj for repeated list_index calls
        self._emit_value(first_list)
        self._emit_local_set(list_local)

        # limit = list_length(list) — unbox to i64
        llength_idx = self._shared_imports.get("tcl_list_length")
        if llength_idx is not None:
            self._emit_local_get(list_local)
            self._emit_call(llength_idx)
            self._emit_unbox_int()
        else:
            # Fallback: use integer value as count
            self._emit_local_get(list_local)
            self._emit_unbox_int()
        self._emit_local_set(limit)

        # counter = 0
        self._emit_i64_const(0)
        self._emit_local_set(counter)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]), label="foreach break")
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]), label="foreach")

        # if counter >= limit, break
        self._emit_local_get(counter)
        self._emit_local_get(limit)
        self._emit(WasmOp.I64_GE_S)
        self._emit_br_if(1)

        # For each loop variable: load list_index(list, counter+slot)
        # and assign.  When ``counter + slot >= limit`` the runtime
        # returns the empty-string TclObj, so trailing slots in the
        # last iteration bind to "" as expected.
        lindex_idx = self._shared_imports.get("tcl_list_index")
        for slot, var_name in enumerate(first_vars):
            var_local = self._intern_local(var_name)
            if lindex_idx is not None:
                self._emit_local_get(list_local)
                self._emit_local_get(counter)
                if slot > 0:
                    self._emit_i64_const(slot)
                    self._emit(WasmOp.I64_ADD)
                self._emit_box_int()  # index as TclObj
                self._emit_call(lindex_idx)
            else:
                # Fallback: use counter as element value
                self._emit_local_get(counter)
                if slot > 0:
                    self._emit_i64_const(slot)
                    self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
            # Both branches produce a fresh TclObj (tcl_list_index
            # returns obj_new_string_copy, the fallback boxes a fresh
            # int).  S2.3 migration.
            self._emit_owned_local_write(var_local, Ownership.OWNED, keep_on_stack=False)

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._emit_script(body)
        self._loop_ctrl_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        # counter += step
        self._emit_local_get(counter)
        self._emit_i64_const(step)
        self._emit(WasmOp.I64_ADD)
        self._emit_local_set(counter)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

    def _emit_switch(self, subject: str, arms, default_body, *, mode: str = "exact") -> None:
        """Emit a switch statement as a chain of if/else.

        Uses ``string_equal`` for exact matching and ``string_match``
        for glob matching.  Falls back to i64 integer comparison when
        the runtime imports are unavailable.
        """
        # Determine comparison function
        if mode == "glob":
            cmp_import = self._shared_imports.get("tcl_string_match")
        else:
            cmp_import = self._shared_imports.get("tcl_string_equal")

        if cmp_import is not None:
            # String-based comparison: subject is i32 TclObj
            subject_local = self._add_extra_local(f"_switch_{subject}", ValType.I32)
            self._emit_value(subject)
            self._emit_local_set(subject_local)

            # Build groups: consecutive fallthrough arms share a body.
            # Each group is (list_of_patterns, body).  "default" arms are
            # excluded here and handled separately after the chain.
            groups: list[tuple[list[str], IRScript]] = []
            pending: list[str] = []
            for arm in arms:
                if arm.pattern == "default":
                    continue
                pending.append(arm.pattern)
                if arm.body is not None:
                    groups.append((pending, arm.body))
                    pending = []

            group_count = len(groups)
            for g_idx, (patterns, body) in enumerate(groups):
                # Emit the first pattern comparison
                if mode == "glob":
                    self._emit_obj_literal(patterns[0])
                    self._emit_local_get(subject_local)
                else:
                    self._emit_local_get(subject_local)
                    self._emit_obj_literal(patterns[0])
                self._emit_call(cmp_import)
                # string_equal/string_match returns TclObj wrapping 0 or 1
                self._emit_unbox_int()
                self._emit(WasmOp.I32_WRAP_I64)
                # OR in any additional fallthrough patterns
                for pattern in patterns[1:]:
                    if mode == "glob":
                        self._emit_obj_literal(pattern)
                        self._emit_local_get(subject_local)
                    else:
                        self._emit_local_get(subject_local)
                        self._emit_obj_literal(pattern)
                    self._emit_call(cmp_import)
                    self._emit_unbox_int()
                    self._emit(WasmOp.I32_WRAP_I64)
                    self._emit(WasmOp.I32_OR)
                self._emit(
                    WasmOp.IF,
                    bytes([_BLOCK_VOID]),
                    label=f"switch arm group {patterns!r}",
                )
                self._emit_script(body)
                if g_idx < group_count - 1 or default_body:
                    self._emit(WasmOp.ELSE)

            if default_body:
                self._emit_script(default_body)

            # Close one IF block per group
            for _ in groups:
                self._emit(WasmOp.END)
        else:
            # Fallback: integer comparison
            subject_local = self._add_extra_local(f"_switch_{subject}")
            self._emit_value(subject)
            self._emit_unbox_int()
            self._emit_local_set(subject_local)

            arm_count = len(arms)
            for i, arm in enumerate(arms):
                if arm.body is None:
                    continue
                self._emit_local_get(subject_local)
                self._emit_literal(arm.pattern)
                self._emit(WasmOp.I64_EQ)
                self._emit(
                    WasmOp.IF,
                    bytes([_BLOCK_VOID]),
                    label=f"switch arm {arm.pattern!r}",
                )
                self._emit_script(arm.body)
                if i < arm_count - 1 or default_body:
                    self._emit(WasmOp.ELSE)

            if default_body:
                self._emit_script(default_body)

            for arm in arms:
                if arm.body is not None:
                    self._emit(WasmOp.END)

    def _emit_catch(
        self,
        body: IRScript,
        result_var: str | None,
        *,
        options_var: str | None = None,
        keep_on_stack: bool = False,
    ) -> None:
        """Emit a catch block with error-flag based error propagation.

        Inside the catch body, ``error()`` sets a flag and returns
        instead of trapping.  After each statement we check the flag
        and break out of the body block if an error occurred.

        If *keep_on_stack* is True, the catch return code (i32 TclObj
        wrapping 0 or 1) is left on the WASM stack for implicit return.
        """
        enter_idx = self._shared_imports.get("tcl_catch_enter")
        leave_idx = self._shared_imports.get("tcl_catch_leave")
        result_idx = self._shared_imports.get("tcl_catch_result")
        has_error_idx = self._shared_imports.get("tcl_catch_has_error")

        if enter_idx is None or leave_idx is None:
            self._emit_script(body)
            if result_var:
                idx = self._intern_local(result_var)
                self._emit_i32_const(0)
                self._emit_local_set(idx)
            if keep_on_stack:
                self._emit_i32_const(0)
            return

        # Enter catch scope
        self._emit_call(enter_idx)

        # Body inside a block — error check after each statement breaks out.
        # For the LAST statement, when a result variable is needed, emit it in
        # "keep result" mode and record the value via catch_set_ok_result so
        # that catch_result() returns the body's last-command value on success.
        set_ok_idx = self._shared_imports.get("tcl_catch_set_ok_result")
        capture_last = (
            result_var is not None
            and result_idx is not None
            and set_ok_idx is not None
            and len(body.statements) > 0
        )
        self._catch_depth += 1
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]), label="catch body")
        stmts = body.statements
        for i, stmt in enumerate(stmts):
            is_last = capture_last and (i == len(stmts) - 1)
            if is_last:
                # Emit in "keep result" mode, then record for catch_result().
                assert set_ok_idx is not None  # guaranteed by capture_last
                self._emit_stmt_keep_result(stmt)
                self._emit_call(set_ok_idx)  # consumes the i32 result
            else:
                self._emit_stmt(stmt)
            if has_error_idx is not None:
                self._emit_call(has_error_idx)
                self._emit_br_if(0)
        self._emit(WasmOp.END)
        self._catch_depth -= 1

        # Leave catch scope — get return code (0=OK, 1=ERROR)
        self._emit_call(leave_idx)
        if keep_on_stack:
            # Leave the i32 TclObj on stack for implicit return
            pass
        else:
            self._emit(WasmOp.DROP)

        # Store result/error message if requested
        if result_var and result_idx is not None:
            rv_idx = self._intern_local(result_var)
            self._emit_call(result_idx)
            self._emit_local_set(rv_idx)
            # At top level, also mirror to the global table so reads
            # of ``$result_var`` — which under the Phase 4.5 fix go
            # through ``tcl_global_get`` — return the catch result.
            # Inside procs, the local mirror is the canonical location.
            if not self._is_proc and "tcl_global_set" in self._shared_imports:
                gset_idx = self._shared_imports["tcl_global_set"]
                self._emit_obj_literal(result_var)
                self._emit_local_get(rv_idx)
                self._emit_call(gset_idx)
                self._emit(WasmOp.DROP)

        # Store options dict if requested (3-arg ``catch`` form).
        # ``catch_options`` builds the standard ``-code`` /
        # ``-level`` / ``-errorcode`` / ``-errorinfo`` dict from the
        # last-catch state captured by ``catch_leave``; subsequent
        # ``dict get $opt -errorcode`` reads observe the right code.
        if options_var:
            opts_idx = self._shared_imports.get("tcl_catch_options")
            if opts_idx is not None:
                ov_idx = self._intern_local(options_var)
                self._emit_call(opts_idx)
                self._emit_local_set(ov_idx)
                if not self._is_proc and "tcl_global_set" in self._shared_imports:
                    gset_idx = self._shared_imports["tcl_global_set"]
                    self._emit_obj_literal(options_var)
                    self._emit_local_get(ov_idx)
                    self._emit_call(gset_idx)
                    self._emit(WasmOp.DROP)

    def _emit_stmt_keep_result(self, stmt: "IRStatement") -> None:
        """Emit a statement leaving its i32 TclObj result on the WASM stack.

        Used by ``_emit_catch`` to capture the last body statement's value
        for the ``catch body result`` result variable.  Unlike
        ``_emit_stmt``, this variant does NOT drop the result.

        For statements that naturally produce no value (declarations,
        ``unset``, etc.) we push a null TclObj (0) as the result.
        """
        from ....ir import (
            IRAssignConst,
            IRAssignExpr,
            IRAssignValue,
            IRBarrier,
            IRCall,
            IRCatch,
            IRExprEval,
            IRIncr,
        )

        match stmt:
            case IRCall(command=command, args=args, defs=defs, tokens=tokens):
                if (
                    tokens is not None
                    and tokens.expand_word is not None
                    and any(tokens.expand_word)
                    and tokens.argv_texts
                ):
                    # {*} expansion — eval fallback leaves result on stack.
                    ew = tokens.expand_word
                    parts = [
                        (f"{{*}}{t}" if (i < len(ew) and ew[i]) else t)
                        for i, t in enumerate(tokens.argv_texts)
                    ]
                    script = " ".join(parts)
                    self._emit_eval_fallback(command, args, script_override=script)
                    # result is on stack; no DROP here
                else:
                    self._emit_call_stmt_tail(command, args, defs)
            case IRBarrier(command=barrier_cmd, args=barrier_args, reason=reason):
                # Static parse-error barriers (e.g. ``if {…} else {…}
                # elseif {…}``) emit a direct ``tcl_cmd_error`` call so
                # the WASM backend reports the same diagnostic as the VM
                # / C Tcl.  Inside catch the trap sets ``error_flag`` /
                # ``error_msg`` instead of unwinding; the catch wrapper
                # then routes the message to the result variable.  We
                # push a null result to satisfy the keep-result protocol;
                # the value never reaches ``catch_set_ok_result`` because
                # the catch's per-statement ``has_error`` check breaks
                # out of the body block before it runs (see issue #259).
                from ._statements import _static_parse_error_message  # noqa: PLC0415

                parse_error_msg = _static_parse_error_message(
                    barrier_cmd, reason, tuple(barrier_args)
                )
                if parse_error_msg is not None:
                    self._emit_static_parse_error_trap(barrier_cmd or "", parse_error_msg)
                    self._emit_i32_const(0)
                elif barrier_cmd:
                    self._emit_eval_fallback(barrier_cmd, barrier_args)
                    # result is on stack; no DROP
                else:
                    self._emit_eval_fallback(reason)
                    # result is on stack; no DROP
            case IRExprEval(expr=expr):
                self._emit_expr(expr)
                self._emit_box_int()
            case IRCatch(body=inner_body, result_var=inner_result_var, options_var=inner_options_var):
                # Nested ``catch`` as the last stmt of the outer body —
                # emit with ``keep_on_stack=True`` so the inner catch's
                # return code (0 or 1) lands on the stack for the outer
                # catch_set_ok_result to latch.  Without this branch the
                # default case below would fall through to _emit_stmt
                # (which drops the inner result) and then push a null,
                # turning ``catch {catch {error x} m1} m2`` into
                # ``m2 == ""``.
                self._emit_catch(
                    inner_body,
                    inner_result_var,
                    options_var=inner_options_var,
                    keep_on_stack=True,
                )
            case IRIncr(name=name, amount=amount):
                # Issue #262: route through ``tcl_incr`` for the
                # strict-integer guard (rejects float strings).
                # Lenient read so an unset scalar initialises to 0
                # (Tcl 8.5+: ``incr x`` returns 1, doesn't raise).
                incr_idx = self._shared_imports.get("tcl_incr")
                if incr_idx is not None:
                    self._emit_var_read_obj_lenient(name)
                    if amount is None:
                        self._emit_i64_const(1)
                        self._emit_box_int()
                    else:
                        try:
                            self._emit_i64_const(int(amount))
                            self._emit_box_int()
                        except ValueError:
                            self._emit_value(amount)
                    self._emit_call(incr_idx)
                    self._emit_var_write_obj_keep(name)
                else:
                    # Mimic the incr emitter (keep value on stack)
                    self._emit_var_read_obj(name)
                    self._emit_unbox_int()
                    amt: int | None = 1
                    if amount is not None:
                        try:
                            amt = int(amount)
                        except ValueError:
                            self._emit_value(amount)
                            self._emit_unbox_int()
                            self._emit(WasmOp.I64_ADD)
                            amt = None  # signal: already handled
                    if amt is not None:
                        self._emit_i64_const(amt)
                        self._emit(WasmOp.I64_ADD)
                    self._emit_box_int()
                    self._emit_var_write_obj_keep(name)
            case IRAssignConst(name=name, value=value):
                # ``set x 42`` in tail position — store + keep the
                # assigned object on the stack so ``catch_set_ok_result``
                # records the new value.  Without this, ``catch {set x
                # 42} msg`` emitted a null and ``msg`` came back empty.
                self._emit_obj_literal(value)
                self._emit_var_write_obj_keep(name)
                if self._optimise and name not in self._aliases:
                    try:
                        self._const_map[name] = int(value)
                    except ValueError:
                        self._const_map.pop(name, None)
            case IRAssignValue(name=name, value=value):
                # ``set x $something`` — evaluate the RHS, store, keep.
                self._emit_value(value)
                self._emit_var_write_obj_keep(name)
                if self._optimise:
                    self._const_map.pop(name, None)
            case IRAssignExpr(name=name, expr=expr):
                self._emit_expr_obj(expr)
                self._emit_var_write_obj_keep(name)
                if self._optimise:
                    self._const_map.pop(name, None)
            case _:
                # All other statement types (declarations, scoping, etc.):
                # emit normally, then push a null result.
                self._emit_stmt(stmt)
                self._emit_i32_const(0)

    def _emit_catch_from_args(
        self,
        args: tuple[str, ...],
        defs: tuple[str, ...],
        *,
        keep_on_stack: bool = False,
    ) -> None:
        """Emit catch from the CFG's IRCall("catch", (body_text, ...)).

        Re-parses the body text to IR and delegates to ``_emit_catch``.
        If *keep_on_stack* is True, the catch return code (i32 TclObj)
        is left on the WASM stack for implicit return.
        """
        body_text = args[0].strip() if args else ""
        # Strip outer braces preserved by _split_command_subst so
        # lower_to_ir receives the raw script, not "{script}".
        if body_text.startswith("{") and body_text.endswith("}"):
            body_text = body_text[1:-1]
        result_var = args[1] if len(args) >= 2 else None
        options_var = args[2] if len(args) >= 3 else None

        if not body_text:
            enter_idx = self._shared_imports.get("tcl_catch_enter")
            leave_idx = self._shared_imports.get("tcl_catch_leave")
            if enter_idx is not None and leave_idx is not None:
                self._emit_call(enter_idx)
                self._emit_call(leave_idx)
                if not keep_on_stack:
                    self._emit(WasmOp.DROP)
            elif keep_on_stack:
                self._emit_i32_const(0)
            return

        from ....lowering import lower_to_ir

        try:
            ir_module = lower_to_ir(body_text)
            body_script = ir_module.top_level
        except Exception:
            self._emit_unsupported_trap("catch (unparseable body)")
            return

        self._emit_catch(
            body_script,
            result_var,
            options_var=options_var,
            keep_on_stack=keep_on_stack,
        )

    def _emit_try(self, body: IRScript, handlers, finally_body) -> None:
        """Emit a try block (simplified — just run the body + finally)."""
        self._emit_script(body)
        if finally_body:
            self._emit_script(finally_body)

    def _emit_script(self, script: IRScript) -> None:
        """Emit all statements in a script."""
        for stmt in script.statements:
            self._emit_stmt(stmt)
