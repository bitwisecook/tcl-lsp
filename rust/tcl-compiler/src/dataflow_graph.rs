//! Data-flow graph types for SSA + SCCP + memory-SSA visualisation.
//!
//! Provides a uniform representation of the data-flow facts that
//! downstream tooling (compiler explorer, LSP inlay hints,
//! interprocedural analysis) needs to consume without directly
//! touching the SSA / memory-SSA / SCCP result structures.
//!
//! Nodes describe SSA value definitions; edges link each def to
//! its use sites; aliases summarise the upvar/global/variable
//! relationships detected by [`crate::memory_ssa`].
//!
//! Ported from `core/compiler/dataflow_graph.py` in two strips:
//! - **C25c** (this file) — data types.
//! - **C25d** — extraction functions that walk SSA + SCCP +
//!   memory-SSA results and build [`FunctionDataFlowGraph`] /
//!   [`DataFlowGraph`] records.

// ---------------------------------------------------------------------------
// Edge classification (C25c)
// ---------------------------------------------------------------------------

/// Classification of a data-flow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Direct def → use within the same or dominated block.
    Direct,
    /// Flow through a phi node (conditional merge).
    Phi,
    /// Flow through an aliased binding (upvar / global / variable).
    Alias,
    /// Value may be invalidated by a barrier / eval.
    Clobber,
}

impl EdgeKind {
    /// Stable short text form, matching Python's Enum value strings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Phi => "phi",
            Self::Alias => "alias",
            Self::Clobber => "clobber",
        }
    }
}

// ---------------------------------------------------------------------------
// Nodes and edges (C25c)
// ---------------------------------------------------------------------------

/// A single SSA value in the data-flow graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataFlowNode {
    /// Variable name.
    pub name: String,
    /// SSA version.
    pub version: u32,
    /// CFG block where the def was emitted.
    pub block: String,
    /// Kind of def — `"statement"`, `"phi"`, or `"parameter"`.
    pub def_kind: String,
    /// Statement index within the block (`-1` for phis and
    /// parameters).
    pub statement_index: i32,
    /// Rendered lattice value for display, e.g.
    /// `"CONST(Int(42))"`, `"OVERDEFINED"`, `"UNKNOWN"`.
    pub lattice: String,
    /// Rendered type-lattice entry for display, e.g. `"Int"`.
    pub type_info: String,
    /// True when the def has no uses.
    pub is_dead: bool,
    /// Number of uses observed.
    pub use_count: u32,
}

impl DataFlowNode {
    /// Minimal constructor used by the extractor and tests.
    #[must_use]
    pub fn new(name: impl Into<String>, version: u32, block: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version,
            block: block.into(),
            def_kind: String::new(),
            statement_index: -1,
            lattice: String::new(),
            type_info: String::new(),
            is_dead: false,
            use_count: 0,
        }
    }
}

/// An edge from a definition to a use site.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataFlowEdge {
    /// Source variable name.
    pub from_name: String,
    /// Source SSA version.
    pub from_version: u32,
    /// Block containing the use.
    pub to_block: String,
    /// Statement index within the use block (`-1` for phi
    /// incomings and terminator-condition uses).
    pub to_statement_index: i32,
    /// Edge classification.
    pub edge_kind: EdgeKind,
    /// For phi edges: the phi variable's name on the receiving side.
    pub to_name: String,
    /// For phi edges: the phi's defined version (`-1` otherwise).
    pub to_version: i32,
}

impl DataFlowEdge {
    /// Build a direct edge with phi fields defaulted.
    #[must_use]
    pub fn direct(
        from_name: impl Into<String>,
        from_version: u32,
        to_block: impl Into<String>,
        to_statement_index: i32,
    ) -> Self {
        Self {
            from_name: from_name.into(),
            from_version,
            to_block: to_block.into(),
            to_statement_index,
            edge_kind: EdgeKind::Direct,
            to_name: String::new(),
            to_version: -1,
        }
    }

    /// Build a phi edge annotated with the receiving phi variable
    /// and version.
    #[must_use]
    pub fn phi(
        from_name: impl Into<String>,
        from_version: u32,
        to_block: impl Into<String>,
        to_name: impl Into<String>,
        to_version: u32,
    ) -> Self {
        Self {
            from_name: from_name.into(),
            from_version,
            to_block: to_block.into(),
            to_statement_index: -1,
            edge_kind: EdgeKind::Phi,
            to_name: to_name.into(),
            to_version: i32::try_from(to_version).unwrap_or(i32::MAX),
        }
    }
}

