"""Data-flow graph construction from def-use chains and memory-SSA.

Produces a structured graph suitable for:
- Compiler explorer visualisation (JSON/HTML)
- MCP tool responses
- Mermaid diagram generation for AI skills
- CLI exploration

Each node represents an SSA value (definition), and each edge
represents a def→use relationship.  Edges are annotated with
the kind of flow (direct, phi, alias) and control-flow context.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from .core_analyses import FunctionAnalysis, LatticeKind
from .def_use import DefKind, DefUseResult, UseKind
from .memory_ssa import MemorySSAFunction
from .ssa import SSAFunction, SSAValueKey


class EdgeKind(Enum):
    """Classification of a data-flow edge."""

    DIRECT = auto()
    """Direct def→use within the same or dominated block."""

    PHI = auto()
    """Flow through a phi node (conditional merge)."""

    ALIAS = auto()
    """Flow through an aliased binding (upvar/global/variable)."""

    CLOBBER = auto()
    """Value may be invalidated by a barrier/eval."""


@dataclass(frozen=True, slots=True)
class DataFlowNode:
    """A single SSA value in the data-flow graph."""

    name: str
    version: int
    block: str
    def_kind: str  # "statement", "phi", "parameter"
    statement_index: int = -1
    lattice: str = ""  # "CONST(42)", "OVERDEFINED", "UNKNOWN"
    type_info: str = ""  # "INT", "STRING", etc.
    is_dead: bool = False
    use_count: int = 0


@dataclass(frozen=True, slots=True)
class DataFlowEdge:
    """An edge from a definition to a use site."""

    from_name: str
    from_version: int
    to_block: str
    to_statement_index: int
    edge_kind: str  # "direct", "phi", "alias", "clobber"
    to_name: str = ""  # For phi edges: the phi variable
    to_version: int = -1  # For phi edges: the phi's version


@dataclass(frozen=True, slots=True)
class AliasInfo:
    """Summary of an alias relationship."""

    local_name: str
    target_name: str
    reason: str  # "upvar", "global", "variable"


@dataclass(frozen=True, slots=True)
class FunctionDataFlowGraph:
    """Complete data-flow graph for one function."""

    function_name: str
    nodes: list[DataFlowNode]
    edges: list[DataFlowEdge]
    aliases: list[AliasInfo]
    total_defs: int = 0
    total_uses: int = 0
    dead_defs: int = 0
    aliased_vars: int = 0


@dataclass(frozen=True, slots=True)
class DataFlowGraph:
    """Data-flow graph for an entire module (all functions)."""

    functions: list[FunctionDataFlowGraph]

    @property
    def total_defs(self) -> int:
        return sum(f.total_defs for f in self.functions)

    @property
    def total_uses(self) -> int:
        return sum(f.total_uses for f in self.functions)

    @property
    def total_aliases(self) -> int:
        return sum(len(f.aliases) for f in self.functions)


def _format_lattice(analysis: FunctionAnalysis, key: SSAValueKey) -> str:
    """Format the lattice value for display."""
    val = analysis.values.get(key)
    if val is None:
        return ""
    if val.kind is LatticeKind.UNKNOWN:
        return "UNKNOWN"
    if val.kind is LatticeKind.CONST:
        return f"CONST({val.value!r})"
    return "OVERDEFINED"


def _format_type(analysis: FunctionAnalysis, key: SSAValueKey) -> str:
    """Format the type lattice for display."""
    tl = analysis.types.get(key)
    if tl is None:
        return ""
    return tl.kind.name


def extract_function_dataflow(
    name: str,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
) -> FunctionDataFlowGraph:
    """Extract the data-flow graph for a single function."""
    du = analysis.def_use_chains
    mem = analysis.memory_ssa

    nodes: list[DataFlowNode] = []
    edges: list[DataFlowEdge] = []
    aliases: list[AliasInfo] = []

    if du is None:
        return FunctionDataFlowGraph(
            function_name=name, nodes=[], edges=[], aliases=[]
        )

    # Build nodes from def-use chains
    for key, chain in du.chains.items():
        var_name, version = key
        nodes.append(
            DataFlowNode(
                name=var_name,
                version=version,
                block=chain.definition.block,
                def_kind=chain.definition.kind.name.lower(),
                statement_index=chain.definition.statement_index,
                lattice=_format_lattice(analysis, key),
                type_info=_format_type(analysis, key),
                is_dead=chain.is_dead,
                use_count=chain.use_count,
            )
        )

        # Build edges from uses
        for use in chain.uses:
            if use.kind is UseKind.PHI_INCOMING:
                edges.append(
                    DataFlowEdge(
                        from_name=var_name,
                        from_version=version,
                        to_block=use.block,
                        to_statement_index=-1,
                        edge_kind="phi",
                        to_name=use.variable,
                        to_version=use.phi_version,
                    )
                )
            else:
                edges.append(
                    DataFlowEdge(
                        from_name=var_name,
                        from_version=version,
                        to_block=use.block,
                        to_statement_index=use.statement_index,
                        edge_kind="direct",
                    )
                )

    # Build alias info from memory-SSA
    if mem is not None:
        for alias_set in mem.alias_sets:
            locs = list(alias_set.locations)
            if len(locs) >= 2:
                aliases.append(
                    AliasInfo(
                        local_name=locs[0].name,
                        target_name=locs[1].name,
                        reason=alias_set.reason,
                    )
                )

    dead_count = sum(1 for n in nodes if n.is_dead)
    aliased_count = len(set(a.local_name for a in aliases) | set(a.target_name for a in aliases))

    return FunctionDataFlowGraph(
        function_name=name,
        nodes=nodes,
        edges=edges,
        aliases=aliases,
        total_defs=len(nodes),
        total_uses=sum(n.use_count for n in nodes),
        dead_defs=dead_count,
        aliased_vars=aliased_count,
    )


def extract_dataflow_graph(
    source: str,
    *,
    cu=None,
) -> DataFlowGraph:
    """Extract the complete data-flow graph for a source file.

    Builds compilation artefacts if not already available via *cu*.
    Returns a ``DataFlowGraph`` containing per-function graphs
    with def-use nodes, edges, and alias information.
    """
    from .compilation_unit import CompilationUnit, ensure_compilation_unit

    cu = ensure_compilation_unit(source, cu, context="dataflow_graph")
    if cu is None:
        return DataFlowGraph(functions=[])

    functions: list[FunctionDataFlowGraph] = []

    # Top-level
    functions.append(
        extract_function_dataflow(
            "::top",
            cu.top_level.ssa,
            cu.top_level.analysis,
        )
    )

    # Procedures
    for qname in sorted(cu.procedures):
        fu = cu.procedures[qname]
        functions.append(
            extract_function_dataflow(qname, fu.ssa, fu.analysis)
        )

    return DataFlowGraph(functions=functions)


# ---------------------------------------------------------------------------
# Serialisation helpers
# ---------------------------------------------------------------------------


def dataflow_graph_to_dict(graph: DataFlowGraph) -> dict:
    """Convert a DataFlowGraph to a JSON-serialisable dict."""
    return {
        "functions": [
            {
                "name": f.function_name,
                "nodes": [
                    {
                        "name": n.name,
                        "version": n.version,
                        "block": n.block,
                        "defKind": n.def_kind,
                        "statementIndex": n.statement_index,
                        "lattice": n.lattice,
                        "typeInfo": n.type_info,
                        "isDead": n.is_dead,
                        "useCount": n.use_count,
                    }
                    for n in f.nodes
                ],
                "edges": [
                    {
                        "fromName": e.from_name,
                        "fromVersion": e.from_version,
                        "toBlock": e.to_block,
                        "toStatementIndex": e.to_statement_index,
                        "edgeKind": e.edge_kind,
                        "toName": e.to_name,
                        "toVersion": e.to_version,
                    }
                    for e in f.edges
                ],
                "aliases": [
                    {
                        "localName": a.local_name,
                        "targetName": a.target_name,
                        "reason": a.reason,
                    }
                    for a in f.aliases
                ],
                "summary": {
                    "totalDefs": f.total_defs,
                    "totalUses": f.total_uses,
                    "deadDefs": f.dead_defs,
                    "aliasedVars": f.aliased_vars,
                },
            }
            for f in graph.functions
        ],
        "summary": {
            "totalDefs": graph.total_defs,
            "totalUses": graph.total_uses,
            "totalAliases": graph.total_aliases,
            "functionCount": len(graph.functions),
        },
    }


def dataflow_graph_to_mermaid(graph: DataFlowGraph) -> str:
    """Render the data-flow graph as a Mermaid flowchart."""
    lines = ["flowchart TD"]

    for func in graph.functions:
        func_id = func.function_name.replace("::", "_").lstrip("_") or "top"
        lines.append(f"  subgraph {func_id}[\"{func.function_name}\"]")

        # Nodes
        for node in func.nodes:
            nid = f"{func_id}_{node.name}_{node.version}"
            label = f"{node.name}#{node.version}"
            if node.lattice and node.lattice.startswith("CONST"):
                label += f" = {node.lattice}"
            if node.is_dead:
                lines.append(f"    {nid}[/\"{label} DEAD\"/]")
            elif node.def_kind == "phi":
                lines.append(f"    {nid}{{\"{label} (phi)\"}}")
            else:
                lines.append(f"    {nid}[\"{label}\"]")

        # Edges
        for edge in func.edges:
            src = f"{func_id}_{edge.from_name}_{edge.from_version}"
            if edge.edge_kind == "phi" and edge.to_name:
                dst = f"{func_id}_{edge.to_name}_{edge.to_version}"
                lines.append(f"    {src} -.->|phi| {dst}")
            else:
                dst = f"{func_id}_use_{edge.to_block}_{edge.to_statement_index}"
                lines.append(f"    {src} --> {dst}")

        # Aliases
        for alias in func.aliases:
            a_id = f"{func_id}_{alias.local_name}_alias"
            b_id = f"{func_id}_{alias.target_name}_alias"
            lines.append(f"    {a_id}((\"{alias.local_name}\")) <-.->|{alias.reason}| {b_id}((\"{alias.target_name}\"))")

        lines.append("  end")

    return "\n".join(lines)
