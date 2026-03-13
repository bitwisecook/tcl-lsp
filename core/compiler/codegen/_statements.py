"""Mixin: statement emission."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ...parsing.substitution import backslash_subst as _tcl_backslash_subst
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRExprEval,
    IRIncr,
    IRReturn,
    IRStatement,
)
from ._types import Instruction
from .opcodes import Op

if TYPE_CHECKING:
    from ._emitter import _Emitter


class _StatementsMixin:
    """Mixin: statement emission."""

    # Tag used by _fixup_top_level_start_cmd to identify startCommand
    # instructions wrapping generic invokes vs compiled commands.
    _SC_GENERIC_TAG = "sc:generic"

    def _emit_stmt_with_start_cmd(
        self: _Emitter,
        stmt: IRStatement,
        *,
        count_override: int | None = None,
        deferred_end_label: str | None = None,
    ) -> None:
        """Emit a statement, wrapping with ``startCommand`` if needed.

        Emits ``startCommand`` for every non-first command.  Post-passes
        then clean up:

        - ``_strip_unused_start_cmd``: strips ALL startCommand when the
          unit has no generic invokes (all compiled).
        - ``_fixup_top_level_start_cmd``: for top-level scripts, removes
          startCommand for generic invokes that appear before the first
          compiled command's startCommand.

        *count_override* overrides the default count of 1 (e.g. 2 when
        a compound command and its init share the same bytecode offset).
        *deferred_end_label* uses a caller-provided label instead of
        creating and placing one automatically; the caller is responsible
        for placing it later.
        """
        # Determine startCommand count.  When the statement contains an
        # inlined ``[cmd ...]`` (e.g. ``set a [string is ...]``), two
        # commands start at this offset: the statement and the inner cmd.
        if count_override is not None:
            count = count_override
        elif isinstance(stmt, IRIncr) and stmt.amount and "[expr " in stmt.amount:
            count = 2
        elif isinstance(stmt, IRCall) and any(
            isinstance(a, str) and a.startswith("[list ") and ("[break]" in a or "[continue]" in a)
            for a in stmt.args
        ):
            count = 2
        else:
            count = 1

        emit_sc = self._cmd_index > 0
        end_label: str | None = None
        sc_idx: int | None = None

        if emit_sc:
            end_label = deferred_end_label or self._fresh_label("cmd_end")
            sc_idx = self._emit(Op.START_CMD, end_label, count)

        self._used_generic_invoke = False
        self._used_inline_cmd_subst = False
        self._emit_stmt(stmt)

        if self._used_generic_invoke:
            self._seen_generic_invoke = True
            # Tag this startCommand as wrapping a generic invoke so
            # the post-pass can selectively remove it for top-level.
            if sc_idx is not None and not self._is_proc:
                self._instrs[sc_idx] = Instruction(
                    op=self._instrs[sc_idx].op,
                    operands=self._instrs[sc_idx].operands,
                    comment=self._SC_GENERIC_TAG,
                )

        # Patch startCommand count to 2 when the statement contains
        # an inlined command substitution (set x [inlined_cmd ...]).
        if (
            sc_idx is not None
            and count == 1
            and self._used_inline_cmd_subst
            and not self._used_generic_invoke
        ):
            old = self._instrs[sc_idx]
            self._instrs[sc_idx] = Instruction(
                op=old.op,
                operands=(old.operands[0], 2),
                comment=old.comment,
            )

        if end_label is not None and deferred_end_label is None:
            # Place end label before the trailing pop (or at end if
            # no pop — e.g. terminators or last statement).
            if self._instrs and self._instrs[-1].op == Op.POP:
                self._labels[end_label] = len(self._instrs) - 1
            else:
                self._labels[end_label] = len(self._instrs)

        self._cmd_index += 1

    def _emit_stmt(self: _Emitter, stmt: IRStatement) -> None:  # noqa: C901
        # Track source line for errorInfo (1-based).
        if hasattr(stmt, "range"):
            self._current_source_line = stmt.range.start.line + 1
        match stmt:
            case IRAssignConst(name=name, value=value):
                if self._needs_stk_var_ref(name):
                    self._push_var_ref(name)
                self._push_lit(str(value))
                self._store_var(name)
                self._emit(Op.POP)

            case IRAssignValue(name=name, value=value):
                # Apply Tcl backslash substitution when flagged by lowering.
                if stmt.value_needs_backsubst:
                    value = _tcl_backslash_subst(value)
                # Handle ``set d [dict create key val ...]`` pattern:
                # tclsh compiles the dict create as a sub-command with
                # startCommand, push literal, dup, verifyDict.
                if not self._is_proc and value.startswith("[dict create ") and value.endswith("]"):
                    dict_fold = self._fold_dict_create_cmd(value)
                    if dict_fold is not None:
                        self._push_lit(name)
                        # Emit startCommand for dict create sub-command.
                        end_label = self._fresh_label("cmd_end")
                        self._emit(Op.START_CMD, end_label, 1)
                        self._push_lit(dict_fold)
                        self._emit(Op.DUP)
                        self._emit(Op.VERIFY_DICT)
                        # Place end label after verifyDict (before storeStk).
                        self._labels[end_label] = len(self._instrs)
                        self._store_var(name)
                        self._emit(Op.POP)
                    else:
                        # Non-constant dict create (nested cmd substitutions):
                        # use ``::tcl::dict::create`` FQ name + invokeStk1.
                        self._push_var_ref(name)
                        end_label = self._fresh_label("cmd_end")
                        self._emit(Op.START_CMD, end_label, 1)
                        parts = self._parse_cmd_parts(value)
                        # parts[0]=dict, parts[1]=create, parts[2:]=args
                        cmd_args = parts[2:]
                        self._push_lit("::tcl::dict::create")
                        for arg, braced in cmd_args:
                            self._emit_value(arg)
                        self._emit(Op.INVOKE_STK1, 1 + len(cmd_args))
                        self._labels[end_label] = len(self._instrs)
                        self._store_var(name)
                        self._emit(Op.POP)
                        self._used_generic_invoke = True
                    # Trigger startCommand for all subsequent commands.
                    self._seen_generic_invoke = True
                elif (
                    not self._is_proc
                    and value.startswith("[set ")
                    and (nested := self._unroll_nested_set(value)) is not None
                ):
                    # ``set x [set y [set z 42]]`` → push x, y, z, 42,
                    # then chain storeStk for each variable (innermost first).
                    self._push_var_ref(name)
                    for part in nested[:-1]:
                        self._push_var_ref(part)
                    self._push_lit(nested[-1])
                    for _v in reversed(nested[:-1]):
                        self._emit(Op.STORE_STK)
                    self._store_var(name)
                    self._emit(Op.POP)
                elif not self._is_proc and (fmt_result := self._try_format_fold(value)) is not None:
                    # ``set s [format "%s is %d" Alice 30]`` → constant-fold
                    # the format to its result string, with startCommand.
                    self._push_var_ref(name)
                    end_label = self._fresh_label("cmd_end")
                    self._emit(Op.START_CMD, end_label, 1)
                    self._push_lit(fmt_result)
                    self._labels[end_label] = len(self._instrs)
                    self._store_var(name)
                    self._emit(Op.POP)
                    self._seen_generic_invoke = True
                elif (
                    self._is_pure_cmd_subst(value)
                    and not value.startswith("[list ")
                    and not value.startswith("[dict create ")
                    and not value.startswith("[set ")
                ):
                    # Inline-compile ``set x [cmd ...]`` by emitting the
                    # sub-command as ``push cmd; <args>; invokeStk1 N``
                    # instead of pushing the raw ``[cmd ...]`` text as a
                    # literal.  This matches tclsh 9.0 which always
                    # evaluates command substitutions in set values.
                    # Exclude [list ...] and [dict create ...] which are
                    # constant-folded by _emit_value / the dict handler.
                    if self._needs_stk_var_ref(name):
                        self._push_var_ref(name)
                    elif self._is_proc and not self._is_qualified(name):
                        # Pre-intern the target variable so it gets a
                        # lower LVT slot than any variables introduced
                        # inside the command substitution (e.g. catch
                        # result vars).  This matches tclsh slot order.
                        self._lvt.intern(name)
                    self._emit_inline_cmd_subst(value)
                    self._store_var(name)
                    self._emit(Op.POP)
                else:
                    if self._needs_stk_var_ref(name):
                        self._push_var_ref(name)
                    self._emit_value(value, interpolate=True)
                    self._store_var(name)
                    self._emit(Op.POP)

            case IRAssignExpr(name=name, expr=expr):
                if self._needs_stk_var_ref(name):
                    self._push_var_ref(name)
                # Inner startCommand for the [expr {...}] portion.
                # tclsh wraps each nested expr in its own startCommand;
                # the post-pass strips these when there are no generic
                # invokes in the script.
                inner_end = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, inner_end, 1)
                guaranteed_numeric = self._emit_expr(expr, fold_no_dedup=True)
                if not guaranteed_numeric:
                    self._emit(Op.TRY_CVT_TO_NUMERIC)
                self._place_label(inner_end)
                self._store_var(name)
                self._emit(Op.POP)

            case IRIncr(name=name, amount=amount):
                self._emit_incr(name, amount)
                self._emit(Op.POP)

            case IRExprEval(expr=expr):
                guaranteed_numeric = self._emit_expr(expr, fold_no_dedup=True)
                if not guaranteed_numeric:
                    self._emit(Op.TRY_CVT_TO_NUMERIC)
                self._emit(Op.POP)

            case IRCall(command=cmd, args=args, tokens=tokens):
                if (
                    cmd == "catch"
                    and self._is_proc
                    and 1 <= len(args) <= 3
                    and not (len(args) > 1 and args[1].startswith("::"))
                ):
                    body_text = args[0]
                    result_var = args[1] if len(args) > 1 else None
                    options_var = args[2] if len(args) > 2 else None
                    self._emit_catch_inline(body_text, result_var, options_var)
                elif cmd == "<cond>":
                    pass  # CFG placeholder — condition is handled by the block terminator
                elif cmd == "<empty_clause>":
                    # Empty for-loop clause — tclsh 9.0 emits 3 nops
                    # (from constant-folding the empty clause body).
                    # Intern "" to match 9.0's literal pool ordering
                    # (the original push ""; pop interns before folding).
                    self._lit.intern("")
                    self._emit(Op.NOP)
                    self._emit(Op.NOP)
                    self._emit(Op.NOP)
                elif tokens and tokens.expand_word and any(tokens.expand_word):
                    ew = tokens.expand_word
                    # {*} on the command name with a single-word literal:
                    # {*}cmd is equivalent to cmd — strip the expansion.
                    # But keep expansion for variable/command refs whose
                    # runtime value may be a multi-word list.
                    if ew[0] and " " not in cmd and "$" not in cmd and "[" not in cmd:
                        ew = (False,) + ew[1:]
                    if not any(ew):
                        # No expansion remaining — normal call.
                        self._emit_call(cmd, args)
                    elif cmd == "list" and not ew[0]:
                        if self._try_list_expand_call(args, ew):
                            pass  # handled
                        else:
                            self._emit_expanded_call(cmd, args, ew)
                    else:
                        self._emit_expanded_call(cmd, args, ew)
                else:
                    self._emit_call(cmd, args)
                # Tag the last INVOKE instruction with original source text
                # for accurate errorInfo "invoked from within" frames.
                if tokens and tokens.argv_texts:
                    self._tag_last_invoke_source(tokens.argv_texts)

            case IRBarrier(command=cmd, args=args, reason=reason, tokens=tokens):
                if cmd:
                    if tokens and tokens.expand_word and any(tokens.expand_word):
                        self._emit_expanded_call(cmd, args, tokens.expand_word)
                    else:
                        self._emit_call(cmd, args)
                    # Tag the last INVOKE instruction with original source text.
                    if tokens and tokens.argv_texts:
                        self._tag_last_invoke_source(tokens.argv_texts)
                    # Commands with CompileProcs in Tcl 9.0 keep their
                    # startCommand even when they fall through to invokeStk.
                    # Prevent these from being tagged as generic invokes.
                    if cmd.startswith("::tcl::dict::"):
                        self._used_generic_invoke = False
                else:
                    self._emit(Op.NOP, comment=f"barrier: {reason}")

            case IRReturn(value=value):
                self._emit_value(value if value is not None else "", interpolate=True)
                self._emit(Op.RETURN_IMM, 0, 0)

            case _:
                self._emit(Op.NOP, comment=f"unhandled: {type(stmt).__name__}")

    # -- command invocation --

    def _try_list_expand_call(
        self: _Emitter, args: tuple[str, ...], expand_word: tuple[bool, ...]
    ) -> bool:
        """Compile ``list`` with {*} expansion specially.

        Returns True if handled.  Patterns:
        - ``list {*}{literal}`` → push literal
        - ``list {*}$var`` → load var; listRangeImm 0 end
        - ``list {*}[cmd]`` → <cmd>; listRangeImm 0 end
        - ``list a {*}$x b`` → (push a; list 1); load x; listConcat; ...
        - ``list {*}$a {*}$b`` → load a; load b; listConcat
        """
        from .opcodes import _INDEX_END

        # expand_word[0] is cmd ("list") — always False.
        # expand_word[1:] maps to args.
        arg_expand = list(expand_word[1:]) if len(expand_word) > 1 else []
        # Pad to match args length
        while len(arg_expand) < len(args):
            arg_expand.append(False)

        # Single arg, expanded
        if len(args) == 1 and arg_expand[0]:
            val = args[0]
            is_lit = not val.startswith("$") and not (val.startswith("[") and val.endswith("]"))
            if is_lit:
                # Case 1: list {*}{literal} → push literal
                # Use listRangeImm when the value contains characters
                # that need canonical normalisation or could indicate
                # a malformed list (braces, backslash, quotes).
                self._push_lit(val)
                if "\\" in val or '"' in val or "{" in val or "}" in val:
                    self._emit(Op.LIST_RANGE_IMM, 0, _INDEX_END)
            else:
                # Case 2/3: list {*}$var / {*}$a$b / {*}[cmd]
                # Use _emit_value to handle variable concatenation,
                # command substitution, and all interpolation forms.
                self._emit_value(val)
                self._emit(Op.LIST_RANGE_IMM, 0, _INDEX_END)
            self._emit(Op.POP)
            return True

        # Multiple args — build via list 1 + listConcat chain
        if not args:
            return False

        # Emit first element
        first_emitted = False
        for i, (val, expanded) in enumerate(zip(args, arg_expand)):
            if expanded:
                is_lit = not val.startswith("$") and not (val.startswith("[") and val.endswith("]"))
                if is_lit:
                    self._push_lit(val)
                else:
                    self._emit_value(val)
            else:
                # Non-expanded: push value, wrap in list 1
                self._emit_value(val)
                self._emit(Op.LIST, 1)
            if first_emitted:
                self._emit(Op.LIST_CONCAT)
            first_emitted = True

        self._emit(Op.POP)
        return True

    def _emit_expanded_call(
        self: _Emitter, cmd: str, args: tuple[str, ...], expand_word: tuple[bool, ...]
    ) -> None:
        """Emit a command call with {*} expansion on marked arguments."""
        self._emit(Op.EXPAND_START, comment=f"{cmd} (expanded)")
        # Build full word list: [cmd, *args]
        all_words = (cmd,) + args
        word_count = 0
        for i, word in enumerate(all_words):
            self._emit_value(word)
            word_count += 1
            if i < len(expand_word) and expand_word[i]:
                self._emit(Op.EXPAND_STKTOP, word_count)
        self._emit(Op.INVOKE_EXPANDED, comment=cmd)
        self._emit(Op.POP)
        self._used_generic_invoke = True

    def _emit_call(self: _Emitter, cmd: str, args: tuple[str, ...]) -> None:
        # break/continue inside loops: emit jump4 instead of invokeStk.
        if cmd == "continue" and self._continue_target is not None and not args:
            self._emit(Op.JUMP4, self._continue_target, comment="continue")
            return
        if cmd == "break" and self._break_target is not None and not args:
            self._emit(Op.JUMP4, self._break_target, comment="break")
            return
        if self._try_bytecoded(cmd, args):
            return
        self._push_lit(cmd)
        for a in args:
            self._emit_value(a)
        argc = 1 + len(args)
        op = Op.INVOKE_STK1 if argc < 256 else Op.INVOKE_STK4
        self._emit(op, argc, comment=cmd)
        self._emit(Op.POP)
        self._used_generic_invoke = True

    def _tag_last_invoke_source(self: _Emitter, argv_texts: tuple[str, ...]) -> None:
        """Set ``source_cmd_text`` on the most recent INVOKE instruction.

        Walks backwards through ``self._instrs`` to find the last
        INVOKE_STK1/INVOKE_STK4/INVOKE_EXPANDED and stores the original
        (pre-substitution) command text so that the VM can produce
        accurate errorInfo "invoked from within" frames.
        """
        _INVOKE_OPS = (Op.INVOKE_STK1, Op.INVOKE_STK4, Op.INVOKE_EXPANDED)
        for i in range(len(self._instrs) - 1, -1, -1):
            if self._instrs[i].op in _INVOKE_OPS:
                # Reconstruct the command text, quoting args that contain
                # whitespace to approximate the original source.  Args
                # containing $ or [ are compound words (with substitutions)
                # that were not originally quoted, so skip quoting those.
                parts: list[str] = []
                for a in argv_texts:
                    needs_quote = " " in a or "\t" in a or "\n" in a or not a
                    has_subst = "$" in a or "[" in a
                    if needs_quote and not has_subst:
                        parts.append(f'"{a}"')
                    else:
                        parts.append(a)
                self._instrs[i].source_cmd_text = " ".join(parts)
                break
