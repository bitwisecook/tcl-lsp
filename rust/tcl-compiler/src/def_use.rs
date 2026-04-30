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
//! The chain is derived from an [`SsaFunction`] (produced by C6's
//! `build_ssa`) in two passes over all blocks. Phi nodes act as both
//! definitions (LHS) and uses (incoming edges from predecessor
//! blocks).
//!
//! Ported from `core/compiler/def_use.py` (C24).

use std::collections::HashMap;

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ssa::{SsaFunction, Version};

/// How a variable definition was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefKind {
    /// Ordinary assignment (`set`, `incr`, `IRAssignConst`, …).
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
#[derive(Debug, Clone, Default)]
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
            .map_or(true, DefUseChain::is_dead)
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

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Build def-use chains from an SSA function in two passes.
///
/// Pass 1 collects definitions (from phi nodes and statements).
/// Pass 2 collects uses (from statement operands, phi incoming
/// edges, and, when `cfg` is provided, branch-condition reads).
#[must_use]
pub fn build_def_use_chains(ssa: &SsaFunction, cfg: Option<&CfgFunction>) -> DefUseResult {
    let mut chains: HashMap<SsaValueKey, DefUseChain> = HashMap::new();

    // ── Pass 1: definitions ─────────────────────────────────────────
    for (bn, block) in &ssa.blocks {
        // Phi definitions.
        for phi in &block.phis {
            let key = (phi.name.clone(), phi.version);
            chains.entry(key.clone()).or_insert_with(|| DefUseChain {
                key,
                definition: DefSite {
                    block: bn.clone(),
                    kind: DefKind::Phi,
                    statement_index: -1,
                },
                uses: Vec::new(),
            });
        }
        // Statement definitions.
        for (idx, stmt) in block.statements.iter().enumerate() {
            for (name, ver) in &stmt.defs {
                let key = (name.clone(), *ver);
                chains.entry(key.clone()).or_insert_with(|| DefUseChain {
                    key,
                    definition: DefSite {
                        block: bn.clone(),
                        kind: DefKind::Statement,
                        statement_index: i32::try_from(idx).unwrap_or(i32::MAX),
                    },
                    uses: Vec::new(),
                });
            }
        }
    }

    // ── Pass 2: uses ────────────────────────────────────────────────
    for (bn, block) in &ssa.blocks {
        // Phi incoming edges are uses of the incoming versions.
        for phi in &block.phis {
            for (pred_block, incoming_ver) in &phi.incoming {
                let key = (phi.name.clone(), *incoming_ver);
                add_use(
                    &mut chains,
                    &ssa.entry,
                    key,
                    UseSite {
                        block: pred_block.clone(),
                        kind: UseKind::PhiIncoming,
                        statement_index: -1,
                        variable: phi.name.clone(),
                        phi_version: phi.version,
                    },
                );
            }
        }

        // Statement operand uses.
        for (idx, stmt) in block.statements.iter().enumerate() {
            for (name, ver) in &stmt.uses {
                let key = (name.clone(), *ver);
                add_use(
                    &mut chains,
                    &ssa.entry,
                    key,
                    UseSite {
                        block: bn.clone(),
                        kind: UseKind::Operand,
                        statement_index: i32::try_from(idx).unwrap_or(i32::MAX),
                        variable: String::new(),
                        phi_version: 0,
                    },
                );
            }
        }

        // Terminator uses (branch conditions).
        if let Some(cfg) = cfg {
            if let Some(cfg_block) = cfg.blocks.get(bn) {
                if let Some(Terminator::Branch { condition, .. }) = &cfg_block.terminator {
                    for var_name in condition.vars() {
                        let version = block.exit_versions.get(&var_name).copied().unwrap_or(0);
                        let key = (var_name, version);
                        add_use(
                            &mut chains,
                            &ssa.entry,
                            key,
                            UseSite {
                                block: bn.clone(),
                                kind: UseKind::Terminator,
                                statement_index: -1,
                                variable: String::new(),
                                phi_version: 0,
                            },
                        );
                    }
                }
            }
        }
    }

    DefUseResult { chains }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssa::{Phi, SsaBlock, SsaStatement};
    use crate::Statement;
    use std::collections::HashMap;
    use tcl_lexer::Span;

    fn empty_ssa(name: &str, entry: &str) -> SsaFunction {
        let mut blocks = HashMap::new();
        blocks.insert(
            entry.to_owned(),
            SsaBlock {
                name: entry.to_owned(),
                phis: Vec::new(),
                statements: Vec::new(),
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        SsaFunction {
            name: name.to_owned(),
            entry: entry.to_owned(),
            blocks,
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        }
    }

    fn assign_stmt(name: &str, value: &str, ver: Version) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(name.to_owned(), ver);
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: name.to_owned(),
                value: value.to_owned(),
            },
            uses: HashMap::new(),
            defs,
        }
    }

    fn assign_uses_stmt(
        defs_name: &str,
        defs_ver: Version,
        uses: &[(&str, Version)],
    ) -> SsaStatement {
        let mut d = HashMap::new();
        d.insert(defs_name.to_owned(), defs_ver);
        let mut u = HashMap::new();
        for (name, ver) in uses {
            u.insert((*name).to_owned(), *ver);
        }
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: defs_name.to_owned(),
                value: "x".into(),
            },
            uses: u,
            defs: d,
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
        ssa.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(assign_stmt("x", "1", 1));
        let r = build_def_use_chains(&ssa, None);
        assert_eq!(r.total_defs(), 1);
        assert!(r.is_dead("x", 1));
        assert_eq!(r.dead_chains().len(), 1);
    }

    #[test]
    fn def_with_single_use() {
        let mut ssa = empty_ssa("f", "entry");
        let blk = ssa.blocks.get_mut("entry").unwrap();
        blk.statements.push(assign_stmt("x", "1", 1));
        blk.statements.push(assign_uses_stmt("y", 1, &[("x", 1)]));
        let r = build_def_use_chains(&ssa, None);
        assert!(!r.is_dead("x", 1));
        assert_eq!(r.uses_of("x", 1).len(), 1);
        assert_eq!(r.uses_of("x", 1)[0].kind, UseKind::Operand);
        assert!(r.is_dead("y", 1));
    }

    #[test]
    fn phi_def_and_incoming_uses() {
        let mut ssa = empty_ssa("f", "entry");
        // entry defines x@1 and x@2 via two hypothetical predecessors
        // modelled inline, then `join` has a phi merging them.
        ssa.blocks.insert(
            "p1".into(),
            SsaBlock {
                name: "p1".into(),
                phis: Vec::new(),
                statements: vec![assign_stmt("x", "1", 1)],
                entry_versions: HashMap::new(),
                exit_versions: {
                    let mut m = HashMap::new();
                    m.insert("x".into(), 1);
                    m
                },
            },
        );
        ssa.blocks.insert(
            "p2".into(),
            SsaBlock {
                name: "p2".into(),
                phis: Vec::new(),
                statements: vec![assign_stmt("x", "2", 2)],
                entry_versions: HashMap::new(),
                exit_versions: {
                    let mut m = HashMap::new();
                    m.insert("x".into(), 2);
                    m
                },
            },
        );
        let mut incoming = HashMap::new();
        incoming.insert("p1".into(), 1);
        incoming.insert("p2".into(), 2);
        ssa.blocks.insert(
            "join".into(),
            SsaBlock {
                name: "join".into(),
                phis: vec![Phi {
                    name: "x".into(),
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
        let blk = ssa.blocks.get_mut("entry").unwrap();
        blk.statements.push(assign_stmt("x", "1", 1));
        blk.statements.push(assign_stmt("x", "2", 2));
        blk.statements.push(assign_stmt("x", "3", 3));
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
        ssa.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(assign_uses_stmt("y", 1, &[("x", 0)]));
        let r = build_def_use_chains(&ssa, None);
        let chain = r.chain_for("x", 0).expect("synthesised param chain");
        assert_eq!(chain.definition.kind, DefKind::Parameter);
        assert_eq!(chain.uses.len(), 1);
    }
}
