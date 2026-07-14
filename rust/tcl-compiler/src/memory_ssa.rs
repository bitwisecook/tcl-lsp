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

//! Memory-SSA: versioned memory operations with alias analysis.
//!
//! Memory-SSA extends scalar SSA to track *memory locations* — stores
//! and loads that go through aliased bindings (`upvar`, `global`,
//! `variable`, `namespace upvar`).
//!
//! Key concepts:
//!
//! - [`MemoryLocation`] identifies *what* is being stored/loaded
//!   (local, upvar alias, global, namespace variable, array element).
//! - [`AliasSet`] groups [`MemoryLocation`]s that may refer to the
//!   same underlying storage.
//! - [`MemoryOp`] annotates statements with versioned memory
//!   operations so that downstream passes (GVN, DSE, copy
//!   propagation) can reason about aliased state precisely.
//!
//! This module operates *after* scalar SSA construction and
//! inspects the IR for aliasing commands (`upvar`, `global`,
//! `variable`, `namespace upvar`) + barriers (`eval`/`uplevel`).
//!
//! Organised in three layers:
//! - location types and alias-set queries.
//! - memory-op types + `MemorySsaFunction` + detection helpers.
//! - `compute_aliases` + `build_memory_ssa` driver.

use std::collections::{BTreeSet, HashMap};
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, Version};
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

// MemoryLocationKind / MemoryLocation

/// Classification of a memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemoryLocationKind {
    /// Procedure-local variable (no aliasing concerns).
    Local,
    /// Aliased from a caller frame via `upvar`.
    Upvar,
    /// Global variable (`global` command or `::` prefix).
    Global,
    /// Namespace-scoped variable (`variable` or `namespace upvar`).
    NamespaceVar,
    /// Element of a Tcl array (`arrayName(index)`).
    ArrayElement,
    /// OO instance variable (`variable` in method body).
    InstanceVar,
    /// Cannot be determined statically.
    Unknown,
}

/// A specific memory location in the program.
///
/// The `qualifier` field carries location-specific context:
/// namespace (for [`MemoryLocationKind::NamespaceVar`]), caller-side
/// variable name (for [`MemoryLocationKind::Upvar`]), or array index
/// text (for [`MemoryLocationKind::ArrayElement`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemoryLocation {
    /// Classification of the location.
    pub kind: MemoryLocationKind,
    /// Variable name.
    pub name: String,
    /// Location-specific context.
    pub qualifier: String,
}

impl MemoryLocation {
    /// Build a location with `kind`, `name`, and an empty qualifier.
    #[must_use]
    pub fn new(kind: MemoryLocationKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            qualifier: String::new(),
        }
    }

    /// Build a location with an explicit qualifier.
    #[must_use]
    pub fn with_qualifier(
        kind: MemoryLocationKind,
        name: impl Into<String>,
        qualifier: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            qualifier: qualifier.into(),
        }
    }

    /// Render the location in its canonical textual format for
    /// diagnostics.
    #[must_use]
    pub fn display(&self) -> String {
        match self.kind {
            MemoryLocationKind::Upvar => {
                format!("upvar({} -> {})", self.qualifier, self.name)
            }
            MemoryLocationKind::Global => format!("global({})", self.name),
            MemoryLocationKind::NamespaceVar => {
                format!("ns({}::{})", self.qualifier, self.name)
            }
            MemoryLocationKind::InstanceVar => {
                format!("ivar({}::{})", self.qualifier, self.name)
            }
            MemoryLocationKind::ArrayElement => {
                format!("{}({})", self.name, self.qualifier)
            }
            MemoryLocationKind::Unknown => format!("?({})", self.name),
            MemoryLocationKind::Local => self.name.clone(),
        }
    }
}

// AliasSet

/// A group of memory locations that may alias each other.
///
/// For example, if `upvar 1 caller_x local_x` is in scope then
/// `caller_x` and `local_x` form an alias set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasSet {
    /// Locations merged into this set. Ordered for stable output.
    pub locations: BTreeSet<MemoryLocation>,
    /// Reason describing why the set was formed — e.g. `"upvar"`,
    /// `"global"`, `"variable"`, or a combination (comma-separated,
    /// sorted) when multiple detection paths merged into the same
    /// set.
    pub reason: String,
}

impl AliasSet {
    /// Construct an alias set from an owned location set + reason.
    #[must_use]
    pub fn new(locations: BTreeSet<MemoryLocation>, reason: impl Into<String>) -> Self {
        Self {
            locations,
            reason: reason.into(),
        }
    }

    /// True when `loc` is in this alias set.
    #[must_use]
    pub fn may_alias(&self, loc: &MemoryLocation) -> bool {
        self.locations.contains(loc)
    }

    /// All variable names in this set.
    #[must_use]
    pub fn names(&self) -> BTreeSet<String> {
        self.locations.iter().map(|l| l.name.clone()).collect()
    }

