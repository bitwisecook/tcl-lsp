"""Mixin: command substitution parsing and emission."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ...parsing.expr_parser import parse_expr as _parse_expr_ast
from ...parsing.substitution import backslash_subst as _tcl_backslash_subst
from ._helpers import _regexp_to_glob
from .opcodes import (
    _INDEX_END,
    _STR_CLASS_IDS,
    Op,
    _parse_tcl_index,
)

if TYPE_CHECKING:
    from ._emitter import _Emitter


class _CmdSubstMixin:
    """Mixin: command substitution parsing and emission."""

    @staticmethod
    def _unroll_nested_set(value: str) -> list[str] | None:
        """Unroll ``[set y [set z 42]]`` into ``["y", "z", "42"]``.

        Returns the variable names followed by the innermost value,
        or ``None`` if *value* is not a chain of nested ``set`` commands.
        """
        chain: list[str] = []
        v = value
        while v.startswith("[set ") and v.endswith("]"):
            inner = v[1:-1]  # strip [ ]
            # Parse: set <varname> <rest>
            parts = inner.split(None, 2)
            if len(parts) < 3 or parts[0] != "set":
                return None
            chain.append(parts[1])
            v = parts[2]
        if not chain:
            return None
        chain.append(v)  # innermost value
        return chain

    @staticmethod
    def _is_pure_cmd_subst(value: str) -> bool:
        """Return True if *value* is a single balanced ``[cmd ...]``."""
        if not value.startswith("[") or not value.endswith("]"):
            return False
        depth = 0
        for i, ch in enumerate(value):
            if ch == "\\":
                continue  # skip escaped chars (crude but sufficient)
            if ch == "[":
                depth += 1
            elif ch == "]":
                depth -= 1
            if depth == 0:
                return i == len(value) - 1
        return False

    @staticmethod
    def _has_command_separator(text: str) -> bool:
        """Return True if *text* contains ``;`` or newline outside quotes/braces.

        Used to detect multi-command scripts in ``[...]`` substitutions
        so that they can be deferred to runtime ``EVAL_STK`` instead of
        being inlined as a single command call.
        """
        inner = text
        if inner.startswith("[") and inner.endswith("]"):
            inner = inner[1:-1]
        in_quotes = False
        brace_depth = 0
        bracket_depth = 0
        i = 0
        while i < len(inner):
            ch = inner[i]
            if ch == "\\" and i + 1 < len(inner):
                i += 2  # skip escaped character
                continue
            if ch == '"' and brace_depth == 0:
                in_quotes = not in_quotes
            elif not in_quotes:
                if ch == "{":
                    brace_depth += 1
                elif ch == "}" and brace_depth > 0:
                    brace_depth -= 1
                elif ch == "[":
                    bracket_depth += 1
                elif ch == "]" and bracket_depth > 0:
                    bracket_depth -= 1
                elif (ch == ";" or ch == "\n") and brace_depth == 0 and bracket_depth == 0:
                    return True
            i += 1
        return False

    @staticmethod
    def _parse_cmd_parts(text: str) -> list[tuple[str, bool]]:
        """Split ``[cmd arg1 ...]`` into ``(text, was_braced)`` tuples.

        *was_braced* is ``True`` when the original argument was wrapped
        in ``{…}`` (braces are stripped from the returned text).  This
        lets the caller decide whether to re-wrap the value in braces
        so that the VM suppresses variable/command substitution.
        """
        text = text.strip()
        if text.startswith("[") and text.endswith("]"):
            text = text[1:-1].strip()
        parts: list[tuple[str, bool]] = []
        i = 0
        while i < len(text):
            while i < len(text) and text[i] in (" ", "\t"):
                i += 1
            if i >= len(text):
                break
            if text[i] == '"':
                i += 1
                start = i
                while i < len(text) and text[i] != '"':
                    if text[i] == "\\":
                        i += 1
                    i += 1
                parts.append((text[start:i], False))
                if i < len(text):
                    i += 1
            elif text[i] == "{":
                depth = 1
                i += 1
                start = i
                while i < len(text) and depth > 0:
                    if text[i] == "\\":
                        i += 2
                        continue
                    if text[i] == "{":
                        depth += 1
                    elif text[i] == "}":
                        depth -= 1
                    if depth > 0:
                        i += 1
                    else:
                        break
                parts.append((text[start:i], True))
                i += 1  # skip closing }
            elif text[i] == "[":
                depth = 0
                start = i
                while i < len(text):
                    if text[i] == "[":
                        depth += 1
                    elif text[i] == "]":
                        depth -= 1
                        if depth == 0:
                            i += 1
                            break
                    i += 1
                parts.append((text[start:i], False))
            else:
                start = i
                while i < len(text) and text[i] not in (" ", "\t"):
                    i += 1
                parts.append((text[start:i], False))
        return parts

    def _emit_cmd_subst_arg(self: _Emitter, arg_pair: tuple[str, bool]) -> None:
        """Emit a single arg from a parsed command substitution."""
        arg, braced = arg_pair
        if not braced and arg.startswith("$"):
            # Braced scalar marker: $={name} → push + loadStk.
            braced_scalar = self._parse_braced_scalar_ref(arg)
            if braced_scalar is not None:
                self._push_lit(braced_scalar)
                self._emit(Op.LOAD_STK)
                return
            var_name = self._parse_simple_var_ref(arg)
            if var_name is None and len(arg) > 1 and arg[1:].isidentifier():
                # Bare $varname form (not normalised to ${var})
                var_name = arg[1:]
            if var_name is None and len(arg) > 1:
                # Bare $name(index) array ref form
                bare = arg[1:]
                if self._split_array_ref(bare) is not None:
                    var_name = bare
            if var_name is not None:
                self._load_var(var_name)
            else:
                self._push_lit(arg)
        elif not braced and arg.startswith("[") and arg.endswith("]"):
            # Nested command substitution — compile inline.
            # In proc bodies, non-expr commands need startCommand wrapping
            # for command boundary tracking (matching tclsh behaviour).
            inner_parts = self._parse_cmd_parts(arg)
            needs_sc = self._is_proc and inner_parts and inner_parts[0][0] != "expr"
            if needs_sc:
                sc_end = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, sc_end, 1)
                self._cmd_index += 1
                self._emit_inline_cmd_subst(arg)
                self._place_label(sc_end)
            else:
                self._emit_inline_cmd_subst(arg)
        elif braced and ("$" in arg or "[" in arg):
            self._push_lit("{" + arg + "}")
        elif not braced and "\\" in arg:
            processed = _tcl_backslash_subst(arg)
            if "$" in processed or ("[" in processed and "]" in processed):
                self._push_raw_lit(processed)
            else:
                self._push_lit(processed)
        else:
            self._push_lit(arg)

    def _emit_generic_cmd_subst(self: _Emitter, cmd: str, args: list[tuple[str, bool]]) -> None:
        """Emit a generic command substitution as push + invokeStk."""
        self._push_lit(cmd)
        for arg, braced in args:
            if not braced and arg.startswith("[") and arg.endswith("]"):
                end_label = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, end_label, 1)
                self._emit_inline_cmd_subst(arg)
                self._place_label(end_label)
            elif not braced and arg.startswith("$"):
                braced_scalar = self._parse_braced_scalar_ref(arg)
                if braced_scalar is not None:
                    self._push_lit(braced_scalar)
                    self._emit(Op.LOAD_STK)
                else:
                    var_name = self._parse_simple_var_ref(arg)
                    if var_name is not None:
                        self._load_var(var_name)
                    else:
                        self._push_lit(arg)
            elif braced and ("$" in arg or "[" in arg):
                self._push_lit("{" + arg + "}")
            elif not braced and "\\" in arg:
                processed = _tcl_backslash_subst(arg)
                if "$" in processed or ("[" in processed and "]" in processed):
                    self._push_raw_lit(processed)
                else:
                    self._push_lit(processed)
            else:
                self._push_lit(arg)
        argc = 1 + len(args)
        op = Op.INVOKE_STK1 if argc < 256 else Op.INVOKE_STK4
        self._emit(op, argc)

    def _emit_inline_cmd_subst(self: _Emitter, text: str) -> None:  # noqa: C901
        """Inline-compile a ``[cmd arg ...]`` command substitution.

        Handles ``[expr {...}]`` specially by parsing and inlining the
        expression body.  Other commands are compiled as
        ``push cmd; <args>; invokeStk1 N``.

        Multi-command scripts (containing ``;`` or newlines outside
        quotes/braces) fall back to runtime ``EVAL_STK``.
        """
        # Detect multi-command scripts and fall back to runtime eval.
        if self._has_command_separator(text):
            inner = text
            if inner.startswith("[") and inner.endswith("]"):
                inner = inner[1:-1]
            # Wrap in braces so PUSH suppresses variable/command
            # substitution — EVAL_STK will handle them at runtime.
            self._push_lit("{" + inner + "}")
            self._emit(Op.EVAL_STK)
            return

        parts = self._parse_cmd_parts(text)
        if not parts:
            self._push_lit("")
            return

        cmd, _ = parts[0]
        args = parts[1:]

        if cmd == "expr" and len(args) == 1:
            # Inline the expression body
            expr_body = args[0][0]
            node = _parse_expr_ast(expr_body)
            self._emit_expr(node)
        elif cmd == "incr" and 1 <= len(args) <= 2:
            # In proc context with a local variable, use LVT-based incr:
            #   ``[incr var]`` → incrScalar1Imm %vN +1
            # Otherwise fall back to stack-based:
            #   ``[incr var]`` → push var; incrStkImm +1
            var_name = args[0][0]
            if self._is_proc and not self._is_qualified(var_name):
                slot = self._lvt.intern(var_name)
                if len(args) == 1:
                    self._emit(Op.INCR_SCALAR1_IMM, slot, 1, comment=f'var "{var_name}"')
                else:
                    amt_str = args[1][0]
                    if amt_str.lstrip("-").isdigit():
                        amt = int(amt_str)
                        if -128 <= amt <= 127:
                            self._emit(Op.INCR_SCALAR1_IMM, slot, amt, comment=f'var "{var_name}"')
                        else:
                            self._push_lit(amt_str)
                            self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{var_name}"')
                    else:
                        self._load_var(amt_str.lstrip("$"))
                        self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{var_name}"')
            else:
                self._push_lit(var_name)
                if len(args) == 1:
                    self._emit(Op.INCR_STK_IMM, 1)
                else:
                    amt_str = args[1][0]
                    try:
                        amt = int(amt_str)
                        self._emit(Op.INCR_STK_IMM, amt)
                    except ValueError:
                        self._load_var(amt_str.lstrip("$"))
                        self._emit(Op.INCR_STK)
        elif cmd == "info" and len(args) >= 1 and args[0][0] == "exists" and len(args) == 2:
            # ``[info exists var]`` → existScalar %vN; nop (proc)
            #                       → push var; existStk (global)
            self._used_inline_cmd_subst = True
            var_name = args[1][0]
            if self._is_proc and not self._is_qualified(var_name):
                slot = self._lvt.intern(var_name)
                self._emit(Op.EXIST_SCALAR, slot, comment=f'var "{var_name}"')
                self._emit(Op.NOP)
            else:
                self._push_lit(var_name)
                self._emit(Op.EXIST_STK)
        elif cmd == "string" and len(args) >= 2:
            subcmd = args[0][0]
            sargs = args[1:]  # string subcommand args
            _prev_inline = self._used_inline_cmd_subst
            self._used_inline_cmd_subst = True
            if subcmd == "index" and len(sargs) == 2:
                self._emit_cmd_subst_arg(sargs[0])
                self._emit_cmd_subst_arg(sargs[1])
                self._emit(Op.STR_INDEX)
            elif subcmd == "range" and len(sargs) == 3:
                self._emit_cmd_subst_arg(sargs[0])
                start_idx = _parse_tcl_index(sargs[1][0])
                end_idx = _parse_tcl_index(sargs[2][0])
                if start_idx is not None and end_idx is not None:
                    self._emit(Op.STR_RANGE_IMM, start_idx, end_idx)
                else:
                    self._emit_cmd_subst_arg(sargs[1])
                    self._emit_cmd_subst_arg(sargs[2])
                    self._emit(Op.STR_RANGE)
            elif subcmd == "equal" and len(sargs) == 2:
                if not self._is_proc:
                    self._used_inline_cmd_subst = _prev_inline
                    sc_end = self._fresh_label("subcmd_end")
                    self._emit(Op.START_CMD, sc_end, 1)
                self._emit_cmd_subst_arg(sargs[0])
                self._emit_cmd_subst_arg(sargs[1])
                self._emit(Op.STR_EQ)
                if not self._is_proc:
                    self._place_label(sc_end)
            elif subcmd == "equal" and len(sargs) == 3 and sargs[0][0] == "-nocase":
                self._used_inline_cmd_subst = _prev_inline
                sc_end = self._fresh_label("subcmd_end")
                self._emit(Op.START_CMD, sc_end, 1)
                self._push_lit("string")
                self._push_lit("equal")
                self._push_lit("-nocase")
                self._emit_cmd_subst_arg(sargs[1])
                self._emit_cmd_subst_arg(sargs[2])
                self._push_lit("::tcl::string::equal")
                self._emit(Op.INVOKE_REPLACE, 5, 2)
                self._place_label(sc_end)
                self._seen_generic_invoke = True
            elif subcmd == "compare" and len(sargs) == 2:
                if not self._is_proc:
                    self._used_inline_cmd_subst = _prev_inline
                    sc_end = self._fresh_label("subcmd_end")
                    self._emit(Op.START_CMD, sc_end, 1)
                self._emit_cmd_subst_arg(sargs[0])
                self._emit_cmd_subst_arg(sargs[1])
                self._emit(Op.STR_CMP)
                if not self._is_proc:
                    self._place_label(sc_end)
            elif subcmd == "compare" and len(sargs) == 3 and sargs[0][0] == "-nocase":
                self._used_inline_cmd_subst = _prev_inline
                sc_end = self._fresh_label("subcmd_end")
                self._emit(Op.START_CMD, sc_end, 1)
                self._push_lit("string")
                self._push_lit("compare")
                self._push_lit("-nocase")
                self._emit_cmd_subst_arg(sargs[1])
                self._emit_cmd_subst_arg(sargs[2])
                self._push_lit("::tcl::string::compare")
                self._emit(Op.INVOKE_REPLACE, 5, 2)
                self._place_label(sc_end)
                self._seen_generic_invoke = True
            elif subcmd == "compare" and len(sargs) == 4 and sargs[0][0] == "-length":
                self._used_inline_cmd_subst = _prev_inline
                sc_end = self._fresh_label("subcmd_end")
                self._emit(Op.START_CMD, sc_end, 1)
                self._push_lit("string")
                self._push_lit("compare")
                self._push_lit("-length")
                self._emit_cmd_subst_arg(sargs[1])
                self._emit_cmd_subst_arg(sargs[2])
                self._emit_cmd_subst_arg(sargs[3])
                self._push_lit("::tcl::string::compare")
                self._emit(Op.INVOKE_REPLACE, 6, 2)
                self._place_label(sc_end)
                self._seen_generic_invoke = True
            elif subcmd == "replace" and len(sargs) == 4:
                first_lit = sargs[1][0]  # text portion of (text, braced)
                last_lit = sargs[2][0]
                # Optimisation: string replace $s 0 N repl
                # → repl + s[(N+1)..end] via reverse/strrangeImm/strcat
                if first_lit == "0":
                    try:
                        last_int = int(last_lit)
                    except (ValueError, TypeError):
                        last_int = None
                    if last_int is not None and last_int >= 0:
                        self._emit_cmd_subst_arg(sargs[0])  # source
                        self._emit_cmd_subst_arg(sargs[3])  # replacement
                        self._emit(Op.REVERSE, 2)
                        self._emit(Op.STR_RANGE_IMM, last_int + 1, _INDEX_END)
                        self._emit(Op.STR_CONCAT1, 2)
                    else:
                        self._emit_cmd_subst_arg(sargs[0])
                        self._emit_cmd_subst_arg(sargs[1])
                        self._emit_cmd_subst_arg(sargs[2])
                        self._emit_cmd_subst_arg(sargs[3])
                        self._emit(Op.STR_REPLACE)
                else:
                    self._emit_cmd_subst_arg(sargs[0])
                    self._emit_cmd_subst_arg(sargs[1])
                    self._emit_cmd_subst_arg(sargs[2])
                    self._emit_cmd_subst_arg(sargs[3])
                    self._emit(Op.STR_REPLACE)
            elif subcmd == "length" and len(sargs) == 1:
                self._emit_cmd_subst_arg(sargs[0])
                self._emit(Op.STR_LEN)
            elif subcmd == "is" and len(sargs) >= 2:
                class_name = sargs[0][0]
                # Detect -strict flag and value
                if len(sargs) == 3 and sargs[1][0] == "-strict":
                    strict = True
                    val_arg = sargs[2]
                else:
                    strict = False
                    val_arg = sargs[-1]
                class_id = _STR_CLASS_IDS.get(class_name)
                if class_id is not None:
                    # strclass types: alpha, digit, alnum, etc.
                    self._emit_cmd_subst_arg(val_arg)
                    self._emit(Op.STR_CLASS, class_id)
                elif class_name == "integer":
                    self._emit_cmd_subst_arg(val_arg)
                    if strict:
                        self._emit(Op.NUMERIC_TYPE)
                        self._emit(Op.DUP)
                        end_lbl = self._fresh_label("si_end")
                        self._emit(Op.JUMP_FALSE1, end_lbl)
                        self._push_lit("3")
                        self._emit(Op.LE)
                        self._place_label(end_lbl)
                    else:
                        # Non-strict: dup, numericType, dup, jumpTrue → reverse+pop+push 3+le
                        #                                    jumpFalse → pop+push ""+streq
                        self._emit(Op.DUP)
                        self._emit(Op.NUMERIC_TYPE)
                        self._emit(Op.DUP)
                        has_type = self._fresh_label("si_has_type")
                        self._emit(Op.JUMP_TRUE1, has_type)
                        self._emit(Op.POP)
                        self._push_lit("")
                        self._emit(Op.STR_EQ)
                        end_lbl = self._fresh_label("si_end")
                        self._emit(Op.JUMP1, end_lbl)
                        self._place_label(has_type)
                        self._emit(Op.REVERSE, 2)
                        self._emit(Op.POP)
                        self._push_lit("3")
                        self._emit(Op.LE)
                        self._place_label(end_lbl)
                elif class_name == "double":
                    self._emit_cmd_subst_arg(val_arg)
                    if strict:
                        self._emit(Op.NUMERIC_TYPE)
                        true_lbl = self._fresh_label("si_true")
                        self._emit(Op.JUMP_TRUE1, true_lbl)
                        self._push_lit("0")
                        end_lbl = self._fresh_label("si_end")
                        self._emit(Op.JUMP1, end_lbl)
                        self._place_label(true_lbl)
                        self._push_lit("1")
                        self._place_label(end_lbl)
                    else:
                        # Non-strict: dup, "" streq check, then numericType
                        self._emit(Op.DUP)
                        self._push_lit("")
                        self._emit(Op.STR_EQ)
                        true_lbl = self._fresh_label("si_true")
                        self._emit(Op.JUMP_TRUE1, true_lbl)
                        self._emit(Op.NUMERIC_TYPE)
                        has_type = self._fresh_label("si_has_type")
                        self._emit(Op.JUMP_TRUE1, has_type)
                        self._push_lit("0")
                        end_lbl = self._fresh_label("si_end")
                        self._emit(Op.JUMP1, end_lbl)
                        self._place_label(true_lbl)
                        self._emit(Op.POP)
                        self._place_label(has_type)
                        self._push_lit("1")
                        self._place_label(end_lbl)
                elif class_name == "boolean":
                    self._emit_cmd_subst_arg(val_arg)
                    self._emit(Op.TRY_CVT_TO_BOOLEAN)
                    true_lbl = self._fresh_label("si_true")
                    self._emit(Op.JUMP_TRUE1, true_lbl)
                    self._push_lit("")
                    self._emit(Op.STR_EQ)
                    end_lbl = self._fresh_label("si_end")
                    self._emit(Op.JUMP1, end_lbl)
                    self._place_label(true_lbl)
                    self._emit(Op.POP)
                    self._push_lit("1")
                    self._place_label(end_lbl)
                else:
                    self._used_inline_cmd_subst = False
                    self._emit_generic_cmd_subst(cmd, args)
            else:
                # Unhandled string subcommand: use FQN invoke.
                # tclsh 9.0 resolves to ::tcl::string::<sub>.
                self._used_inline_cmd_subst = _prev_inline
                sc_end = self._fresh_label("subcmd_end")
                self._emit(Op.START_CMD, sc_end, 1)
                fqn = f"::tcl::string::{subcmd}"
                self._push_lit(fqn)
                for sarg in sargs:
                    self._emit_cmd_subst_arg(sarg)
                argc = 1 + len(sargs)
                self._emit(Op.INVOKE_STK1, argc)
                self._place_label(sc_end)
                self._seen_generic_invoke = True
        elif cmd == "lindex" and len(args) >= 2:
            # ``[lindex $lst idx ...]``
            self._used_inline_cmd_subst = True
            self._emit_cmd_subst_arg(args[0])  # list
            if len(args) == 2:
                # Single index: use listIndexImm for constants, listIndex otherwise
                idx = _parse_tcl_index(args[1][0])
                if idx is not None and (idx >= 0 or idx <= _INDEX_END):
                    self._emit(Op.LIST_INDEX_IMM, idx)
                else:
                    self._emit_cmd_subst_arg(args[1])
                    self._emit(Op.LIST_INDEX)
            else:
                # Multiple indices: push all, lindexMulti N
                for a in args[1:]:
                    self._emit_cmd_subst_arg(a)
                self._emit(Op.LINDEX_MULTI, len(args))
        elif cmd == "lrange" and len(args) == 3:
            # ``[lrange $lst start end]`` → listRangeImm
            self._used_inline_cmd_subst = True
            self._emit_cmd_subst_arg(args[0])  # list
            start_idx = _parse_tcl_index(args[1][0])
            end_idx = _parse_tcl_index(args[2][0])
            if start_idx is not None and end_idx is not None:
                self._emit(Op.LIST_RANGE_IMM, start_idx, end_idx)
            else:
                self._used_inline_cmd_subst = False
                self._emit_generic_cmd_subst(cmd, args)
        elif cmd == "lreplace" and len(args) >= 3:
            # ``[lreplace $lst first last ?elem ...?]`` → lreplace4 N 1
            self._used_inline_cmd_subst = True
            self._emit_cmd_subst_arg(args[0])  # list
            for a in args[1:]:
                self._emit_cmd_subst_arg(a)
            self._emit(Op.LREPLACE4, len(args), 1)
        elif cmd == "linsert" and len(args) >= 2:
            # ``[linsert $lst index ?elem ...?]`` → lreplace4 N 2
            self._used_inline_cmd_subst = True
            self._emit_cmd_subst_arg(args[0])  # list
            for a in args[1:]:
                self._emit_cmd_subst_arg(a)
            self._emit(Op.LREPLACE4, len(args), 2)
        elif cmd == "regexp" and len(args) >= 2:
            # ``[regexp -nocase pattern string]`` → strmatch +1 with glob
            rargs = list(args)
            nocase = False
            if rargs and rargs[0][0] == "-nocase":
                nocase = True
                rargs.pop(0)
            if rargs and rargs[0][0] == "--":
                rargs.pop(0)
            if len(rargs) == 2 and nocase:
                glob = _regexp_to_glob(rargs[0][0])
                if glob is not None:
                    self._used_inline_cmd_subst = True
                    self._push_lit(glob)
                    self._emit_cmd_subst_arg(rargs[1])
                    self._emit(Op.STR_MATCH, 1)
                else:
                    self._used_inline_cmd_subst = False
                    self._emit_generic_cmd_subst(cmd, args)
            elif len(rargs) == 2 and not nocase:
                self._used_inline_cmd_subst = True
                for a in args:
                    self._emit_cmd_subst_arg(a)
                self._emit(Op.REGEXP, len(args) + 1)
            else:
                self._used_inline_cmd_subst = False
                self._emit_generic_cmd_subst(cmd, args)
        elif cmd == "list" and args and "{*}" not in text:
            self._used_inline_cmd_subst = True
            for a in args:
                self._emit_cmd_subst_arg(a)
            self._emit(Op.LIST, len(args))
        elif cmd == "array" and len(args) >= 2:
            sub = args[0][0]
            rest = args[1:]
            if (
                sub == "exists"
                and len(rest) == 1
                and self._is_proc
                and not self._is_qualified(rest[0][0])
            ):
                self._used_inline_cmd_subst = True
                slot = self._lvt.intern(rest[0][0])
                self._emit(Op.ARRAY_EXISTS_IMM, slot, comment=f'var "{rest[0][0]}"')
            elif sub in ("names", "size") and len(rest) >= 1:
                sc_end = self._fresh_label("subcmd_end")
                self._emit(Op.START_CMD, sc_end, 1)
                fqn = f"::tcl::array::{sub}"
                self._push_lit(fqn)
                for a in rest:
                    self._emit_cmd_subst_arg(a)
                self._emit(Op.INVOKE_STK1, 1 + len(rest))
                self._place_label(sc_end)
                self._seen_generic_invoke = True
            else:
                self._used_inline_cmd_subst = False
                self._emit_generic_cmd_subst(cmd, args)
        elif cmd == "dict" and len(args) >= 2 and args[0][0] == "get" and len(args) >= 3:
            # ``[dict get $d key ...]`` → load d, push keys, dictGet N
            self._used_inline_cmd_subst = True
            dict_args = args[1:]  # skip "get"
            self._emit_cmd_subst_arg(dict_args[0])  # dict value
            keys = dict_args[1:]
            for k in keys:
                self._emit_cmd_subst_arg(k)
            self._emit(Op.DICT_GET, len(keys))
        elif cmd == "catch" and self._is_proc and 1 <= len(args) <= 3:
            result_var = args[1][0] if len(args) > 1 else None
            if result_var and result_var.startswith("::"):
                # Global result var: fall back to generic invoke.
                self._used_inline_cmd_subst = False
                self._emit_generic_cmd_subst(cmd, args)
            else:
                self._used_inline_cmd_subst = True
                body_text = args[0][0]
                options_var = args[2][0] if len(args) > 2 else None
                self._emit_catch_inline(body_text, result_var, options_var)
        else:
            self._used_inline_cmd_subst = False
            self._emit_generic_cmd_subst(cmd, args)
