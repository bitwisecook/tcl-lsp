"""Mixin: peephole optimisations."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ._types import Instruction
from .opcodes import Op

if TYPE_CHECKING:
    from ._emitter import _Emitter


class _PeepholeMixin:
    """Mixin: peephole optimisations."""

    def _remove_trailing_pop(self: _Emitter) -> None:
        """Remove ``pop`` immediately before the final ``done``.

        In tclsh, the last command's result stays on TOS and ``done``
        returns it.  Our codegen always pops after each statement, so
        the final ``pop; done`` pair should be collapsed to ``done``.

        Does NOT remove ``pop`` that is part of inline catch cleanup
        (after ``reverse 2``) — that pop discards the body result,
        not the command value.
        """
        if len(self._instrs) < 2:
            return
        if self._instrs[-1].op != Op.DONE:
            return
        if self._instrs[-2].op != Op.POP:
            return
        # Don't strip pop after reverse — it's part of catch epilogue.
        if len(self._instrs) >= 3 and self._instrs[-3].op == Op.REVERSE:
            return

        done_old_idx = len(self._instrs) - 1
        pop_idx = len(self._instrs) - 2

        del self._instrs[pop_idx]

        # Labels that pointed at the old done index now point at the
        # new position (shifted down by one).
        done_new_idx = len(self._instrs) - 1
        for label in self._labels:
            if self._labels[label] == done_old_idx:
                self._labels[label] = done_new_idx

    def _fold_const_push_pop_nops(self: _Emitter) -> None:
        """Replace ``push1 N; pop`` with ``nop; nop; nop``.

        tclsh constant-folds ``expr`` commands that aren't the script's
        last statement into 3 nops.  After ``_remove_trailing_pop()``
        has removed the final ``pop; done`` pair, any remaining
        ``push1; pop`` pairs are discarded constant results.

        Skip pairs that are inside a ``startCommand`` wrapper — tclsh
        keeps ``push; pop`` when startCommand is present so the epoch
        check spans a real instruction sequence.
        """
        i = 0
        while i < len(self._instrs) - 1:
            after_unset = i > 0 and self._instrs[i - 1].op == Op.UNSET_STK
            after_start_cmd = i > 0 and self._instrs[i - 1].op == Op.START_CMD
            if (
                self._instrs[i].op == Op.PUSH1
                and self._instrs[i + 1].op == Op.POP
                and (self._instrs[i].comment != '""' or after_unset)
                and not self._instrs[i].no_fold
                and not after_start_cmd
            ):
                self._instrs[i] = Instruction(op=Op.NOP, operands=())
                self._instrs[i + 1] = Instruction(op=Op.NOP, operands=())
                self._instrs.insert(i + 2, Instruction(op=Op.NOP, operands=()))
                # Shift labels past the insertion point.
                for label in self._labels:
                    if self._labels[label] > i + 1:
                        self._labels[label] += 1
                i += 3
            else:
                i += 1

    def _dedup_push_literals(self: _Emitter) -> None:
        """Re-dedup push operands whose earliest slot was nop-ed.

        After ``_fold_const_push_pop_nops`` converts dead ``push; pop``
        pairs to nops, surviving push instructions may reference later
        literal slots that duplicate an earlier nop-ed slot.  Patch
        surviving pushes to reuse the earliest occurrence **only** if
        that slot is no longer referenced by any other surviving push.

        tclsh 9.0 deduplicates when the earlier slot was folded away
        but keeps separate slots when both pushes survive.
        """
        entries = self._lit.entries()
        # Collect literal indices still referenced by surviving pushes.
        live_indices: set[int] = set()
        for instr in self._instrs:
            if instr.op in (Op.PUSH1, Op.PUSH4) and instr.operands:
                lit_idx = instr.operands[0]
                if isinstance(lit_idx, int) and lit_idx < len(entries):
                    live_indices.add(lit_idx)
        # Build first-occurrence map.
        first: dict[str, int] = {}
        for idx, val in enumerate(entries):
            if val not in first:
                first[val] = idx
        # Patch surviving pushes to use the earliest slot, but only
        # when that earliest slot is NOT live (i.e. its push was nop-ed).
        for i, instr in enumerate(self._instrs):
            if instr.op in (Op.PUSH1, Op.PUSH4) and instr.operands:
                # Skip no-dedup pushes — tclsh 9.0 keeps their indices.
                if instr.comment and self._NO_DEDUP_TAG in instr.comment:
                    continue
                lit_idx = instr.operands[0]
                if isinstance(lit_idx, int) and lit_idx < len(entries):
                    earliest = first[entries[lit_idx]]
                    if earliest != lit_idx and earliest not in live_indices:
                        self._instrs[i] = Instruction(
                            op=instr.op,
                            operands=(earliest,) + instr.operands[1:],
                            comment=instr.comment,
                        )
                        # Update live set: this push now references earliest.
                        live_indices.discard(lit_idx)
                        live_indices.add(earliest)

    def _fold_tail_return_to_done(self: _Emitter) -> None:
        """Replace a final ``returnImm 0 0`` with ``done``.

        tclsh uses ``done`` for the proc body's last return, not
        ``returnImm``.  Only applies to proc bodies — in top-level
        scripts, ``returnImm`` must be preserved so ``catch`` can
        detect the return.
        """
        if not self._is_proc or not self._instrs:
            return
        last = self._instrs[-1]
        if last.op == Op.RETURN_IMM and last.operands == (0, 0):
            self._instrs[-1] = Instruction(op=Op.DONE, operands=())

    def _strip_unused_start_cmd(self: _Emitter) -> None:
        """Strip all ``startCommand`` when no generic invoke exists.

        tclsh 9.0 only strips ``startCommand`` in **top-level** scripts
        where all commands are compiled to specialised bytecodes.  In proc
        bodies, ``startCommand`` is always kept (for command tracing and
        epoch checking), regardless of whether generic invokes are present.
        """
        # Proc bodies always keep startCommand.
        if self._is_proc:
            return
        generic_ops = {
            Op.INVOKE_STK1,
            Op.INVOKE_STK4,
            Op.INVOKE_EXPANDED,
            Op.INVOKE_REPLACE,
        }
        # Opcodes that replace a generic invoke — startCommands must survive.
        replaced_ops = {Op.UPVAR, Op.NSUPVAR}
        has_generic = any(
            i.op in generic_ops
            or i.op in replaced_ops
            # returnImm with non-0,0 (from error/return -code) replaces a generic invoke.
            or (i.op == Op.RETURN_IMM and i.operands != (0, 0))
            for i in self._instrs
        )
        if has_generic:
            return
        # No generic invokes — strip all startCommand instructions.
        removed = 0
        i = 0
        while i < len(self._instrs):
            if self._instrs[i].op == Op.START_CMD:
                del self._instrs[i]
                # Adjust all labels that reference positions after the removed instruction.
                for lbl in self._labels:
                    if self._labels[lbl] > i:
                        self._labels[lbl] -= 1
                    elif self._labels[lbl] == i:
                        # Label pointed at the startCommand — keep pointing at
                        # what follows (now at the same index).
                        pass
                removed += 1
            else:
                i += 1

    def _fixup_top_level_start_cmd(self: _Emitter) -> None:
        """Remove ``startCommand`` for generic invoke commands in top-level.

        In top-level compilation units, Tcl 9.0 only wraps *compiled*
        commands with ``startCommand``.  Commands that fall through to
        ``invokeStk`` (generic invokes) never get ``startCommand`` — their
        epoch check is handled by the invoke machinery itself.

        Note: commands that are *compiled* but internally use ``invokeStk``
        (e.g. ``expr {wide(42)}`` calling a math function) are **not**
        tagged as generic — the ``ExprCall`` path sets
        ``_seen_generic_invoke`` but not ``_used_generic_invoke``.

        Generic-invoke ``startCommand`` instructions are identified by the
        ``_SC_GENERIC_TAG`` comment placed by ``_emit_stmt_with_start_cmd``.
        """
        if self._is_proc:
            return

        # Collect indices of all generic-tagged startCommand instructions.
        to_remove = [
            i
            for i, instr in enumerate(self._instrs)
            if instr.op == Op.START_CMD and instr.comment == self._SC_GENERIC_TAG
        ]
        if not to_remove:
            return

        # Remove in reverse order so earlier indices stay valid.
        for idx in reversed(to_remove):
            del self._instrs[idx]
            for lbl in self._labels:
                if self._labels[lbl] > idx:
                    self._labels[lbl] -= 1
