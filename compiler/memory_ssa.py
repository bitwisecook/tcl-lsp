"""Memory-SSA: versioned memory operations with alias analysis.

Memory-SSA extends scalar SSA to track *memory locations* — stores and
loads that go through aliased bindings (``upvar``, ``global``,
``variable``, ``namespace upvar``).

Key concepts:

- **MemoryLocation**: identifies *what* is being stored/loaded (local,
  upvar alias, global, namespace variable, array element).
- **AliasSet**: groups of MemoryLocations that may refer to the same
  underlying storage.
- **MemoryDef / MemoryUse**: annotate statements with versioned memory
  operations so that downstream passes (GVN, DSE, copy propagation)
  can reason about aliased state precisely.

This module operates *after* scalar SSA construction and inspects
``IRCall`` / ``IRBarrier`` nodes for aliasing commands.
"""

# canonicalisation: audited #246

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from compiler.var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)

from .ir import IRBarrier, IRBlock, IRCall, IRStatement, IRUpFrame
from .ssa import BlockName, SSAFunction

# Memory location model


class MemoryLocationKind(Enum):
    """Classification of a memory location."""

    LOCAL = auto()
    """Procedure-local variable (no aliasing concerns)."""

    UPVAR = auto()
    """Aliased from a caller frame via ``upvar``."""

    GLOBAL = auto()
    """Global variable (``global`` command or ``::`` prefix)."""

    NAMESPACE_VAR = auto()
    """Namespace-scoped variable (``variable`` or ``namespace upvar``)."""

    ARRAY_ELEMENT = auto()
    """Element of a Tcl array (``arrayName(index)``)."""

    INSTANCE_VAR = auto()
    """OO instance variable (``variable`` in method body)."""

    UNKNOWN = auto()
    """Cannot be determined statically."""


@dataclass(frozen=True, slots=True)
class MemoryLocation:
    """A specific memory location in the program."""

    kind: MemoryLocationKind
    name: str  # variable name
    qualifier: str = ""  # namespace, caller var, or array index

    def __str__(self) -> str:
        if self.kind is MemoryLocationKind.UPVAR:
            return f"upvar({self.qualifier} -> {self.name})"
        if self.kind is MemoryLocationKind.GLOBAL:
            return f"global({self.name})"
        if self.kind is MemoryLocationKind.NAMESPACE_VAR:
            return f"ns({self.qualifier}::{self.name})"
        if self.kind is MemoryLocationKind.INSTANCE_VAR:
            return f"ivar({self.qualifier}::{self.name})"
        if self.kind is MemoryLocationKind.ARRAY_ELEMENT:
            return f"{self.name}({self.qualifier})"
        if self.kind is MemoryLocationKind.UNKNOWN:
            return f"?({self.name})"
        return self.name


@dataclass(frozen=True, slots=True)
class AliasSet:
    """A group of memory locations that may alias each other.

    For example, if ``upvar 1 caller_x local_x`` then ``caller_x``
    and ``local_x`` form an alias set.
    """

    locations: frozenset[MemoryLocation]
    reason: str = ""  # e.g. "upvar", "global", "variable"

    def may_alias(self, loc: MemoryLocation) -> bool:
        """True if *loc* is in this alias set."""
        return loc in self.locations

    @property
    def names(self) -> frozenset[str]:
        """All variable names in this set."""
        return frozenset(loc.name for loc in self.locations)


# Memory-SSA versions


class MemoryOpKind(Enum):
    """Kind of memory operation."""

    DEF = auto()
    """Memory write (store)."""

    USE = auto()
    """Memory read (load)."""

    PHI = auto()
    """Memory phi at a merge point."""

    CLOBBER = auto()
    """Barrier/call that may modify any aliased memory."""


@dataclass(frozen=True, slots=True)
class MemoryOp:
    """A versioned memory operation attached to a statement."""

    kind: MemoryOpKind
    location: MemoryLocation
    version: int  # memory version number
    reaching_version: int = 0  # version that reaches this point (for uses)
    block: BlockName = ""
    statement_index: int = -1


@dataclass(slots=True)
class MemorySSAFunction:
    """Memory-SSA annotations for a single function.

    Provides per-statement memory versioning and alias information
    so that downstream passes can reason about aliased stores/loads.
    """

    alias_sets: list[AliasSet]
    memory_ops: list[MemoryOp]
    # Per-block memory phis: block -> list of memory phi ops
    memory_phis: dict[BlockName, list[MemoryOp]] = field(default_factory=dict)
    # Pre-computed counts (set by build_memory_ssa).
    _count_defs: int = 0
    _count_uses: int = 0
    _count_clobbers: int = 0

    @property
    def aliased_names(self) -> frozenset[str]:
        """All variable names involved in aliasing."""
        names: set[str] = set()
        for alias_set in self.alias_sets:
            names |= alias_set.names
        return frozenset(names)

    def aliases_for(self, name: str) -> list[AliasSet]:
        """Return all alias sets that contain *name*."""
        return [s for s in self.alias_sets if name in s.names]

    def may_alias(self, name_a: str, name_b: str) -> bool:
        """True if two variable names may refer to the same storage."""
        if name_a == name_b:
            return True
        for alias_set in self.alias_sets:
            if name_a in alias_set.names and name_b in alias_set.names:
                return True
        return False

    @property
    def total_memory_defs(self) -> int:
        return self._count_defs

    @property
    def total_memory_uses(self) -> int:
        return self._count_uses

    @property
    def total_clobbers(self) -> int:
        return self._count_clobbers