    /// True when `name` appears as any location's variable name.
    #[must_use]
    pub fn contains_name(&self, name: &str) -> bool {
        self.locations.iter().any(|l| l.name == name)
    }
}

// MemoryOpKind / MemoryOp

/// Kind of memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryOpKind {
    /// Memory write (store).
    Def,
    /// Memory read (load).
    Use,
    /// Memory phi at a merge point.
    Phi,
    /// Barrier / call that may modify any aliased memory.
    Clobber,
}

/// A versioned memory operation attached to a statement.
///
/// Uses are annotated with `reaching_version` indicating the version
/// of the memory state visible at the read; defs/phis/clobbers bump
/// `version` monotonically across a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryOp {
    /// Operation kind.
    pub kind: MemoryOpKind,
    /// Location being read or written.
    pub location: MemoryLocation,
    /// New version number assigned by this op (for Def/Phi/Clobber).
    /// For Use, matches the reaching version.
    pub version: Version,
    /// Version of memory state reaching this read (Use only).
    pub reaching_version: Version,
    /// Block containing the op.
    pub block: String,
    /// Statement index within the block (`-1` for phi).
    pub statement_index: i32,
}

impl MemoryOp {
    /// Build a def/clobber attached to a block+index.
    #[must_use]
    pub fn new_def(
        location: MemoryLocation,
        version: Version,
        block: impl Into<String>,
        statement_index: i32,
    ) -> Self {
        Self {
            kind: MemoryOpKind::Def,
            location,
            version,
            reaching_version: 0,
            block: block.into(),
            statement_index,
        }
    }

    /// Build a use annotated with its reaching version.
    #[must_use]
    pub fn new_use(
        location: MemoryLocation,
        reaching_version: Version,
        block: impl Into<String>,
        statement_index: i32,
    ) -> Self {
        Self {
            kind: MemoryOpKind::Use,
            location,
            version: reaching_version,
            reaching_version,
            block: block.into(),
            statement_index,
        }
    }

    /// Build a memory phi at the start of a block.
    #[must_use]
    pub fn new_phi(location: MemoryLocation, version: Version, block: impl Into<String>) -> Self {
        Self {
            kind: MemoryOpKind::Phi,
            location,
            version,
            reaching_version: 0,
            block: block.into(),
            statement_index: -1,
        }
    }

    /// Build a clobber (e.g. for `eval`/`uplevel` barriers).
    #[must_use]
    pub fn new_clobber(version: Version, block: impl Into<String>, statement_index: i32) -> Self {
        Self {
            kind: MemoryOpKind::Clobber,
            location: MemoryLocation::new(MemoryLocationKind::Unknown, "*"),
            version,
            reaching_version: 0,
            block: block.into(),
            statement_index,
        }
    }
}

// MemorySsaFunction

/// Memory-SSA annotations for a single function.
///
/// Produced by `build_memory_ssa`. Carries:
/// - `alias_sets`: every detected alias set in the function.
/// - `memory_ops`: one entry per def/use/phi/clobber, in emission
///   order.
/// - `memory_phis`: per-block memory phis, keyed by block name.
/// - pre-computed counts (`count_defs`, `count_uses`, `count_clobbers`)
///   for O(1) summary queries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemorySsaFunction {
    /// Alias sets covering this function's aliased variables.
    pub alias_sets: Vec<AliasSet>,
    /// Memory operations in emission order.
    pub memory_ops: Vec<MemoryOp>,
    /// Block-indexed memory phi nodes.
    pub memory_phis: HashMap<String, Vec<MemoryOp>>,
    /// Number of [`MemoryOpKind::Def`] ops.
    pub count_defs: usize,
    /// Number of [`MemoryOpKind::Use`] ops.
    pub count_uses: usize,
    /// Number of [`MemoryOpKind::Clobber`] ops.
    pub count_clobbers: usize,
}

impl MemorySsaFunction {
    /// All variable names involved in aliasing.
    #[must_use]
    pub fn aliased_names(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for aset in &self.alias_sets {
            out.extend(aset.names());
        }
        out
    }

    /// Alias sets that contain `name`.
    #[must_use]
    pub fn aliases_for(&self, name: &str) -> Vec<&AliasSet> {
        self.alias_sets
            .iter()
            .filter(|s| s.contains_name(name))
            .collect()
    }

    /// True when two variable names may refer to the same storage.
    /// Same-name always aliases; otherwise names must share an alias
    /// set.
    #[must_use]
    pub fn may_alias(&self, name_a: &str, name_b: &str) -> bool {
        if name_a == name_b {
            return true;
        }
        self.alias_sets
            .iter()
            .any(|s| s.contains_name(name_a) && s.contains_name(name_b))
    }
}

// Detection helpers

/// Return `true` if `stmt` is the call form `cmd args…` or the
/// equivalent barrier form. Used by the detection helpers below.
fn call_parts(stmt: &Statement) -> Option<(&str, &[String])> {
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            Some((command.as_str(), args.as_slice()))
        }
        _ => None,
    }
}

