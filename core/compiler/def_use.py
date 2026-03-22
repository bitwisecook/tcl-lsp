"""Def-use chain construction over SSA form.

A def-use chain links each SSA definition to the set of statements
that read (use) it.  This enables precise analyses:

- Exact dead-store detection (no uses → dead).
- Reaching definitions at each use site.
- Precise unused-variable detection.
- Foundation for copy propagation (O127).

The chain is derived from ``SSAFunction`` in a single pass over all
blocks.  Phi nodes act as both definitions (LHS) and uses (incoming
edges from predecessor blocks).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from .cfg import CFGBranch
from .expr_ast import vars_in_expr_node
from .ssa import BlockName, SSAFunction, SSAValueKey, SSAVersion


class DefKind(Enum):
    """How a variable definition was produced."""

    STATEMENT = auto()
    """Ordinary assignment (``set``, ``incr``, ``IRAssignConst``, etc.)."""

    PHI = auto()
    """Phi node at a control-flow merge point."""

    PARAMETER = auto()
    """Procedure parameter (version 0, read-before-set)."""


class UseKind(Enum):
    """How a variable is consumed."""

    OPERAND = auto()
    """Read as an operand of a statement."""

    PHI_INCOMING = auto()
    """Incoming edge of a phi node."""

    TERMINATOR = auto()
    """Read by a branch condition (terminator)."""


@dataclass(frozen=True, slots=True)
class DefSite:
    """Location where an SSA value is defined."""

    block: BlockName
    kind: DefKind
    statement_index: int = -1  # -1 for phi/parameter defs


@dataclass(frozen=True, slots=True)
class UseSite:
    """Location where an SSA value is consumed."""

    block: BlockName
    kind: UseKind
    statement_index: int = -1  # -1 for phi incoming / terminator
    variable: str = ""  # For phi incoming: the phi variable name
    phi_version: SSAVersion = -1  # For phi incoming: the phi's def version


@dataclass(slots=True)
class DefUseChain:
    """A single def-use chain: one definition and all its uses."""

    key: SSAValueKey
    definition: DefSite
    uses: list[UseSite] = field(default_factory=list)

    @property
    def is_dead(self) -> bool:
        """True if the definition has no uses at all."""
        return len(self.uses) == 0

    @property
    def use_count(self) -> int:
        return len(self.uses)

    @property
    def has_phi_use(self) -> bool:
        """True if any use is a phi incoming edge."""
        return any(u.kind is UseKind.PHI_INCOMING for u in self.uses)


@dataclass(frozen=True, slots=True)
class DefUseResult:
    """Complete def-use analysis for one function."""

    chains: dict[SSAValueKey, DefUseChain]

    def chain_for(self, name: str, version: SSAVersion) -> DefUseChain | None:
        """Look up the chain for a specific SSA value."""
        return self.chains.get((name, version))

    def uses_of(self, name: str, version: SSAVersion) -> list[UseSite]:
        """Return all use sites for a given SSA value, or empty list."""
        chain = self.chains.get((name, version))
        return chain.uses if chain else []

    def is_dead(self, name: str, version: SSAVersion) -> bool:
        """True if the given SSA value has no uses."""
        chain = self.chains.get((name, version))
        return chain.is_dead if chain else True

    def reaching_defs(self, name: str) -> list[SSAValueKey]:
        """Return all SSA definitions of *name* across the function."""
        return [key for key in self.chains if key[0] == name]

    @property
    def dead_chains(self) -> list[DefUseChain]:
        """All chains with zero uses (dead definitions)."""
        return [c for c in self.chains.values() if c.is_dead]

    @property
    def total_defs(self) -> int:
        return len(self.chains)

    @property
    def total_uses(self) -> int:
        return sum(c.use_count for c in self.chains.values())


def build_def_use_chains(ssa: SSAFunction, cfg=None) -> DefUseResult:
    """Build def-use chains from an SSA function in a single pass.

    Walks all blocks collecting definitions (from phi nodes and
    statements) and uses (from statement operands, phi incoming edges,
    and branch conditions).

    If *cfg* is provided (a ``CFGFunction``), branch condition variables
    are resolved from terminators so that control-flow uses are tracked.
    """
    chains: dict[SSAValueKey, DefUseChain] = {}

    def _ensure_chain(key: SSAValueKey, definition: DefSite) -> DefUseChain:
        if key not in chains:
            chains[key] = DefUseChain(key=key, definition=definition)
        return chains[key]

    def _add_use(key: SSAValueKey, use: UseSite) -> None:
        if key in chains:
            chains[key].uses.append(use)
        else:
            # Synthesize a definition for values we haven't seen.
            # Version 0 = parameter/read-before-set; non-zero without a
            # known def is an SSA inconsistency but we handle it
            # gracefully with PARAMETER kind rather than crashing.
            _name, ver = key
            kind = DefKind.PARAMETER if ver == 0 else DefKind.STATEMENT
            chain = DefUseChain(
                key=key,
                definition=DefSite(
                    block=ssa.entry,
                    kind=kind,
                ),
            )
            chain.uses.append(use)
            chains[key] = chain

    # Pass 1: Collect all definitions.
    for bn, block in ssa.blocks.items():
        # Phi definitions
        for phi in block.phis:
            key: SSAValueKey = (phi.name, phi.version)
            _ensure_chain(
                key,
                DefSite(block=bn, kind=DefKind.PHI),
            )

        # Statement definitions
        for idx, stmt in enumerate(block.statements):
            for name, ver in stmt.defs.items():
                key = (name, ver)
                _ensure_chain(
                    key,
                    DefSite(block=bn, kind=DefKind.STATEMENT, statement_index=idx),
                )

    # Pass 2: Collect all uses.
    for bn, block in ssa.blocks.items():
        # Phi incoming edges are uses of the incoming versions.
        # Version 0 (parameter/read-before-set) is included so that
        # parameter chains correctly reflect phi consumption.
        for phi in block.phis:
            for pred_block, incoming_ver in phi.incoming.items():
                key = (phi.name, incoming_ver)
                _add_use(
                    key,
                    UseSite(
                        block=pred_block,
                        kind=UseKind.PHI_INCOMING,
                        variable=phi.name,
                        phi_version=phi.version,
                    ),
                )

        # Statement operand uses
        for idx, stmt in enumerate(block.statements):
            for name, ver in stmt.uses.items():
                key = (name, ver)
                _add_use(
                    key,
                    UseSite(
                        block=bn,
                        kind=UseKind.OPERAND,
                        statement_index=idx,
                    ),
                )

        # Terminator uses — branch conditions read variables that may
        # not appear in any statement's uses map.
        if cfg is not None and bn in cfg.blocks:
            term = cfg.blocks[bn].terminator
            if isinstance(term, CFGBranch):
                for name in vars_in_expr_node(term.condition):
                    ver = block.exit_versions.get(name, 0)
                    key = (name, ver)
                    _add_use(
                        key,
                        UseSite(
                            block=bn,
                            kind=UseKind.TERMINATOR,
                        ),
                    )

    return DefUseResult(chains=chains)