# Alias detection


def _detect_upvar(stmt: IRStatement) -> list[tuple[str, str]]:
    """Detect ``upvar ?level? otherVar myVar`` aliasing pairs.

    Returns a list of ``(caller_var, local_var)`` pairs.

    Delegates to :func:`analyser.var_scoping.upvar_local_declaration_indices`
    so the LSP declaration provider and the compiler share a single source
    of truth for the upvar grammar.
    """
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return []
    local_indices = upvar_local_declaration_indices(stmt.command, stmt.args)
    # Translate the local-alias indices back to (caller, local) pairs.
    # The caller name is always one slot to the left of the local name.
    return [(stmt.args[i - 1], stmt.args[i]) for i in local_indices if i - 1 >= 0]


def _detect_global(stmt: IRStatement) -> list[str]:
    """Detect ``global varName ...`` declarations.

    Delegates to :func:`analyser.var_scoping.global_declaration_indices`.
    """
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return []
    if stmt.canonical_command != "::global":
        return []
    return [stmt.args[i] for i in global_declaration_indices(stmt.args)]


def _detect_namespace_variable(stmt: IRStatement) -> list[str]:
    """Detect ``variable varName ?value?`` declarations.

    Delegates to :func:`analyser.var_scoping.variable_declaration_indices`.
    """
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return []
    if stmt.canonical_command != "::variable":
        return []
    return [stmt.args[i] for i in variable_declaration_indices(stmt.args)]


def _is_clobber(stmt: IRStatement) -> bool:
    """True if the statement may clobber arbitrary memory locations."""
    if isinstance(stmt, IRBarrier):
        return True
    if isinstance(stmt, IRUpFrame):
        # Barrier-relaxed ``uplevel`` — the body runs in a caller's
        # frame and can touch arbitrary caller locals or globals, so
        # from this proc's memory-SSA view it is still a clobber.
        return True
    if isinstance(stmt, IRBlock):
        # Barrier-relaxed ``eval`` — the body's IR statements run
        # inline in the current scope, but for memory-SSA purposes
        # we preserve the classic ``eval`` barrier pessimism: the
        # body may touch names the outer analysis cannot enumerate.
        # Distinguished from a ``namespace eval`` IRBlock (which has
        # scoped semantics) by ``source_tokens.argv_texts[0]``.
        tokens = stmt.source_tokens
        if tokens is not None and tokens.argv_texts and tokens.argv_texts[0] == "eval":
            return True
    if isinstance(stmt, IRCall):
        # Commands that can modify arbitrary variables
        if stmt.canonical_command in ("::eval", "::uplevel", "::interp eval", "::namespace eval"):
            return True
    return False


def compute_aliases(ssa: SSAFunction) -> list[AliasSet]:
    """Scan the SSA function for aliasing commands and build alias sets.

    Uses union-find to merge transitive/overlapping aliases into connected
    components.  For example, ``upvar 1 x a; upvar 1 x b`` correctly
    merges ``a`` and ``b`` into the same alias set because they share
    the caller-side variable ``x``.

    Detects:
    - ``upvar`` / ``namespace upvar``: links caller variable to local alias
    - ``global``: links local name to global namespace
    - ``variable``: links local name to namespace variable
    """
    # Union-find over MemoryLocation for merging overlapping aliases.
    parent: dict[MemoryLocation, MemoryLocation] = {}
    reasons: dict[MemoryLocation, set[str]] = {}

    def find(loc: MemoryLocation) -> MemoryLocation:
        if loc not in parent:
            parent[loc] = loc
            reasons[loc] = set()
            return loc
        if parent[loc] is not loc:
            parent[loc] = find(parent[loc])
        return parent[loc]

    def union(a: MemoryLocation, b: MemoryLocation, reason: str) -> None:
        ra, rb = find(a), find(b)
        if ra is rb:
            reasons[ra].add(reason)
            return
        parent[rb] = ra
        reasons[ra] = reasons.get(ra, set()) | reasons.get(rb, set())
        reasons[ra].add(reason)

    for block in ssa.blocks.values():
        for stmt_ssa in block.statements:
            stmt = stmt_ssa.statement

            # upvar / namespace upvar
            for caller_var, local_var in _detect_upvar(stmt):
                upvar_loc = MemoryLocation(MemoryLocationKind.UPVAR, local_var, caller_var)
                caller_loc = MemoryLocation(MemoryLocationKind.UPVAR, caller_var, local_var)
                union(upvar_loc, caller_loc, "upvar")

            # global
            for gname in _detect_global(stmt):
                global_loc = MemoryLocation(MemoryLocationKind.GLOBAL, gname)
                local_loc = MemoryLocation(MemoryLocationKind.LOCAL, gname)
                union(global_loc, local_loc, "global")

            # variable
            for vname in _detect_namespace_variable(stmt):
                ns_loc = MemoryLocation(MemoryLocationKind.NAMESPACE_VAR, vname)
                local_loc = MemoryLocation(MemoryLocationKind.LOCAL, vname)
                union(ns_loc, local_loc, "variable")

    # Build AliasSet objects from connected components.
    components: dict[MemoryLocation, set[MemoryLocation]] = {}
    for loc in parent:
        root = find(loc)
        components.setdefault(root, set()).add(loc)

    alias_sets: list[AliasSet] = []
    for root, locs in sorted(components.items(), key=lambda kv: str(kv[0])):
        reason_set = reasons.get(root, set())
        reason_str = ",".join(sorted(reason_set)) if reason_set else "alias"
        alias_sets.append(AliasSet(locations=frozenset(locs), reason=reason_str))

    return alias_sets