/// Detect `upvar ?level? otherVar myVar ?…?` aliasing pairs.
///
/// Returns a list of `(caller_var, local_var)` pairs. Delegates to
/// [`upvar_local_declaration_indices`] for the grammar, so all
/// level-word forms (decimal, negative decimal, `#N`) and both
/// `namespace upvar` entry points (lowered or pre-composed) are
/// handled the same way the LSP declaration provider sees them.
#[must_use]
pub fn detect_upvar(stmt: &Statement) -> Vec<(String, String)> {
    let Some((cmd, args)) = call_parts(stmt) else {
        return Vec::new();
    };
    let args_vec: Vec<String> = args.to_vec();
    let indices = upvar_local_declaration_indices(cmd, &args_vec);
    // The helper returns *local* indices; each pair is at
    // (local - 1, local).
    let mut out = Vec::with_capacity(indices.len());
    for local_idx in indices {
        if local_idx >= 1 && local_idx < args_vec.len() {
            out.push((args_vec[local_idx - 1].clone(), args_vec[local_idx].clone()));
        }
    }
    out
}

/// Detect `global varName …` declarations, skipping any
/// substituted `$ref` arguments.
#[must_use]
pub fn detect_global(stmt: &Statement) -> Vec<String> {
    let Some((cmd, args)) = call_parts(stmt) else {
        return Vec::new();
    };
    if cmd != "global" {
        return Vec::new();
    }
    let args_vec: Vec<String> = args.to_vec();
    global_declaration_indices(&args_vec)
        .into_iter()
        .map(|i| args_vec[i].clone())
        .collect()
}

/// Detect `variable varName ?value? …` declarations, filtering out
/// any substituted `$ref` name positions via
/// [`variable_declaration_indices`].
#[must_use]
pub fn detect_namespace_variable(stmt: &Statement) -> Vec<String> {
    let Some((cmd, args)) = call_parts(stmt) else {
        return Vec::new();
    };
    if cmd != "variable" {
        return Vec::new();
    }
    let args_vec: Vec<String> = args.to_vec();
    variable_declaration_indices(&args_vec)
        .into_iter()
        .map(|i| args_vec[i].clone())
        .collect()
}

/// The spec-declared trait set that classifies a call as a memory clobber.
const CLOBBER_TRAITS: Traits = Traits::EVALUATES_CODE.union(Traits::CREATES_BARRIER);

/// True if `stmt` may clobber arbitrary memory locations.
///
/// Barriers always clobber. A static-body [`Statement::UpFrame`]
/// (barrier-relaxed `uplevel`) clobbers because its body runs in a
/// caller's frame and can touch arbitrary caller locals or globals.
/// Calls clobber when the registry marks the resolved command (or the
/// resolved subcommand — `interp eval`, `namespace eval`) as evaluating
/// code or acting as a barrier ([`Traits::EVALUATES_CODE`] |
/// [`Traits::CREATES_BARRIER`]) — the same spec-declared classification
/// `side_effects.rs` dispatches on, replacing the old hardcoded name list
/// (whose compound `"interp eval"` spellings never matched a bare
/// `command` field, silently under-clobbering those calls).
#[must_use]
pub fn is_clobber(stmt: &Statement, registry: &CommandRegistry) -> bool {
    match stmt {
        Statement::Barrier { .. } | Statement::UpFrame { .. } => true,
        Statement::Call { command, args, .. } => {
            let Some(spec) = registry.get(command) else {
                return false;
            };
            if spec.traits.intersects(CLOBBER_TRAITS) {
                return true;
            }
            // Subcommand-level classification (`interp eval`, `namespace
            // eval`, …) — resolve the first word the way ensemble dispatch
            // does and consult its traits.
            args.first()
                .and_then(|sub| spec.resolve_subcommand(sub))
                .is_some_and(|sub| sub.traits.intersects(CLOBBER_TRAITS))
        }
        _ => false,
    }
}

// compute_aliases + build_memory_ssa

/// Union-find over [`MemoryLocation`] values with per-root reason
/// aggregation. Used by [`compute_aliases`] to merge aliases
/// discovered by multiple detection paths.
#[derive(Default)]
struct AliasUnionFind {
    parent: HashMap<MemoryLocation, MemoryLocation>,
    reasons: HashMap<MemoryLocation, BTreeSet<String>>,
}

impl AliasUnionFind {
    fn find(&mut self, loc: &MemoryLocation) -> MemoryLocation {
        if !self.parent.contains_key(loc) {
            self.parent.insert(loc.clone(), loc.clone());
            self.reasons.insert(loc.clone(), BTreeSet::new());
            return loc.clone();
        }
        // Path compression via iterative walk.
        let mut node = loc.clone();
        loop {
            let parent = self.parent.get(&node).expect("node registered").clone();
            if parent == node {
                break;
            }
            node = parent;
        }
        // Compress.
        let root = node.clone();
        let mut curr = loc.clone();
        while curr != root {
            let next = self.parent.get(&curr).expect("node registered").clone();
            self.parent.insert(curr, root.clone());
            curr = next;
        }
        root
    }

