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

use std::collections::{BTreeSet, HashMap};

use crate::analyses::{ConstValue, LatticeValue};
use crate::def_use::{DefKind, DefUseResult, UseKind};
use crate::memory_ssa::{MemoryLocation, MemorySSAFunction};
use crate::sccp::SccpResult;
use crate::ssa::{SsaFunction, ValueKey};

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

// ---------------------------------------------------------------------------
// Extractors (C25d)
// ---------------------------------------------------------------------------

/// Render a lattice entry for the node display column.
fn format_lattice(values: &HashMap<ValueKey, LatticeValue>, key: &ValueKey) -> String {
    let Some(val) = values.get(key) else {
        return String::new();
    };
    match val {
        LatticeValue::Unknown => "UNKNOWN".into(),
        LatticeValue::Overdefined => "OVERDEFINED".into(),
        LatticeValue::Const(c) => format!("CONST({})", format_const(c)),
        LatticeValue::ConstSet(vs) => {
            let items: Vec<String> = vs.iter().map(format_const).collect();
            format!("CONSTSET({})", items.join(","))
        }
    }
}

fn format_const(c: &ConstValue) -> String {
    match c {
        ConstValue::Int(i) => i.to_string(),
        ConstValue::Float(f) => f.to_string(),
        ConstValue::Bool(b) => b.to_string(),
        ConstValue::String(s) => format!("{s:?}"),
    }
}

/// Map a [`DefKind`] onto the lowercase text form used in the
/// node's `def_kind` field, matching Python's `.name.lower()`.
fn def_kind_name(k: DefKind) -> &'static str {
    match k {
        DefKind::Statement => "statement",
        DefKind::Phi => "phi",
        DefKind::Parameter => "parameter",
    }
}

/// Build a [`FunctionDataFlowGraph`] from the per-function
/// analysis outputs — SSA, def-use chains, optional SCCP result,
/// and optional memory-SSA.
///
/// `sccp` and `mem` may be `None` when those analyses haven't
/// been run; the extractor then leaves the corresponding display
/// fields empty / skips alias entries.
#[must_use]
pub fn extract_function_dataflow(
    name: &str,
    ssa: &SsaFunction,
    du: &DefUseResult,
    sccp: Option<&SccpResult>,
    mem: Option<&MemorySSAFunction>,
) -> FunctionDataFlowGraph {
    let _ = ssa; // reserved for future type-lattice wiring.
    let empty_values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let values = sccp.map_or(&empty_values, |r| &r.values);

    let mut nodes: Vec<DataFlowNode> = Vec::new();
    let mut edges: Vec<DataFlowEdge> = Vec::new();

    // Sort chain keys so output is deterministic.
    let mut keys: Vec<&ValueKey> = du.chains.keys().collect();
    keys.sort();

    for key in keys {
        let chain = &du.chains[key];
        let (var_name, version) = key;
        nodes.push(DataFlowNode {
            name: var_name.clone(),
            version: *version,
            block: chain.definition.block.clone(),
            def_kind: def_kind_name(chain.definition.kind).to_owned(),
            statement_index: chain.definition.statement_index,
            lattice: format_lattice(values, key),
            type_info: String::new(),
            is_dead: chain.is_dead(),
            use_count: u32::try_from(chain.use_count()).unwrap_or(u32::MAX),
        });

        for use_site in &chain.uses {
            let edge = if use_site.kind == UseKind::PhiIncoming {
                DataFlowEdge::phi(
                    var_name.clone(),
                    *version,
                    use_site.block.clone(),
                    use_site.variable.clone(),
                    use_site.phi_version,
                )
            } else {
                DataFlowEdge::direct(
                    var_name.clone(),
                    *version,
                    use_site.block.clone(),
                    use_site.statement_index,
                )
            };
            edges.push(edge);
        }
    }

    // Alias info from memory-SSA.
    let mut aliases: Vec<AliasInfo> = Vec::new();
    if let Some(m) = mem {
        for aset in &m.alias_sets {
            let mut locs: Vec<&MemoryLocation> = aset.locations.iter().collect();
            locs.sort_by(|a, b| {
                (format!("{:?}", a.kind), &a.name, &a.qualifier).cmp(&(
                    format!("{:?}", b.kind),
                    &b.name,
                    &b.qualifier,
                ))
            });
            if locs.len() >= 2 {
                aliases.push(AliasInfo {
                    local_name: locs[0].name.clone(),
                    local_kind: format!("{:?}", locs[0].kind),
                    target_name: locs[1].name.clone(),
                    target_kind: format!("{:?}", locs[1].kind),
                    reason: aset.reason.clone(),
                });
            }
        }
    }

    let dead_defs = u32::try_from(nodes.iter().filter(|n| n.is_dead).count()).unwrap_or(u32::MAX);
    let total_defs = u32::try_from(nodes.len()).unwrap_or(u32::MAX);
    let total_uses = nodes.iter().map(|n| n.use_count).sum::<u32>();
    let mut aliased_names: BTreeSet<String> = BTreeSet::new();
    for a in &aliases {
        aliased_names.insert(a.local_name.clone());
        aliased_names.insert(a.target_name.clone());
    }
    let aliased_vars = u32::try_from(aliased_names.len()).unwrap_or(u32::MAX);

    FunctionDataFlowGraph {
        function_name: name.to_owned(),
        nodes,
        edges,
        aliases,
        total_defs,
        total_uses,
        dead_defs,
        aliased_vars,
    }
}

