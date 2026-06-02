"""Flow-sensitive variable alias + trace-observability lattice.

A single, principled source of truth for "what does an access to this variable
*mean* at this program point" — the property the optimiser must respect before
treating a variable as a private local it can fold, propagate, or eliminate.

Two intertwined effects are tracked per variable, per program point:

* **aliasing** — the name is bound to storage *outside* the current frame by a
  ``global`` / ``variable`` / ``upvar`` / ``namespace upvar`` declaration, so a
  write is visible to another scope and a read may observe an external write;
* **observability** — the name is under a ``trace``, so *every* access fires a
  callback (an observable side effect).

Both are computed as a forward data-flow lattice over the CFG (``EscapeFlag``
per variable, joined by union at merges), so the analysis is **flow-sensitive**:
a ``trace add variable x`` or ``global x`` only marks accesses that follow it,
leaving code before the declaration freely optimisable.  This is the lattice the
whole pipeline shares — the whole-function :func:`escaping_names` view (used by
the legacy escaping checks) is just the union over every point.

The declaration *grammar* is the shared :mod:`compiler.var_scoping` one, so a
new alias form is recognised here identically to memory-SSA and the escaping
checks — one grammar, one lattice.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntFlag, auto

from shared.naming import normalise_var_name as _normalise_var_name

from .cfg import CFGBranch, CFGFunction, CFGGoto
from .ir import IRBarrier, IRCall, IRStatement
from .var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)


class EscapeFlag(IntFlag):
    """Why an access to a variable is not a private-local access.

    A lattice over set-union: ``NONE`` is bottom, each reason is an independent
    bit, and the join of two states is their bitwise OR.
    """

    NONE = 0
    GLOBAL = auto()  # global:: / upvar #0 — aliases the global frame
    NAMESPACE = auto()  # variable / namespace upvar — aliases an enclosing ns
    UPVAR = auto()  # upvar N>=1 — aliases a *caller* frame
    TRACED = auto()  # under a `trace` — every access is observable

    @property
    def aliased(self) -> bool:
        """True when the name is bound to any out-of-frame storage."""
        return bool(self & (EscapeFlag.GLOBAL | EscapeFlag.NAMESPACE | EscapeFlag.UPVAR))

    @property
    def writes_outer_scope(self) -> bool:
        """True when a write reaches a global / enclosing-namespace variable.

        (Caller-frame ``upvar`` writes escape the frame too, but to the caller —
        not necessarily a global — so they are handled separately.)
        """
        return bool(self & (EscapeFlag.GLOBAL | EscapeFlag.NAMESPACE))


# A per-variable map; absent names default to ``EscapeFlag.NONE``.
_State = dict[str, EscapeFlag]


def _trace_target(args: list[str]) -> str | None:
    """Variable named by a ``trace`` command, or ``None``.

    Recognises both ``trace add variable NAME ...`` (8.5+) and the 8.4
    ``trace variable NAME ...`` spelling, for any operation.
    """
    if len(args) >= 3 and args[0] == "add" and args[1] == "variable":
        return args[2]
    if len(args) >= 2 and args[0] == "variable":
        return args[1]
    return None


def _stmt_gen(stmt: IRStatement, state: _State) -> None:
    """Apply *stmt*'s alias/trace declarations to *state* in place."""
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return
    args = list(stmt.args)
    cmd = stmt.canonical_command

    def _mark(idx: int, flag: EscapeFlag) -> None:
        if 0 <= idx < len(args):
            name = _normalise_var_name(args[idx])
            if name:
                state[name] = state.get(name, EscapeFlag.NONE) | flag

    if cmd == "::global":
        for i in global_declaration_indices(args):
            _mark(i, EscapeFlag.GLOBAL)
    elif cmd == "::variable":
        for i in variable_declaration_indices(args):
            _mark(i, EscapeFlag.NAMESPACE)
    elif cmd == "::trace":
        target = _trace_target(args)
        if target:
            nm = _normalise_var_name(target)
            if nm:
                state[nm] = state.get(nm, EscapeFlag.NONE) | EscapeFlag.TRACED

    # ``upvar`` / ``namespace upvar`` — the IR command for ``namespace upvar`` is
    # ``namespace``, so detect structurally via the shared grammar rather than
    # the command name.  ``upvar #0`` / ``0`` aliases the global frame; any other
    # level aliases a caller frame; ``namespace upvar`` aliases a namespace.
    is_ns_upvar = stmt.command == "namespace" and args and args[0] == "upvar"
    upvar_flag = EscapeFlag.NAMESPACE if is_ns_upvar else EscapeFlag.UPVAR
    if stmt.command == "upvar" and args:
        level = args[0].strip()
        if level in ("#0", "0"):
            upvar_flag = EscapeFlag.GLOBAL
    for i in upvar_local_declaration_indices(stmt.command, args, allow_dynamic_target=True):
        _mark(i, upvar_flag)