def build_memory_ssa(ssa: SSAFunction) -> MemorySSAFunction:
    """Build memory-SSA annotations for an SSA function.

    Produces versioned memory operations and alias sets. Memory
    versions increment at each store to an aliased location or at
    clobber points (barriers/eval).
    """
    alias_sets = compute_aliases(ssa)
    aliased_names: set[str] = set()
    for aset in alias_sets:
        aliased_names |= aset.names

    memory_ops: list[MemoryOp] = []
    memory_phis: dict[str, list[MemoryOp]] = {}
    version_counter = 0

    # Walk blocks in dominator-tree order for consistent versioning
    visited: set[str] = set()
    stack = [ssa.entry]

    while stack:
        bn = stack.pop()
        if bn in visited or bn not in ssa.blocks:
            continue
        visited.add(bn)

        block = ssa.blocks[bn]

        # Memory phis at merge points (blocks with phi nodes for aliased vars)
        block_mem_phis: list[MemoryOp] = []
        for phi in block.phis:
            if phi.name in aliased_names:
                version_counter += 1
                mem_phi = MemoryOp(
                    kind=MemoryOpKind.PHI,
                    location=MemoryLocation(MemoryLocationKind.LOCAL, phi.name),
                    version=version_counter,
                    block=bn,
                )
                block_mem_phis.append(mem_phi)
                memory_ops.append(mem_phi)
        if block_mem_phis:
            memory_phis[bn] = block_mem_phis

        # Statements
        for idx, stmt_ssa in enumerate(block.statements):
            stmt = stmt_ssa.statement

            # Check for clobber (barrier/eval) — clobbers all aliased memory
            if _is_clobber(stmt):
                version_counter += 1
                memory_ops.append(
                    MemoryOp(
                        kind=MemoryOpKind.CLOBBER,
                        location=MemoryLocation(MemoryLocationKind.UNKNOWN, "*"),
                        version=version_counter,
                        block=bn,
                        statement_index=idx,
                    )
                )
                # Fall through to also record individual defs/uses on
                # the same statement (e.g. IRBarrier that also defines
                # aliased variables).

            # Check for defs to aliased variables
            for name, _ver in stmt_ssa.defs.items():
                if name in aliased_names:
                    version_counter += 1
                    memory_ops.append(
                        MemoryOp(
                            kind=MemoryOpKind.DEF,
                            location=MemoryLocation(MemoryLocationKind.LOCAL, name),
                            version=version_counter,
                            block=bn,
                            statement_index=idx,
                        )
                    )

            # Check for uses of aliased variables
            for name, _ver in stmt_ssa.uses.items():
                if name in aliased_names:
                    memory_ops.append(
                        MemoryOp(
                            kind=MemoryOpKind.USE,
                            location=MemoryLocation(MemoryLocationKind.LOCAL, name),
                            version=version_counter,
                            reaching_version=version_counter,
                            block=bn,
                            statement_index=idx,
                        )
                    )

        # Add children from dominator tree
        for child in reversed(ssa.dominator_tree.get(bn, ())):
            stack.append(child)

    # Pre-compute counts for cached property access.
    count_defs = sum(1 for op in memory_ops if op.kind is MemoryOpKind.DEF)
    count_uses = sum(1 for op in memory_ops if op.kind is MemoryOpKind.USE)
    count_clobbers = sum(1 for op in memory_ops if op.kind is MemoryOpKind.CLOBBER)

    return MemorySSAFunction(
        alias_sets=alias_sets,
        memory_ops=memory_ops,
        memory_phis=memory_phis,
        _count_defs=count_defs,
        _count_uses=count_uses,
        _count_clobbers=count_clobbers,
    )
