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

//! Def-use chains over SSA form.
//!
//! A def-use chain links each SSA definition to the set of statements
//! that read (use) it. This enables precise analyses:
//!
//! - Exact dead-store detection (no uses → dead).
//! - Reaching definitions at each use site.
//! - Precise unused-variable detection.
//! - Foundation for copy propagation and GVN.
//!
//! The chain is derived from an [`SsaFunction`] (produced by
//! `build_ssa`) in two passes over all blocks. Phi nodes act as both
//! definitions (LHS) and uses (incoming edges from predecessor
//! blocks).

use std::collections::HashMap;

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ssa::{SsaFunction, UseClass, Version};

/// How a variable definition was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefKind {
    /// Ordinary assignment (`set`, `incr`, `Statement::AssignConst`, …).
    Statement,
    /// Phi node at a control-flow merge point.
    Phi,
    /// Procedure parameter (version 0, read-before-set).
    Parameter,
}

/// How a variable is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseKind {
    /// Read as an operand of a statement.
    Operand,
    /// Incoming edge of a phi node.
    PhiIncoming,
    /// Read by a branch condition (terminator).
    Terminator,
}

/// Location where an SSA value is defined.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefSite {
    /// Block containing the definition.
    pub block: String,
    /// How the definition was produced.
    pub kind: DefKind,
    /// Statement index within the block (`-1` for phi / parameter).
    pub statement_index: i32,
}

/// Location where an SSA value is consumed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UseSite {
    /// Block containing the use.
    pub block: String,
    /// How the variable is consumed.
    pub kind: UseKind,
    /// Statement index within the block (`-1` for phi-incoming / terminator).
    pub statement_index: i32,
    /// For `PhiIncoming`: the phi variable name.
    pub variable: String,
    /// For `PhiIncoming`: the phi's defined version.
    pub phi_version: Version,
    /// Whether the name is carried only by a brace-quoted word this statement
    /// does not substitute ([`UseClass::Quoted`]). The use is real for
    /// liveness — the text may be evaluated later — but is not a read *here*,
    /// so read-before-set must not claim it (issues #1142, #1237).
    pub class: UseClass,
}

/// SSA value key: `(variable name, version)`.
pub type SsaValueKey = (String, Version);

/// A single def-use chain: one definition and all its uses.
#[derive(Debug, Clone, PartialEq)]
pub struct DefUseChain {
    /// The SSA value this chain is for.
    pub key: SsaValueKey,
    /// Where the value is defined.
    pub definition: DefSite,
    /// All uses of this SSA value.
    pub uses: Vec<UseSite>,
}

impl DefUseChain {
    /// True when the definition has no uses at all.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.uses.is_empty()
    }

    /// Number of uses.
    #[must_use]
    pub fn use_count(&self) -> usize {
        self.uses.len()
    }

    /// True when any use is a phi-incoming edge.
    #[must_use]
    pub fn has_phi_use(&self) -> bool {
        self.uses.iter().any(|u| u.kind == UseKind::PhiIncoming)
    }
}

/// Complete def-use analysis for one function.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefUseResult {
    /// All chains keyed by `(variable, version)`.
    pub chains: HashMap<SsaValueKey, DefUseChain>,
}

impl DefUseResult {
    /// Look up the chain for a specific SSA value.
    #[must_use]
    pub fn chain_for(&self, name: &str, version: Version) -> Option<&DefUseChain> {
        self.chains.get(&(name.to_owned(), version))
    }

    /// Return all use sites for a given SSA value.
    #[must_use]
    pub fn uses_of(&self, name: &str, version: Version) -> &[UseSite] {
        self.chains
            .get(&(name.to_owned(), version))
            .map_or(&[], |c| c.uses.as_slice())
    }

    /// True when the given SSA value has no uses.
    #[must_use]
    pub fn is_dead(&self, name: &str, version: Version) -> bool {
        self.chain_for(name, version)
            .is_none_or(DefUseChain::is_dead)
    }

