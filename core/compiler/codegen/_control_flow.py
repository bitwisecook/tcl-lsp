"""Mixin: catch/try/error handling control flow."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ...commands.registry import REGISTRY
from ..expr_ast import ExprBinary, ExprNode
from ..ir import IRCall, IRStatement
from ..tcl_expr_eval import eval_tcl_expr
from .opcodes import Op

if TYPE_CHECKING:
    from ._emitter import _Emitter

# Avoid circular import — use string annotation for BinOp
from ..expr_ast import BinOp


class _ControlFlowMixin:
    """Mixin: catch/try/error handling."""

    def _emit_catch_inline(
        self: _Emitter,
        body_text: str,
        result_var: str | None,
        options_var: str | None,
    ) -> None:
        """Emit inline ``beginCatch4``/``endCatch`` bytecodes for ``catch``.

        Compiles the body as a single command inline, then emits the
        normal/handler paths and stores result/options variables.

        The catch return code is left on the stack as the value of
        the ``catch`` expression.
        """
        # Strip outer braces from body text.
        body = body_text.strip()
        if body.startswith("{") and body.endswith("}"):
            body = body[1:-1]

        # Pre-intern result_var so it gets a lower LVT slot than
        # variables allocated inside the catch body (e.g. try/on
        # handler vars and temps).  This matches tclsh slot order.
        if result_var and self._is_proc and not self._is_qualified(result_var):
            self._lvt.intern(result_var)

        # beginCatch4 with current nesting depth.
        self._emit(Op.BEGIN_CATCH4, self._catch_depth)
        self._catch_depth += 1

        # Compile body command with startCommand wrapping.
        self._emit_catch_body(body)

        # Normal completion: push code "0".
        self._push_lit("0")

        if options_var:
            # 3-arg catch: normal path jumps to shared pushReturnOpts.
            opts_label = self._fresh_label("catch_opts")
            self._emit(Op.JUMP1, opts_label)
        else:
            # 2-arg catch: jump over handler to endCatch.
            end_label = self._fresh_label("catch_end")
            self._emit(Op.JUMP1, end_label)

        # Handler entry: push caught result and return code.
        self._emit(Op.PUSH_RESULT)
        self._emit(Op.PUSH_RETURN_CODE)

        # For 3-arg catch: pushReturnOpts shared by both paths.
        if options_var:
            self._place_label(opts_label)
            self._emit(Op.PUSH_RETURN_OPTS)

        # endCatch.
        if not options_var:
            self._place_label(end_label)
        self._catch_depth -= 1
        self._emit(Op.END_CATCH)

        # Stack: [result, code] (or [result, code, opts] for 3-arg).
        if options_var:
            # Store opts, then swap result/code.
            self._store_var(options_var)
            self._emit(Op.POP)

        self._emit(Op.REVERSE, 2)

        # Store result in result_var (2-arg or 3-arg catch).
        if result_var:
            self._store_var(result_var)
            self._emit(Op.POP)
        else:
            # Discard result; code remains on stack.
            self._emit(Op.POP)

        # Return code stays on the stack as value of catch.

    def _emit_catch_body(self: _Emitter, body: str) -> None:
        """Compile a single-command catch body inline.

        Built-in compiled commands (return, error, break, continue) get
        wrapped in ``startCommand``; generic ``invokeStk1`` calls do not
        — the invoke opcode already handles command counting.
        """
        from ...parsing.expr_parser import parse_expr as _parse_expr_ast

        body_parts = self._parse_cmd_parts(body)
        if not body_parts:
            # Empty body: push empty string result.
            self._push_lit("")
            return

        body_cmd = body_parts[0][0]
        body_args = body_parts[1:]

        # Built-in commands that compile to specific opcodes need
        # startCommand wrapping; generic invokeStk1 calls do not.
        if REGISTRY.is_needs_start_cmd(body_cmd):
            sc_label = self._fresh_label("catch_body_end")
            self._emit(Op.START_CMD, sc_label, 1)
            self._cmd_index += 1
        else:
            sc_label = None

        match body_cmd:
            case "return":
                self._emit_catch_return(body_args)
            case "error":
                self._emit_catch_error(body_args)
            case "break":
                self._emit(Op.BREAK)
            case "continue":
                self._emit(Op.CONTINUE)
            case "expr" if len(body_args) == 1:
                expr_text = body_args[0][0]
                node = _parse_expr_ast(expr_text)
                ct_error = self._detect_const_expr_error(node)
                if ct_error is not None:
                    msg, opts = ct_error
                    self._push_lit(msg)
                    self._push_lit(opts)
                    self._emit(Op.SYNTAX, 1, 0)
                else:
                    self._emit_expr(node)
            case "try" if (
                self._is_proc
                and len(body_args) == 5
                and body_args[1][0] == "on"
                and body_args[2][0] == "error"
            ):
                # ``try { body } on error {var} { handler }``
                # Compiled inline with nested beginCatch4/endCatch,
                # return code dispatch, and -during option merging.
                try_sc = self._fresh_label("catch_body_end")
                self._emit(Op.START_CMD, try_sc, 1)
                self._cmd_index += 1
                self._emit_try_on_error_inline(body_args, try_sc)
                self._place_label(try_sc)
                self._seen_generic_invoke = True
            case _:
                # Generic command call.
                self._push_lit(body_cmd)
                for arg in body_args:
                    self._emit_cmd_subst_arg(arg)
                self._emit(Op.INVOKE_STK1, 1 + len(body_args))
                self._seen_generic_invoke = True

        if sc_label is not None:
            self._place_label(sc_label)

    @staticmethod
    def _detect_const_expr_error(node: ExprNode) -> tuple[str, str] | None:
        """Detect compile-time expression errors (e.g. divide by zero).

        Returns ``(error_message, error_options_string)`` when the AST
        is a fully constant expression that would raise a known error
        at runtime.  Returns ``None`` otherwise.
        """
        match node:
            case ExprBinary(op=BinOp.DIV | BinOp.MOD, left=left, right=right):
                lv = eval_tcl_expr(left)
                rv = eval_tcl_expr(right)
                if lv is not None and rv is not None and rv == 0:
                    return (
                        "divide by zero",
                        "-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}}",
                    )
        return None

    def _emit_catch_return(self: _Emitter, args: list[tuple[str, bool]]) -> None:
        """Compile ``return ?-code C? ?-level L? ?value?`` inside a catch body."""
        code_names = {"ok": 0, "error": 1, "return": 2, "break": 3, "continue": 4}
        i = 0
        code: int | None = None
        level: int | None = None

        while i < len(args):
            flag = args[i][0]
            if flag == "-code" and i + 1 < len(args):
                code = code_names.get(args[i + 1][0])
                if code is None:
                    try:
                        code = int(args[i + 1][0])
                    except ValueError:
                        break
                i += 2
            elif flag == "-level" and i + 1 < len(args):
                try:
                    level = int(args[i + 1][0])
                except ValueError:
                    break
                i += 2
            elif flag == "--":
                i += 1
                break
            elif flag.startswith("-"):
                break
            else:
                break

        remaining = args[i:]
        value = remaining[0][0] if remaining else ""

        if code is None:
            # Simple return: code=0, level=1.
            ret_code, ret_level = 0, 1
        elif code == 1:  # error
            ret_code = 1
            ret_level = level if level is not None else 1
        elif code >= 2:  # return/break/continue
            ret_code = 0
            ret_level = level if level is not None else code
        else:
            ret_code = 0
            ret_level = level if level is not None else 1

        self._emit_value(value, interpolate=True)
        self._push_lit("")
        self._emit(Op.RETURN_IMM, ret_code, ret_level)

    def _emit_catch_error(self: _Emitter, args: list[tuple[str, bool]]) -> None:
        """Compile ``error msg ?info? ?code?`` inside a catch body."""
        if args:
            self._emit_cmd_subst_arg(args[0])  # message
        else:
            self._push_lit("")
        self._push_lit("")  # options
        self._emit(Op.RETURN_IMM, 1, 0)  # code=error, level=0

    # -- inline try/on error compilation --

    def _emit_try_on_error_inline(
        self: _Emitter,
        args: list[tuple[str, bool]],
        normal_exit: str,
    ) -> None:
        """Emit inline ``try { body } on error {var} { handler }`` bytecodes.

        Generates nested ``beginCatch4``/``endCatch`` pairs for the try
        body and handler body, return-code dispatch, and ``-during``
        option merging via ``dictSet``.  All successful paths jump to
        *normal_exit*; exception paths re-raise via ``returnStk``.
        """
        try_body_text = args[0][0]
        handler_var = args[3][0].strip()
        handler_body_text = args[4][0]

        # Allocate LVT slots in tclsh order:
        # handler var, temp (result), temp (opts).
        msg_slot = self._lvt.intern(handler_var)
        temp_result_slot = self._lvt.intern(f"#temp{self._catch_depth}")
        temp_opts_slot = self._lvt.intern(f"#temp{self._catch_depth + 1}")

        initial_depth = self._catch_depth

        # Try body exception range
        self._emit(Op.BEGIN_CATCH4, self._catch_depth)
        self._catch_depth += 1

        # Compile try body (with startCommand via _emit_catch_body).
        self._emit_catch_body(try_body_text)

        # Normal exit from try body.
        self._emit(Op.END_CATCH)
        self._emit(Op.JUMP4, normal_exit, comment="try_on")

        # Exception handler for try body.
        self._emit(Op.PUSH_RETURN_CODE)
        self._emit(Op.PUSH_RESULT)
        self._emit(Op.PUSH_RETURN_OPTS)
        self._emit(Op.END_CATCH)

        # Store return opts and result to temps.
        self._emit(Op.STORE_SCALAR1, temp_opts_slot, comment=f"temp var {temp_opts_slot}")
        self._emit(Op.POP)
        self._emit(Op.STORE_SCALAR1, temp_result_slot, comment=f"temp var {temp_result_slot}")
        self._emit(Op.POP)

        # Return code dispatch: check if code == 1 (TCL_ERROR).
        self._emit(Op.DUP)
        self._push_lit("1")
        self._emit(Op.EQ)
        no_match = self._fresh_label("try_on_nomatch")
        self._emit(Op.JUMP_FALSE4, no_match, comment="try_on")

        # Matched error handler (code == 1):
        # discard code, load result → store to handler var.
        self._emit(Op.POP)
        self._emit(Op.LOAD_SCALAR1, temp_result_slot, comment=f"temp var {temp_result_slot}")
        self._emit(Op.STORE_SCALAR1, msg_slot, comment=f'var "{handler_var}"')
        self._emit(Op.POP)

        # Handler body exception range
        self._emit(Op.BEGIN_CATCH4, self._catch_depth)
        self._catch_depth += 1

        # Compile handler body with startCommand.
        self._emit_try_handler_body(handler_body_text)

        # Normal exit from handler body.
        self._emit(Op.END_CATCH)
        self._emit(Op.JUMP4, normal_exit, comment="try_on")

        # Exception handler for handler body.
        self._emit(Op.PUSH_RESULT)
        self._emit(Op.PUSH_RETURN_OPTS)
        self._emit(Op.PUSH_RETURN_CODE)
        self._emit(Op.END_CATCH)

        # Check if handler exception code == 1 for -during merge.
        self._push_lit("1")
        self._emit(Op.EQ)
        shared_cleanup = self._fresh_label("try_on_cleanup")
        self._emit(Op.JUMP_FALSE1, shared_cleanup)

        # -during merge: handler_opts[-during] = original_opts.
        self._emit(Op.LOAD_SCALAR1, temp_opts_slot, comment=f"temp var {temp_opts_slot}")
        self._emit(Op.REVERSE, 2)
        self._emit(Op.STORE_SCALAR1, temp_opts_slot, comment=f"temp var {temp_opts_slot}")
        self._emit(Op.POP)
        self._push_lit("-during")
        self._emit(Op.REVERSE, 2)
        self._emit(Op.DICT_SET, 1, temp_opts_slot, comment=f"temp var {temp_opts_slot}")

        # Shared cleanup: reverse result/opts and re-raise.
        self._place_label(shared_cleanup)
        self._emit(Op.REVERSE, 2)
        self._emit(Op.RETURN_STK)
        self._emit(Op.JUMP4, normal_exit, comment="try_on")

        # No match (code != 1): re-raise with original result/opts.
        self._place_label(no_match)
        self._emit(Op.POP)
        self._emit(Op.LOAD_SCALAR1, temp_opts_slot, comment=f"temp var {temp_opts_slot}")
        self._emit(Op.LOAD_SCALAR1, temp_result_slot, comment=f"temp var {temp_result_slot}")
        self._emit(Op.RETURN_STK)

        # Restore catch depth (two sequential ranges consumed).
        self._catch_depth = initial_depth

    def _emit_try_handler_body(self: _Emitter, body_text: str) -> None:
        """Compile a try/on handler body inline with ``startCommand``."""
        parts = self._parse_cmd_parts(body_text)
        if not parts:
            self._push_lit("")
            return

        cmd = parts[0][0]
        cmd_args = parts[1:]

        sc_label = self._fresh_label("handler_body_end")
        self._emit(Op.START_CMD, sc_label, 1)
        self._cmd_index += 1

        match cmd:
            case "set" if len(cmd_args) == 2:
                # ``set var value`` → load value, store to var.
                self._emit_cmd_subst_arg(cmd_args[1])
                self._store_var(cmd_args[0][0])
            case _:
                # Generic command.
                self._push_lit(cmd)
                for a in cmd_args:
                    self._emit_cmd_subst_arg(a)
                self._emit(Op.INVOKE_STK1, 1 + len(cmd_args))

        self._place_label(sc_label)

    # -- inline try/finally compilation --

    def _emit_try_finally_inline(
        self: _Emitter,
        try_body_name: str,
        try_finally_name: str,
    ) -> None:
        """Emit inline ``beginCatch4``/``endCatch`` bytecodes for
        ``try { body } finally { cleanup }``.

        Compiles both the try body and finally body inline, with the
        full exception-handling epilogue (``-during`` option merging,
        ``returnStk`` re-raise).
        """
        body_blk = self._cfg.blocks[try_body_name]
        finally_blk = self._cfg.blocks[try_finally_name]

        # try body
        self._emit(Op.BEGIN_CATCH4, self._catch_depth)
        self._catch_depth += 1

        for stmt in body_blk.statements:
            self._emit_try_body_stmt(stmt)

        # Transition: jump over pushResult (normal path), then both
        # paths converge at pushReturnOpts + endCatch.
        conv = self._fresh_label("try_conv")
        self._emit(Op.JUMP1, conv)
        self._emit(Op.PUSH_RESULT)
        self._place_label(conv)
        self._emit(Op.PUSH_RETURN_OPTS)
        self._emit(Op.END_CATCH)

        # finally body
        self._emit(Op.BEGIN_CATCH4, self._catch_depth)
        self._catch_depth += 1

        for stmt in finally_blk.statements:
            self._emit_try_finally_stmt(stmt)

        # Normal exit from finally: endCatch, pop finally result,
        # jump past the exception handler to the normal exit path.
        self._emit(Op.END_CATCH)
        self._emit(Op.POP)
        normal_exit = self._fresh_label("try_normal")
        self._emit(Op.JUMP1, normal_exit)

        # Exception handler for the finally body.
        self._emit(Op.PUSH_RESULT)
        self._emit(Op.PUSH_RETURN_OPTS)
        self._emit(Op.PUSH_RETURN_CODE)
        self._emit(Op.END_CATCH)

        # Check if return code == 1 (TCL_ERROR): merge ``-during``
        # option from the original error into the finally error's
        # return options dict.
        self._push_lit("1")
        self._emit(Op.EQ)
        shared_cleanup = self._fresh_label("try_cleanup")
        self._emit(Op.JUMP_FALSE1, shared_cleanup)

        # -during option merging (only when original was an error).
        self._push_lit("-during")
        self._emit(Op.OVER, 3)
        self._emit(Op.LIST, 2)
        self._emit(Op.LIST_CONCAT)

        # Shared cleanup: discard original result/opts, re-raise
        # the finally body's error.
        self._place_label(shared_cleanup)
        self._emit(Op.REVERSE, 4)
        self._emit(Op.POP)
        self._emit(Op.POP)
        return_target = self._fresh_label("try_return")
        self._emit(Op.JUMP1, return_target)

        # Normal exit: swap result and return opts for returnStk.
        self._place_label(normal_exit)
        self._emit(Op.REVERSE, 2)

        # Return / re-raise (shared by exception and normal paths).
        self._place_label(return_target)
        self._emit(Op.RETURN_STK)

        # Restore catch depth (two sequential ranges consumed).
        self._catch_depth -= 2

    def _emit_try_body_stmt(self: _Emitter, stmt: IRStatement) -> None:
        """Emit a statement in try-body context.

        No ``startCommand`` wrapping.  No trailing ``pop`` — the result
        stays on the stack for the ``pushReturnOpts`` transition.
        ``error`` commands are compiled as ``returnImm`` instead of
        ``invokeStk``.
        """
        if isinstance(stmt, IRCall) and stmt.command == "error":
            # ``error msg`` → push msg, push "", returnImm +1 0
            if stmt.args:
                self._emit_value(stmt.args[0])
            else:
                self._push_lit("")
            self._push_lit("")
            self._emit(Op.RETURN_IMM, 1, 0)
        else:
            self._emit_stmt(stmt)
            # Remove trailing pop — result stays on stack.
            if self._instrs and self._instrs[-1].op == Op.POP:
                del self._instrs[-1]
        self._cmd_index += 1

    def _emit_try_finally_stmt(self: _Emitter, stmt: IRStatement) -> None:
        """Emit a statement in finally-body context.

        No ``startCommand`` wrapping.  No trailing ``pop`` — the result
        stays on stack until the ``endCatch; pop`` epilogue.
        """
        self._emit_stmt(stmt)
        # Remove trailing pop — result stays on stack.
        if self._instrs and self._instrs[-1].op == Op.POP:
            del self._instrs[-1]
        self._cmd_index += 1
