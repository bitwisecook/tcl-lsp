// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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

use std::collections::{BTreeSet, HashMap};

use crate::analyses::{ConstValue, LatticeValue};
use crate::def_use::{DefKind, DefUseResult, UseKind};
use crate::memory_ssa::{MemoryLocation, MemorySsaFunction};
use crate::sccp::SccpResult;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::{TypeKind, TypeLattice};

// Edge classification

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
    /// Stable short text form.
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

// Nodes and edges

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

// Aliases and function/module graphs

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

// Extractors

/// Render a lattice entry for the node display column.
///
/// `key` is `None` when the def-use chain's variable name is not an interned
/// SSA value of this function — the same "no entry" case as a map miss.
fn format_lattice(values: &HashMap<ValueKey, LatticeValue>, key: Option<ValueKey>) -> String {
    let Some(val) = key.and_then(|k| values.get(&k)) else {
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

/// Render the type-lattice kind for a node's `typeInfo`: the lattice
/// *kind* name, or the empty string when no type was inferred for this
/// SSA value.
fn format_type<S: std::hash::BuildHasher>(
    types: Option<&HashMap<ValueKey, TypeLattice, S>>,
    key: Option<ValueKey>,
) -> String {
    match key.and_then(|k| types.and_then(|t| t.get(&k))) {
        None => String::new(),
        Some(tl) => match tl.kind() {
            TypeKind::Unknown => "UNKNOWN",
            TypeKind::Known => "KNOWN",
            TypeKind::Shimmered => "SHIMMERED",
            TypeKind::Overdefined => "OVERDEFINED",
        }
        .to_owned(),
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
/// node's `def_kind` field.
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
pub fn extract_function_dataflow<S: std::hash::BuildHasher>(
    name: &str,
    ssa: &SsaFunction,
    du: &DefUseResult,
    sccp: Option<&SccpResult>,
    mem: Option<&MemorySsaFunction>,
    types: Option<&HashMap<ValueKey, TypeLattice, S>>,
) -> FunctionDataFlowGraph {
    let empty_values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let values = sccp.map_or(&empty_values, |r| &r.values);

    let mut nodes: Vec<DataFlowNode> = Vec::new();
    let mut edges: Vec<DataFlowEdge> = Vec::new();

    // Sort chain keys so output is deterministic. `du.chains` is keyed by the
    // String-based `SsaValueKey`, so this already sorts by name then version.
    let mut keys: Vec<&crate::def_use::SsaValueKey> = du.chains.keys().collect();
    keys.sort();

    for key in keys {
        let chain = &du.chains[key];
        let (var_name, version) = key;
        // The SCCP / type maps are keyed by the interned-`Symbol` `ValueKey`;
        // resolve this chain's display name to its symbol to query them. A name
        // with no SSA symbol yields `None` (an empty display column, same as a
        // map miss).
        let ssa_key: Option<ValueKey> = ssa.var_symbol(var_name).map(|sym| (sym, *version));
        nodes.push(DataFlowNode {
            name: var_name.clone(),
            version: *version,
            block: chain.definition.block.clone(),
            def_kind: def_kind_name(chain.definition.kind).to_owned(),
            statement_index: chain.definition.statement_index,
            lattice: format_lattice(values, ssa_key),
            type_info: format_type(types, ssa_key),
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

// Module-level aggregator

/// Per-function inputs to the module-level aggregator.
///
/// The caller runs SSA + def-use + (optional) SCCP + (optional)
/// memory-SSA for each function once and hands the resulting
/// references to [`extract_dataflow_graph`], which merges them
/// into a [`DataFlowGraph`] sorted by function name.
#[derive(Debug, Clone, Copy)]
pub struct FunctionInputs<'a, S = std::collections::hash_map::RandomState> {
    /// Qualified function name.
    pub name: &'a str,
    /// SSA form.
    pub ssa: &'a SsaFunction,
    /// Def-use chains.
    pub du: &'a DefUseResult,
    /// Optional SCCP result for lattice rendering.
    pub sccp: Option<&'a SccpResult>,
    /// Optional memory-SSA for alias-info extraction.
    pub mem: Option<&'a MemorySsaFunction>,
    /// Optional type-lattice map for the per-node `typeInfo` projection.
    pub types: Option<&'a HashMap<ValueKey, TypeLattice, S>>,
}

/// Build a [`DataFlowGraph`] for an entire module from per-function
/// analysis outputs.
///
/// Function graphs are appended in the order of `inputs` and then sorted
/// `::top` first, then the remaining functions alphabetically, so the
/// output is stable across runs and byte-comparable.
#[must_use]
pub fn extract_dataflow_graph<S: std::hash::BuildHasher>(
    inputs: &[FunctionInputs<'_, S>],
) -> DataFlowGraph {
    let mut functions: Vec<FunctionDataFlowGraph> = inputs
        .iter()
        .map(|i| extract_function_dataflow(i.name, i.ssa, i.du, i.sccp, i.mem, i.types))
        .collect();
    functions.sort_by(|a, b| {
        // Top-level (`::top`) sorts before every procedure; the rest go
        // alphabetically by qualified name.
        let a_top = a.function_name == "::top";
        let b_top = b.function_name == "::top";
        b_top
            .cmp(&a_top)
            .then_with(|| a.function_name.cmp(&b.function_name))
    });
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

    // -- extract_function_dataflow --

    use crate::cfg::{BlockId, Function as CfgFunction};
    use crate::def_use::build_def_use_chains;
    use crate::ir::Statement;
    use crate::sccp::sccp;
    use crate::ssa::{SsaBlock, SsaStatement};
    use crate::tcl_expr_eval::FoldPolicy;
    use std::collections::HashMap as Map;
    use tcl_lexer::Span;

    fn assign_const_ssa(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
        // `SsaStatement.defs` is keyed by interned `Symbol`; intern the name
        // against the function so the def-use chain resolves it back by name.
        let sym = ssa.intern_var(name);
        let mut defs = Map::new();
        defs.insert(sym, ver);
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: name.into(),
                value: value.into(),
                name_braced: false,
                value_span: None,
            },
            uses: Map::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
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
        let entry_id = cfg.entry;
        cfg.blocks.get_mut(&entry_id).unwrap().terminator = Some(crate::cfg::Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let mut entry = empty_ssa_block("entry");
        let s = assign_const_ssa(&mut ssa, "x", "42", 1);
        entry.statements.push(s);
        ssa.blocks.insert(entry_id, entry);

        let du = build_def_use_chains(&ssa, Some(&cfg));
        let registry = tcl_registry::CommandRegistry::build_default();
        let sccp_result = sccp(
            &cfg,
            &ssa,
            None,
            FoldPolicy::default(),
            crate::sccp::TraceInputs {
                registry: &registry,
                traced_variables: &BTreeSet::new(),
                has_dynamic_variable_trace: false,
            },
        );
        let g = extract_function_dataflow::<std::collections::hash_map::RandomState>(
            "::top",
            &ssa,
            &du,
            Some(&sccp_result),
            None,
            None,
        );

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
    fn type_info_projects_lattice_kind_name() {
        let mut ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let mut entry = empty_ssa_block("entry");
        let s = assign_const_ssa(&mut ssa, "x", "42", 1);
        entry.statements.push(s);
        ssa.blocks.insert(BlockId(0), entry);
        let du = build_def_use_chains(&ssa, None);

        // A `Known` type lattice for x@1 must render as the uppercase
        // kind name "KNOWN"; a value absent from the map renders as the
        // empty string. The type map is keyed by the interned `Symbol`.
        let xsym = ssa.var_symbol("x").expect("x interned");
        let mut types: HashMap<ValueKey, TypeLattice> = HashMap::new();
        types.insert((xsym, 1), TypeLattice::of(crate::types::TclType::Int));

        let g = extract_function_dataflow::<std::collections::hash_map::RandomState>(
            "::top",
            &ssa,
            &du,
            None,
            None,
            Some(&types),
        );
        let node = g.nodes.iter().find(|n| n.name == "x").expect("x node");
        assert_eq!(node.type_info, "KNOWN");

        // Without a type map every node's typeInfo is empty.
        let g0 = extract_function_dataflow::<std::collections::hash_map::RandomState>(
            "::top", &ssa, &du, None, None, None,
        );
        assert_eq!(g0.nodes[0].type_info, "");
    }

    #[test]
    fn extract_direct_edge_for_operand_use() {
        // entry:
        //   x@1 = 1 (def)
        //   y@1 = {uses x@1} (use)
        let mut cfg = CfgFunction::new("::top", "entry");
        let entry_id = cfg.entry;
        cfg.blocks.get_mut(&entry_id).unwrap().terminator = Some(crate::cfg::Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let mut entry = empty_ssa_block("entry");
        let xstmt = assign_const_ssa(&mut ssa, "x", "1", 1);
        entry.statements.push(xstmt);
        // `uses` / `defs` are keyed by interned `Symbol`.
        let xsym = ssa.var_symbol("x").expect("x interned");
        let ysym = ssa.intern_var("y");
        let mut uses = Map::new();
        uses.insert(xsym, 1);
        let mut ydefs = Map::new();
        ydefs.insert(ysym, 1);
        entry.statements.push(SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: "y".into(),
                value: "x".into(),
                name_braced: false,
                value_span: None,
            },
            uses,
            defs: ydefs,
            may_defs: std::collections::HashSet::new(),
        });
        ssa.blocks.insert(entry_id, entry);

        let du = build_def_use_chains(&ssa, Some(&cfg));
        let g = extract_function_dataflow::<std::collections::hash_map::RandomState>(
            "::top", &ssa, &du, None, None, None,
        );

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
        let mem = MemorySsaFunction {
            alias_sets: vec![AliasSet::new(locs, "global")],
            ..MemorySsaFunction::default()
        };
        let du = DefUseResult::default();
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let g = extract_function_dataflow::<std::collections::hash_map::RandomState>(
            "::top",
            &ssa,
            &du,
            None,
            Some(&mem),
            None,
        );
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
        let mut ssa_a = SsaFunction::trivial("::a", BlockId(0), vec!["entry".into()]);
        let mut entry_a = empty_ssa_block("entry");
        let sa = assign_const_ssa(&mut ssa_a, "x", "1", 1);
        entry_a.statements.push(sa);
        ssa_a.blocks.insert(BlockId(0), entry_a);

        let mut ssa_b = SsaFunction::trivial("::b", BlockId(0), vec!["entry".into()]);
        let mut entry_b = empty_ssa_block("entry");
        let sb = assign_const_ssa(&mut ssa_b, "y", "2", 1);
        entry_b.statements.push(sb);
        ssa_b.blocks.insert(BlockId(0), entry_b);

        let du_a = crate::def_use::build_def_use_chains(&ssa_a, None);
        let du_b = crate::def_use::build_def_use_chains(&ssa_b, None);

        // Pass inputs in reverse name order to exercise the sort.
        let inputs: Vec<FunctionInputs> = vec![
            FunctionInputs {
                name: "::z",
                ssa: &ssa_b,
                du: &du_b,
                sccp: None,
                mem: None,
                types: None,
            },
            FunctionInputs {
                name: "::a",
                ssa: &ssa_a,
                du: &du_a,
                sccp: None,
                mem: None,
                types: None,
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
        let g = extract_dataflow_graph(&[] as &[FunctionInputs]);
        assert!(g.functions.is_empty());
        assert_eq!(g.total_defs(), 0);
        assert_eq!(g.total_uses(), 0);
        assert_eq!(g.total_aliases(), 0);
    }

    #[test]
    fn extract_empty_when_no_chains() {
        let du = DefUseResult::default();
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let g = extract_function_dataflow::<std::collections::hash_map::RandomState>(
            "::top", &ssa, &du, None, None, None,
        );
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