    fn union(&mut self, a: &MemoryLocation, b: &MemoryLocation, reason: &str) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            self.reasons
                .entry(ra)
                .or_default()
                .insert(reason.to_owned());
            return;
        }
        // Merge rb into ra.
        let rb_reasons = self.reasons.remove(&rb).unwrap_or_default();
        self.parent.insert(rb, ra.clone());
        let entry = self.reasons.entry(ra).or_default();
        for r in rb_reasons {
            entry.insert(r);
        }
        entry.insert(reason.to_owned());
    }
}

/// Scan the SSA function for aliasing commands and build alias
/// sets.
///
/// Uses union-find to merge transitive/overlapping aliases into
/// connected components — for example, `upvar 1 x a; upvar 1 x b`
/// correctly merges `a` and `b` into the same alias set because
/// they share the caller-side variable `x`.
///
/// Detects:
/// - `upvar` / `namespace upvar` → [`MemoryLocationKind::Upvar`]
///   locations linking caller and local names.
/// - `global` → [`MemoryLocationKind::Global`] / [`MemoryLocationKind::Local`]
///   pair per name.
/// - `variable` → [`MemoryLocationKind::NamespaceVar`] /
///   [`MemoryLocationKind::Local`] pair per name.
#[must_use]
pub fn compute_aliases(ssa: &SsaFunction) -> Vec<AliasSet> {
    let mut uf = AliasUnionFind::default();

    // Walk blocks in deterministic id order so alias sets are
    // reproducible across runs.
    let mut block_ids: Vec<BlockId> = ssa.blocks.keys().copied().collect();
    block_ids.sort_unstable();
    for &bid in &block_ids {
        let block = &ssa.blocks[&bid];
        for stmt_ssa in &block.statements {
            let stmt = &stmt_ssa.statement;

            for (caller, local) in detect_upvar(stmt) {
                let upvar_loc =
                    MemoryLocation::with_qualifier(MemoryLocationKind::Upvar, &local, &caller);
                // The caller-side node is keyed on the caller variable alone
                // (empty qualifier) so two aliases of the *same* caller var —
                // `upvar 1 x a; upvar 1 x b` — share this node and union-find
                // merges `a` and `b` into one set. Encoding the local name in
                // the qualifier would make each pair's caller node distinct and
                // silently break transitive merging.
                let caller_loc = MemoryLocation::new(MemoryLocationKind::Upvar, &caller);
                uf.union(&upvar_loc, &caller_loc, "upvar");
            }
            for gname in detect_global(stmt) {
                let global_loc = MemoryLocation::new(MemoryLocationKind::Global, &gname);
                let local_loc = MemoryLocation::new(MemoryLocationKind::Local, &gname);
                uf.union(&global_loc, &local_loc, "global");
            }
            for vname in detect_namespace_variable(stmt) {
                let ns_loc = MemoryLocation::new(MemoryLocationKind::NamespaceVar, &vname);
                let local_loc = MemoryLocation::new(MemoryLocationKind::Local, &vname);
                uf.union(&ns_loc, &local_loc, "variable");
            }
        }
    }

    // Build connected components.
    let mut components: HashMap<MemoryLocation, BTreeSet<MemoryLocation>> = HashMap::new();
    let all_locs: Vec<MemoryLocation> = uf.parent.keys().cloned().collect();
    for loc in all_locs {
        let root = uf.find(&loc);
        components.entry(root).or_default().insert(loc);
    }

    let mut alias_sets: Vec<AliasSet> = Vec::new();
    // Sort roots by their display form for deterministic output.
    let mut roots: Vec<MemoryLocation> = components.keys().cloned().collect();
    roots.sort_by_key(MemoryLocation::display);
    for root in roots {
        let reasons = uf.reasons.get(&root).cloned().unwrap_or_default();
        let reason = if reasons.is_empty() {
            "alias".to_string()
        } else {
            reasons.into_iter().collect::<Vec<_>>().join(",")
        };
        let locs = components.remove(&root).unwrap_or_default();
        alias_sets.push(AliasSet::new(locs, reason));
    }
    alias_sets
}

