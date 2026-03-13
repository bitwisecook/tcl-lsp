"""Mixin: expression emission."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ...parsing.substitution import backslash_subst as _tcl_backslash_subst
from ..cfg import CFGBranch, CFGGoto, CFGReturn
from ..expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    render_expr,
)
from ..tcl_expr_eval import _parse_literal, eval_tcl_expr, format_tcl_value
from ._helpers import _tcl_hash_table_order
from .opcodes import (
    _BINOP_MAP,
    _UNARYOP_MAP,
    Op,
)

if TYPE_CHECKING:
    from ._emitter import _Emitter


class _ExpressionsMixin:
    """Mixin: expression emission."""

    def _emit_expr(self: _Emitter, node: ExprNode, *, fold_no_dedup: bool = False) -> bool:  # noqa: C901
        """Compile an expression AST; leaves result on TOS.

        Returns ``True`` when the result is *guaranteed numeric* (arithmetic,
        comparison, logical, bitwise, or literal number), ``False`` when it
        might be a string (variable, ternary, function call, command subst).

        ``tryCvtToNumeric`` is **never** emitted by this method.  The caller
        is responsible for emitting it when the return value is ``False``
        and the context requires numeric coercion (e.g. top-level ``expr``).
        """
        # Constant folding: evaluate pure-constant expressions at compile time.
        # ExprCall is excluded — Tcl 9.0 always emits invokeStk for math
        # functions, even with constant arguments (e.g. sin(2.0)).
        if not isinstance(node, (ExprVar, ExprCommand, ExprRaw, ExprCall)):
            folded = eval_tcl_expr(node)
            if folded is not None:
                val = format_tcl_value(folded)
                # Tcl shares small boolean objects (0 and 1) so they
                # are always interned/deduped.  Larger computed values
                # create fresh Tcl_Obj instances → no dedup.
                if fold_no_dedup and val not in ("0", "1"):
                    self._push_lit_no_dedup(val)
                else:
                    self._push_lit(val)
                return True  # folded constants are always numeric

        match node:
            case ExprLiteral(text=text):
                # Validate prefix literals at compile time — invalid
                # prefixed numbers (0o289, 0xGG) must error at runtime.
                # Route through EXPR_STK for validation.
                _clean = text.strip().lstrip("+-")
                if len(_clean) > 1 and _clean[0] == "0" and _clean[1:2] in ("oOxXbB"):
                    try:
                        int(text.strip(), 0)
                    except (ValueError, TypeError):
                        self._push_lit(text)
                        self._emit(Op.EXPR_STK)
                        return True  # exprStk guarantees numeric
                self._push_lit(text)
                return True  # literal number

            case ExprString(text=text):
                # Strip surrounding delimiters — the parser keeps them
                if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
                    text = text[1:-1]
                elif len(text) >= 2 and text[0] == "{" and text[-1] == "}":
                    text = text[1:-1]
                # Process Tcl backslash escapes (\n, \t, \xNN, etc.)
                if "\\" in text:
                    text = _tcl_backslash_subst(text)
                self._push_lit(text)
                return False  # string literal — not guaranteed numeric

            case ExprVar(text=text, name=name):
                # text includes $ and optional array index: $arr(key)
                var_ref = text.lstrip("$") if "(" in text else name
                self._load_var(var_ref)
                return False  # variable — not guaranteed numeric

            case ExprBinary(op=op, left=left, right=right):
                # Short-circuit evaluation for && and ||
                if op == BinOp.AND:
                    false_lbl = self._fresh_label("and_f")
                    end_lbl = self._fresh_label("and_end")
                    self._emit_expr(left)
                    self._emit(Op.JUMP_FALSE4, false_lbl)
                    self._emit_expr(right)
                    self._emit(Op.JUMP_FALSE4, false_lbl)
                    self._push_lit("1")
                    self._emit(Op.JUMP4, end_lbl)
                    self._place_label(false_lbl)
                    self._push_lit("0")
                    self._place_label(end_lbl)
                elif op == BinOp.OR:
                    true_lbl = self._fresh_label("or_t")
                    end_lbl = self._fresh_label("or_end")
                    self._emit_expr(left)
                    self._emit(Op.JUMP_TRUE4, true_lbl)
                    self._emit_expr(right)
                    self._emit(Op.JUMP_TRUE4, true_lbl)
                    self._push_lit("0")
                    self._emit(Op.JUMP4, end_lbl)
                    self._place_label(true_lbl)
                    self._push_lit("1")
                    self._place_label(end_lbl)
                else:
                    bc = _BINOP_MAP.get(op)
                    if bc is not None:
                        self._emit_expr(left)
                        self._emit_expr(right)
                        self._emit(bc)
                    else:
                        # fallback to exprStk
                        self._push_lit(render_expr(node))
                        self._emit(Op.EXPR_STK)
                return True  # all binary ops produce numeric results

            case ExprUnary(op=op, operand=operand):
                bc = _UNARYOP_MAP.get(op)
                if bc is not None:
                    self._emit_expr(operand)
                    self._emit(bc)
                else:
                    self._push_lit(render_expr(node))
                    self._emit(Op.EXPR_STK)
                return True  # all unary ops produce numeric results

            case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
                # Try to fold: if condition is a compile-time constant
                # AND the selected branch is also constant (no variables
                # or command substitutions), emit just that branch.
                # tclsh 9.0 only folds when all parts are constant.
                cond_val = eval_tcl_expr(cond)
                if cond_val is None and isinstance(cond, ExprString):
                    # ExprString like "yes" — try to parse as boolean.
                    inner = cond.text
                    if len(inner) >= 2 and inner[0] == '"' and inner[-1] == '"':
                        inner = inner[1:-1]
                    cond_val = _parse_literal(inner)
                if cond_val is not None:
                    selected = tb if cond_val else fb
                    branch_ok = eval_tcl_expr(selected) is not None or isinstance(
                        selected, (ExprLiteral, ExprString)
                    )
                    if branch_ok:
                        self._emit_expr(selected, fold_no_dedup=fold_no_dedup)
                        return True  # constant-folded — no tryCvtToNumeric needed
                false_lbl = self._fresh_label("tern_f")
                end_lbl = self._fresh_label("tern_end")
                self._emit_expr(cond)
                self._emit(Op.JUMP_FALSE4, false_lbl)
                self._emit_expr(tb)
                self._emit(Op.JUMP4, end_lbl)
                self._place_label(false_lbl)
                self._emit_expr(fb)
                self._place_label(end_lbl)
                return False  # ternary branches might be strings

            case ExprRaw(text=text):
                # Simple ${var} reference: emit as variable load.
                braced_scalar = self._parse_braced_scalar_ref(text)
                if braced_scalar is not None:
                    self._push_lit(braced_scalar)
                    self._emit(Op.LOAD_STK)
                else:
                    var_name = self._parse_simple_var_ref(text)
                    if var_name is not None:
                        self._load_var(var_name)
                    else:
                        self._push_lit(text)
                        self._emit(Op.EXPR_STK)
                        return False  # exprStk — not guaranteed numeric
                return False  # variable — not guaranteed numeric

            case ExprCall(function=func, args=call_args):
                self._push_lit(f"tcl::mathfunc::{func}")
                for arg in call_args:
                    self._emit_expr(arg)
                self._emit(Op.INVOKE_STK1, 1 + len(call_args))
                # Mark that a generic invoke has been seen so subsequent
                # commands get startCommand, but do NOT set
                # _used_generic_invoke (that would remove the tentative
                # startCommand for the current command).
                self._seen_generic_invoke = True
                return False  # function call — not guaranteed numeric

            case ExprCommand(text=text):
                self._emit_inline_cmd_subst(text)
                # Place deferred <cond> startCommand end label after
                # the command substitution instructions.
                if self._pending_cond_end_label is not None:
                    self._place_label(self._pending_cond_end_label)
                    self._pending_cond_end_label = None
                return False  # command substitution — not guaranteed numeric

            case _:
                self._push_lit(str(node))
                return False  # unknown — not guaranteed numeric
                self._emit(Op.EXPR_STK)

    # -- switch jumpTable --

    def _try_emit_jump_table(
        self: _Emitter,
        blk: object,
        next_block: str | None,
        skip_blocks: set[str],
    ) -> bool:
        """Detect a switch dispatch chain and emit a jumpTable opcode.

        Returns True if a jumpTable was emitted (caller should skip
        normal terminator emission).
        """

        term = blk.terminator  # type: ignore[attr-defined]
        if not isinstance(term, CFGBranch):
            return False

        # Check if this starts a switch dispatch chain:
        # a sequence of CFGBranch blocks comparing the same subject
        # via STR_EQ against literal patterns.
        cases: list[tuple[str, str]] = []  # (pattern, target_label)
        subject: ExprNode | None = None
        current_term = term
        dispatch_blocks: list[str] = []

        while isinstance(current_term, CFGBranch):
            cond = current_term.condition
            if not isinstance(cond, ExprBinary) or cond.op != BinOp.STR_EQ:
                break
            if not isinstance(cond.right, ExprLiteral):
                break

            this_subject = cond.left
            if subject is None:
                subject = this_subject
            elif render_expr(subject) != render_expr(this_subject):
                break

            cases.append((cond.right.text, current_term.true_target))

            # Follow false_target to next dispatch block
            next_blk = self._cfg.blocks.get(current_term.false_target)
            if next_blk is None:
                break
            dispatch_blocks.append(current_term.false_target)

            # Dispatch blocks should have no statements
            if next_blk.statements:
                break

            current_term = next_blk.terminator

        if len(cases) < 2 or subject is None:
            return False

        # The final dispatch block should have a CFGGoto to the default
        default_target: str | None = None
        if isinstance(current_term, CFGGoto):
            default_target = current_term.target
        else:
            return False

        # Emit: push subject, jumpTable, jump default
        self._emit_expr(subject)

        # Tcl 9.0's jumpTable entries appear in Tcl hash-table iteration
        # order (bucket-then-LIFO within each bucket).
        jt: dict[str, str] = {}
        for pattern, target in _tcl_hash_table_order(cases):
            jt[pattern] = target
        idx = self._emit(Op.JUMP_TABLE, 0)
        self._instrs[idx].jump_table = jt

        if default_target != next_block:
            self._emit(Op.JUMP4, default_target, comment=f"-> {default_target}")

        # Mark intermediate dispatch blocks to skip
        for db in dispatch_blocks:
            skip_blocks.add(db)

        return True

    # -- terminators --

    def _emit_term(
        self: _Emitter, term: CFGGoto | CFGBranch | CFGReturn, next_block: str | None
    ) -> None:
        match term:
            case CFGGoto(target=target):
                if target != next_block:
                    self._emit(Op.JUMP4, target, comment=f"-> {target}")

            case CFGBranch(condition=cond, true_target=tt, false_target=ft):
                # In proc bodies, the branch condition (e.g. from `if`)
                # counts as a command for startCommand numbering.
                if self._is_proc:
                    self._cmd_index += 1
                folded = self._fold_const_branch(cond)
                if folded is True:
                    # Constant true — unconditional goto true branch.
                    # Emit startCommand for the constant-folded if command
                    # in non-proc scripts (tclsh marks command boundaries
                    # even when the condition is constant-folded).
                    if not self._is_proc:
                        true_blk = self._cfg.blocks.get(tt)
                        if true_blk and isinstance(true_blk.terminator, CFGGoto):
                            join = true_blk.terminator.target
                            if join.startswith("if_end_"):
                                end_label = self._fresh_label("cmd_end")
                                self._emit(Op.START_CMD, end_label, 1)
                                self._cmd_index += 1
                                self._seen_generic_invoke = True
                                self._pending_join_labels[join] = end_label
                    if tt != next_block:
                        self._emit(Op.JUMP4, tt, comment=f"-> {tt}")
                elif folded is False:
                    # Constant false — unconditional goto false branch
                    if ft != next_block:
                        self._emit(Op.JUMP4, ft, comment=f"-> {ft}")
                else:
                    self._emit_expr(cond)
                    # tclsh emits a nop between a simple variable condition
                    # and the conditional jump (placeholder for tryCvtToNumeric).
                    # Inline catch also leaves a raw return code on the stack
                    # that needs the same treatment.
                    if isinstance(cond, (ExprVar, ExprRaw)):
                        self._emit(Op.NOP)
                    elif isinstance(cond, ExprCommand):
                        inner = cond.text.strip()
                        if inner.startswith("["):
                            inner = inner[1:]
                        if inner.startswith("catch "):
                            self._emit(Op.NOP)
                    if ft == next_block:
                        self._emit(Op.JUMP_TRUE4, tt, comment=f"-> {tt}")
                    elif tt == next_block:
                        self._emit(Op.JUMP_FALSE4, ft, comment=f"-> {ft}")
                    else:
                        self._emit(Op.JUMP_FALSE4, ft, comment=f"!-> {ft}")
                        self._emit(Op.JUMP4, tt, comment=f"-> {tt}")

            case CFGReturn(value=value):
                val = value if value is not None else ""
                self._emit_return_value(val)
                if self._is_proc:
                    self._emit(Op.DONE)
                else:
                    self._emit(Op.RETURN_IMM, 0, 0)

    def _emit_proc_return(
        self: _Emitter,
        term: CFGReturn,
        bname: str,
        next_block: str | None,
        block_order: list[str],
        block_idx: int,
    ) -> None:
        """Emit a ``return`` in a proc body with startCommand wrapping.

        tclsh wraps each compiled command in a proc body with
        ``startCommand``.  For ``return VALUE``, the count is 1; when
        *VALUE* is ``[expr {...}]``, the count is 2 (both ``return``
        and ``expr`` start at the same bytecode offset).

        When the return is in a then-branch (not the final block),
        tclsh also emits a dead-code ``jump`` past the else path
        after the ``done``.
        """
        val = term.value if term.value is not None else ""
        is_cmd_subst = val.startswith("[") and val.endswith("]")
        is_final = next_block is None

        # Determine startCommand count: 2 when return wraps [expr {...}]
        count = 2 if is_cmd_subst else 1

        # Find the join block for dead-code jump after done.
        # When this CFGReturn is inside a then-branch or switch arm,
        # tclsh emits a jump past the remaining arms/else path.
        join_block: str | None = None
        if not is_final and bname.startswith("if_then_"):
            # Look for the if_end_* block in the remaining block order
            for future_name in block_order[block_idx + 1 :]:
                if future_name.startswith("if_end_"):
                    join_block = future_name
                    break
        elif not is_final and bname.startswith(("switch_arm_body_", "switch_default_")):
            # Switch arm dead-code jump targets the proc's trailing done.
            # The switch_end block may be unreachable (all arms return),
            # so use a lazily-created proc-exit label instead.
            if self._proc_exit_label is None:
                self._proc_exit_label = self._fresh_label("proc_exit")
            join_block = self._proc_exit_label

        # Only emit startCommand for non-first commands in the proc body.
        end_label: str | None = None
        if self._cmd_index > 0:
            end_label = self._fresh_label("ret_end")
            self._emit(Op.START_CMD, end_label, count)
        self._cmd_index += 1

        self._emit_return_value(val)
        self._emit(Op.DONE)

        if end_label is not None:
            self._place_label(end_label)

        # tclsh 9.0 appends an unreachable ``done`` as the function
        # exit point when the tail return has a startCommand wrapper
        # (i.e. it is not the first/only command in the proc body).
        # Skip when control flow (CFGBranch) exists — the post-loop
        # "proc with control flow" check already adds the trailing done.
        has_branches = any(isinstance(b.terminator, CFGBranch) for b in self._cfg.blocks.values())
        if is_final and end_label is not None and not has_branches:
            self._emit(Op.DONE)

        if join_block is not None:
            # Dead-code jump past the else path (tclsh always emits this).
            self._emit(Op.JUMP4, join_block, comment="dead-skip-else")