def _join_into(dst: _State, src: _State) -> bool:
    """Union *src* into *dst*; return True if *dst* changed."""
    changed = False
    for name, flag in src.items():
        merged = dst.get(name, EscapeFlag.NONE) | flag
        if merged != dst.get(name, EscapeFlag.NONE):
            dst[name] = merged
            changed = True
    return changed


def _successors(block) -> tuple[str, ...]:
    term = block.terminator
    if isinstance(term, CFGGoto):
        return (term.target,)
    if isinstance(term, CFGBranch):
        return (term.true_target, term.false_target)
    return ()


@dataclass(frozen=True, slots=True)
class VarObservability:
    """Result of the alias/observability analysis for one function.

    ``block_entry`` maps each block name to the lattice state at its entry; the
    point-wise queries replay the gen of the statements before the queried index
    to recover the state when that statement executes.
    """

    block_entry: dict[str, _State]
    _ordered_blocks: tuple[str, ...]
    _block_stmts: dict[str, tuple[IRStatement, ...]]

    def state_at(self, block: str, stmt_idx: int) -> _State:
        """Lattice state in force when ``block``'s statement *stmt_idx* runs.

        Includes every alias/trace declaration on a path reaching this point,
        i.e. the declarations in statements ``[0, stmt_idx)`` of this block on
        top of the block-entry state.
        """
        state = dict(self.block_entry.get(block, {}))
        stmts = self._block_stmts.get(block, ())
        for i in range(min(stmt_idx, len(stmts))):
            _stmt_gen(stmts[i], state)
        return state

    def flag_at(self, block: str, stmt_idx: int, name: str) -> EscapeFlag:
        return self.state_at(block, stmt_idx).get(_normalise_var_name(name), EscapeFlag.NONE)

    def is_escaping_at(self, block: str, stmt_idx: int, name: str) -> bool:
        """True when *name* is aliased or traced at this point (any reason)."""
        return self.flag_at(block, stmt_idx, name) is not EscapeFlag.NONE

    def is_observed_at(self, block: str, stmt_idx: int, name: str) -> bool:
        """True when *name* is under a trace at this point."""
        return bool(self.flag_at(block, stmt_idx, name) & EscapeFlag.TRACED)

    def escaping_names(self) -> frozenset[str]:
        """Whole-function union: every name aliased or traced *anywhere*.

        The flow-insensitive view the legacy escaping checks consume — a single
        source of truth derived from the same lattice as the point-wise queries.
        """
        names: set[str] = set()
        for block in self._ordered_blocks:
            # The exit state of a block subsumes its entry plus its own gens.
            state = dict(self.block_entry.get(block, {}))
            for stmt in self._block_stmts.get(block, ()):
                _stmt_gen(stmt, state)
            names.update(state)
        return frozenset(names)


def analyse_var_observability(cfg: CFGFunction) -> VarObservability:
    """Compute the flow-sensitive alias/observability lattice for *cfg*."""
    blocks = cfg.blocks
    block_stmts = {name: blk.statements for name, blk in blocks.items()}

    # Predecessor map (invert successor edges, incl. exception edges).
    preds: dict[str, list[str]] = {name: [] for name in blocks}
    for name, blk in blocks.items():
        for succ in _successors(blk):
            if succ in preds:
                preds[succ].append(name)
    for src, handler in cfg.exception_edges:
        if handler in preds and src in blocks:
            preds[handler].append(src)

    order = cfg.reverse_postorder()

    block_entry: dict[str, _State] = {name: {} for name in blocks}
    block_exit: dict[str, _State] = {name: {} for name in blocks}

    # Monotonic forward fixpoint (union join → terminates on the finite flag
    # lattice).  Iterate in RPO; repeat until no block-exit changes.
    changed = True
    while changed:
        changed = False
        for name in order:
            entry: _State = {}
            for pred in preds.get(name, ()):
                _join_into(entry, block_exit[pred])
            block_entry[name] = entry
            exit_state = dict(entry)
            for stmt in block_stmts.get(name, ()):
                _stmt_gen(stmt, exit_state)
            if exit_state != block_exit[name]:
                block_exit[name] = exit_state
                changed = True

    return VarObservability(
        block_entry=block_entry,
        _ordered_blocks=tuple(order),
        _block_stmts=block_stmts,
    )
