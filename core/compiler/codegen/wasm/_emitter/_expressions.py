"""_WasmEmitterExprMixin: expression evaluation."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from .....parsing.substitution import backslash_subst as _tcl_backslash_subst
from ....expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from .._imports import (
    runtime_import_for,
    subcommand_runtime_import_for,
)
from .._ir import (
    _BLOCK_I64,
    _BLOCK_VOID,
    WasmOp,
)
from ._ops import _BINOP_WASM


class _WasmEmitterExprMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj_keep(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_namespace_eval_bridge(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterValuesMixin
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_args_list(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_prepare_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_push_pending_argv0(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _resolve_proc_qname(self, *a: Any, **kw: Any) -> Any: ...
        def _resolve_proc(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCtrlMixin
        def _emit_catch_from_args(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCmdMixin
        def _emit_array_subcmd_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_clock_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_info_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_uplevel(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_lassign(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_expr(self, node: ExprNode) -> None:
        """Emit WASM instructions to evaluate an expression, leaving i64 on stack.

        Expression evaluation operates on raw i64 integers.  Variables
        (i32 TclObj pointers) are unboxed via ``tcl_obj_get_int`` before
        use; the caller is responsible for boxing the result back to i32
        when storing into a local.
        """
        match node:
            case ExprLiteral(text=value):
                self._emit_literal(value)
            case ExprVar(name=name, text=text):
                # Alias-aware: reads through _emit_var_read_obj so upvar'd
                # and variable-declared vars resolve via the global table.
                # For array refs (``$a(key)``) the parser reports only the
                # base name; the ``text`` field carries the full token
                # including the ``(key)`` suffix, which _emit_var_read_obj
                # re-parses via _parse_array_ref.
                resolved = name
                if "(" in text and text.endswith(")"):
                    # Drop the leading ``$``/``${`` so _parse_array_ref
                    # sees ``name(key)``.
                    bare = text[1:] if text.startswith("$") else text
                    if bare.startswith("{") and bare.endswith("}"):
                        bare = bare[1:-1]
                    resolved = bare
                self._emit_var_read_obj(resolved)
                self._emit_unbox_int()
            case ExprBinary(op=op, left=left, right=right):
                self._emit_binary(op, left, right)
            case ExprUnary(op=op, operand=operand):
                self._emit_unary(op, operand)
            case ExprTernary(condition=cond, true_branch=te, false_branch=fe):
                self._emit_ternary(cond, te, fe)
            case ExprCall(function=func, args=args):
                self._emit_func_call(func, args)
            case ExprCommand(text=text):
                self._emit_command_subst(text)
            case ExprString(text=text):
                # Quoted string in expr context — try numeric parse
                self._emit_literal(text)
            case _:
                # Fallback for unsupported expression types (ExprRaw)
                self._emit_i64_const(0)

    # --- Float-aware arithmetic path ---
    # Maps arithmetic BinOp to the float-aware runtime function name.
    _ARITH_FLOAT_FUNC: dict[BinOp, str] = {
        BinOp.ADD: "tcl_arith_add",
        BinOp.SUB: "tcl_arith_sub",
        BinOp.MUL: "tcl_arith_mul",
        BinOp.DIV: "tcl_arith_div",
        BinOp.MOD: "tcl_arith_mod",
        # Bignum-aware ``**`` — the inline ``_emit_power`` i64 loop
        # silently wraps past the i64 boundary; routing through
        # ``tcl_arith_pow`` promotes the result to TYPE_BIGNUM via
        # ``Managed.pow`` for arbitrary precision.
        BinOp.POW: "tcl_arith_pow",
    }

    # Maps bitwise / shift BinOp to the runtime helper that enforces the
    # integer-only domain (issues #260, #261): floats raise ``cannot use
    # floating-point value …``; negative shift counts raise ``negative
    # shift argument``.  The previous inline ``i64.shl``/``i64.and`` etc.
    # path silently truncated floats and masked the shift count by 63.
    _BITWISE_INT_FUNC: dict[BinOp, str] = {
        BinOp.LSHIFT: "tcl_arith_lshift",
        BinOp.RSHIFT: "tcl_arith_rshift",
        BinOp.BIT_AND: "tcl_arith_band",
        BinOp.BIT_OR: "tcl_arith_bor",
        BinOp.BIT_XOR: "tcl_arith_bxor",
    }

    def _emit_expr_obj(self, node: ExprNode) -> None:
        """Emit WASM instructions leaving a TclObj i32 on the stack.

        Counterpart to ``_emit_expr`` (which leaves i64).  Used for
        float-aware arithmetic: tcl_arith_* take and return i32 TclObjs,
        so operands must not be unboxed to i64 beforehand.

        Falls back to ``_emit_expr`` + ``_emit_box_int`` for nodes that
        don't need special handling (e.g. comparison results, logical
        ops).
        """
        match node:
            case ExprVar(name=name, text=text):
                # Read variable as TclObj directly (no unbox).
                resolved = name
                if "(" in text and text.endswith(")"):
                    bare = text[1:] if text.startswith("$") else text
                    if bare.startswith("{") and bare.endswith("}"):
                        bare = bare[1:-1]
                    resolved = bare
                self._emit_var_read_obj(resolved)
            case ExprLiteral(text=value):
                # Emit TclObj literal.
                self._emit_obj_literal_for_expr(value)
            case ExprString(text=value):
                self._emit_obj_literal_for_expr(value)
            case ExprBinary(op=op, left=left, right=right):
                func_name = self._ARITH_FLOAT_FUNC.get(op)
                if func_name is None:
                    func_name = self._BITWISE_INT_FUNC.get(op)
                if func_name is not None:
                    func_idx = self._shared_imports.get(func_name)
                    if func_idx is not None:
                        self._emit_expr_obj(left)
                        self._emit_expr_obj(right)
                        self._emit_call(func_idx)
                        return
                # Non-arithmetic or missing import: fall back to i64 path.
                self._emit_expr(node)
                self._emit_box_int()
            case ExprUnary(op=uop, operand=operand):
                # Negation of a float must preserve the TYPE_FLOAT tag —
                # the inline ``0 - x`` path goes through i64 and loses
                # the float-ness, which would let a bitwise/shift caller
                # (e.g. ``-23.55 << 4`` or ``(-$x) << 1`` with $x = "23.55"
                # from issue #261) silently truncate the negative float
                # to an integer instead of raising the domain error.
                # Route through the runtime ``tcl_arith_neg`` helper so
                # the float tag survives to ``tcl_arith_lshift`` /
                # ``tcl_arith_bnot`` for any operand shape (literal,
                # variable, command substitution, nested expression).
                if uop == UnaryOp.NEG:
                    neg_idx = self._shared_imports.get("tcl_arith_neg")
                    if neg_idx is not None:
                        self._emit_expr_obj(operand)
                        self._emit_call(neg_idx)
                        return
                if uop == UnaryOp.BIT_NOT:
                    bnot_idx = self._shared_imports.get("tcl_arith_bnot")
                    if bnot_idx is not None:
                        self._emit_expr_obj(operand)
                        self._emit_call(bnot_idx)
                        return
                # Fall back: evaluate as i64, box.
                self._emit_expr(node)
                self._emit_box_int()
            case ExprCall(function=func, args=args):
                # Math functions that return TclObj.
                func_idx = None
                if func == "double" and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_double")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func in ("int", "entier", "wide") and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_int")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func == "round" and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_round")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func == "log" and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_log")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func == "sqrt" and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_sqrt")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func == "exp" and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_exp")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func == "log10" and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_log10")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func in ("sin", "cos", "tan", "asin", "acos", "atan",
                              "sinh", "cosh", "tanh", "floor", "ceil") and len(args) == 1:
                    # Trig + hyperbolic + floor/ceil — single-arg float
                    # math functions.  Each maps to a tcl_math_<name>
                    # runtime helper; routed through obj_context so the
                    # result stays a TYPE_FLOAT TclObj.
                    fn = "tcl_math_" + func
                    func_idx = self._shared_imports.get(fn)
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func in ("fabs", "abs") and len(args) == 1:
                    func_idx = self._shared_imports.get("tcl_math_fabs")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_call(func_idx)
                        return
                elif func in ("atan2", "fmod", "hypot") and len(args) == 2:
                    # Two-arg float math functions.
                    fn = "tcl_math_" + func
                    func_idx = self._shared_imports.get(fn)
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_expr_obj(args[1])
                        self._emit_call(func_idx)
                        return
                elif func == "pow" and len(args) == 2:
                    # ``pow(a, b)`` math-function form — route through the
                    # bignum-aware ``tcl_arith_pow`` runtime helper, which
                    # picks float / integer / bignum based on operand
                    # types.  The legacy ``_emit_power`` inline-loop
                    # truncates to i64 even when both operands are floats,
                    # so ``pow(2.1, 3.1)`` mis-rendered as 8.
                    func_idx = self._shared_imports.get("tcl_arith_pow")
                    if func_idx is not None:
                        self._emit_expr_obj(args[0])
                        self._emit_expr_obj(args[1])
                        self._emit_call(func_idx)
                        return
                # Fall through to i64 path.
                self._emit_func_call(func, args)
                self._emit_box_int()
            case ExprCommand(text=text):
                # Command substitution already returns a TclObj.
                self._emit_command_subst_obj(text)
            case _:
                # Fall back: evaluate as i64, box to TclObj.
                self._emit_expr(node)
                self._emit_box_int()

    def _emit_obj_literal_for_expr(self, value: str) -> None:
        """Emit a TclObj for a literal value in expression context."""
        new_int_idx = self._shared_imports.get("tcl_obj_new_int")
        new_float_idx = self._shared_imports.get("tcl_obj_new_float")
        new_str_idx = self._shared_imports.get("tcl_obj_new_string")
        is_integer_literal = False
        try:
            int_val = int(value, 0) if not value.lstrip("+-").isdigit() else int(value)
            is_integer_literal = True
            # Tcl 9 BigInt literals: ``expr {0x8000000000000000}`` or
            # ``expr {-9223372036854775808}`` (parsed as unary-minus
            # of 2^63) produce values outside signed i64.  Falling
            # through to the string path lets the runtime parse the
            # source bytes lazily and preserve precision; emitting
            # ``i64.const`` here would saturate to the i64 boundary
            # and silently miscompute every BigInt expression.
            if -(1 << 63) <= int_val <= (1 << 63) - 1:
                if new_int_idx is not None:
                    self._emit_i64_const(int_val)
                    self._emit_call(new_int_idx)
                else:
                    self._emit_i64_const(int_val)
                    self._emit_box_int()
                return
            # Out-of-i64 integer literal — skip the float fallback
            # entirely (``float(value)`` truncates ``2^63`` to
            # ``9.22e18`` and silently loses 11 bits of precision)
            # and emit the source bytes as a string TclObj.  The
            # runtime's ``try_parse_int`` / ``bignum.parse_i128``
            # path then promotes the value to TYPE_BIGNUM the first
            # time an arithmetic helper reads it, which is the path
            # exercised by ``expr-32.{3..9}`` and the
            # ``Tcl_NewWideIntObj`` round-trip cases.
        except ValueError:
            pass
        if not is_integer_literal:
            try:
                float_val = float(value)
                if new_float_idx is not None:
                    self._emit_f64_const(float_val)
                    self._emit_call(new_float_idx)
                else:
                    self._emit_i64_const(int(float_val))
                    self._emit_box_int()
                return
            except ValueError:
                pass
        # String literal (or out-of-i64 integer): create string TclObj
        # and let the runtime parse it as int / bignum / float on first
        # access.
        if new_str_idx is not None:
            offset = self._intern_string(value)
            encoded = value.encode("utf-8", errors="surrogatepass")
            self._emit_i32_const(offset + 4)
            self._emit_i32_const(len(encoded))
            self._emit_call(new_str_idx)
        else:
            self._emit_i64_const(0)
            self._emit_box_int()

    def _emit_command_subst_obj(self, text: str) -> None:
        """Like ``_emit_command_subst`` but leaves TclObj i32 on the stack.

        Known limitation: this routes through ``_emit_command_subst``
        (which unboxes the command result to i64) and then re-boxes
        via ``_emit_box_int``.  That round-trip preserves integer
        results exactly but loses non-integer types (float, string)
        from the original command result.  A true object-preserving
        path that dispatches known procs directly (without the i64
        hop) is tracked for a future commit; switching to a blanket
        ``_emit_eval_fallback`` here breaks recursive proc calls in
        expr context (the compiled frame sees the callee go through
        the interpreter and loses its local bindings) so the safer
        round-trip is kept for now.
        """
        self._emit_command_subst(text)
        self._emit_box_int()

    def _emit_command_subst(self, text: str) -> None:
        """Emit WASM for a command substitution in expression context.

        Parses ``[cmd arg1 arg2 ...]`` text.  If *cmd* is a known proc,
        emits a direct call and unboxes the i32 result to i64.  If it's
        ``[expr {...}]``, recursively compiles the nested expression.
        Otherwise falls back to i64.const 0.

        The result is always i64 (consistent with expression context).
        """
        # Strip surrounding brackets if present
        cmd_text = text
        if cmd_text.startswith("[") and cmd_text.endswith("]"):
            cmd_text = cmd_text[1:-1]
        cmd_text = cmd_text.strip()
        if not cmd_text:
            self._emit_i64_const(0)
            return

        # Simple split — handles most Tcl command forms.
        # For nested brackets, we rely on the expression parser having
        # already extracted the outermost command boundary.
        parts = self._split_command_subst(cmd_text)
        if not parts:
            self._emit_i64_const(0)
            return

        cmd_name = parts[0]
        cmd_args = parts[1:]

        # Handle [expr {...}] — recursively compile the expression.
        # Strip outer braces from expr_arg ({...} kept by splitter).
        if cmd_name == "expr" and len(cmd_args) == 1:
            expr_arg = cmd_args[0]
            if expr_arg.startswith("{") and expr_arg.endswith("}"):
                expr_arg = expr_arg[1:-1]
            # Parse and emit the nested expression
            from .....parsing.expr_parser import parse_expr

            try:
                nested_expr = parse_expr(expr_arg)
                self._emit_expr(nested_expr)
                return
            except Exception:
                self._emit_i64_const(0)
                return

        # [catch {body} ?varName?] in expression context — emit the
        # real catch codegen and unbox the i32 return code to i64 so
        # ``if {[catch { ... } msg]} { ... }`` tests OK vs ERROR via
        # integer comparison.  Falling through to the eval fallback
        # here would rebuild ``catch`` as a space-joined script,
        # losing the body's word boundaries when the first body word
        # starts with ``$``.
        if cmd_name == "catch" and cmd_args:
            self._emit_catch_from_args(tuple(cmd_args), defs=(), keep_on_stack=True)
            self._emit_unbox_int()
            return

        # ``[set varname]`` — 1-arg read form in expression context
        # (e.g. ``if {[set a(x)] == 1} ...``).  Mirror the value-
        # context special-case: route through ``_emit_var_read_obj``
        # and unbox to i64.  See comment in
        # ``_emit_command_subst_value``.
        if cmd_name == "set" and len(cmd_args) == 1:
            self._emit_var_read_obj(cmd_args[0])
            self._emit_unbox_int()
            return
        # ``[set varname value]`` — 2-arg write form in expression
        # context (e.g. ``[set scriptCompare [catch {...} var]] == 0``).
        # Evaluate the value, store it keeping it on the stack, then
        # unbox to i64 for the surrounding expression comparison.
        if cmd_name == "set" and len(cmd_args) == 2:
            self._emit_value(cmd_args[1])
            self._emit_var_write_obj_keep(cmd_args[0])
            self._emit_unbox_int()
            return

        # ``[namespace origin <name>]`` — compile-time qualified name;
        # mirrors the value-context handler.
        if cmd_name == "namespace" and len(cmd_args) >= 2 and cmd_args[0] == "origin":
            bare_name = cmd_args[1]
            qualified = self._resolve_proc_qname(bare_name) or bare_name
            if not qualified.startswith("::"):
                ns = self._proc_namespace or "::"
                qualified = f"{ns}::{bare_name}" if ns != "::" else f"::{bare_name}"
            self._emit_obj_literal(qualified)
            self._emit_unbox_int()
            return

        # Handle proc calls — resolve command to a known procedure
        proc_info = self._resolve_proc(cmd_name)
        if proc_info is not None:
            func_idx, n_params = proc_info
            # Stash the caller's invoked word for the callee's
            # ``info level 0`` — see :meth:`_emit_prepare_pending_argv0`.
            argv0_local = self._emit_prepare_pending_argv0(cmd_name)
            for i in range(min(n_params, len(cmd_args))):
                self._emit_value(cmd_args[i])
            # Pad missing args with declared defaults (see the mirror
            # comment in ``_emit_command_subst_value``).
            for _slot in range(len(cmd_args), n_params):
                # Pad with null TclObj; the compiled-proc prologue
                # substitutes the declared default (if any) and
                # the unsubstituted null lets ``frame_set_argv``
                # report an accurate argv for ``info level 0``.
                self._emit_i32_const(0)
            self._emit_push_pending_argv0(argv0_local)
            self._emit_call(func_idx)
            # Result is i32 TclObj — unbox to i64 for expression context
            self._emit_unbox_int()
            return

        # dict sub-command — returns i32 TclObj, unbox to i64
        if cmd_name == "dict" and cmd_args:
            subcmd = cmd_args[0]
            sri = subcommand_runtime_import_for("dict", subcmd)
            if sri is not None and sri.import_key in self._shared_imports:
                func_idx = self._shared_imports[sri.import_key]
                param_count = len(sri.params)
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if sri.results:
                    self._emit_unbox_int()
                else:
                    self._emit_i64_const(0)
                return

        # string sub-command — returns i32 TclObj, unbox to i64
        if cmd_name == "string" and cmd_args:
            subcmd = cmd_args[0]
            sri = subcommand_runtime_import_for("string", subcmd)
            if sri is not None and sri.import_key in self._shared_imports:
                func_idx = self._shared_imports[sri.import_key]
                param_count = len(sri.params)
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if sri.results:
                    self._emit_unbox_int()
                else:
                    self._emit_i64_const(0)
                return

        # info sub-command — returns i32 TclObj, unbox to i64 for exprs.
        if cmd_name == "info" and cmd_args:
            self._emit_info_value(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # lassign in expression context — returns leftover list (i32 TclObj).
        if cmd_name == "lassign" and cmd_args:
            self._emit_cmd_lassign(tuple(cmd_args), defs=(), keep_on_stack=True)
            self._emit_unbox_int()
            return

        # clock in expression context — integer timer value.
        if cmd_name == "clock" and cmd_args:
            self._emit_clock_value(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # array subcommand in expression context.
        if cmd_name == "array" and cmd_args:
            self._emit_array_subcmd_value(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # uplevel in expression context.
        if cmd_name == "uplevel" and cmd_args:
            self._emit_cmd_uplevel(tuple(cmd_args))
            self._emit_unbox_int()
            return

        # ``regexp`` / ``regsub`` with options or capture vars — same
        # rationale as the value-context branch in ``_values.py`` and
        # the statement-context hook in ``cmds/regexp_.py``: the
        # fixed-argc runtime import would silently take ``-indices``
        # as the pattern arg.  Route through eval-fallback (with
        # ``script_override``) so the dispatcher in ``cmds/regexp.zig``
        # → ``eval_regexp_cmd`` parses options correctly.
        if cmd_name in ("regexp", "regsub") and cmd_args:
            n_pos = 0
            uses_options = False
            for a in cmd_args:
                if a.startswith("-") and len(a) > 1:
                    uses_options = True
                    break
                n_pos += 1
            min_positional = 2 if cmd_name == "regexp" else 3
            if uses_options or n_pos > min_positional:
                from .cmds.regexp_ import _capture_vars_for as _cap_vars

                for vname in _cap_vars(cmd_name, tuple(cmd_args)):
                    self._intern_local(vname)
                self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)
                self._emit_unbox_int()
                return

        # Runtime command — returns i32 TclObj, unbox to i64
        rimp = runtime_import_for(cmd_name)
        if rimp is not None:
            func_idx = self._shared_imports.get(rimp.import_key)
            if func_idx is not None:
                param_count = len(rimp.params)
                if cmd_name == "apply":
                    # ``apply LAMBDA ?arg ...?`` — pack tail into a Tcl
                    # list (see _values.py for the rationale).  Strip
                    # outer braces from braced args first so we don't
                    # double-brace inside ``_emit_args_list``.
                    def _strip_braces(s: str) -> str:
                        return (
                            s[1:-1]
                            if (len(s) >= 2 and s.startswith("{") and s.endswith("}"))
                            else s
                        )

                    self._emit_value(cmd_args[0] if cmd_args else "")
                    self._emit_args_list(tuple(_strip_braces(a) for a in cmd_args[1:]))
                else:
                    for i in range(min(param_count, len(cmd_args))):
                        self._emit_value(cmd_args[i])
                    for _ in range(param_count - len(cmd_args)):
                        self._emit_i32_const(0)
                self._emit_call(func_idx)
                if rimp.results:
                    self._emit_unbox_int()
                else:
                    self._emit_i64_const(0)
                return

        # Unknown command in expression context — fall back to interpreter,
        # unbox result to i64 for expression arithmetic.  Use the original
        # command text so braced words are not mangled during reconstruction.
        self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)
        self._emit_unbox_int()

    def _split_command_subst(self, text: str) -> list[str]:
        """Split a command substitution body into command name + args.

        Handles braced arguments ``{...}`` and nested brackets ``[...]``
        as atomic units.  Braces and quotes are stripped from the
        resulting word so downstream emission sees the raw content
        (e.g. ``{5}`` becomes ``5``).

        ``\\<newline>`` is Tcl line-continuation whitespace — it
        collapses to a single space between words (TclParseAllWhiteSpace).
        Without treating it as whitespace here a bare backslash would
        leak into the argv as its own word.
        """
        parts: list[str] = []
        i = 0
        n = len(text)
        while i < n:
            # Skip whitespace, including ``\<newline>`` line-continuation.
            while i < n:
                if text[i] in " \t\n":
                    i += 1
                elif text[i] == "\\" and i + 1 < n and text[i + 1] == "\n":
                    i += 2
                else:
                    break
            if i >= n:
                break
            # Braced argument — keep outer braces so _emit_value can
            # recognise {…} words as literals (no substitution in Tcl).
            if text[i] == "{":
                depth = 1
                start = i
                i += 1
                while i < n and depth > 0:
                    # Per Tcl spec: \{ and \} inside a braced word are
                    # backslash-escaped and do NOT count for brace depth.
                    if text[i] == "\\" and i + 1 < n and text[i + 1] in "{}":
                        i += 2  # skip backslash + brace
                        continue
                    if text[i] == "{":
                        depth += 1
                    elif text[i] == "}":
                        depth -= 1
                    i += 1
                # Keep outer { } intact; _emit_value strips them.
                parts.append(text[start:i])
            # Bracketed argument (nested command) — keep brackets
            elif text[i] == "[":
                depth = 1
                start = i
                i += 1
                while i < n and depth > 0:
                    if text[i] == "[":
                        depth += 1
                    elif text[i] == "]":
                        depth -= 1
                    i += 1
                parts.append(text[start:i])
            # Quoted argument — strip outer quotes
            elif text[i] == '"':
                start = i
                i += 1
                while i < n and text[i] != '"':
                    if text[i] == "\\":
                        i += 1  # skip escaped char
                    i += 1
                i += 1  # skip closing quote
                # Strip outer " "
                parts.append(text[start + 1 : i - 1])
            # Bare word (delimited by whitespace)
            else:
                start = i
                while i < n and text[i] not in " \t\n":
                    # ``\<newline>`` terminates the word — the
                    # line-continuation collapses to a space which is a
                    # word separator.  Without this the backslash is
                    # captured as a literal byte in the word span.
                    if text[i] == "\\" and i + 1 < n and text[i + 1] == "\n":
                        break
                    # Handle nested brackets within bare words
                    if text[i] == "[":
                        depth = 1
                        i += 1
                        while i < n and depth > 0:
                            if text[i] == "[":
                                depth += 1
                            elif text[i] == "]":
                                depth -= 1
                            i += 1
                    else:
                        i += 1
                parts.append(text[start:i])
        return parts

    def _emit_literal(self, value: str) -> None:
        """Emit a literal as a raw i64 constant (for expression context).

        ``int(value, 0)`` (not bare ``int(value)``) handles Tcl's
        ``0x``/``0o``/``0b`` prefixes as well as decimal integers.  The
        old ``int(value)`` path raised ``ValueError`` on ``0x10`` and
        fell through to ``float("0x10")`` (also an error) and finally
        to ``0`` — so ``expr {0x10 + 1}`` evaluated to ``1``.
        """
        try:
            int_val = int(value, 0) if not value.lstrip("+-").isdigit() else int(value)
            self._emit_i64_const(int_val)
        except ValueError:
            try:
                float_val = float(value)
                self._emit_i64_const(int(float_val))
            except ValueError:
                # String literal in expr context — no meaningful integer value
                self._emit_i64_const(0)

    # WASM comparison ops that return i32 instead of i64
    _I32_RESULT_OPS = frozenset(
        {
            WasmOp.I64_EQ,
            WasmOp.I64_NE,
            WasmOp.I64_LT_S,
            WasmOp.I64_GT_S,
            WasmOp.I64_LE_S,
            WasmOp.I64_GE_S,
            WasmOp.I64_EQZ,
        }
    )

    def _emit_binary(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit a binary operation."""
        # Optimisation: constant folding for known-constant operands
        if self._optimise:
            folded = self._try_fold_binary(op, left, right)
            if folded is not None:
                self._emit_i64_const(folded)
                return

        # Ordering comparisons: use tcl_expr_order_cmp which tries
        # numeric first then falls back to string comparison.
        #
        # Tcl 9 semantics: ``"a" < "b"`` → 1 (lexicographic).
        # Tcl 8.x semantics: same expression raises ``expected floating-
        # point number but got "a"``.  The WASM runtime currently
        # targets Tcl 9.0 only — no dialect gating is threaded through
        # the emitter.  If 8.x compatibility is needed in future,
        # switch this branch based on ``self._target_dialect`` (not
        # yet plumbed) so the emitter falls through to the plain
        # ``I64_LT_S``/... path that errors on non-numeric operands.
        if op in (BinOp.LT, BinOp.GT, BinOp.LE, BinOp.GE):
            self._emit_expr_order_cmp(op, left, right)
            return

        # Numeric ``==`` / ``!=`` route through ``tcl_expr_order_cmp``
        # so bignum and float-vs-int operands compare correctly.  The
        # inline ``i64.eq`` path silently truncates a bignum operand
        # to its low 64 bits — ``expr {(1<<200) == (1<<200)}`` then
        # compared 0 == 0 and reported equal-by-truncation.  Routing
        # through the runtime helper picks the right rule (i64 fast
        # path for both-fit, bignum for any-bignum, float for any-
        # float, string fallback otherwise).
        if op in (BinOp.EQ, BinOp.NE) and self._shared_imports.get("tcl_expr_order_cmp") is not None:
            cmp_idx = self._shared_imports["tcl_expr_order_cmp"]
            self._emit_str_value(left)
            self._emit_str_value(right)
            self._emit_call(cmp_idx)
            self._emit_unbox_int()  # i64: -1, 0, or 1
            self._emit_i64_const(0)
            if op is BinOp.EQ:
                self._emit(WasmOp.I64_EQ)
            else:
                self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            return

        # Float-aware arithmetic: delegate to tcl_arith_* runtime helpers
        # which check at runtime whether either operand is a float and
        # produce the correct result (float or int) accordingly.
        arith_func = self._ARITH_FLOAT_FUNC.get(op)
        if arith_func is not None:
            arith_idx = self._shared_imports.get(arith_func)
            if arith_idx is not None:
                self._emit_expr_obj(left)
                self._emit_expr_obj(right)
                self._emit_call(arith_idx)  # → i32 TclObj
                self._emit_unbox_int()  # → i64
                return

        # Bitwise / shift: delegate to tcl_arith_{lshift,rshift,band,bor,bxor}
        # which enforce the integer-only domain (issues #260, #261).  The
        # previous direct ``i64.shl`` / ``i64.and`` path silently truncated
        # float operands and masked negative shift counts by 63.
        bitwise_func = self._BITWISE_INT_FUNC.get(op)
        if bitwise_func is not None:
            bw_idx = self._shared_imports.get(bitwise_func)
            if bw_idx is not None:
                self._emit_expr_obj(left)
                self._emit_expr_obj(right)
                self._emit_call(bw_idx)  # → i32 TclObj
                self._emit_unbox_int()  # → i64
                return

        wasm_op = _BINOP_WASM.get(op)
        if wasm_op is not None:
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(wasm_op)
            # WASM comparisons return i32; extend to i64 for our type model
            if wasm_op in self._I32_RESULT_OPS:
                self._emit(WasmOp.I64_EXTEND_I32_S)
        elif op == BinOp.POW:
            # Exponentiation — emit as a runtime call or loop
            self._emit_power(left, right)
        elif op in (BinOp.AND, BinOp.OR):
            self._emit_logical(op, left, right)
        elif op in (BinOp.STR_EQ, BinOp.STR_EQUALS):
            self._emit_str_eq(left, right)
        elif op == BinOp.STR_NE:
            self._emit_str_eq(left, right)
            # Negate: result XOR 1
            self._emit_i64_const(1)
            self._emit(WasmOp.I64_XOR)
        elif op in (BinOp.STR_LT, BinOp.STR_GT, BinOp.STR_LE, BinOp.STR_GE):
            self._emit_str_cmp(op, left, right)
        elif op in (BinOp.IN, BinOp.NI):
            self._emit_in(op, left, right)
        else:
            # Unsupported binary op — emit operands and drop
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(WasmOp.I64_ADD)  # placeholder

    def _emit_in(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit ``expr {x in list}`` / ``x ni list``.

        Evaluates each operand as a TclObj, calls ``tcl_cmd_list_contains``,
        and unboxes to i64. For ``ni`` the result is XORed with 1.
        """
        contains_idx = self._shared_imports.get("tcl_list_contains")
        if contains_idx is None:
            # Runtime helper missing — best effort: always false.
            self._emit_i64_const(0)
            return
        self._emit_str_value(right)  # list
        self._emit_str_value(left)  # value
        self._emit_call(contains_idx)
        self._emit_unbox_int()
        if op == BinOp.NI:
            self._emit_i64_const(1)
            self._emit(WasmOp.I64_XOR)

    def _emit_str_eq(self, left: ExprNode, right: ExprNode) -> None:
        """Emit string equality comparison — result is i64 (0 or 1).

        Uses the ``string_equal`` runtime import.  Both operands are
        emitted as i32 TclObj values, the runtime returns an i32 TclObj
        wrapping 0 or 1, which is then unboxed to i64.
        """
        seq_idx = self._shared_imports.get("tcl_string_equal")
        if seq_idx is not None:
            # Emit left and right as i32 TclObj values
            self._emit_str_value(left)
            self._emit_str_value(right)
            self._emit_call(seq_idx)
            # string_equal returns TclObj wrapping 0/1 — unbox to i64
            self._emit_unbox_int()
        else:
            # Fallback: integer comparison
            self._emit_expr(left)
            self._emit_expr(right)
            self._emit(WasmOp.I64_EQ)
            self._emit(WasmOp.I64_EXTEND_I32_S)

    def _emit_str_cmp(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit a string ordering comparison — result is i64 (0 or 1).

        Calls ``string_compare(a, b)`` which returns a TclObj wrapping
        -1, 0, or 1, unboxes to i64, then compares with 0.
        """
        cmp_idx = self._shared_imports.get("tcl_string_compare")
        if cmp_idx is None:
            self._emit_i64_const(0)
            return
        self._emit_str_value(left)
        self._emit_str_value(right)
        self._emit_call(cmp_idx)
        self._emit_unbox_int()  # i64: -1, 0, or 1
        self._emit_i64_const(0)
        match op:
            case BinOp.STR_LT:
                self._emit(WasmOp.I64_LT_S)
            case BinOp.STR_GT:
                self._emit(WasmOp.I64_GT_S)
            case BinOp.STR_LE:
                self._emit(WasmOp.I64_LE_S)
            case BinOp.STR_GE:
                self._emit(WasmOp.I64_GE_S)
        self._emit(WasmOp.I64_EXTEND_I32_S)

    def _emit_expr_order_cmp(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit an ordering comparison using tcl_expr_order_cmp.

        Tries numeric comparison first; falls back to string comparison when
        either operand is non-numeric (Tcl 9 semantics for < > <= >=).
        """
        cmp_idx = self._shared_imports.get("tcl_expr_order_cmp")
        if cmp_idx is None:
            # Fallback: plain integer comparison
            self._emit_expr(left)
            self._emit_expr(right)
            wasm_op = _BINOP_WASM.get(op)
            if wasm_op is not None:
                self._emit(wasm_op)
                self._emit(WasmOp.I64_EXTEND_I32_S)
            else:
                self._emit_i64_const(0)
            return
        self._emit_str_value(left)
        self._emit_str_value(right)
        self._emit_call(cmp_idx)
        self._emit_unbox_int()  # i64: -1, 0, or 1
        self._emit_i64_const(0)
        match op:
            case BinOp.LT:
                self._emit(WasmOp.I64_LT_S)
            case BinOp.GT:
                self._emit(WasmOp.I64_GT_S)
            case BinOp.LE:
                self._emit(WasmOp.I64_LE_S)
            case BinOp.GE:
                self._emit(WasmOp.I64_GE_S)
        self._emit(WasmOp.I64_EXTEND_I32_S)

    def _emit_str_value(self, node: ExprNode) -> None:
        """Emit an expression as an i32 TclObj pointer (for string ops).

        For ExprVar and ExprRaw, emits as a variable read.
        For ExprLiteral, creates a boxed string/int.
        For other nodes, evaluates as i64 and boxes.
        """
        from ....expr_ast import ExprCommand, ExprLiteral, ExprRaw, ExprString, ExprVar

        match node:
            case ExprVar(text=text):
                # Array ref in value context: ``$a(key)`` — strip the
                # leading ``$`` / ``${`` / trailing ``}`` and route
                # through the array-aware _emit_var_read_obj path.
                if "(" in text and text.endswith(")"):
                    bare = text[1:] if text.startswith("$") else text
                    if bare.startswith("{") and bare.endswith("}"):
                        bare = bare[1:-1]
                    self._emit_var_read_obj(bare)
                    return
                var = self._resolve_var_name(text)
                if var is not None:
                    self._emit_var_read_obj(var)
                    return
            case ExprCommand(text=text):
                # Command substitution ``[cmd ...]`` — evaluate via
                # _emit_value to get a TclObj string pointer directly.
                # Going through the i64 fallback path loses the string
                # representation (empty string → int 0 → "0").
                self._emit_value(text)
                return
            case ExprRaw(text=text):
                # ``ExprRaw`` carries the literal source text of a
                # subject / operand that the expression parser
                # couldn't classify further — commonly the subject
                # of a ``switch`` whose CFG builder wraps it as
                # ``ExprBinary(STR_EQ, ExprRaw(subject), ...)``.
                # Route through ``_emit_value`` so ``[cmd …]``
                # command substitutions and ``"text $var"``
                # interpolated strings get evaluated, not emitted
                # as the raw literal source.  The old path only
                # handled bare ``$var`` references via
                # ``_resolve_var_name``, so ``switch -- [llength
                # $m] {…}`` compared ``"[llength $m]"`` (the
                # literal string) against each arm's pattern and
                # always fell through to the ``default`` branch.
                self._emit_value(text)
                return
            case ExprLiteral(text=text):
                self._emit_obj_literal(text)
                return
            case ExprString(text=text):
                # Quoted string literal — strip outer quotes/braces and
                # apply backslash substitution for double-quoted strings.
                inner = text
                if len(inner) >= 2:
                    if inner[0] == '"' and inner[-1] == '"':
                        inner = inner[1:-1]
                        if "\\" in inner:
                            inner = _tcl_backslash_subst(inner)
                    elif inner[0] == "{" and inner[-1] == "}":
                        inner = inner[1:-1]
                self._emit_obj_literal(inner)
                return
        # Fallback: evaluate as TclObj (preserves TYPE_BIGNUM /
        # TYPE_FLOAT through nested arithmetic).  The previous path
        # routed through ``_emit_expr`` + ``_emit_box_int``, which
        # unboxed any bignum result to i64 and re-boxed as TYPE_INT
        # — silently truncating ``expr {99 < (1<<70)}`` to compare
        # 99 against ``i64::MAX`` instead of the real ``2^70``.
        # ``_emit_expr_obj`` keeps the TclObj intact so the receiving
        # ``tcl_expr_order_cmp`` (or string op) sees the bignum tag.
        self._emit_expr_obj(node)

    def _emit_power(self, base: ExprNode, exp: ExprNode) -> None:
        """Emit integer exponentiation as a loop."""
        base_local = self._add_extra_local("_pow_base")
        exp_local = self._add_extra_local("_pow_exp")
        result_local = self._add_extra_local("_pow_result")

        # result = 1
        self._emit_i64_const(1)
        self._emit_local_set(result_local)

        # base_local = base
        self._emit_expr(base)
        self._emit_local_set(base_local)

        # exp_local = exp
        self._emit_expr(exp)
        self._emit_local_set(exp_local)

        # loop: while exp > 0
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

        # if exp <= 0, break
        self._emit_local_get(exp_local)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_LE_S)
        self._emit_br_if(1)  # break out of block

        # result *= base
        self._emit_local_get(result_local)
        self._emit_local_get(base_local)
        self._emit(WasmOp.I64_MUL)
        self._emit_local_set(result_local)

        # exp -= 1
        self._emit_local_get(exp_local)
        self._emit_i64_const(1)
        self._emit(WasmOp.I64_SUB)
        self._emit_local_set(exp_local)

        # continue loop
        self._emit_br(0)
        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end block

        # Push result
        self._emit_local_get(result_local)

    def _emit_logical(self, op: BinOp, left: ExprNode, right: ExprNode) -> None:
        """Emit short-circuit logical AND/OR.

        Tcl ``expr`` logical operators always produce boolean ``0``/``1``,
        so both branches must normalise their result rather than returning
        the raw operand value.
        """
        if op == BinOp.AND:
            # if left == 0 then 0 else !!right
            self._emit_expr(left)
            self._emit(WasmOp.I64_EQZ)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_i64_const(0)
            self._emit(WasmOp.ELSE)
            self._emit_expr(right)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            self._emit(WasmOp.END)
        else:
            # if left != 0 then 1 else !!right
            self._emit_expr(left)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_i64_const(1)
            self._emit(WasmOp.ELSE)
            self._emit_expr(right)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
            self._emit(WasmOp.END)

    def _emit_unary(self, op: UnaryOp, operand: ExprNode) -> None:
        """Emit a unary operation."""
        if op == UnaryOp.NEG:
            self._emit_i64_const(0)
            self._emit_expr(operand)
            self._emit(WasmOp.I64_SUB)
        elif op == UnaryOp.POS:
            self._emit_expr(operand)  # no-op
        elif op == UnaryOp.BIT_NOT:
            # Issue #261: ``~`` rejects float operands with ``cannot use
            # floating-point value "X" as operand of "~"``.  Route through
            # the runtime helper so the domain check fires before the
            # XOR; falling back to inline ``x ^ -1`` would silently
            # truncate floats and accept them.
            bnot_idx = self._shared_imports.get("tcl_arith_bnot")
            if bnot_idx is not None:
                self._emit_expr_obj(operand)
                self._emit_call(bnot_idx)
                self._emit_unbox_int()
            else:
                self._emit_expr(operand)
                self._emit_i64_const(-1)
                self._emit(WasmOp.I64_XOR)
        elif op == UnaryOp.NOT:
            self._emit_expr(operand)
            self._emit(WasmOp.I64_EQZ)
            # Extend i32 result back to i64
            self._emit(WasmOp.I64_EXTEND_I32_S)
        else:
            self._emit_expr(operand)

    def _emit_ternary(self, cond: ExprNode, true_expr: ExprNode, false_expr: ExprNode) -> None:
        """Emit a ternary (cond ? a : b) expression."""
        self._emit_expr(cond)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_NE)
        self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
        self._emit_expr(true_expr)
        self._emit(WasmOp.ELSE)
        self._emit_expr(false_expr)
        self._emit(WasmOp.END)

    def _emit_func_call(self, func: str, args: tuple[ExprNode, ...]) -> None:
        """Emit a math function call.

        Integer-valued built-ins are inlined.  Float-valued ones
        (sqrt, log, sin, ...) still need float infrastructure through
        the IR; they fall through to the ``0`` placeholder and will
        be picked up in a follow-up.
        """
        if func == "abs" and len(args) == 1:
            # abs(x) = x < 0 ? -x : x
            tmp = self._add_extra_local("_abs_tmp")
            self._emit_expr(args[0])
            self._emit_local_tee(tmp)
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_LT_S)
            self._emit(WasmOp.IF, bytes([_BLOCK_I64]))
            self._emit_i64_const(0)
            self._emit_local_get(tmp)
            self._emit(WasmOp.I64_SUB)
            self._emit(WasmOp.ELSE)
            self._emit_local_get(tmp)
            self._emit(WasmOp.END)
        elif func == "min" and len(args) >= 2:
            # Variadic min: fold over args by pairwise I64_LT_S compare.
            running = self._add_extra_local("_min_acc")
            self._emit_expr(args[0])
            self._emit_local_set(running)
            for arg in args[1:]:
                candidate = self._add_extra_local("_min_cand")
                self._emit_expr(arg)
                self._emit_local_tee(candidate)
                self._emit_local_get(running)
                self._emit(WasmOp.I64_LT_S)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                self._emit_local_get(candidate)
                self._emit_local_set(running)
                self._emit(WasmOp.END)
            self._emit_local_get(running)
        elif func == "max" and len(args) >= 2:
            running = self._add_extra_local("_max_acc")
            self._emit_expr(args[0])
            self._emit_local_set(running)
            for arg in args[1:]:
                candidate = self._add_extra_local("_max_cand")
                self._emit_expr(arg)
                self._emit_local_tee(candidate)
                self._emit_local_get(running)
                self._emit(WasmOp.I64_GT_S)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                self._emit_local_get(candidate)
                self._emit_local_set(running)
                self._emit(WasmOp.END)
            self._emit_local_get(running)
        elif func in ("int", "entier", "wide") and len(args) == 1:
            # Truncate to integer — use runtime helper that handles TYPE_FLOAT.
            int_idx = self._shared_imports.get("tcl_math_int")
            if int_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(int_idx)
                self._emit_unbox_int()
            else:
                self._emit_expr(args[0])
        elif func in ("double", "float") and len(args) == 1:
            # Coerce to float — use runtime helper so downstream / stays float.
            dbl_idx = self._shared_imports.get("tcl_math_double")
            if dbl_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(dbl_idx)
                self._emit_unbox_int()
            else:
                self._emit_expr(args[0])
        elif func == "log" and len(args) == 1:
            log_idx = self._shared_imports.get("tcl_math_log")
            if log_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(log_idx)
                self._emit_unbox_int()
            else:
                self._emit_i64_const(0)
        elif func == "sqrt" and len(args) == 1:
            sqrt_idx = self._shared_imports.get("tcl_math_sqrt")
            if sqrt_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(sqrt_idx)
                self._emit_unbox_int()
            else:
                self._emit_i64_const(0)
        elif func == "exp" and len(args) == 1:
            exp_idx = self._shared_imports.get("tcl_math_exp")
            if exp_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(exp_idx)
                self._emit_unbox_int()
            else:
                self._emit_i64_const(0)
        elif func in ("log10",) and len(args) == 1:
            l10_idx = self._shared_imports.get("tcl_math_log10")
            if l10_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(l10_idx)
                self._emit_unbox_int()
            else:
                self._emit_i64_const(0)
        elif func == "round" and len(args) == 1:
            rnd_idx = self._shared_imports.get("tcl_math_round")
            if rnd_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(rnd_idx)
                self._emit_unbox_int()
            else:
                self._emit_expr(args[0])
        elif func in ("sin", "cos") and len(args) == 1:
            fn = "tcl_math_sin" if func == "sin" else "tcl_math_cos"
            trig_idx = self._shared_imports.get(fn)
            if trig_idx is not None:
                self._emit_expr_obj(args[0])
                self._emit_call(trig_idx)
                self._emit_unbox_int()
            else:
                self._emit_i64_const(0)
        elif func == "bool" and len(args) == 1:
            # bool(x) == x != 0
            self._emit_expr(args[0])
            self._emit_i64_const(0)
            self._emit(WasmOp.I64_NE)
            self._emit(WasmOp.I64_EXTEND_I32_S)
        elif func == "pow" and len(args) == 2:
            # Delegate to the existing integer-power loop.  Float pow
            # isn't covered; callers with float operands hit this path
            # and get truncated-integer power.
            self._emit_power(args[0], args[1])
        else:
            # Unknown function — push 0
            self._emit_i64_const(0)

    # -- Optimisation passes --

    def _try_fold_binary(self, op: BinOp, left: ExprNode, right: ExprNode) -> int | None:
        """Try to constant-fold a binary expression at compile time."""
        left_val = self._try_const_value(left)
        right_val = self._try_const_value(right)
        if left_val is None or right_val is None:
            return None

        match op:
            case BinOp.ADD:
                return left_val + right_val
            case BinOp.SUB:
                return left_val - right_val
            case BinOp.MUL:
                return left_val * right_val
            case BinOp.DIV:
                return left_val // right_val if right_val != 0 else None
            case BinOp.MOD:
                return left_val % right_val if right_val != 0 else None
            case BinOp.LSHIFT:
                return left_val << right_val if 0 <= right_val < 64 else None
            case BinOp.RSHIFT:
                return left_val >> right_val if 0 <= right_val < 64 else None
            case BinOp.BIT_AND:
                return left_val & right_val
            case BinOp.BIT_OR:
                return left_val | right_val
            case BinOp.BIT_XOR:
                return left_val ^ right_val
            case BinOp.EQ:
                return int(left_val == right_val)
            case BinOp.NE:
                return int(left_val != right_val)
            case BinOp.LT:
                return int(left_val < right_val)
            case BinOp.GT:
                return int(left_val > right_val)
            case BinOp.LE:
                return int(left_val <= right_val)
            case BinOp.GE:
                return int(left_val >= right_val)
            case _:
                return None

    def _try_const_value(self, node: ExprNode) -> int | None:
        """Try to extract a constant integer value from an expression node."""
        match node:
            case ExprLiteral(text=value):
                try:
                    # ``int(value, 0)`` handles the Tcl ``0x``/``0o``/``0b``
                    # prefixes alongside plain decimals.
                    return int(value, 0) if not value.lstrip("+-").isdigit() else int(value)
                except ValueError:
                    return None
            case ExprVar(name=name, text=text):
                # Array refs and aliased vars have time-varying values — don't
                # treat them as constants even if _const_map was populated
                # before the alias was established.
                if "(" in text:
                    return None
                if name in self._aliases:
                    return None
                return self._const_map.get(name)
            case _:
                return None

    # -- Statement emission --
