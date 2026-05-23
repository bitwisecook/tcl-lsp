"""_WasmEmitterOptMixin: peephole optimiser, dead-code removal, CFG loop codegen."""

# canonicalisation: audited #246

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from ....cfg import (
    CFGBranch,
    CFGGoto,
    CFGReturn,
)
from ....expr_ast import (
    ExprNode,
    ExprRaw,
)
from ....ir import (
    IRBarrier,
    IRCall,
    IRExprEval,
    IRIncr,
)
from .._encoding import (
    _leb128_signed,
)
from .._ir import (
    _BLOCK_VOID,
    ValType,
    WasmInstruction,
    WasmOp,
    _decode_leb128_signed,
)
from .._ownership import Ownership
from ._statements import _escape_dquote
from ._variables import _is_dynamic_var_name


class _WasmEmitterOptMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterExprMixin
        def _emit_expr(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_stmt(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_frame_set_line(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_call_stmt_tail(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCmdMixin
        def _emit_cmd_return(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_uplevel(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterVarMixin
        def _emit_namespace_eval_bridge(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_read_obj_lenient(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj_keep(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterValuesMixin
        def _emit_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_obj_literal(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_box_int(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_unbox_int(self, *a: Any, **kw: Any) -> Any: ...

    def _body_references_info_level(self) -> bool:
        """Walk the CFG's IR for any ``info level`` / ``info frame``
        call so the prologue knows to push a frame even when the
        escape analysis says one isn't needed for local storage.

        Three shapes count:
          * bare ``info level`` / ``info frame`` (depth),
          * ``info level N`` / ``info frame N`` (returns the argv
            list / frame metadata for the referenced frame),
          * any bracketed ``[info level …]`` / ``[info frame …]``
            embedded in a value or expression string.

        All three need a real ``frame_depth`` / ``frame_argv`` /
        ``frame_info`` slot — without ``frame_push`` the callee
        observes its caller's values, which is silently wrong.
        Phase 8 follow-up: ``info frame`` joins ``info level`` here so
        compiled procs reach the prologue's ``frame_set_type`` /
        ``frame_set_proc_name`` / ``frame_set_script`` stamps, which
        produce the proc's metadata for the new query path.  Run
        once per proc at prologue time; linear in the number of
        statements.
        """
        from ....cfg import CFGFunction
        from ....ir import IRCall

        _INTROSPECT_SUBS = ("level", "frame")

        def _walk(stmt) -> bool:
            if isinstance(stmt, IRCall):
                if (
                    stmt.canonical_command == "::info"
                    and stmt.args
                    and stmt.args[0] in _INTROSPECT_SUBS
                ):
                    return True
                # Bracketed ``[info level …]`` / ``[info frame …]``
                # can appear as command-subst inside another call's
                # arguments (``puts [llength [info level 0]]``).
                # Substring scan finds them without re-parsing.
                for a in stmt.args:
                    if "[info level" in a or "[info frame" in a:
                        return True
            # ``return [info level ...]`` is an IRReturn whose
            # ``value`` carries the bracketed substitution as a
            # string.  Same substring rule applies — without this
            # branch the elision path missed every proc that
            # returns its own invocation argv (e.g. trace wrappers,
            # tcltest helpers).
            value = getattr(stmt, "value", None)
            if isinstance(value, str) and ("[info level" in value or "[info frame" in value):
                return True
            expr = getattr(stmt, "expr", None)
            if isinstance(expr, str) and ("[info level" in expr or "[info frame" in expr):
                return True
            # Recurse into bodies of compound statements.
            for attr in ("body", "init", "next", "else_body", "finally_body"):
                sub = getattr(stmt, attr, None)
                if sub is not None and hasattr(sub, "statements"):
                    for s in sub.statements:
                        if _walk(s):
                            return True
            arms = getattr(stmt, "arms", None)
            if arms is not None:
                for arm in arms:
                    arm_body = getattr(arm, "body", None)
                    if arm_body is not None and hasattr(arm_body, "statements"):
                        for s in arm_body.statements:
                            if _walk(s):
                                return True
            clauses = getattr(stmt, "clauses", None)
            if clauses is not None:
                for clause in clauses:
                    clause_body = getattr(clause, "body", None)
                    if clause_body is not None and hasattr(clause_body, "statements"):
                        for s in clause_body.statements:
                            if _walk(s):
                                return True
            return False

        if not isinstance(self._cfg, CFGFunction):
            return False
        # Walk every statement in every block of the CFG.  The
        # structured bodies we care about (``if``, ``for``,
        # ``foreach``, ``switch``) are flattened into the block
        # graph by the CFG builder, so block statements are the
        # complete expression of the proc body's IR calls.
        for block in self._cfg.blocks.values():
            for stmt in block.statements:
                if _walk(stmt):
                    return True
        return False

    # -- Optimisation passes applied after emission --

    def _run_optimisations(self) -> None:
        """Apply WASM-level optimisation passes."""
        if not self._optimise:
            return

        self._opt_peephole()
        self._opt_dead_code()

    def _opt_peephole(self) -> None:
        """Peephole optimisations on the instruction stream."""
        optimised: list[WasmInstruction] = []
        i = 0
        while i < len(self._body):
            instr = self._body[i]

            # Pattern: i64.const 0; i64.add → remove both (identity add)
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.I64_CONST
                and _decode_leb128_signed(instr.operands) == 0
                and self._body[i + 1].op == WasmOp.I64_ADD
            ):
                i += 2
                continue

            # Pattern: i64.const 1; i64.mul → remove both (identity multiply)
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.I64_CONST
                and _decode_leb128_signed(instr.operands) == 1
                and self._body[i + 1].op == WasmOp.I64_MUL
            ):
                i += 2
                continue

            # Pattern: i64.const 0; i64.mul → replace with i64.const 0 (zero multiply)
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.I64_CONST
                and _decode_leb128_signed(instr.operands) == 0
                and self._body[i + 1].op == WasmOp.I64_MUL
            ):
                # Drop the value on stack, push 0
                optimised.append(WasmInstruction(op=WasmOp.DROP, range=instr.range))
                optimised.append(
                    WasmInstruction(
                        op=WasmOp.I64_CONST, operands=_leb128_signed(0), range=instr.range
                    )
                )
                i += 2
                continue

            # Pattern: local.set X; local.get X → local.tee X
            if (
                i + 1 < len(self._body)
                and instr.op == WasmOp.LOCAL_SET
                and self._body[i + 1].op == WasmOp.LOCAL_GET
                and instr.operands == self._body[i + 1].operands
            ):
                optimised.append(
                    WasmInstruction(op=WasmOp.LOCAL_TEE, operands=instr.operands, range=instr.range)
                )
                i += 2
                continue

            # Pattern: nop → skip
            if instr.op == WasmOp.NOP:
                i += 1
                continue

            optimised.append(instr)
            i += 1

        self._body = optimised

    def _opt_dead_code(self) -> None:
        """Remove instructions after unconditional branches within blocks."""
        optimised: list[WasmInstruction] = []
        dead = False
        depth = 0

        for instr in self._body:
            if instr.op in (WasmOp.BLOCK, WasmOp.LOOP, WasmOp.IF):
                depth += 1
                dead = False
                optimised.append(instr)
            elif instr.op == WasmOp.ELSE:
                dead = False
                optimised.append(instr)
            elif instr.op == WasmOp.END:
                depth = max(0, depth - 1)
                dead = False
                optimised.append(instr)
            elif dead:
                continue  # skip dead instruction
            else:
                optimised.append(instr)
                if instr.op in (WasmOp.RETURN, WasmOp.UNREACHABLE, WasmOp.BR):
                    dead = True

        self._body = optimised

    # -- CFG traversal --

    def _is_exit_block(self, block_name: str) -> bool:
        """Check if a block is an exit (no statements, no terminator or return)."""
        block = self._cfg.blocks.get(block_name)
        if block is None:
            return True
        if not block.statements and block.terminator is None:
            return True
        if not block.statements and isinstance(block.terminator, CFGReturn):
            return block.terminator.value is None
        return False

    def _find_merge_block(self, *roots: str) -> str | None:
        """Find the first block reachable from all *roots* via CFGGoto chains.

        This detects the common "merge" or "join" block where divergent
        control-flow paths reconverge (e.g. after an ``if``/``else``).
        Only follows unconditional goto edges so that nested branches
        are not confused with the immediate post-dominator.
        """

        def _goto_successors(start: str) -> set[str]:
            reachable: set[str] = set()
            worklist = [start]
            while worklist:
                name = worklist.pop()
                if name in reachable:
                    continue
                reachable.add(name)
                # Don't follow successors of active loop headers (other than
                # the start block itself) — this prevents back-edge traversal
                # from causing blocks inside one branch to appear reachable
                # from the other branch (via the outer loop iteration).
                if name != start and name in self._active_loop_headers:
                    continue
                blk = self._cfg.blocks.get(name)
                if blk is None:
                    continue
                match blk.terminator:
                    case CFGGoto(target=t):
                        worklist.append(t)
                    case CFGBranch(true_target=tt, false_target=ft):
                        worklist.append(tt)
                        worklist.append(ft)
            return reachable

        sets = [_goto_successors(r) for r in roots]
        common = sets[0]
        for s in sets[1:]:
            common = common & s
        if not common:
            return None

        # Pick the merge block closest to the roots: walk from the first
        # root along goto edges and return the first block in *common*.
        visited: set[str] = set()
        frontier = [roots[0]]
        while frontier:
            name = frontier.pop(0)
            if name in visited:
                continue
            visited.add(name)
            if name in common and name not in roots:
                return name
            blk = self._cfg.blocks.get(name)
            if blk is None:
                continue
            match blk.terminator:
                case CFGGoto(target=t):
                    frontier.append(t)
                case CFGBranch(true_target=tt, false_target=ft):
                    frontier.append(tt)
                    frontier.append(ft)
        return None

    def _is_loop_header(self, header: str, body_start: str) -> bool:
        """Check if *header* is a loop header with a back-edge from the body.

        Returns ``True`` when following goto/branch edges from *body_start*
        reaches *header* again, indicating a loop back-edge.

        Stops the walk at any block in ``self._active_loop_headers``: if
        we reach an OUTER loop's header, we have followed the outer
        loop's back-edge and this is not a new nested loop.
        """
        visited: set[str] = set()
        worklist = [body_start]
        while worklist:
            name = worklist.pop()
            if name == header:
                return True
            if name in visited:
                continue
            # Outer loop header — stop the walk here so we don't wrap
            # around through the outer loop back to `header`.
            if name in self._active_loop_headers and name != header:
                continue
            visited.add(name)
            blk = self._cfg.blocks.get(name)
            if blk is None:
                continue
            match blk.terminator:
                case CFGGoto(target=t):
                    worklist.append(t)
                case CFGBranch(true_target=tt, false_target=ft):
                    worklist.append(tt)
                    worklist.append(ft)
        return False

    def _emit_cfg_loop(
        self,
        header: str,
        condition: ExprNode,
        body_start: str,
        exit_block: str,
    ) -> None:
        """Emit a WASM block/loop for a CFG for/while loop pattern.

        Structure: ``block { loop { br_if(exit); block { body };
        step; br(loop) } }``.  The *step* block (for-loop
        ``next_script``) is emitted **outside** the inner continue
        block so ``continue`` — which ``br``s out of the inner block
        — still runs the step before looping back to the header.
        Without this, ``for {set i 0} {$i<5} {incr i} {if {$i==2}
        continue; ...}`` would skip the incr and spin forever.

        Detection: the step block is any CFG block that (a) is
        reachable from *body_start* and (b) terminates with
        ``CFGGoto(header)``.  For ``while`` loops no such block
        exists, so the inner body is just the loop body as before.
        """
        # Mark the header as visited to prevent re-entry from the back-edge
        self._visited.add(header)
        # Track header so nested _is_loop_header walks don't confuse
        # outer back-edges with new nested loops.
        self._active_loop_headers.add(header)

        step_block = self._find_step_block(body_start, header)
        loop_label = "for" if step_block is not None else "while"

        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]), label=f"{loop_label} break")
        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]), label=loop_label)

        # Evaluate loop condition — break if false
        self._emit_expr(condition)
        self._emit_i64_const(0)
        self._emit(WasmOp.I64_EQ)
        self._emit_br_if(1)  # break out of block

        # Wrap body in a block for break/continue support
        self._emit(
            WasmOp.BLOCK,
            bytes([_BLOCK_VOID]),
            label=f"{loop_label} continue",
        )  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._loop_catch_depths.append(self._catch_depth)

        # Reserve the step block so ``_emit_loop_body`` stops before it
        # (treating its incoming goto as a "back-edge via step").
        if step_block is not None:
            self._visited.add(step_block)

        # Emit body blocks
        self._emit_loop_body(body_start, header)

        self._loop_ctrl_depths.pop()
        self._loop_catch_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        # Propagate ``break`` / ``continue`` from interpreter-side
        # bodies that ran through eval-fallback.  Compiled ``break``
        # already emits a wasm ``br`` directly, so these consume calls
        # only fire after a runtime-side flow-control signal (e.g.
        # ``while 1 { dict update d k v { break } }`` — the dict-
        # update body's ``break`` flips ``break_flag`` and we need to
        # see it here to exit the wasm loop).  We're inside the
        # ``LOOP{}`` (post-continue-BLOCK end), so depth 0 = LOOP,
        # depth 1 = outer break BLOCK.  Inside the ``IF{}`` the
        # depths shift up by 1, so ``br 2`` exits to the break block.
        bbreak = self._shared_imports.get("tcl_flow_consume_break")
        bcont = self._shared_imports.get("tcl_flow_consume_continue")
        if bbreak is not None:
            self._emit_call(bbreak)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_br(2)
            self._emit(WasmOp.END)
        if bcont is not None:
            self._emit_call(bcont)
            self._emit(WasmOp.DROP)

        # Emit the step block (if any) AFTER the continue block so
        # `continue` still runs it.
        if step_block is not None:
            self._visited.discard(step_block)
            step_blk = self._cfg.blocks.get(step_block)
            if step_blk is not None:
                self._visited.add(step_block)
                for stmt in step_blk.statements:
                    self._emit_stmt(stmt)

        # Loop back
        self._emit_br(0)
        self._emit(WasmOp.END)  # end loop
        self._emit(WasmOp.END)  # end block

        self._active_loop_headers.discard(header)

        # Continue with the exit block
        self._emit_block(exit_block)

    def _find_step_block(self, body_start: str, header: str) -> str | None:
        """Return the for-loop step block (``next_script``) in the body
        subgraph of a CFG loop, or ``None`` for while loops.

        The CFG builder names for-loop step blocks ``for_step_N``; any
        block with that prefix that (a) is reachable from *body_start*
        and (b) ``CFGGoto``-s back to *header* is the step block.
        Restricting to that name avoids mis-hoisting trailing body
        blocks of while-loops (which also goto the header but run on
        every iteration and must not be skipped by ``continue``).
        """
        visited: set[str] = set()
        stack = [body_start]
        while stack:
            name = stack.pop()
            if name in visited or name == header:
                continue
            visited.add(name)
            blk = self._cfg.blocks.get(name)
            if blk is None:
                continue
            term = blk.terminator
            match term:
                case CFGGoto(target=target):
                    if target == header and name.startswith("for_step_"):
                        return name
                    stack.append(target)
                case CFGBranch(true_target=tt, false_target=ft):
                    stack.append(tt)
                    stack.append(ft)
        return None

    def _emit_loop_body(self, start: str, header: str) -> None:
        """Emit all blocks reachable from *start* until reaching *header*.

        Follows goto chains linearly.  When a CFGBranch is encountered,
        checks whether it is a nested loop (back-edge from the true
        branch) and emits ``block{loop{...}}`` for it.  Plain branches
        (if/else) are emitted with ``if/else/end``.  Stops when a goto
        target is the outer loop's *header* (the back-edge).
        """
        current = start
        while current and current != header:
            if current in self._visited:
                return
            self._visited.add(current)

            blk = self._cfg.blocks.get(current)
            if blk is None:
                return

            # Emit statements
            for stmt in blk.statements:
                self._emit_stmt(stmt)

            match blk.terminator:
                case CFGGoto(target=target):
                    current = target
                case CFGBranch(condition=cond, true_target=tt, false_target=ft):
                    # Check for nested loop: the true-target body
                    # eventually loops back to this block.
                    if self._is_loop_header(current, tt):
                        # Emit a nested WASM loop for this inner header.
                        self._active_loop_headers.add(current)
                        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))
                        self._emit(WasmOp.LOOP, bytes([_BLOCK_VOID]))

                        self._emit_expr(cond)
                        self._emit_i64_const(0)
                        self._emit(WasmOp.I64_EQ)
                        self._emit_br_if(1)  # break if false

                        # Emit inner loop body — stops at current (inner back-edge)
                        self._emit_loop_body(tt, current)

                        self._emit_br(0)  # loop back
                        self._emit(WasmOp.END)  # end loop
                        self._emit(WasmOp.END)  # end block
                        self._active_loop_headers.discard(current)

                        # Continue with the false-target (loop exit)
                        current = ft
                    else:
                        # Plain branch (if/else inside loop body)
                        merge = self._find_merge_block(tt, ft)
                        merge_newly_added = merge is not None and merge not in self._visited
                        if merge_newly_added:
                            assert merge is not None  # implied by merge_newly_added
                            self._visited.add(merge)

                        self._emit_expr(cond)
                        self._emit_i64_const(0)
                        self._emit(WasmOp.I64_NE)
                        self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
                        self._emit_loop_body(tt, header)
                        self._emit(WasmOp.ELSE)
                        self._emit_loop_body(ft, header)
                        self._emit(WasmOp.END)

                        if merge_newly_added:
                            assert merge is not None  # implied by merge_newly_added
                            self._visited.discard(merge)
                            current = merge
                        else:
                            return
                case CFGReturn(value=value):
                    if value is not None:
                        self._emit_value(value)
                    else:
                        self._emit_i32_const(0)
                    self._emit(WasmOp.RETURN)
                    return
                case _:
                    return

    def _emit_foreach_loop(
        self,
        header_stmts: tuple,
        body_block: str,
        end_block: str,
        *,
        header_block: str = "",
    ) -> None:
        """Emit a list-iteration loop for a foreach CFG pattern.

        Supports both single-iterator (``foreach var list body``) and
        multi-iterator (``foreach v1 l1 v2 l2 ... body``) forms,
        including multi-var groups (``foreach {a b} l1 c l2 body``).

        The counter tracks the *iteration index* (0, 1, 2, …); element
        indices for each group are ``counter * group_size + slot``.
        The overall limit is ``max(ceil(len(Li)/group_size_i))`` across
        all iterators — matching reference Tcl semantics where the loop
        runs until the longest list is exhausted.

        Internal counter/limit are i64; the loop variable is i32 TclObj.
        """
        # Locate the foreach/lmap IRCall in the header
        foreach_call: "IRCall | None" = None
        for stmt in header_stmts:
            if isinstance(stmt, IRCall) and stmt.canonical_command in ("::foreach", "::lmap"):
                foreach_call = stmt
                break

        loop_vars: tuple[str, ...] = foreach_call.defs if foreach_call else ()
        list_args: tuple[str, ...] = foreach_call.args if foreach_call else ()
        groups: tuple[int, ...] | None = (
            foreach_call.foreach_groups if foreach_call is not None else None
        )

        # Build iterator descriptors: list of (list_arg, group_vars)
        iterators: list[tuple[str, tuple[str, ...]]] = []
        if groups and len(list_args) == len(groups):
            var_offset = 0
            for list_arg, gsize in zip(list_args, groups):
                iterators.append((list_arg, loop_vars[var_offset : var_offset + gsize]))
                var_offset += gsize
        elif list_args:
            # Fallback: single iterator, all vars form one group
            iterators = [(list_args[0], loop_vars)]
        else:
            iterators = [("", loop_vars)]

        llength_idx = self._shared_imports.get("tcl_list_length")
        lindex_idx = self._shared_imports.get("tcl_list_index")

        counter = self._add_extra_local("_foreach_i")
        limit = self._add_extra_local("_foreach_n")

        # Allocate one WASM local per iterator and load each list
        list_locals: list[int] = []
        for list_arg, _ in iterators:
            ll = self._add_extra_local("_foreach_list", ValType.I32)
            list_locals.append(ll)
            if list_arg:
                self._emit_value(list_arg)
            else:
                self._emit_i32_const(0)
            self._emit_local_set(ll)

        # Compute limit = max(ceil(len(Li) / group_size_i)) over all iterators.
        # Uses a running max: initialise to 0, then update for each iterator.
        # For each iterator: n_iters_i = ceil(len / gsize) = (len + gsize-1) / gsize.
        # Max update: if n_iters_i > limit: limit = n_iters_i.
        # Uses a scratch i64 local for the compare-and-select.
        limit_tmp = self._add_extra_local("_foreach_n_tmp")
        self._emit_i64_const(0)
        self._emit_local_set(limit)
        for ll, (_, group_vars) in zip(list_locals, iterators):
            gsize = max(len(group_vars), 1)
            if llength_idx is not None:
                self._emit_local_get(ll)
                self._emit_call(llength_idx)
                self._emit_unbox_int()
            else:
                self._emit_i64_const(0)
            if gsize > 1:
                # ceil(n, gsize) = (n + gsize - 1) / gsize (non-negative n)
                self._emit_i64_const(gsize - 1)
                self._emit(WasmOp.I64_ADD)
                self._emit_i64_const(gsize)
                self._emit(WasmOp.I64_DIV_S)
            # stack: [n_iters_i]  — update limit = max(limit, n_iters_i)
            self._emit_local_set(limit_tmp)
            self._emit_local_get(limit_tmp)
            self._emit_local_get(limit)
            self._emit(WasmOp.I64_GT_S)
            self._emit(WasmOp.IF, bytes([_BLOCK_VOID]))
            self._emit_local_get(limit_tmp)
            self._emit_local_set(limit)
            self._emit(WasmOp.END)

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

        # For each iterator, bind its variables from list[counter*gsize + slot].
        # Past-end indices return empty-string TclObjs from the runtime,
        # matching reference Tcl's ``foreach {a b} {1 2 3}`` semantics.
        # Route through ``_emit_var_write_obj`` (matching the non-CFG
        # path in ``_control_flow.py::_emit_foreach``) so FRAME-tagged
        # loop vars land in the runtime frame, not just a WASM local.
        # A bypass to plain ``local.set`` here makes ``foreach k L { ...
        # unset arr($k) ... }`` (and any other body that triggers
        # ``escape_all_known``) write the loop var to a WASM local while
        # the body's reads go through ``tcl_local_get_or_error`` — they
        # never meet, and the read raises ``can't read "<v>": no such
        # variable``.  The non-proc top-level case picks up the
        # ``tcl_global_set`` mirror inside ``_emit_var_write_obj`` so
        # eval-fallbacks see the binding too.
        for ll, (_, group_vars) in zip(list_locals, iterators):
            gsize = max(len(group_vars), 1)
            for slot, var_name in enumerate(group_vars):
                if lindex_idx is not None:
                    self._emit_local_get(ll)
                    # element index = counter * gsize + slot
                    self._emit_local_get(counter)
                    if gsize > 1:
                        self._emit_i64_const(gsize)
                        self._emit(WasmOp.I64_MUL)
                    if slot > 0:
                        self._emit_i64_const(slot)
                        self._emit(WasmOp.I64_ADD)
                    self._emit_box_int()
                    self._emit_call(lindex_idx)
                else:
                    # Fallback: use raw counter as value
                    self._emit_local_get(counter)
                    if slot > 0:
                        self._emit_i64_const(slot)
                        self._emit(WasmOp.I64_ADD)
                    self._emit_box_int()
                self._emit_var_write_obj(var_name, source=Ownership.OWNED)

        # Wrap body in block for break/continue
        self._emit(WasmOp.BLOCK, bytes([_BLOCK_VOID]))  # continue target
        self._loop_depth += 1
        self._loop_ctrl_depths.append(self._ctrl_depth)
        self._loop_catch_depths.append(self._catch_depth)

        # Mark the foreach header as an active outer loop so that
        # _is_loop_header doesn't treat the header→body back-edge as a
        # new nested loop when it walks successor blocks of the body.
        if header_block:
            self._active_loop_headers.add(header_block)
        self._emit_block(body_block)
        if header_block:
            self._active_loop_headers.discard(header_block)

        self._loop_ctrl_depths.pop()
        self._loop_catch_depths.pop()
        self._loop_depth -= 1
        self._emit(WasmOp.END)  # end continue block

        # After body: if any control-flow flag is set (error / return /
        # break / continue via runtime handler), exit the loop immediately
        # so that the variable bindings from THIS iteration survive and
        # the enclosing catch (or top-level trap) sees the flag.
        # br(1) targets the foreach-break BLOCK (0=LOOP, 1=BLOCK).
        has_error_idx = self._shared_imports.get("tcl_catch_has_error")
        if has_error_idx is not None:
            self._emit_call(has_error_idx)
            self._emit_br_if(1)

        # counter += 1 (counter is the iteration index now)
        self._emit_local_get(counter)
        self._emit_i64_const(1)
        self._emit(WasmOp.I64_ADD)
        self._emit_local_set(counter)

        self._emit_br(0)
        self._emit(WasmOp.END)
        self._emit(WasmOp.END)

        # Continue with the end block.
        self._emit_block(end_block)

    def _emit_block(self, block_name: str) -> None:
        """Emit all statements in a CFG block."""
        if block_name in self._visited:
            return
        self._visited.add(block_name)

        block = self._cfg.blocks.get(block_name)
        if block is None:
            return

        # Detect implicit return pattern: last statement is IRExprEval,
        # IRCall, IRIncr, or an IRBarrier that has a real runtime result
        # (e.g. ``namespace eval :: $cmd $args``), followed by goto to an
        # empty exit block.  In Tcl, the last command's result is the
        # proc's return value.
        stmts = block.statements
        use_implicit_return = False
        if (
            stmts
            and isinstance(stmts[-1], (IRExprEval, IRCall, IRIncr, IRBarrier))
            and isinstance(block.terminator, CFGGoto)
            and self._is_exit_block(block.terminator.target)
            and self._is_proc
        ):
            use_implicit_return = True

        if use_implicit_return:
            for stmt in stmts[:-1]:
                self._emit_stmt(stmt)
            last = stmts[-1]
            # Update diag-site context before the tail-call branches
            # dispatch — otherwise a fallback or unsupported-trap in
            # the tail would stamp a site with whichever range the
            # previous statement happened to leave behind.
            self._record_stmt_context(last)
            # Phase 8 follow-up: stamp ``frame_set_line`` for the
            # implicit-return tail so a callee's ``info frame -line``
            # reports the tail statement's line — without this the
            # tail's source position is silently dropped.
            self._emit_frame_set_line(last)
            if isinstance(last, IRExprEval):
                # Emit the final expression (i64), box to i32 for return
                self._emit_expr(last.expr)
                self._emit_box_int()
            elif isinstance(last, IRCall):
                # Emit the call keeping its i32 result on the stack
                last_tokens = last.tokens
                if (
                    last_tokens is not None
                    and last_tokens.expand_word is not None
                    and any(last_tokens.expand_word)
                    and last_tokens.argv_texts
                ):
                    # ``{*}`` expansion in tail position — reconstruct
                    # command text with ``{*}`` prefixes for eval.
                    ew = last_tokens.expand_word
                    single_word = (
                        last_tokens.single_token_word
                        if last_tokens.single_token_word is not None
                        else ()
                    )
                    argv = last_tokens.argv if last_tokens.argv is not None else ()
                    from shared.tokens import TokenType

                    parts: list[str] = []
                    for i, t in enumerate(last_tokens.argv_texts):
                        prefix = "{*}" if (i < len(ew) and ew[i]) else ""
                        if i < len(argv) and argv[i] is not None and argv[i].type is TokenType.STR:
                            t = "{" + t + "}"
                        elif i < len(single_word) and not single_word[i] and t:
                            # Multi-token concatenation (``"$body (suffix)"``,
                            # ``foo$bar``) — re-wrap in ``"…"`` so the
                            # interpreter sees a single word at eval time.
                            t = '"' + _escape_dquote(t) + '"'
                        parts.append(prefix + t)
                    script = " ".join(parts)
                    self._emit_eval_fallback(last.command, last.args, script_override=script)
                    if self._optimise:
                        self._const_map.clear()
                else:
                    # Pass source spelling so eval-fallback proc resolution
                    # walks Tcl's normal namespace stack; canonical form
                    # drives the literal-string dispatches.  Issue #246.
                    # Forward ``tokens`` so the hook can probe per-arg
                    # ``was_braced`` info — required for braced-pattern
                    # commands like ``string match {\?*} $name``.
                    self._emit_call_stmt_tail(
                        last.command,
                        last.args,
                        last.defs,
                        canonical_command=last.canonical_command,
                        tokens=getattr(last, "tokens", None),
                    )
            elif isinstance(last, IRBarrier):
                # IRBarrier in tail position — keep result on stack (no DROP).
                # ``namespace eval ns arg1 arg2 ...`` with dynamic args uses
                # WASM-level assembly so compiled-frame aliases resolve.
                barrier_cmd = last.canonical_command
                barrier_args = last.args
                if (
                    barrier_cmd == "::namespace"
                    and barrier_args
                    and barrier_args[0] == "eval"
                    and len(barrier_args) > 2
                ):
                    if not self._emit_namespace_eval_bridge(
                        barrier_args[2:], drop_result=False, ns_name=barrier_args[1]
                    ):
                        self._emit_eval_fallback(barrier_cmd, barrier_args)
                        # result stays on stack (no DROP)
                elif barrier_cmd == "::uplevel" and barrier_args:
                    # Tail-position uplevel: shift frame depth, eval body,
                    # restore — result stays on stack (no DROP).
                    self._emit_cmd_uplevel(barrier_args)
                elif (
                    barrier_cmd == "::return"
                    and barrier_args
                    and len(barrier_args) == 3
                    and barrier_args[0] == "-code"
                    and barrier_args[1] == "error"
                ):
                    # Tail-position ``return -code error <msg>``: evaluate
                    # msg so embedded $var/[cmd] substitutions resolve, then
                    # signal error.  _emit_cmd_return emits its own RETURN;
                    # the implicit-return RETURN below is unreachable.
                    self._emit_cmd_return(barrier_args)
                elif barrier_cmd == "::while" and len(barrier_args) >= 2:
                    # Mirror the ``::while`` brace-wrap special case
                    # from :mod:`_statements`.  The condition is a
                    # script that must be re-evaluated each
                    # iteration; without ``{}`` the runtime parser
                    # substitutes brackets ONCE at script-eval time
                    # and freezes the result, turning ``while
                    # {[info exists arr($n)]} { ... }`` into
                    # ``while CONST_BOOL { ... }`` that loops
                    # forever (or never).
                    cond, body = barrier_args[0], barrier_args[1]
                    cond_q = (
                        cond if cond.startswith("{") and cond.endswith("}") else "{" + cond + "}"
                    )
                    body_q = (
                        body if body.startswith("{") and body.endswith("}") else "{" + body + "}"
                    )
                    script = "while " + cond_q + " " + body_q
                    self._emit_eval_fallback(barrier_cmd, barrier_args, script_override=script)
                elif barrier_cmd == "::for" and len(barrier_args) >= 4:
                    # Same brace-wrap rule for ``::for`` -- init /
                    # cond / next / body all need ``{}`` so the
                    # runtime sees them as scripts to be re-
                    # evaluated, not pre-substituted constants.
                    init, cond, nxt, body = (
                        barrier_args[0],
                        barrier_args[1],
                        barrier_args[2],
                        barrier_args[3],
                    )

                    def _br(s):
                        return s if s.startswith("{") and s.endswith("}") else "{" + s + "}"

                    script = "for " + _br(init) + " " + _br(cond) + " " + _br(nxt) + " " + _br(body)
                    self._emit_eval_fallback(barrier_cmd, barrier_args, script_override=script)
                else:
                    # Generic barrier in tail position — eval fallback,
                    # result stays on stack.
                    #
                    # We deliberately do NOT special-case static parse
                    # errors (e.g. malformed ``if`` shapes) here even
                    # though :mod:`_statements` and :mod:`_control_flow`
                    # do — the implicit-return position would force an
                    # ``UNREACHABLE`` after ``tcl_cmd_error`` to satisfy
                    # the WASM verifier, which would then trap
                    # unconditionally even when the proc was called
                    # from inside a runtime ``catch``.  The eval-fallback
                    # path matches the pre-#259 behaviour for this rare
                    # tail-position case (the script seeds in #259 hit
                    # ``if`` barriers at non-tail positions where the
                    # codegen-side trap is safe).
                    if barrier_cmd:
                        # Use the barrier's own tokens so multi-token
                        # words (``"$body (suffix)"``) and ``{*}``
                        # expansion get the same quoting treatment as
                        # an IRCall — without this the runtime sees
                        # ``::tcltest::test x ${body} (suffix) {*}args``
                        # with the embedded space splitting the
                        # description into multiple words.  See the
                        # mirror logic in :mod:`_statements`.
                        last_tokens_b = last.tokens
                        if (
                            last_tokens_b is not None
                            and last_tokens_b.expand_word is not None
                            and any(last_tokens_b.expand_word)
                            and last_tokens_b.argv_texts
                        ):
                            ew = last_tokens_b.expand_word
                            single_word = (
                                last_tokens_b.single_token_word
                                if last_tokens_b.single_token_word is not None
                                else ()
                            )
                            argv = last_tokens_b.argv if last_tokens_b.argv is not None else ()
                            from shared.tokens import TokenType

                            parts: list[str] = []
                            for i, t in enumerate(last_tokens_b.argv_texts):
                                prefix = "{*}" if (i < len(ew) and ew[i]) else ""
                                if (
                                    i < len(argv)
                                    and argv[i] is not None
                                    and argv[i].type is TokenType.STR
                                ):
                                    t = "{" + t + "}"
                                elif i < len(single_word) and not single_word[i] and t:
                                    t = '"' + _escape_dquote(t) + '"'
                                parts.append(prefix + t)
                            script = " ".join(parts)
                            self._emit_eval_fallback(
                                barrier_cmd, barrier_args, script_override=script
                            )
                        else:
                            self._emit_eval_fallback(
                                barrier_cmd, barrier_args, tokens=last_tokens_b
                            )
                    else:
                        self._emit_eval_fallback(last.reason)
                if self._optimise:
                    self._const_map.clear()
            elif isinstance(last, IRIncr):
                # Emit incr keeping the new value (i32 TclObj) on stack.
                # Alias-aware: reads/writes route through globals for
                # upvar/variable-bound locals.
                # Issue #262: route through ``tcl_incr`` to enforce the
                # strict-integer guard.
                incr_idx = self._shared_imports.get("tcl_incr")
                if incr_idx is not None:
                    # Lenient read so an unset scalar initialises to 0
                    # (Tcl 8.5+: ``incr x`` returns 1, doesn't raise).
                    self._emit_var_read_obj_lenient(last.name)
                    if last.amount is None:
                        self._emit_i64_const(1)
                        self._emit_box_int()
                    else:
                        try:
                            self._emit_i64_const(int(last.amount))
                            self._emit_box_int()
                        except ValueError:
                            self._emit_value(last.amount)
                    self._emit_call(incr_idx)
                    if last.name in self._aliases or _is_dynamic_var_name(last.name):
                        # Dynamic names route through ``_emit_var_write_obj_keep``
                        # which dispatches via ``tcl_var_set`` with the
                        # substituted name; see :meth:`_emit_dynamic_var_write`.
                        self._emit_var_write_obj_keep(last.name)
                    else:
                        idx = self._intern_local(last.name)
                        self._emit_local_tee(idx)
                    self._emit(WasmOp.RETURN)
                    return
                self._emit_var_read_obj(last.name)
                self._emit_unbox_int()
                amt = 1
                if last.amount is not None:
                    try:
                        amt = int(last.amount)
                    except ValueError:
                        self._emit_value(last.amount)
                        self._emit_unbox_int()
                        self._emit(WasmOp.I64_ADD)
                        self._emit_box_int()
                        if last.name in self._aliases or _is_dynamic_var_name(last.name):
                            self._emit_var_write_obj_keep(last.name)
                        else:
                            idx = self._intern_local(last.name)
                            self._emit_local_tee(idx)
                        self._emit(WasmOp.RETURN)
                        return
                self._emit_i64_const(amt)
                self._emit(WasmOp.I64_ADD)
                self._emit_box_int()
                if last.name in self._aliases or _is_dynamic_var_name(last.name):
                    self._emit_var_write_obj_keep(last.name)
                else:
                    idx = self._intern_local(last.name)
                    self._emit_local_tee(idx)
            self._emit(WasmOp.RETURN)
            return

        for stmt in stmts:
            self._emit_stmt(stmt)

        # Stamp the terminator's source range onto instructions emitted
        # below so the explorer can click-through from the ``return`` /
        # branch / goto opcode to the originating Tcl construct.  The
        # range is optional on each terminator kind; we leave
        # ``_current_range`` untouched when missing so we fall back to
        # whatever the last ``_emit_stmt`` recorded.
        term_range = getattr(block.terminator, "range", None)
        if term_range is not None:
            self._current_range = term_range

        match block.terminator:
            case CFGReturn(value=value):
                if value is not None:
                    self._emit_value(value)
                else:
                    self._emit_i32_const(0)
                self._emit(WasmOp.RETURN)

            case CFGBranch(condition=condition, true_target=tt, false_target=ft):
                # Foreach pattern: the CFG builder desugars foreach into
                # a header block with an opaque <foreach_has_next> branch.
                # Detect this and emit a counter-based WASM loop instead.
                if isinstance(condition, ExprRaw) and condition.text == "<foreach_has_next>":
                    self._emit_foreach_loop(stmts, tt, ft, header_block=block_name)
                    return

                # For/while loop pattern: the header block branches on a
                # condition; the true-target body eventually loops back
                # to this header via a goto chain.  Detect back-edges
                # and emit block{loop{...}} instead of if/else.
                if self._is_loop_header(block_name, tt):
                    self._emit_cfg_loop(block_name, condition, tt, ft)
                    return

                # Find the merge block where both branches reconverge
                # so we can emit it *after* the if/else/end rather than
                # inlining it into one branch and skipping it for the other.
                merge = self._find_merge_block(tt, ft)
                merge_newly_added = merge is not None and merge not in self._visited
                if merge_newly_added:
                    assert merge is not None  # implied by merge_newly_added
                    self._visited.add(merge)

                self._emit_expr(condition)
                self._emit_i64_const(0)
                self._emit(WasmOp.I64_NE)
                self._emit(WasmOp.IF, bytes([_BLOCK_VOID]), label="if")
                self._emit_block(tt)
                self._emit(WasmOp.ELSE)
                self._emit_block(ft)
                self._emit(WasmOp.END)

                if merge_newly_added:
                    assert merge is not None  # implied by merge_newly_added
                    self._visited.discard(merge)
                    self._emit_block(merge)

            case CFGGoto(target=target):
                self._emit_block(target)

    # -- Entry point --
