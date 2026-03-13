"""Composed _Emitter class and public API functions."""

from __future__ import annotations

from ..cfg import (
    CFGBranch,
    CFGFunction,
    CFGGoto,
    CFGModule,
    CFGReturn,
)
from ..expr_ast import (
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprVar,
)
from ..ir import (
    IRCall,
    IRModule,
    IRProcedure,
)
from ..tcl_expr_eval import eval_tcl_expr
from ._bytecoded import _BytecodedMixin
from ._cmd_subst import _CmdSubstMixin
from ._control_flow import _ControlFlowMixin
from ._expressions import _ExpressionsMixin
from ._peephole import _PeepholeMixin
from ._statements import _StatementsMixin
from ._types import FunctionAsm, Instruction, LiteralTable, LocalVarTable, ModuleAsm
from ._values import _ValuesMixin
from .layout import optimise_jumps, resolve_layout
from .opcodes import Op


class _Emitter(
    _ValuesMixin,
    _CmdSubstMixin,
    _ControlFlowMixin,
    _StatementsMixin,
    _BytecodedMixin,
    _ExpressionsMixin,
    _PeepholeMixin,
):
    """Emits bytecode instructions for a single CFGFunction."""

    def __init__(
        self,
        cfg: CFGFunction,
        params: tuple[str, ...] = (),
        *,
        optimise: bool = False,
        is_proc: bool = False,
        proc_defs: tuple[IRProcedure, ...] = (),
    ) -> None:
        self._cfg = cfg
        self._lit = LiteralTable()
        self._lvt = LocalVarTable(params)
        self._instrs: list[Instruction] = []
        self._labels: dict[str, int] = {}  # label → instruction index
        self._label_seq = 0
        self._optimise = optimise
        self._is_proc = is_proc
        self._proc_defs = proc_defs
        # Pending proc defs sorted by source line for interleaved emission.
        self._pending_proc_defs: list[IRProcedure] = sorted(
            (p for p in proc_defs if "$" not in p.name and "[" not in p.name),
            key=lambda p: p.range.start.line,
        )
        # Loop context for compiling break/continue as jump instructions.
        self._break_target: str | None = None
        self._continue_target: str | None = None
        # Catch nesting depth for beginCatch4 operand.
        self._catch_depth = 0
        # Command index for startCommand emission.
        self._cmd_index = 0
        # Label for the current startCommand's end-of-command marker.
        self._start_cmd_end_label: str | None = None
        # Whether a generic invoke (invokeStk1) has been seen.
        # After the first generic invoke, specialised commands get
        # startCommand markers (matching tclsh 9.0 behaviour).
        self._seen_generic_invoke = False
        self._used_generic_invoke = False
        self._used_inline_cmd_subst = False
        # 1-based source line of the current statement (for errorInfo).
        self._current_source_line: int = 0
        # Pending startCommand end labels for constant-folded branches.
        # Maps join-block name → label to place after the join pop.
        self._pending_join_labels: dict[str, str] = {}
        # Depth counter for nested math-function calls in expressions.
        # tryCvtToNumeric is emitted only at the outermost call (depth 0).
        self._expr_func_depth = 0
        # Deferred startCommand end label for ``<cond>`` synthetic
        # statements.  The label is placed after the first ExprCommand
        # in the condition expression is fully emitted.
        self._pending_cond_end_label: str | None = None
        # Label targeting the trailing proc done (used for dead-code
        # jumps after return in switch arm bodies).
        self._proc_exit_label: str | None = None

    # -- helpers --

    def _fresh_label(self, prefix: str = "L") -> str:
        self._label_seq += 1
        return f"{prefix}_{self._label_seq}"

    def _emit(self, op: Op, *operands: int | str, comment: str = "", source_line: int = 0) -> int:
        idx = len(self._instrs)
        sl = source_line or self._current_source_line
        self._instrs.append(Instruction(op=op, operands=operands, comment=comment, source_line=sl))
        return idx

    def _place_label(self, label: str) -> None:
        self._labels[label] = len(self._instrs)

    # -- block ordering --

    @staticmethod
    def _fold_const_branch(cond: ExprNode) -> bool | None:
        """Evaluate a branch condition at compile time.

        Returns ``True``/``False`` for constant conditions, ``None`` if
        the value is unknown at compile time.
        """
        if isinstance(cond, (ExprVar, ExprCommand, ExprRaw)):
            return None
        folded = eval_tcl_expr(cond)
        if folded is None:
            return None
        if isinstance(folded, float):
            return folded != 0.0
        if isinstance(folded, int):
            return folded != 0
        # String — try int conversion (Tcl truthiness)
        try:
            return int(str(folded)) != 0
        except (ValueError, TypeError):
            return None

    def _linearise(self) -> list[str]:
        """RPO traversal from entry, with dead-branch elimination and
        bottom-tested loop reordering."""
        visited: set[str] = set()
        order: list[str] = []
        fold = self._fold_const_branch

        def dfs(name: str) -> None:
            if name in visited or name not in self._cfg.blocks:
                return
            visited.add(name)
            blk = self._cfg.blocks[name]
            match blk.terminator:
                case CFGGoto(target=t):
                    dfs(t)
                case CFGBranch(condition=cond, true_target=tt, false_target=ft):
                    folded = fold(cond)
                    if folded is True:
                        # For loop headers, always visit the end block
                        # so break jumps have a valid target.
                        if name.startswith(self._LOOP_HEADER_PREFIXES):
                            dfs(ft)
                        dfs(tt)
                    elif folded is False:
                        dfs(ft)  # dead-branch: skip true path
                    else:
                        # Visit false-target first so that in the reversed
                        # post-order the true-target (then-body) appears
                        # immediately after the condition block.  This
                        # matches tclsh's layout where the condition falls
                        # through into the then-body and jumpFalse skips it.
                        dfs(ft)
                        dfs(tt)
                case _:
                    pass
            order.append(name)

        dfs(self._cfg.entry)
        order.reverse()
        # Unreachable blocks (e.g. dead branches from constant-folded
        # conditions) are intentionally omitted.
        return self._reorder_bottom_tested(order)

    def _reorder_bottom_tested(self, order: list[str]) -> list[str]:
        """Move loop body/step blocks before their header (condition test).

        This produces a bottom-tested loop pattern where the condition is
        at the bottom: the conditional jump becomes the back-edge and the
        unconditional back-edge jump is eliminated via fallthrough.

        Before (top-tested):
            header: <cond> jumpTrue body | end: done | body: ... jump header

        After (bottom-tested):
            jump header | body: ... [fallthrough] | header: <cond> jumpTrue body | end: done
        """
        pos = {name: i for i, name in enumerate(order)}

        # Find back-edges: blocks with a goto to an earlier block in RPO
        back_edges: list[tuple[str, str]] = []  # (source, header)
        for name in order:
            blk = self._cfg.blocks.get(name)
            if blk is None:
                continue
            match blk.terminator:
                case CFGGoto(target=t):
                    if t in pos and pos[t] < pos[name]:
                        back_edges.append((name, t))

        if not back_edges:
            return order

        result = list(order)

        # Process each back-edge: innermost loops first (later in RPO)
        for _back_src, header in sorted(back_edges, key=lambda x: -pos[x[0]]):
            header_blk = self._cfg.blocks.get(header)
            if header_blk is None:
                continue
            # Only reorder loops where the header is a conditional branch
            if not isinstance(header_blk.terminator, CFGBranch):
                continue
            # foreach uses a top-test pattern (foreach_start handles
            # the branch); don't reorder to bottom-test.
            if header.startswith("foreach_header_"):
                continue

            body_start = header_blk.terminator.true_target
            exit_block = header_blk.terminator.false_target
            # Collect all blocks in the loop body (reachable from body_start
            # that eventually reach back to header)
            loop_blocks: set[str] = set()
            self._collect_loop_body(body_start, header, loop_blocks, exit_block)

            if not loop_blocks:
                continue

            # Extract loop blocks preserving their relative order
            loop_ordered = [b for b in result if b in loop_blocks]
            for b in loop_ordered:
                result.remove(b)
            # Insert them just before header
            h_pos = result.index(header)
            for i, b in enumerate(loop_ordered):
                result.insert(h_pos + i, b)

        return result

    def _collect_loop_body(
        self, start: str, header: str, result: set[str], exit_block: str | None = None
    ) -> None:
        """Collect blocks reachable from *start* that are part of a loop
        back to *header* (i.e. all blocks on paths from body to header).

        Blocks that are the loop's *exit_block* (or beyond) are excluded
        so that ``break`` jumps don't pull exit blocks into the body.
        """
        if start == header or start in result or start not in self._cfg.blocks:
            return
        if start == exit_block:
            return
        result.add(start)
        blk = self._cfg.blocks[start]
        match blk.terminator:
            case CFGGoto(target=t):
                self._collect_loop_body(t, header, result, exit_block)
            case CFGBranch(true_target=tt, false_target=ft):
                self._collect_loop_body(tt, header, result, exit_block)
                self._collect_loop_body(ft, header, result, exit_block)

    # -- top-level --

    def _emit_one_proc_def(self, proc_def: IRProcedure) -> None:
        """Emit a single proc definition call.

        tclsh emits each ``proc`` definition at top level as::

            push "proc"
            push <name>
            push <params>
            push <body_source>
            invokeStk1 4
            pop
        """
        self._push_lit("proc")
        self._push_lit(proc_def.name)
        # Wrap params and body in braces so the VM's push handler
        # strips them without performing variable/command substitution.
        # tclsh stores the raw values without braces, but our PUSH
        # handler substitutes ``$var``/``[cmd]`` in unbraced literals;
        # the test normalisation layer strips the outer braces when
        # comparing against reference disassembly.
        # Skip wrapping for empty strings — they never trigger substitution.
        params = proc_def.params_raw
        body = proc_def.body_source or ""
        self._push_lit("{" + params + "}" if params else "")
        self._push_lit("{" + body + "}" if body else "")
        self._emit(Op.INVOKE_STK1, 4)
        self._emit(Op.POP)
        self._cmd_index += 1
        self._seen_generic_invoke = True

    def _emit_pending_proc_defs(self, before_line: int) -> None:
        """Emit pending proc definitions that appear before *before_line*."""
        while self._pending_proc_defs and self._pending_proc_defs[0].range.start.line < before_line:
            self._emit_one_proc_def(self._pending_proc_defs.pop(0))

    def _flush_proc_defs(self) -> None:
        """Emit any remaining pending proc definitions."""
        while self._pending_proc_defs:
            self._emit_one_proc_def(self._pending_proc_defs.pop(0))

    # Block name prefixes for if/switch join blocks — their incoming
    # edges carry a value on TOS (the arm's result).
    _VALUE_JOIN_PREFIXES = ("if_end_", "switch_end_")

    # Block name prefixes for loop exit blocks — the loop command's
    # result is always the empty string.
    _LOOP_END_PREFIXES = ("while_end_", "for_end_", "foreach_end_")

    _LOOP_HEADER_PREFIXES = ("for_header_", "while_header_")

    def _build_loop_context(self) -> dict[str, tuple[str, str]]:
        """Map each loop-body block to its (continue_target, break_target).

        For ``for`` loops, continue jumps to the step block; for ``while``
        and ``foreach`` loops, continue jumps to the header block.  Break
        always jumps to the end block.
        """
        _all_loop_headers = self._LOOP_HEADER_PREFIXES + ("foreach_header_",)
        ctx: dict[str, tuple[str, str]] = {}
        for bname, blk in self._cfg.blocks.items():
            if not bname.startswith(_all_loop_headers):
                continue
            if not isinstance(blk.terminator, CFGBranch):
                continue
            body_start = blk.terminator.true_target
            end_block = blk.terminator.false_target
            # Determine continue target: step block for for loops, header for while
            if bname.startswith("for_header_"):
                # Find the step block: it has a goto back to header
                cont_target: str | None = None
                for bn, bl in self._cfg.blocks.items():
                    if bn.startswith("for_step_") and isinstance(bl.terminator, CFGGoto):
                        if bl.terminator.target == bname:
                            cont_target = bn
                            break
                if cont_target is None:
                    continue
            else:
                cont_target = bname
            # Collect body blocks (excluding exit)
            body_blocks: set[str] = set()
            self._collect_loop_body(body_start, bname, body_blocks, end_block)
            for bb in body_blocks:
                ctx[bb] = (cont_target, end_block)
        return ctx

    def generate(self) -> FunctionAsm:  # noqa: C901
        block_order = self._linearise()
        loop_ctx = self._build_loop_context()
        skip_blocks: set[str] = set()
        # Identify foreach blocks for opcode-based compilation.
        # Maps header → (body_block, end_block, synthetic IRCall)
        foreach_info: dict[str, tuple[str, str, IRCall]] = {}
        foreach_bodies: set[str] = set()
        foreach_end_labels: dict[str, str] = {}  # end_block → deferred label
        # For-init tracking: maps for_end block name → deferred
        # startCommand end label.  The for-init's startCommand spans
        # the entire for command; the end label is placed at for_end.
        for_init_end_labels: dict[str, str] = {}
        # For-body startCommand tracking: maps convergence block (e.g.
        # if_end) → deferred end label for the for-body wrapper.
        for_body_end_labels: dict[str, str] = {}
        # While-loop startCommand tracking: maps while_end block name
        # → deferred end label for the while command wrapper.
        while_end_labels: dict[str, str] = {}
        for _bn, _blk in self._cfg.blocks.items():
            if _bn.startswith("foreach_header_") and isinstance(_blk.terminator, CFGBranch):
                _body = _blk.terminator.true_target
                _end = _blk.terminator.false_target
                for _st in _blk.statements:
                    if isinstance(_st, IRCall) and _st.command in (
                        "foreach",
                        "lmap",
                    ):
                        foreach_info[_bn] = (_body, _end, _st)
                        foreach_bodies.add(_body)
                        break
        # Identify try/finally patterns for inline bytecode compilation.
        # Maps try_body block → (try_end, try_finally, try_after_finally).
        try_finally_info: dict[str, tuple[str, str, str]] = {}
        for _bn in block_order:
            if not _bn.startswith("try_body_"):
                continue
            # Follow goto chain from try_body to find try_end.
            _current = _bn
            _try_end: str | None = None
            _tf_visited: set[str] = set()
            while _current not in _tf_visited:
                _tf_visited.add(_current)
                _tf_blk = self._cfg.blocks.get(_current)
                if not _tf_blk or not isinstance(_tf_blk.terminator, CFGGoto):
                    break
                _tf_next = _tf_blk.terminator.target
                if _tf_next.startswith("try_end_"):
                    _try_end = _tf_next
                    break
                _current = _tf_next
            if not _try_end:
                continue
            # Follow from try_end to find try_finally.
            _te_blk = self._cfg.blocks.get(_try_end)
            if not _te_blk or not isinstance(_te_blk.terminator, CFGGoto):
                continue
            _tf_name = _te_blk.terminator.target
            if not _tf_name.startswith("try_finally_"):
                continue
            # Follow from try_finally to find try_after_finally.
            _current = _tf_name
            _try_after: str | None = None
            _tf_visited2: set[str] = set()
            while _current not in _tf_visited2:
                _tf_visited2.add(_current)
                _tf_blk2 = self._cfg.blocks.get(_current)
                if not _tf_blk2 or not isinstance(_tf_blk2.terminator, CFGGoto):
                    break
                _tf_next2 = _tf_blk2.terminator.target
                if _tf_next2.startswith("try_after_finally_"):
                    _try_after = _tf_next2
                    break
                _current = _tf_next2
            if _try_after:
                try_finally_info[_bn] = (_try_end, _tf_name, _try_after)
                skip_blocks.update([_try_end, _tf_name, _try_after])
        # Complex foreach: bodies with a branch terminator (if condition
        # as first element).  These need foreach_step/foreach_end at the
        # foreach_end block (bottom-tested) and startCommand wrapping
        # for the if condition — matching tclsh 9.0.
        foreach_complex: dict[str, str] = {}  # header → end block
        foreach_step_labels: dict[str, str] = {}  # header → step label
        foreach_break_labels: dict[str, str] = {}  # header → break label
        foreach_end_to_header: dict[str, str] = {}  # end_block → header
        foreach_body_blocks: set[str] = set()
        for _fh, (_fb, _fe, _fc) in foreach_info.items():
            _fb_blk = self._cfg.blocks.get(_fb)
            if _fb_blk and not _fb_blk.statements and isinstance(_fb_blk.terminator, CFGBranch):
                foreach_complex[_fh] = _fe
                _sl = self._fresh_label("foreach_continue")
                _bl = self._fresh_label("foreach_break")
                foreach_step_labels[_fh] = _sl
                foreach_break_labels[_fh] = _bl
                foreach_end_to_header[_fe] = _fh
                # Update loop_ctx: redirect continue to step label,
                # break to break label (past foreach_end opcode).
                for _bb in list(loop_ctx):
                    _ct, _bt = loop_ctx[_bb]
                    if _ct == _fh:
                        loop_ctx[_bb] = (_sl, _bl)
                        foreach_body_blocks.add(_bb)
        # Emit proc defs that appear before the first statement in any block.
        # This handles cases where the entry block has no statements (e.g.
        # constant-folded ``if {1}`` condition) but proc defs must appear
        # before the control flow.
        if self._pending_proc_defs:
            first_stmt_line: int | None = None
            for blk in self._cfg.blocks.values():
                for stmt in blk.statements:
                    if hasattr(stmt, "range"):
                        line = stmt.range.start.line
                        if first_stmt_line is None or line < first_stmt_line:
                            first_stmt_line = line
            if first_stmt_line is not None:
                self._emit_pending_proc_defs(first_stmt_line)
        for i, bname in enumerate(block_order):
            if bname in skip_blocks:
                # Dispatch block consumed by a jumpTable; just place label
                self._place_label(bname)
                continue
            blk = self._cfg.blocks[bname]
            self._place_label(bname)

            # try/finally inline compilation: emit the entire
            # try/finally sequence when we reach the try_body block.
            if bname in try_finally_info:
                _try_end, _try_finally, _try_after = try_finally_info[bname]
                self._emit_try_finally_inline(bname, _try_finally)
                continue

            # Update loop context for break/continue compilation.
            if bname in loop_ctx:
                self._continue_target, self._break_target = loop_ctx[bname]

            # Complex foreach: emit foreach_step + foreach_end at the
            # bottom of the loop body (before the loop-result push/pop).
            if bname in foreach_end_to_header:
                _feh = foreach_end_to_header[bname]
                self._place_label(foreach_step_labels[_feh])
                self._emit(Op.FOREACH_STEP)
                self._place_label(foreach_break_labels[_feh])
                self._emit(Op.FOREACH_END)

            # Loop-end blocks: push "" as the loop command's result.
            # Only needed when the loop is the last command (no subsequent
            # statements) — the "" becomes the done result.
            # When the loop is nested (end goes to a step/body block rather
            # than exit), emit 3 nops instead — no result value needed.
            is_loop_end = bname.startswith(self._LOOP_END_PREFIXES)
            if is_loop_end and not blk.statements:
                target = blk.terminator.target if isinstance(blk.terminator, CFGGoto) else None
                if target is not None and not target.startswith("exit_"):
                    self._lit.intern("")
                    self._emit(Op.NOP)
                    self._emit(Op.NOP)
                    self._emit(Op.NOP)
                elif isinstance(blk.terminator, CFGReturn) and blk.terminator.value is not None:
                    # Loop ends with an explicit return — the loop's empty
                    # result is unused; push and immediately pop it.
                    self._push_lit("")
                    self._emit(Op.POP)
                else:
                    self._push_lit("")
            # Place deferred foreach startCommand end label after push/pop.
            if bname in foreach_end_labels:
                _lbl = foreach_end_labels.pop(bname)
                # Place at the pop instruction (or end of instructions).
                if self._instrs and self._instrs[-1].op == Op.POP:
                    self._labels[_lbl] = len(self._instrs) - 1
                else:
                    self._labels[_lbl] = len(self._instrs)

            # Place deferred for-init startCommand end label at
            # the for_end block's pop (the loop-end cleanup).
            if bname in for_init_end_labels:
                _lbl = for_init_end_labels.pop(bname)
                if self._instrs and self._instrs[-1].op == Op.POP:
                    self._labels[_lbl] = len(self._instrs) - 1
                else:
                    self._labels[_lbl] = len(self._instrs)

            # Place deferred while-loop startCommand end label at
            # the while_end block's pop (the loop-end cleanup).
            if bname in while_end_labels:
                _lbl = while_end_labels.pop(bname)
                if self._instrs and self._instrs[-1].op == Op.POP:
                    self._labels[_lbl] = len(self._instrs) - 1
                else:
                    self._labels[_lbl] = len(self._instrs)

            # Empty loop body: emit 3 nops (Tcl 9.0 constant-folds the
            # empty body ``{}`` to push ""; pop which becomes 3 nops).
            _LOOP_BODY_PREFIXES = ("while_body_", "for_body_")
            if (
                bname.startswith(_LOOP_BODY_PREFIXES)
                and not blk.statements
                and isinstance(blk.terminator, CFGGoto)
            ):
                self._lit.intern("")
                self._emit(Op.NOP)
                self._emit(Op.NOP)
                self._emit(Op.NOP)

            # For-body startCommand: wrap the first command inside the
            # for-loop body with a startCommand (matching tclsh 9.0).
            if (
                bname.startswith("for_body_")
                and isinstance(blk.terminator, CFGBranch)
                and self._cmd_index > 0
            ):
                # Find the convergence block (if_end_*) where both
                # branches of the if-body merge.  The startCommand
                # spans from here to that join block's pop.
                _fb_end_label = self._fresh_label("for_body_end")
                self._emit(Op.START_CMD, _fb_end_label, 1)
                # Find the convergence point by following both branches
                _tt = blk.terminator.true_target
                _ft = blk.terminator.false_target
                _tt_blk = self._cfg.blocks.get(_tt)
                if _tt_blk and isinstance(_tt_blk.terminator, CFGGoto):
                    _join = _tt_blk.terminator.target
                    if _join.startswith("if_end_"):
                        for_body_end_labels[_join] = _fb_end_label

            # Place deferred for-body startCommand end labels at
            # the join block's pop (before the pop is emitted).
            if bname in for_body_end_labels:
                _lbl = for_body_end_labels.pop(bname)
                # Place after the pop that discards the if-body result.
                # The pop hasn't been emitted yet; it will be emitted
                # by the join-block handler below.  We need to defer
                # placement until after the pop.
                # Store for placement after the join pop is emitted.
                self._pending_join_labels[bname] = _lbl

            # If/switch join blocks that have their own work: pop the
            # incoming arm result before proceeding.  The value is
            # intermediate and not used.
            if bname.startswith(self._VALUE_JOIN_PREFIXES) and (
                blk.statements
                or isinstance(blk.terminator, CFGBranch)
                or (self._is_proc and isinstance(blk.terminator, CFGReturn))
                or (
                    isinstance(blk.terminator, CFGGoto)
                    and not blk.terminator.target.startswith("exit_")
                    and not blk.statements
                )
            ):
                # Place pending startCommand end labels for constant-folded
                # if statements BEFORE the join pop.  The startCommand covers
                # only the command body, not the result cleanup pop.
                if bname in self._pending_join_labels:
                    self._place_label(self._pending_join_labels.pop(bname))
                self._emit(Op.POP)

            # foreach opcode compilation
            # foreach_header: emit loadScalar + foreach_start (skip branch)
            if bname in foreach_info:
                _fe_body, _fe_end, _fe_call = foreach_info[bname]
                # Emit startCommand spanning entire foreach.
                if self._cmd_index > 0:
                    _fe_lbl = self._fresh_label("cmd_end")
                    self._emit(Op.START_CMD, _fe_lbl, 1)
                    self._seen_generic_invoke = True
                    foreach_end_labels[_fe_end] = _fe_lbl
                # Load list variable(s).
                for _la in _fe_call.args:
                    self._emit_value(_la)
                # Emit foreach_start with aux data index 0.
                self._emit(Op.FOREACH_START, 0)
                self._cmd_index += 1
                continue  # skip normal stmt/terminator emission

            # foreach_body: emit body normally, then foreach_step + foreach_end.
            # Complex bodies (branch terminator = if condition) fall through
            # to normal block processing; step/end emitted at foreach_end.
            if bname in foreach_bodies and bname not in foreach_body_blocks:
                for stmt in blk.statements:
                    if self._pending_proc_defs and hasattr(stmt, "range"):
                        self._emit_pending_proc_defs(stmt.range.start.line)
                    self._emit_stmt_with_start_cmd(stmt)
                self._emit(Op.FOREACH_STEP)
                self._emit(Op.FOREACH_END)
                continue  # skip goto terminator

            # Detect for-init blocks: the first init statement before goto to
            # for_header gets a wider startCommand spanning the entire
            # for loop (count=2: for command + init command).
            _is_for_init_block = (
                self._is_proc
                and isinstance(blk.terminator, CFGGoto)
                and blk.terminator.target.startswith("for_header_")
                and not bname.startswith("for_step_")
                and blk.statements
            )
            # Find the index of the first for-init statement by checking
            # which statements fall within the for command's source range.
            _for_init_first_idx: int | None = None
            if _is_for_init_block:
                assert isinstance(blk.terminator, CFGGoto)
                _fi_header = blk.terminator.target
                _fi_header_blk = self._cfg.blocks.get(_fi_header)
                if _fi_header_blk and isinstance(_fi_header_blk.terminator, CFGBranch):
                    _fi_end = _fi_header_blk.terminator.false_target
                    _fi_info = self._cfg.loop_nodes.get(_fi_end)
                    if _fi_info:
                        _, _ir_for = _fi_info
                        _for_range_off = _ir_for.range.start.offset
                        for _si, _s in enumerate(blk.statements):
                            if hasattr(_s, "range") and _s.range.start.offset >= _for_range_off:
                                _for_init_first_idx = _si
                                break
                # Fallback: if we couldn't find the boundary, use the last statement.
                if _for_init_first_idx is None:
                    _for_init_first_idx = len(blk.statements) - 1
            for stmt_idx, stmt in enumerate(blk.statements):
                # Interleave pending proc defs in source order.
                if self._pending_proc_defs and hasattr(stmt, "range"):
                    self._emit_pending_proc_defs(stmt.range.start.line)
                if _is_for_init_block and stmt_idx == _for_init_first_idx:
                    # For-init: emit startCommand with deferred end label
                    # and count=2 (for + init both start at this offset).
                    assert isinstance(blk.terminator, CFGGoto)
                    _header = blk.terminator.target
                    _header_blk = self._cfg.blocks.get(_header)
                    _for_end: str | None = None
                    if _header_blk and isinstance(_header_blk.terminator, CFGBranch):
                        _for_end = _header_blk.terminator.false_target
                    if _for_end and self._cmd_index > 0:
                        _fi_label = self._fresh_label("for_cmd_end")
                        for_init_end_labels[_for_end] = _fi_label
                        self._emit_stmt_with_start_cmd(
                            stmt, count_override=2, deferred_end_label=_fi_label
                        )
                    else:
                        self._emit_stmt_with_start_cmd(stmt)
                elif isinstance(stmt, IRCall) and stmt.command == "<cond>":
                    # Synthetic condition placeholder: defer the end
                    # label so the startCommand spans the ExprCommand
                    # in the branch condition (emitted by _emit_term).
                    _cond_label = self._fresh_label("cmd_end")
                    self._pending_cond_end_label = _cond_label
                    self._emit_stmt_with_start_cmd(stmt, deferred_end_label=_cond_label)
                else:
                    self._emit_stmt_with_start_cmd(stmt)

            # If/switch arms: keep last statement value on TOS instead
            # of popping it — the value is the arm's result.
            if isinstance(blk.terminator, CFGGoto):
                target = blk.terminator.target
                if target.startswith(self._VALUE_JOIN_PREFIXES):
                    if blk.statements and self._instrs and self._instrs[-1].op == Op.POP:
                        del self._instrs[-1]
                    elif not blk.statements and not is_loop_end:
                        # Else-less if: false path needs an empty-string result.
                        self._push_lit("")

            # While-loop startCommand: emit before the jump to
            # while_header.  For ``while 1`` (constant-true condition),
            # skip the jump entirely — the body follows immediately.
            _skip_while_term = False
            if (
                self._is_proc
                and isinstance(blk.terminator, CFGGoto)
                and blk.terminator.target.startswith("while_header_")
                and not bname.startswith(("while_body_", "while_step_"))
                and self._cmd_index > 0
            ):
                _wh_header = blk.terminator.target
                _wh_header_blk = self._cfg.blocks.get(_wh_header)
                _wh_end: str | None = None
                _wh_const_true = False
                if _wh_header_blk and isinstance(_wh_header_blk.terminator, CFGBranch):
                    _wh_end = _wh_header_blk.terminator.false_target
                    _wh_cond = _wh_header_blk.terminator.condition
                    if isinstance(_wh_cond, ExprLiteral) and _wh_cond.text in ("1", "true"):
                        _wh_const_true = True
                if _wh_end:
                    _wh_label = self._fresh_label("while_cmd_end")
                    self._emit(Op.START_CMD, _wh_label, 1)
                    while_end_labels[_wh_end] = _wh_label
                    self._cmd_index += 1
                    if _wh_const_true:
                        _skip_while_term = True

            # Complex foreach: wrap if-condition branch terminators in
            # startCommand (matching tclsh which wraps each if command).
            if (
                bname in foreach_body_blocks
                and isinstance(blk.terminator, CFGBranch)
                and self._cmd_index > 0
                and not bname.startswith("foreach_header_")
            ):
                _fif_end_label = self._fresh_label("foreach_if_end")
                self._emit(Op.START_CMD, _fif_end_label, 1)
                self._seen_generic_invoke = True
                _tt = blk.terminator.true_target
                _tt_blk = self._cfg.blocks.get(_tt)
                if _tt_blk and isinstance(_tt_blk.terminator, CFGGoto):
                    _join = _tt_blk.terminator.target
                    if _join.startswith(("if_end_", "if_next_")):
                        for_body_end_labels[_join] = _fif_end_label

            # Complex foreach: suppress back-edge gotos to the foreach
            # header from body blocks.  Falls through to
            # foreach_step/foreach_end at the foreach_end block, or jumps
            # to the step label if not adjacent.
            _foreach_backedge = False
            if (
                isinstance(blk.terminator, CFGGoto)
                and blk.terminator.target in foreach_complex
                and bname in foreach_body_blocks
            ):
                _feh = blk.terminator.target
                _fe_end = foreach_complex[_feh]
                _step_lbl = foreach_step_labels[_feh]
                next_peek = block_order[i + 1] if i + 1 < len(block_order) else None
                if next_peek == _fe_end:
                    pass  # fall through to step/end
                else:
                    self._emit(Op.JUMP4, _step_lbl)
                _foreach_backedge = True

            next_block = block_order[i + 1] if i + 1 < len(block_order) else None
            if blk.terminator is not None and not _skip_while_term and not _foreach_backedge:
                if self._try_emit_jump_table(blk, next_block, skip_blocks):
                    # Switch dispatch counts as a command so the first
                    # arm body gets its own startCommand.
                    self._cmd_index += 1
                elif self._is_proc and isinstance(blk.terminator, CFGReturn):
                    # In proc bodies, CFGReturn needs startCommand wrapping
                    # and special handling for dead-code jumps.
                    self._emit_proc_return(
                        blk.terminator,
                        bname,
                        next_block,
                        block_order,
                        i,
                    )
                else:
                    self._emit_term(blk.terminator, next_block)
            elif next_block is not None and not _skip_while_term and not _foreach_backedge:
                # Terminal block not last in layout — emit done to prevent
                # fallthrough into successor blocks.
                self._emit(Op.DONE)
        # Flush any remaining proc defs that come after all statements.
        self._flush_proc_defs()
        # Empty proc body: push "" as the implicit return value.
        # tclsh always emits ``push ""; done`` for empty proc bodies.
        if self._is_proc and not self._instrs:
            self._push_lit("")
        # Proc bodies with control flow: add a trailing done as
        # proc-level exit (catch-all after branching paths).
        if self._is_proc and any(
            isinstance(b.terminator, CFGBranch) for b in self._cfg.blocks.values()
        ):
            # Place the proc-exit label at this trailing done so that
            # dead-code jumps after switch arm returns land here.
            if self._proc_exit_label is not None:
                self._place_label(self._proc_exit_label)
            self._emit(Op.DONE)
        # ensure script ends with done
        elif not self._instrs or self._instrs[-1].op not in (Op.DONE, Op.RETURN_IMM):
            self._emit(Op.DONE)
        # Peephole: remove pop immediately before the final done.
        # tclsh leaves the last command's result on TOS for done to return.
        self._remove_trailing_pop()
        # Peephole: tail-position returnImm → done (tclsh uses done for
        # the last return at the end of a proc body).
        self._fold_tail_return_to_done()
        # Peephole: strip all startCommand if no generic invoke exists.
        # tclsh 9.0 only emits startCommand in units with invokeStk.
        self._strip_unused_start_cmd()
        # Peephole: for top-level scripts with generic invokes, remove
        # startCommand for generic invokes preceding the first compiled
        # command's startCommand (matching tclsh 9.0's two-pass behaviour).
        self._fixup_top_level_start_cmd()
        # Peephole: constant-folded exprs → 3 nops (matching tclsh).
        # Must run AFTER startCommand cleanup so that push+pop pairs
        # inside surviving startCommand wrappers are preserved.
        self._fold_const_push_pop_nops()
        # Peephole: re-dedup surviving push operands after nop conversion,
        # but only when the earliest slot was nop-ed (not still in use).
        # Skip surviving pushes that were created with fold_no_dedup
        # (tagged with _NO_DEDUP_TAG) — tclsh 9.0 keeps those at their
        # original indices.
        self._dedup_push_literals()
        # Strip internal no-dedup tags from comments.
        for i, instr in enumerate(self._instrs):
            if instr.comment and self._NO_DEDUP_TAG in instr.comment:
                self._instrs[i] = Instruction(
                    op=instr.op,
                    operands=instr.operands,
                    comment=instr.comment.replace(self._NO_DEDUP_TAG, ""),
                )
        return self._layout()

    # -- layout pass --

    _JUMP4_TO_JUMP1: dict[Op, Op] = {
        Op.JUMP4: Op.JUMP1,
        Op.JUMP_TRUE4: Op.JUMP_TRUE1,
        Op.JUMP_FALSE4: Op.JUMP_FALSE1,
    }

    def _layout(self) -> FunctionAsm:
        self._optimise_jumps()
        label_offsets = resolve_layout(self._instrs, self._labels)
        return FunctionAsm(
            name=self._cfg.name,
            literals=self._lit,
            lvt=self._lvt,
            instructions=self._instrs,
            labels=label_offsets,
        )

    def _optimise_jumps(self) -> None:
        """Replace 4-byte jumps with 1-byte jumps when offset fits."""
        optimise_jumps(self._instrs, self._labels, self._JUMP4_TO_JUMP1)