// ---------------------------------------------------------------------------
// Aliases and function/module graphs (C25c)
// ---------------------------------------------------------------------------

/// Summary of an alias relationship produced by memory-SSA.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AliasInfo {
    /// Local variable name (the alias as seen inside the procedure).
    pub local_name: String,
    /// Local memory-location kind, e.g. `"Local"`, `"Upvar"`.
    pub local_kind: String,
    /// Target variable name (the backing storage).
    pub target_name: String,
    /// Target memory-location kind, e.g. `"Global"`,
    /// `"NamespaceVar"`.
    pub target_kind: String,
    /// Why the alias was detected: `"upvar"`, `"global"`,
    /// `"variable"`, …
    pub reason: String,
}

/// Complete data-flow graph for one function.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionDataFlowGraph {
    /// Qualified function name.
    pub function_name: String,
    /// One node per SSA def.
    pub nodes: Vec<DataFlowNode>,
    /// One edge per def-use or phi-incoming pair.
    pub edges: Vec<DataFlowEdge>,
    /// Alias summaries derived from memory-SSA.
    pub aliases: Vec<AliasInfo>,
    /// Total number of defs in this function.
    pub total_defs: u32,
    /// Total number of uses in this function.
    pub total_uses: u32,
    /// Number of dead defs (zero `use_count`).
    pub dead_defs: u32,
    /// Number of distinct variable names involved in aliasing.
    pub aliased_vars: u32,
}

/// Data-flow graph for an entire module (all functions).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataFlowGraph {
    /// Per-function graphs, ordered by function name.
    pub functions: Vec<FunctionDataFlowGraph>,
}

impl DataFlowGraph {
    /// Sum of [`FunctionDataFlowGraph::total_defs`] across
    /// functions.
    #[must_use]
    pub fn total_defs(&self) -> u32 {
        self.functions.iter().map(|f| f.total_defs).sum()
    }

    /// Sum of [`FunctionDataFlowGraph::total_uses`] across
    /// functions.
    #[must_use]
    pub fn total_uses(&self) -> u32 {
        self.functions.iter().map(|f| f.total_uses).sum()
    }

    /// Sum of alias count across functions.
    #[must_use]
    pub fn total_aliases(&self) -> u32 {
        self.functions
            .iter()
            .map(|f| u32::try_from(f.aliases.len()).unwrap_or(u32::MAX))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kind_str_values() {
        assert_eq!(EdgeKind::Direct.as_str(), "direct");
        assert_eq!(EdgeKind::Phi.as_str(), "phi");
        assert_eq!(EdgeKind::Alias.as_str(), "alias");
        assert_eq!(EdgeKind::Clobber.as_str(), "clobber");
    }

    #[test]
    fn node_constructor_defaults() {
        let n = DataFlowNode::new("x", 1, "entry");
        assert_eq!(n.statement_index, -1);
        assert_eq!(n.use_count, 0);
        assert!(!n.is_dead);
        assert!(n.lattice.is_empty());
    }

    #[test]
    fn direct_edge_defaults_to_empty_phi_fields() {
        let e = DataFlowEdge::direct("x", 1, "b", 3);
        assert_eq!(e.edge_kind, EdgeKind::Direct);
        assert!(e.to_name.is_empty());
        assert_eq!(e.to_version, -1);
        assert_eq!(e.to_statement_index, 3);
    }

    #[test]
    fn phi_edge_carries_receiving_variable() {
        let e = DataFlowEdge::phi("x", 1, "join", "x", 3);
        assert_eq!(e.edge_kind, EdgeKind::Phi);
        assert_eq!(e.to_name, "x");
        assert_eq!(e.to_version, 3);
        assert_eq!(e.to_statement_index, -1);
    }

    #[test]
    fn data_flow_graph_totals() {
        let f1 = FunctionDataFlowGraph {
            function_name: "::a".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            aliases: vec![AliasInfo {
                local_name: "lo".into(),
                local_kind: "Local".into(),
                target_name: "hi".into(),
                target_kind: "Global".into(),
                reason: "global".into(),
            }],
            total_defs: 2,
            total_uses: 3,
            dead_defs: 0,
            aliased_vars: 1,
        };
        let f2 = FunctionDataFlowGraph {
            function_name: "::b".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            aliases: Vec::new(),
            total_defs: 1,
            total_uses: 4,
            dead_defs: 0,
            aliased_vars: 0,
        };
        let g = DataFlowGraph {
            functions: vec![f1, f2],
        };
        assert_eq!(g.total_defs(), 3);
        assert_eq!(g.total_uses(), 7);
        assert_eq!(g.total_aliases(), 1);
    }
}