    /// All SSA definitions of `name` across the function.
    #[must_use]
    pub fn reaching_defs(&self, name: &str) -> Vec<SsaValueKey> {
        self.chains
            .keys()
            .filter(|(n, _)| n == name)
            .cloned()
            .collect()
    }

    /// All chains with zero uses.
    #[must_use]
    pub fn dead_chains(&self) -> Vec<&DefUseChain> {
        self.chains.values().filter(|c| c.is_dead()).collect()
    }

    /// Total number of distinct SSA definitions.
    #[must_use]
    pub fn total_defs(&self) -> usize {
        self.chains.len()
    }

    /// Total number of uses summed across all chains.
    #[must_use]
    pub fn total_uses(&self) -> usize {
        self.chains.values().map(DefUseChain::use_count).sum()
    }
}

// Builder

/// Build def-use chains from an SSA function in two passes.
///
/// Pass 1 collects definitions (from phi nodes and statements).
/// Pass 2 collects uses (from statement operands, phi incoming
/// edges, and, when `cfg` is provided, branch-condition reads).
#[must_use]
pub fn build_def_use_chains(ssa: &SsaFunction, cfg: Option<&CfgFunction>) -> DefUseResult {
    let mut chains: HashMap<SsaValueKey, DefUseChain> = HashMap::new();

    // Pass 1: definitions
    for block in ssa.blocks.values() {
        // Phi definitions.
        for phi in &block.phis {
            let key = (ssa.var_name(phi.name).to_owned(), phi.version);
            chains.entry(key.clone()).or_insert_with(|| DefUseChain {
                key,
                definition: DefSite {
                    block: block.name.clone(),
                    kind: DefKind::Phi,
                    statement_index: -1,
                },
                uses: Vec::new(),
            });
        }
        // Statement definitions.
        for (idx, stmt) in block.statements.iter().enumerate() {
            for (sym, ver) in &stmt.defs {
                let key = (ssa.var_name(*sym).to_owned(), *ver);
                chains.entry(key.clone()).or_insert_with(|| DefUseChain {
                    key,
                    definition: DefSite {
                        block: block.name.clone(),
                        kind: DefKind::Statement,
                        statement_index: i32::try_from(idx).unwrap_or(i32::MAX),
                    },
                    uses: Vec::new(),
                });
            }
        }
    }

    // Pass 2: uses
    let entry_name = ssa.block_name(ssa.entry);
    for (bn, block) in &ssa.blocks {
        // Phi incoming edges are uses of the incoming versions.
        for phi in &block.phis {
            let phi_var = ssa.var_name(phi.name).to_owned();
            for (pred_block, incoming_ver) in &phi.incoming {
                let key = (phi_var.clone(), *incoming_ver);
                add_use(
                    &mut chains,
                    entry_name,
                    key,
                    UseSite {
                        block: ssa.block_name(*pred_block).to_owned(),
                        kind: UseKind::PhiIncoming,
                        statement_index: -1,
                        variable: phi_var.clone(),
                        phi_version: phi.version,
                        class: UseClass::Substituted,
                    },
                );
            }
        }

        // Statement operand uses.
        for (idx, stmt) in block.statements.iter().enumerate() {
            for (sym, ver) in &stmt.uses {
                let key = (ssa.var_name(*sym).to_owned(), *ver);
                let class = if stmt.quoted_uses.contains(sym) {
                    UseClass::Quoted
                } else {
                    UseClass::Substituted
                };
                add_use(
                    &mut chains,
                    entry_name,
                    key,
                    UseSite {
                        block: block.name.clone(),
                        kind: UseKind::Operand,
                        statement_index: i32::try_from(idx).unwrap_or(i32::MAX),
                        variable: String::new(),
                        phi_version: 0,
                        class,
                    },
                );
            }
        }

        // Terminator uses: branch conditions and `return` value reads.
        if let Some(cfg) = cfg
            && let Some(cfg_block) = cfg.blocks.get(bn)
        {
            add_terminator_uses(&mut chains, entry_name, ssa, block, cfg_block);
        }
    }

    DefUseResult { chains }
}