# Public API


def codegen_function(
    cfg: CFGFunction,
    params: tuple[str, ...] = (),
    *,
    optimise: bool = False,
    is_proc: bool = False,
) -> FunctionAsm:
    """Generate bytecode assembly for a single CFG function.

    When *is_proc* is ``True``, variables are accessed via the LVT
    (``loadScalar1``/``storeScalar1``).  When ``False`` (top-level
    scripts), variables are accessed via the stack
    (``loadStk``/``storeStk``), matching tclsh 9's top-level compilation.

    When *optimise* is ``True``, additional optimisation passes beyond
    the tclsh 9 baseline are applied (reserved for future use).
    """
    return _Emitter(cfg, params=params, optimise=optimise, is_proc=is_proc).generate()


def codegen_module(
    cfg_module: CFGModule,
    ir_module: IRModule,
    *,
    optimise: bool = False,
) -> ModuleAsm:
    """Generate bytecode assembly for an entire module."""
    # Proc definitions are now always emitted as runtime IRCall
    # statements at their original source positions (the lowering
    # always returns IRCall for ``proc``).  We no longer pre-emit
    # them via _pending_proc_defs because that would double-emit
    # and, more critically, emit proc defs from inside ``catch``
    # bodies at the top level where errors cannot be intercepted.
    top = _Emitter(
        cfg_module.top_level,
        optimise=optimise,
        is_proc=False,
    ).generate()
    procs: dict[str, FunctionAsm] = {}
    for qname, cfg_func in cfg_module.procedures.items():
        ir_proc = ir_module.procedures.get(qname)
        # Skip procs defined inside namespace eval — tclsh compiles
        # them lazily at runtime, not at compile time.
        if ir_proc and ir_proc.namespace_scoped:
            continue
        params = ir_proc.params if ir_proc else ()
        procs[qname] = codegen_function(cfg_func, params=params, optimise=optimise, is_proc=True)
    return ModuleAsm(top_level=top, procedures=procs)