/// Build memory-SSA annotations for an SSA function.
///
/// Produces versioned memory operations (defs, uses, phis,
/// clobbers) and alias sets. Memory versions increment at each
/// store to an aliased location and at clobber points (barriers /
/// eval / uplevel).
///
/// Walks blocks in dominator-tree order (reverse iteration for
/// stack emulation) for consistent versioning.
#[must_use]
pub fn build_memory_ssa(ssa: &SsaFunction, registry: &CommandRegistry) -> MemorySsaFunction {
    let alias_sets = compute_aliases(ssa);
    let aliased_names: BTreeSet<String> = alias_sets.iter().flat_map(AliasSet::names).collect();

    let mut memory_ops: Vec<MemoryOp> = Vec::new();
    let mut memory_phis: HashMap<String, Vec<MemoryOp>> = HashMap::new();
    let mut version_counter: Version = 0;

    let mut visited: BTreeSet<BlockId> = BTreeSet::new();
    let mut stack: Vec<BlockId> = vec![ssa.entry];

    while let Some(bid) = stack.pop() {
        if visited.contains(&bid) || !ssa.blocks.contains_key(&bid) {
            continue;
        }
        visited.insert(bid);

        let block = &ssa.blocks[&bid];
        let bn = ssa.block_name(bid);

        // Memory phis at merge points: one phi per aliased variable
        // that has a scalar phi here.
        let mut block_phis: Vec<MemoryOp> = Vec::new();
        for phi in &block.phis {
            let phi_name = ssa.var_name(phi.name);
            if aliased_names.contains(phi_name) {
                version_counter += 1;
                let op = MemoryOp::new_phi(
                    MemoryLocation::new(MemoryLocationKind::Local, phi_name),
                    version_counter,
                    bn,
                );
                block_phis.push(op.clone());
                memory_ops.push(op);
            }
        }
        if !block_phis.is_empty() {
            memory_phis.insert(bn.to_string(), block_phis);
        }

        // Statements: clobbers, then defs, then uses.
        for (idx, stmt_ssa) in block.statements.iter().enumerate() {
            let stmt = &stmt_ssa.statement;
            let idx_i32 = i32::try_from(idx).unwrap_or(i32::MAX);

            if is_clobber(stmt, registry) {
                version_counter += 1;
                memory_ops.push(MemoryOp::new_clobber(version_counter, bn, idx_i32));
                // Fall through — a barrier that also defines
                // aliased vars still emits its defs.
            }

            // The version reaching this statement's *reads*, snapshotted before
            // the statement's own defs bump the counter.  A self-referential
            // aliased write (`upvar c x; set x [expr {$x + 1}]`) reads the
            // *incoming* version of `x`, not the one it is defining — tagging
            // the use with the post-def `version_counter` recorded the write as
            // the read's own reaching def (issue 154).
            let reaching_version = version_counter;

            for &sym in stmt_ssa.defs.keys() {
                let name = ssa.var_name(sym);
                if aliased_names.contains(name) {
                    version_counter += 1;
                    memory_ops.push(MemoryOp::new_def(
                        MemoryLocation::new(MemoryLocationKind::Local, name),
                        version_counter,
                        bn,
                        idx_i32,
                    ));
                }
            }

            for &sym in stmt_ssa.uses.keys() {
                let name = ssa.var_name(sym);
                if aliased_names.contains(name) {
                    memory_ops.push(MemoryOp::new_use(
                        MemoryLocation::new(MemoryLocationKind::Local, name),
                        reaching_version,
                        bn,
                        idx_i32,
                    ));
                }
            }
        }

        // Push dominator-tree children in reverse so the iterative
        // stack visits them left-to-right, preserving the recursive
        // visitation order.
        if let Some(children) = ssa.dominator_tree.get(&bid) {
            for &child in children.iter().rev() {
                stack.push(child);
            }
        }
    }

    let count_defs = memory_ops
        .iter()
        .filter(|o| o.kind == MemoryOpKind::Def)
        .count();
    let count_uses = memory_ops
        .iter()
        .filter(|o| o.kind == MemoryOpKind::Use)
        .count();
    let count_clobbers = memory_ops
        .iter()
        .filter(|o| o.kind == MemoryOpKind::Clobber)
        .count();

    MemorySsaFunction {
        alias_sets,
        memory_ops,
        memory_phis,
        count_defs,
        count_uses,
        count_clobbers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_location_display() {
        let local = MemoryLocation::new(MemoryLocationKind::Local, "x");
        assert_eq!(local.display(), "x");

        let upvar =
            MemoryLocation::with_qualifier(MemoryLocationKind::Upvar, "local_x", "caller_x");
        assert_eq!(upvar.display(), "upvar(caller_x -> local_x)");

        let global = MemoryLocation::new(MemoryLocationKind::Global, "g");
        assert_eq!(global.display(), "global(g)");

        let ns = MemoryLocation::with_qualifier(MemoryLocationKind::NamespaceVar, "var", "::foo");
        assert_eq!(ns.display(), "ns(::foo::var)");

        let ivar =
            MemoryLocation::with_qualifier(MemoryLocationKind::InstanceVar, "self_x", "MyClass");
        assert_eq!(ivar.display(), "ivar(MyClass::self_x)");

        let elem = MemoryLocation::with_qualifier(MemoryLocationKind::ArrayElement, "arr", "key");
        assert_eq!(elem.display(), "arr(key)");

        let unk = MemoryLocation::new(MemoryLocationKind::Unknown, "?");
        assert_eq!(unk.display(), "?(?)");
    }

    #[test]
    fn alias_set_may_alias_and_names() {
        let mut locs = BTreeSet::new();
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "b"));
        let set = AliasSet::new(locs, "upvar");
        assert!(set.may_alias(&MemoryLocation::new(MemoryLocationKind::Local, "a")));
        assert!(!set.may_alias(&MemoryLocation::new(MemoryLocationKind::Local, "c")));
        assert!(set.contains_name("a"));
        assert!(set.contains_name("b"));
        assert!(!set.contains_name("c"));
        let names = set.names();
        assert!(names.contains("a"));
        assert!(names.contains("b"));
        assert_eq!(names.len(), 2);
    }

    // -- MemoryOp + MemorySsaFunction + detection --

    fn call(cmd: &str, args: &[&str]) -> Statement {
        Statement::Call {
            span: tcl_lexer::Span::new(0, 0),
            command: cmd.into(),
            canonical_command: None,
            args: args.iter().map(|s| (*s).into()).collect(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    fn barrier(cmd: &str, args: &[&str]) -> Statement {
        Statement::Barrier {
            span: tcl_lexer::Span::new(0, 0),
            reason: "test".into(),
            command: cmd.into(),
            canonical_command: None,
            args: args.iter().map(|s| (*s).into()).collect(),
            tokens: None,
        }
    }

    #[test]
    fn memory_op_constructors() {
        let loc = MemoryLocation::new(MemoryLocationKind::Local, "x");
        let def = MemoryOp::new_def(loc.clone(), 3, "b", 1);
        assert_eq!(def.kind, MemoryOpKind::Def);
        assert_eq!(def.version, 3);

        let uv = MemoryOp::new_use(loc.clone(), 3, "b", 2);
        assert_eq!(uv.kind, MemoryOpKind::Use);
        assert_eq!(uv.reaching_version, 3);
        assert_eq!(uv.version, 3);

        let phi = MemoryOp::new_phi(loc.clone(), 4, "join");
        assert_eq!(phi.kind, MemoryOpKind::Phi);
        assert_eq!(phi.statement_index, -1);

        let clob = MemoryOp::new_clobber(5, "b", 7);
        assert_eq!(clob.kind, MemoryOpKind::Clobber);
        assert_eq!(clob.location.kind, MemoryLocationKind::Unknown);
        assert_eq!(clob.location.name, "*");
    }

    #[test]
    fn may_alias_same_name_trivial() {
        let f = MemorySsaFunction::default();
        assert!(f.may_alias("x", "x"));
        assert!(!f.may_alias("x", "y"));
    }

    #[test]
    fn may_alias_via_shared_alias_set() {
        let mut locs = BTreeSet::new();
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "b"));
        let f = MemorySsaFunction {
            alias_sets: vec![AliasSet::new(locs, "upvar")],
            ..MemorySsaFunction::default()
        };
        assert!(f.may_alias("a", "b"));
        assert!(f.may_alias("b", "a"));
        assert!(!f.may_alias("a", "c"));
        assert_eq!(f.aliases_for("a").len(), 1);
        assert!(f.aliases_for("c").is_empty());
    }

    #[test]
    fn detect_upvar_pair_no_level() {
        let stmt = call("upvar", &["caller_x", "local_x"]);
        assert_eq!(
            detect_upvar(&stmt),
            vec![("caller_x".to_string(), "local_x".to_string())]
        );
    }

    #[test]
    fn detect_upvar_with_level_and_multi_pairs() {
        let stmt = call("upvar", &["1", "a", "la", "b", "lb"]);
        let pairs = detect_upvar(&stmt);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("a".into(), "la".into()));
        assert_eq!(pairs[1], ("b".into(), "lb".into()));
    }

    #[test]
    fn detect_upvar_with_hash_level() {
        let stmt = call("upvar", &["#0", "caller_x", "local_x"]);
        let pairs = detect_upvar(&stmt);
        assert_eq!(pairs, vec![("caller_x".into(), "local_x".into())]);
    }

    #[test]
    fn detect_namespace_upvar_pairs() {
        let stmt = call("namespace", &["upvar", "::foo", "bar", "lb", "baz", "lz"]);
        let pairs = detect_upvar(&stmt);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("bar".into(), "lb".into()));
    }

    #[test]
    fn detect_upvar_rejects_unrelated_commands() {
        let stmt = call("set", &["a", "1"]);
        assert!(detect_upvar(&stmt).is_empty());
    }

    #[test]
    fn detect_global_names() {
        let stmt = call("global", &["foo", "bar"]);
        assert_eq!(detect_global(&stmt), vec!["foo".to_string(), "bar".into()]);
    }

    #[test]
    fn detect_namespace_variable_pairs() {
        // Name, (optional value) alternating.
        let stmt = call("variable", &["counter", "0", "name"]);
        let names = detect_namespace_variable(&stmt);
        assert_eq!(names, vec!["counter".to_string(), "name".into()]);
    }

    #[test]
    fn is_clobber_barrier_and_eval() {
        let reg = CommandRegistry::build_default();
        assert!(is_clobber(&barrier("eval", &["x"]), &reg));
        assert!(is_clobber(&call("eval", &["script"]), &reg));
        assert!(is_clobber(&call("uplevel", &["1", "script"]), &reg));
        assert!(!is_clobber(&call("set", &["x", "1"]), &reg));
    }

    /// The old hardcoded name list spelt these as the compound strings
    /// `"interp eval"` / `"namespace eval"`, which a bare `command` field
    /// never matches — so the subcommand-resolved forms silently escaped
    /// the clobber classification.  The registry path resolves the
    /// subcommand word (unique prefixes included) and reads its traits.
    #[test]
    fn is_clobber_resolves_code_evaluating_subcommands() {
        let reg = CommandRegistry::build_default();
        assert!(is_clobber(&call("interp", &["eval", "{}", "script"]), &reg));
        assert!(is_clobber(
            &call("namespace", &["eval", "ns", "script"]),
            &reg
        ));
        assert!(!is_clobber(&call("namespace", &["current"]), &reg));
        assert!(!is_clobber(&call("interp", &["exists", "x"]), &reg));
    }

    // -- compute_aliases + build_memory_ssa --

    use crate::ssa::{SsaBlock, SsaStatement};

    fn make_ssa_with_entry_stmts(stmts: Vec<Statement>) -> SsaFunction {
        let mut ssa_stmts: Vec<SsaStatement> = Vec::new();
        for s in stmts {
            ssa_stmts.push(SsaStatement {
                statement: s,
                uses: HashMap::new(),
                defs: HashMap::new(),
            });
        }
        let entry = BlockId(0);
        let mut ssa = SsaFunction::trivial("::test", entry, vec!["entry".into()]);
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: ssa_stmts,
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        ssa.dominator_tree.insert(entry, Vec::new());
        ssa
    }

    #[test]
    fn compute_aliases_finds_upvar_set() {
        let ssa = make_ssa_with_entry_stmts(vec![call("upvar", &["1", "caller_x", "local_x"])]);
        let sets = compute_aliases(&ssa);
        assert_eq!(sets.len(), 1);
        let set = &sets[0];
        assert!(set.contains_name("caller_x"));
        assert!(set.contains_name("local_x"));
        assert!(set.reason.contains("upvar"));
    }

    #[test]
    fn compute_aliases_merges_shared_caller_upvars() {
        // `upvar 1 x a` and `upvar 1 x b` alias the *same* caller
        // variable `x`, so `a` and `b` may alias each other. The
        // caller-side node is keyed on `x` alone, letting union-find
        // collapse both declarations into a single alias set.
        let ssa = make_ssa_with_entry_stmts(vec![
            call("upvar", &["1", "x", "a"]),
            call("upvar", &["1", "x", "b"]),
        ]);
        let sets = compute_aliases(&ssa);
        assert_eq!(sets.len(), 1, "shared-caller upvars must merge");
        let set = &sets[0];
        assert!(set.contains_name("a"));
        assert!(set.contains_name("b"));
        assert!(set.contains_name("x"));

        let m = build_memory_ssa(&ssa, &CommandRegistry::build_default());
        assert!(m.may_alias("a", "b"), "a and b share caller x");
    }

    #[test]
    fn compute_aliases_global_variable_pair() {
        let ssa = make_ssa_with_entry_stmts(vec![call("global", &["shared"])]);
        let sets = compute_aliases(&ssa);
        assert_eq!(sets.len(), 1);
        let set = &sets[0];
        // Global and Local kinds for the same name merge.
        assert!(
            set.locations
                .iter()
                .any(|l| l.kind == MemoryLocationKind::Global)
        );
        assert!(
            set.locations
                .iter()
                .any(|l| l.kind == MemoryLocationKind::Local)
        );
        assert!(set.reason.contains("global"));
    }

    #[test]
    fn compute_aliases_empty_when_no_aliasing_commands() {
        let ssa = make_ssa_with_entry_stmts(vec![call("set", &["x", "1"])]);
        assert!(compute_aliases(&ssa).is_empty());
    }

    #[test]
    fn build_memory_ssa_empty_function() {
        let ssa = make_ssa_with_entry_stmts(Vec::new());
        let m = build_memory_ssa(&ssa, &CommandRegistry::build_default());
        assert!(m.alias_sets.is_empty());
        assert!(m.memory_ops.is_empty());
        assert_eq!(m.count_defs, 0);
        assert_eq!(m.count_uses, 0);
        assert_eq!(m.count_clobbers, 0);
    }

    #[test]
    fn build_memory_ssa_emits_clobber_for_eval() {
        let ssa = make_ssa_with_entry_stmts(vec![call("eval", &["foo"])]);
        let m = build_memory_ssa(&ssa, &CommandRegistry::build_default());
        assert_eq!(m.count_clobbers, 1);
        assert_eq!(m.memory_ops[0].kind, MemoryOpKind::Clobber);
        assert_eq!(m.memory_ops[0].location.name, "*");
    }

    #[test]
    fn build_memory_ssa_emits_clobber_for_upframe() {
        // A static-body `uplevel 1 {…}` lowers to `Statement::UpFrame`;
        // its body runs in the caller's frame and clobbers aliased
        // memory just like the dynamic `uplevel` barrier.
        let upframe = Statement::UpFrame {
            span: tcl_lexer::Span::new(0, 0),
            frame_shift: 1,
            body: crate::ir::Script::new(),
            tokens: None,
        };
        assert!(is_clobber(&upframe, &CommandRegistry::build_default()));
        let ssa = make_ssa_with_entry_stmts(vec![upframe]);
        let m = build_memory_ssa(&ssa, &CommandRegistry::build_default());
        assert_eq!(m.count_clobbers, 1);
        assert_eq!(m.memory_ops[0].kind, MemoryOpKind::Clobber);
    }

    #[test]
    fn build_memory_ssa_tracks_aliased_def_and_use() {
        // global shared; then a statement that uses+defs shared.
        let entry = BlockId(0);
        let mut ssa = SsaFunction::trivial("::test", entry, vec!["entry".into()]);
        let shared = ssa.intern_var("shared");
        let mut stmts: Vec<SsaStatement> = Vec::new();
        stmts.push(SsaStatement {
            statement: call("global", &["shared"]),
            uses: HashMap::new(),
            defs: HashMap::new(),
        });
        let mut defs = HashMap::new();
        defs.insert(shared, 1);
        stmts.push(SsaStatement {
            statement: call("set", &["shared", "1"]),
            uses: HashMap::new(),
            defs,
        });
        let mut uses = HashMap::new();
        uses.insert(shared, 1);
        stmts.push(SsaStatement {
            statement: call("puts", &["$shared"]),
            uses,
            defs: HashMap::new(),
        });

        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: stmts,
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        ssa.dominator_tree.insert(entry, Vec::new());
        let m = build_memory_ssa(&ssa, &CommandRegistry::build_default());
        assert_eq!(m.count_defs, 1);
        assert_eq!(m.count_uses, 1);
        // Version for def must be > version visible at preceding
        // `global` statement (which emitted no memory op).
        let def_op = m
            .memory_ops
            .iter()
            .find(|o| o.kind == MemoryOpKind::Def)
            .expect("def present");
        let load_op = m
            .memory_ops
            .iter()
            .find(|o| o.kind == MemoryOpKind::Use)
            .expect("use present");
        assert_eq!(load_op.reaching_version, def_op.version);
    }

    #[test]
    fn self_referential_aliased_write_reads_incoming_version() {
        // `global shared`; `set shared 0`; then a self-referential
        // `set shared [expr {$shared + 1}]` that both reads and writes `shared`
        // in one statement. The read must carry the version *reaching* the
        // statement, strictly below the version the statement defines — tagging
        // it with the post-def counter recorded the write as its own reaching
        // def (issue 154).
        let entry = BlockId(0);
        let mut ssa = SsaFunction::trivial("::test", entry, vec!["entry".into()]);
        let shared = ssa.intern_var("shared");
        let mut stmts: Vec<SsaStatement> = Vec::new();
        stmts.push(SsaStatement {
            statement: call("global", &["shared"]),
            uses: HashMap::new(),
            defs: HashMap::new(),
        });
        let mut defs1 = HashMap::new();
        defs1.insert(shared, 1);
        stmts.push(SsaStatement {
            statement: call("set", &["shared", "0"]),
            uses: HashMap::new(),
            defs: defs1,
        });
        let mut uses = HashMap::new();
        uses.insert(shared, 1);
        let mut defs2 = HashMap::new();
        defs2.insert(shared, 2);
        stmts.push(SsaStatement {
            statement: call("set", &["shared", "[expr {$shared + 1}]"]),
            uses,
            defs: defs2,
        });
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: stmts,
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        ssa.dominator_tree.insert(entry, Vec::new());
        let m = build_memory_ssa(&ssa, &CommandRegistry::build_default());
        // The def and use from the self-referential statement (index 2).
        let self_def = m
            .memory_ops
            .iter()
            .find(|o| o.kind == MemoryOpKind::Def && o.statement_index == 2)
            .expect("self-ref def");
        let self_use = m
            .memory_ops
            .iter()
            .find(|o| o.kind == MemoryOpKind::Use && o.statement_index == 2)
            .expect("self-ref use");
        assert!(
            self_use.reaching_version < self_def.version,
            "use must read the pre-write version (use {}, def {})",
            self_use.reaching_version,
            self_def.version,
        );
    }

    #[test]
    fn memory_location_ordering_stable() {
        // BTreeSet ordering should be deterministic so AliasSet's
        // rendered output is reproducible across runs.
        let mut set = BTreeSet::new();
        set.insert(MemoryLocation::new(MemoryLocationKind::Local, "z"));
        set.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
        set.insert(MemoryLocation::new(MemoryLocationKind::Global, "m"));
        let names: Vec<_> = set.iter().map(|l| l.name.clone()).collect();
        assert_eq!(
            names,
            vec!["a".to_string(), "z".to_string(), "m".to_string()]
        );
    }
}