/// Variable names a terminator reads, with each name's [`UseClass`]: a
/// `Branch` condition's vars, or a `return $x` value's reads.  The latter is
/// recorded so an earlier overwritten store is a real dead store, not a
/// "truly unused" var.
///
/// A **braced** return value is literal — `return {$y}` returns the two
/// characters `$y` — so its names are [`UseClass::Quoted`]: kept as uses (the
/// string may still be `eval`-ed by the caller) but never a read here.
/// tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]` prints
/// `$y` with `y` undefined.
fn terminator_read_vars(terminator: Option<&Terminator>) -> Vec<(String, UseClass)> {
    match terminator {
        Some(Terminator::Branch { condition, .. }) => condition
            .vars_element_qualified()
            .into_iter()
            .map(|n| (n, UseClass::Substituted))
            .collect(),
        Some(Terminator::Return {
            value,
            expr,
            braced,
            ..
        }) => {
            let mut set: std::collections::BTreeMap<String, UseClass> =
                std::collections::BTreeMap::new();
            let value_class = if *braced {
                UseClass::Quoted
            } else {
                UseClass::Substituted
            };
            if let Some(v) = value {
                set.extend(
                    crate::var_refs::scan_var_ref_forms_braced(v)
                        .into_iter()
                        // A `${…}` read's content is a literal name — `${$n}`
                        // reads the variable called `$n` — so its `$` must
                        // survive canonicalisation (issue #1078).
                        .map(|(n, braced)| {
                            (
                                crate::naming::element_var_name_braced(&n, braced).to_string(),
                                value_class,
                            )
                        }),
                );
            }
            if let Some(e) = expr {
                set.extend(
                    e.vars_element_qualified()
                        .into_iter()
                        .map(|n| (n, UseClass::Substituted)),
                );
            }
            set.into_iter().collect()
        }
        _ => Vec::new(),
    }
}

/// Append a use to an existing chain, synthesising a parameter/
/// statement definition when no def was seen (version 0 → parameter,
/// else a best-effort `Statement` placeholder rooted at `entry`).
fn add_use(
    chains: &mut HashMap<SsaValueKey, DefUseChain>,
    entry: &str,
    key: SsaValueKey,
    use_site: UseSite,
) {
    if let Some(chain) = chains.get_mut(&key) {
        chain.uses.push(use_site);
        return;
    }
    let (_, ver) = &key;
    let kind = if *ver == 0 {
        DefKind::Parameter
    } else {
        DefKind::Statement
    };
    let chain = DefUseChain {
        key: key.clone(),
        definition: DefSite {
            block: entry.to_owned(),
            kind,
            statement_index: -1,
        },
        uses: vec![use_site],
    };
    chains.insert(key, chain);
}

// Tests