// ---------------------------------------------------------------------------
// Module-level aggregator (C25e3)
// ---------------------------------------------------------------------------

/// Per-function inputs to the module-level aggregator.
///
/// The caller runs SSA + def-use + (optional) SCCP + (optional)
/// memory-SSA for each function once and hands the resulting
/// references to [`extract_dataflow_graph`], which merges them
/// into a [`DataFlowGraph`] sorted by function name.
#[derive(Debug, Clone, Copy)]
pub struct FunctionInputs<'a> {
    /// Qualified function name.
    pub name: &'a str,
    /// SSA form.
    pub ssa: &'a SsaFunction,
    /// Def-use chains.
    pub du: &'a DefUseResult,
    /// Optional SCCP result for lattice rendering.
    pub sccp: Option<&'a SccpResult>,
    /// Optional memory-SSA for alias-info extraction.
    pub mem: Option<&'a MemorySSAFunction>,
}

/// Build a [`DataFlowGraph`] for an entire module from per-function
/// analysis outputs.
///
/// Function graphs are appended in the order of `inputs` and then
/// sorted by `function_name` so the output is stable across runs.
#[must_use]
pub fn extract_dataflow_graph(inputs: &[FunctionInputs<'_>]) -> DataFlowGraph {
    let mut functions: Vec<FunctionDataFlowGraph> = inputs
        .iter()
        .map(|i| extract_function_dataflow(i.name, i.ssa, i.du, i.sccp, i.mem))
        .collect();
    functions.sort_by(|a, b| a.function_name.cmp(&b.function_name));
    DataFlowGraph { functions }
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

    // -- C25d: extract_function_dataflow --

    use crate::cfg::Function as CfgFunction;
    use crate::def_use::build_def_use_chains;
    use crate::ir::Statement;
    use crate::sccp::sccp;
    use crate::ssa::{SsaBlock, SsaStatement};
    use std::collections::HashMap as Map;
    use tcl_lexer::Span;

    fn assign_const_ssa(name: &str, value: &str, ver: u32) -> SsaStatement {
        let mut defs = Map::new();
        defs.insert(name.to_string(), ver);
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: name.into(),
                value: value.into(),
            },
            uses: Map::new(),
            defs,
        }
    }

    fn empty_ssa_block(name: &str) -> SsaBlock {
        SsaBlock {
            name: name.into(),
            phis: Vec::new(),
            statements: Vec::new(),
            entry_versions: Map::new(),
            exit_versions: Map::new(),
        }
    }

    #[test]
    fn extract_node_per_def_with_sccp_lattice() {
        let mut cfg = CfgFunction::new("::top", "entry");
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(crate::cfg::Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let mut entry = empty_ssa_block("entry");
        entry.statements.push(assign_const_ssa("x", "42", 1));
        ssa.blocks.insert("entry".into(), entry);

        let du = build_def_use_chains(&ssa, Some(&cfg));
        let sccp_result = sccp(&cfg, &ssa, None);
        let g = extract_function_dataflow("::top", &ssa, &du, Some(&sccp_result), None);

        assert_eq!(g.function_name, "::top");
        assert_eq!(g.total_defs, 1);
        let node = g
            .nodes
            .iter()
            .find(|n| n.name == "x")
            .expect("x node present");
        assert_eq!(node.version, 1);
        assert_eq!(node.def_kind, "statement");
        assert_eq!(node.lattice, "CONST(42)");
        assert!(node.is_dead);
    }

    #[test]
    fn extract_direct_edge_for_operand_use() {
        // entry:
        //   x@1 = 1 (def)
        //   y@1 = {uses x@1} (use)
        let mut cfg = CfgFunction::new("::top", "entry");
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(crate::cfg::Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let mut entry = empty_ssa_block("entry");
        entry.statements.push(assign_const_ssa("x", "1", 1));
        let mut uses = Map::new();
        uses.insert("x".to_string(), 1);
        let mut ydefs = Map::new();
        ydefs.insert("y".to_string(), 1);
        entry.statements.push(SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: "y".into(),
                value: "x".into(),
            },
            uses,
            defs: ydefs,
        });
        ssa.blocks.insert("entry".into(), entry);

        let du = build_def_use_chains(&ssa, Some(&cfg));
        let g = extract_function_dataflow("::top", &ssa, &du, None, None);

        let direct_edges: Vec<&DataFlowEdge> = g
            .edges
            .iter()
            .filter(|e| e.edge_kind == EdgeKind::Direct)
            .collect();
        assert_eq!(direct_edges.len(), 1);
        let e = direct_edges[0];
        assert_eq!(e.from_name, "x");
        assert_eq!(e.from_version, 1);
        assert_eq!(e.to_statement_index, 1);
    }

    #[test]
    fn extract_alias_info_from_memory_ssa() {
        use crate::memory_ssa::{AliasSet, MemoryLocation, MemoryLocationKind};
        let mut locs = BTreeSet::new();
        locs.insert(MemoryLocation::new(MemoryLocationKind::Global, "g"));
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "g"));
        let mem = MemorySSAFunction {
            alias_sets: vec![AliasSet::new(locs, "global")],
            ..MemorySSAFunction::default()
        };
        let du = DefUseResult::default();
        let ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let g = extract_function_dataflow("::top", &ssa, &du, None, Some(&mem));
        assert_eq!(g.aliases.len(), 1);
        let a = &g.aliases[0];
        assert_eq!(a.reason, "global");
        // Deterministic sort should place Global before Local kind
        // (kind name alphabetical ordering).
        assert_eq!(a.local_kind, "Global");
        assert_eq!(a.target_kind, "Local");
        assert_eq!(g.aliased_vars, 1); // same name "g" dedup.
    }

    #[test]
    fn extract_dataflow_graph_merges_and_sorts_functions() {
        let mut ssa_a = SsaFunction {
            name: "::a".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let mut entry_a = empty_ssa_block("entry");
        entry_a.statements.push(assign_const_ssa("x", "1", 1));
        ssa_a.blocks.insert("entry".into(), entry_a);

        let mut ssa_b = SsaFunction {
            name: "::b".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let mut entry_b = empty_ssa_block("entry");
        entry_b.statements.push(assign_const_ssa("y", "2", 1));
        ssa_b.blocks.insert("entry".into(), entry_b);

        let du_a = crate::def_use::build_def_use_chains(&ssa_a, None);
        let du_b = crate::def_use::build_def_use_chains(&ssa_b, None);

        // Pass inputs in reverse name order to exercise the sort.
        let inputs = vec![
            FunctionInputs {
                name: "::z",
                ssa: &ssa_b,
                du: &du_b,
                sccp: None,
                mem: None,
            },
            FunctionInputs {
                name: "::a",
                ssa: &ssa_a,
                du: &du_a,
                sccp: None,
                mem: None,
            },
        ];
        let g = extract_dataflow_graph(&inputs);
        assert_eq!(g.functions.len(), 2);
        assert_eq!(g.functions[0].function_name, "::a");
        assert_eq!(g.functions[1].function_name, "::z");
        assert_eq!(g.total_defs(), 2);
    }

    #[test]
    fn extract_dataflow_graph_empty_input() {
        let g = extract_dataflow_graph(&[]);
        assert!(g.functions.is_empty());
        assert_eq!(g.total_defs(), 0);
        assert_eq!(g.total_uses(), 0);
        assert_eq!(g.total_aliases(), 0);
    }

    #[test]
    fn extract_empty_when_no_chains() {
        let du = DefUseResult::default();
        let ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let g = extract_function_dataflow("::top", &ssa, &du, None, None);
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
        assert_eq!(g.total_defs, 0);
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