/// Record a block terminator's reads as uses at its exit versions. A base
/// read (`return $opts($name)`) also reads every known constant-keyed
/// element — keep their chains live too.
fn add_terminator_uses(
    chains: &mut HashMap<SsaValueKey, DefUseChain>,
    entry_name: &str,
    ssa: &SsaFunction,
    block: &crate::ssa::SsaBlock,
    cfg_block: &crate::cfg::Block,
) {
    for (var_name, class) in terminator_read_vars(cfg_block.terminator.as_ref()) {
        let fanned: Vec<String> = ssa
            .var_names()
            .iter()
            .filter(|n| {
                n.len() > var_name.len()
                    && n.starts_with(&var_name)
                    && n.as_bytes()[var_name.len()] == b'('
            })
            .cloned()
            .collect();
        for name in std::iter::once(var_name).chain(fanned) {
            let version = ssa
                .var_symbol(&name)
                .and_then(|s| block.exit_versions.get(&s))
                .copied()
                .unwrap_or(0);
            let key = (name, version);
            add_use(
                chains,
                entry_name,
                key,
                UseSite {
                    block: block.name.clone(),
                    kind: UseKind::Terminator,
                    statement_index: -1,
                    variable: String::new(),
                    phi_version: 0,
                    class,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Statement;
    use crate::cfg::BlockId;
    use crate::ssa::{Phi, SsaBlock, SsaStatement};
    use std::collections::HashMap;
    use tcl_lexer::Span;

    /// Resolve a block name to its `BlockId` via the function's name table.
    fn bid(ssa: &SsaFunction, name: &str) -> BlockId {
        let idx = ssa
            .block_names()
            .iter()
            .position(|n| n == name)
            .expect("block name interned");
        BlockId(u32::try_from(idx).expect("block count fits in u32"))
    }

    /// Build an SSA function whose name table is exactly `block_names` (in
    /// `BlockId` order), with an empty `SsaBlock` inserted for each name. The
    /// first name is the entry block.
    fn ssa_with_blocks(name: &str, block_names: &[&str]) -> SsaFunction {
        let names: Vec<String> = block_names.iter().map(|n| (*n).to_owned()).collect();
        let entry = BlockId(0);
        let mut ssa = SsaFunction::trivial(name, entry, names);
        for (idx, bn) in block_names.iter().enumerate() {
            let id = BlockId(u32::try_from(idx).expect("block count fits in u32"));
            ssa.blocks.insert(
                id,
                SsaBlock {
                    name: (*bn).to_owned(),
                    phis: Vec::new(),
                    statements: Vec::new(),
                    entry_versions: HashMap::new(),
                    exit_versions: HashMap::new(),
                },
            );
        }
        ssa
    }

    fn empty_ssa(name: &str, entry: &str) -> SsaFunction {
        ssa_with_blocks(name, &[entry])
    }

    /// Intern `name` into `ssa` and build a `set name value` statement whose
    /// `defs` records `name@ver` under its interned symbol.
    fn assign_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: Version) -> SsaStatement {
        let sym = ssa.intern_var(name);
        let mut defs = HashMap::new();
        defs.insert(sym, ver);
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: name.to_owned(),
                value: value.to_owned(),
                name_braced: false,
                value_span: None,
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        }
    }

    /// Intern `defs_name` + every used name into `ssa` and build a statement
    /// recording one symbol-keyed def and the given symbol-keyed uses.
    fn assign_uses_stmt(
        ssa: &mut SsaFunction,
        defs_name: &str,
        defs_ver: Version,
        uses: &[(&str, Version)],
    ) -> SsaStatement {
        let defs_sym = ssa.intern_var(defs_name);
        let mut d = HashMap::new();
        d.insert(defs_sym, defs_ver);
        let mut u = HashMap::new();
        for (name, ver) in uses {
            u.insert(ssa.intern_var(name), *ver);
        }
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: defs_name.to_owned(),
                value: "x".into(),
                name_braced: false,
                value_span: None,
            },
            uses: u,
            defs: d,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn empty_function_has_no_chains() {
        let ssa = empty_ssa("f", "entry");
        let r = build_def_use_chains(&ssa, None);
        assert_eq!(r.total_defs(), 0);
        assert_eq!(r.total_uses(), 0);
    }

    #[test]
    fn single_def_is_dead() {
        let mut ssa = empty_ssa("f", "entry");
        let entry = bid(&ssa, "entry");
        let s = assign_stmt(&mut ssa, "x", "1", 1);
        ssa.blocks.get_mut(&entry).unwrap().statements.push(s);
        let r = build_def_use_chains(&ssa, None);
        assert_eq!(r.total_defs(), 1);
        assert!(r.is_dead("x", 1));
        assert_eq!(r.dead_chains().len(), 1);
    }

    #[test]
    fn def_with_single_use() {
        let mut ssa = empty_ssa("f", "entry");
        let entry = bid(&ssa, "entry");
        let s1 = assign_stmt(&mut ssa, "x", "1", 1);
        let s2 = assign_uses_stmt(&mut ssa, "y", 1, &[("x", 1)]);
        let blk = ssa.blocks.get_mut(&entry).unwrap();
        blk.statements.push(s1);
        blk.statements.push(s2);
        let r = build_def_use_chains(&ssa, None);
        assert!(!r.is_dead("x", 1));
        assert_eq!(r.uses_of("x", 1).len(), 1);
        assert_eq!(r.uses_of("x", 1)[0].kind, UseKind::Operand);
        assert!(r.is_dead("y", 1));
    }

    #[test]
    fn phi_def_and_incoming_uses() {
        // entry defines x@1 and x@2 via two hypothetical predecessors
        // modelled inline, then `join` has a phi merging them.
        let mut ssa = ssa_with_blocks("f", &["entry", "p1", "p2", "join"]);
        let p1 = bid(&ssa, "p1");
        let p2 = bid(&ssa, "p2");
        let join = bid(&ssa, "join");
        let sx = ssa.intern_var("x");
        let p1_stmt = assign_stmt(&mut ssa, "x", "1", 1);
        ssa.blocks.insert(
            p1,
            SsaBlock {
                name: "p1".into(),
                phis: Vec::new(),
                statements: vec![p1_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::from([(sx, 1)]),
            },
        );
        let p2_stmt = assign_stmt(&mut ssa, "x", "2", 2);
        ssa.blocks.insert(
            p2,
            SsaBlock {
                name: "p2".into(),
                phis: Vec::new(),
                statements: vec![p2_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::from([(sx, 2)]),
            },
        );
        let mut incoming = HashMap::new();
        incoming.insert(p1, 1);
        incoming.insert(p2, 2);
        ssa.blocks.insert(
            join,
            SsaBlock {
                name: "join".into(),
                phis: vec![Phi {
                    name: sx,
                    version: 3,
                    incoming,
                }],
                statements: Vec::new(),
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let r = build_def_use_chains(&ssa, None);
        // x@3 is the phi def.
        let phi_chain = r.chain_for("x", 3).expect("phi def chain");
        assert_eq!(phi_chain.definition.kind, DefKind::Phi);
        // x@1 and x@2 each have one phi-incoming use.
        assert_eq!(r.uses_of("x", 1).len(), 1);
        assert_eq!(r.uses_of("x", 1)[0].kind, UseKind::PhiIncoming);
        assert_eq!(r.uses_of("x", 2)[0].kind, UseKind::PhiIncoming);
        assert!(r.chain_for("x", 1).unwrap().has_phi_use());
    }

    #[test]
    fn reaching_defs_lists_all_versions() {
        let mut ssa = empty_ssa("f", "entry");
        let entry = bid(&ssa, "entry");
        let s1 = assign_stmt(&mut ssa, "x", "1", 1);
        let s2 = assign_stmt(&mut ssa, "x", "2", 2);
        let s3 = assign_stmt(&mut ssa, "x", "3", 3);
        let blk = ssa.blocks.get_mut(&entry).unwrap();
        blk.statements.push(s1);
        blk.statements.push(s2);
        blk.statements.push(s3);
        let r = build_def_use_chains(&ssa, None);
        let mut defs = r.reaching_defs("x");
        defs.sort_by_key(|(_, v)| *v);
        assert_eq!(
            defs,
            vec![("x".into(), 1), ("x".into(), 2), ("x".into(), 3),]
        );
    }

    #[test]
    fn unknown_use_synthesises_parameter_def() {
        // Use of x@0 without a prior definition should synthesise a
        // Parameter chain.
        let mut ssa = empty_ssa("f", "entry");
        let entry = bid(&ssa, "entry");
        let s = assign_uses_stmt(&mut ssa, "y", 1, &[("x", 0)]);
        ssa.blocks.get_mut(&entry).unwrap().statements.push(s);
        let r = build_def_use_chains(&ssa, None);
        let chain = r.chain_for("x", 0).expect("synthesised param chain");
        assert_eq!(chain.definition.kind, DefKind::Parameter);
        assert_eq!(chain.uses.len(), 1);
    }
}
