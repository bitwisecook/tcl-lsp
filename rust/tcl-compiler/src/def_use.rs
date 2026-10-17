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
use std::ops::Deref;
use std::sync::Arc;

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ssa::{SsaFunction, Symbol, UseClass, Version};
use crate::var_resolve::VariableCellKey;

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
    /// Named by a statement rather than substituted into it: `incr a`,
    /// `append a x`, `info exists a`, `unset a` — the cell is read (and often
    /// written) through a variable-*name* argument, with no `$a` word.
    ///
    /// Split from [`Self::Operand`] because the two answer different questions
    /// and a pass that conflates them is unsound. Liveness must count both: the
    /// value really is consumed. A pass that *rewrites* the use must count only
    /// `Operand`, because a name position has nothing to rewrite — forwarding
    /// `a`'s single reaching literal into `incr a` yields `incr` over `1`,
    /// which is neither an increment nor a command.
    VariableName,
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
    /// The current SSA symbol; for phi input, the receiving phi symbol.
    pub variable: Symbol,
    /// For `PhiIncoming`: the phi's defined version.
    pub phi_version: Version,
    /// Whether the name is carried only by a brace-quoted word this statement
    /// does not substitute ([`UseClass::Quoted`]). The use is real for
    /// liveness — the text may be evaluated later — but is not a read *here*,
    /// so read-before-set must not claim it.
    pub class: UseClass,
}

/// Function-local SSA value coordinate, independent of physical storage.
pub type SsaValueKey = crate::ssa::ValueKey;

/// Current canonical storage identity and its represented SSA version.
pub type StorageValueKey = (VariableCellKey, Version);

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
    /// Look up one actual function-local SSA value.
    #[must_use]
    pub fn chain_for(&self, symbol: Symbol, version: Version) -> Option<&DefUseChain> {
        self.chains.get(&(symbol, version))
    }

    /// All consumers of one function-local SSA value.
    #[must_use]
    pub fn uses_of(&self, symbol: Symbol, version: Version) -> &[UseSite] {
        self.chain_for(symbol, version)
            .map_or(&[], |chain| chain.uses.as_slice())
    }

    /// Whether a function-local SSA value has no consumers.
    #[must_use]
    pub fn is_dead(&self, symbol: Symbol, version: Version) -> bool {
        self.chain_for(symbol, version)
            .is_none_or(DefUseChain::is_dead)
    }

    /// Every represented definition of this function-local symbol.
    #[must_use]
    pub fn reaching_defs(&self, symbol: Symbol) -> Vec<SsaValueKey> {
        self.chains
            .keys()
            .filter(|(candidate, _)| *candidate == symbol)
            .copied()
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

/// Shared SSA graph with independently checked location and source correspondence.
/// Physical uses retain their separate currentness, execution and outcome guards.
#[derive(Debug, Clone, Default)]
pub struct DefUseFacts {
    result: Arc<DefUseResult>,
    cells: Arc<HashMap<Symbol, VariableCellKey>>,
    sources: Arc<HashMap<Symbol, crate::ssa::SourceVariableSsaSnapshot>>,
}

impl PartialEq for DefUseFacts {
    fn eq(&self, other: &Self) -> bool {
        self.result == other.result
            && self.cells == other.cells
            && self.sources.len() == other.sources.len()
            && self.sources.iter().all(|(symbol, source)| {
                other
                    .sources
                    .get(symbol)
                    .is_some_and(|other| source.same_owner(other))
            })
    }
}

impl Deref for DefUseFacts {
    type Target = Arc<DefUseResult>;
    fn deref(&self) -> &Self::Target {
        &self.result
    }
}

impl DefUseFacts {
    /// Interpret one graph symbol only when its retained location key is current.
    #[must_use]
    pub fn cell_key<'a>(
        &self,
        ssa: &'a SsaFunction,
        symbol: Symbol,
    ) -> Option<&'a VariableCellKey> {
        let current = ssa.cell_key(symbol)?;
        (self.cells.get(&symbol) == Some(current)).then_some(current)
    }

    /// Source labels require the same original source graph, independently of locations.
    #[must_use]
    pub fn source_variable_symbol<'a>(
        &self,
        ssa: &'a SsaFunction,
        symbol: Symbol,
    ) -> Option<&'a crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol> {
        let held = self.sources.get(&symbol)?;
        let current = ssa.source_variable_snapshot(symbol)?;
        held.same_owner(&current).then_some(())?;
        ssa.source_variable_symbol(symbol)
    }

    /// Reborrow an actual graph symbol under its current location interpretation.
    #[must_use]
    pub fn symbol(&self, ssa: &SsaFunction, symbol: Symbol) -> Option<Symbol> {
        self.cell_key(ssa, symbol).map(|_| symbol)
    }

    /// Reborrow a complete current storage/version key from a graph coordinate.
    #[must_use]
    pub fn physical_value_key(
        &self,
        ssa: &SsaFunction,
        key: SsaValueKey,
    ) -> Option<StorageValueKey> {
        Some((self.cell_key(ssa, key.0)?.clone(), key.1))
    }

    /// Query a current exact cell without interpreting a name or display label.
    #[must_use]
    pub fn chain_for_cell(
        &self,
        ssa: &SsaFunction,
        cell: &VariableCellKey,
        version: Version,
    ) -> Option<&DefUseChain> {
        let symbol = ssa.cell_symbol(cell)?;
        (self.cell_key(ssa, symbol)? == cell).then_some(())?;
        self.result.chain_for(symbol, version)
    }

    /// Rebind current storage keys while retaining the same immutable SSA graph.
    pub(crate) fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        self.cells = Arc::new(
            self.cells
                .iter()
                .map(|(&symbol, key)| (symbol, relocation.storage_key(key)))
                .collect(),
        );
    }
}

// Builder

/// Build def-use chains from an SSA function in two passes.
///
/// Pass 1 collects definitions (from phi nodes and statements).
/// Pass 2 collects uses (from statement operands, phi incoming
/// edges, and, when `cfg` is provided, branch-condition reads).
/// Raw expression fallbacks and return words are re-read under `config`, the
/// exact lexer configuration that produced the CFG.
#[must_use]
pub fn build_def_use_chains(
    ssa: &SsaFunction,
    cfg: Option<&CfgFunction>,
    config: tcl_lexer::LexerConfig,
) -> DefUseFacts {
    let mut chains: HashMap<SsaValueKey, DefUseChain> = HashMap::new();

    // Pass 1: definitions
    for block in ssa.blocks.values() {
        // Phi definitions.
        for phi in &block.phis {
            let key = (phi.name, phi.version);
            chains.entry(key).or_insert_with(|| DefUseChain {
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
                let key = (*sym, *ver);
                chains.entry(key).or_insert_with(|| DefUseChain {
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
            let phi_var = phi.name;
            for (pred_block, incoming_ver) in &phi.incoming {
                let key = (phi_var, *incoming_ver);
                add_use(
                    &mut chains,
                    entry_name,
                    key,
                    UseSite {
                        block: ssa.block_name(*pred_block).to_owned(),
                        kind: UseKind::PhiIncoming,
                        statement_index: -1,
                        variable: phi_var,
                        phi_version: phi.version,
                        class: if ssa.source_variable_symbol(phi_var).is_some() {
                            UseClass::Quoted
                        } else {
                            UseClass::Substituted
                        },
                    },
                );
            }
        }

        // Statement operand uses.
        for (idx, stmt) in block.statements.iter().enumerate() {
            for (sym, ver) in &stmt.uses {
                let key = (*sym, *ver);
                let class = if stmt.quoted_uses.contains(sym) {
                    UseClass::Quoted
                } else if stmt.name_only_uses.contains(sym) {
                    UseClass::Name
                } else {
                    UseClass::Substituted
                };
                let kind = if class == UseClass::Name {
                    UseKind::VariableName
                } else {
                    UseKind::Operand
                };
                add_use(
                    &mut chains,
                    entry_name,
                    key,
                    UseSite {
                        block: block.name.clone(),
                        kind,
                        statement_index: i32::try_from(idx).unwrap_or(i32::MAX),
                        variable: *sym,
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
            add_terminator_uses(&mut chains, entry_name, ssa, block, cfg_block, config);
        }
    }

    DefUseFacts {
        result: Arc::new(DefUseResult { chains }),
        cells: Arc::new(
            ssa.cell_symbols()
                .map(|(symbol, key)| (symbol, key.clone()))
                .collect(),
        ),
        sources: Arc::new(
            ssa.source_variable_symbols()
                .filter_map(|(symbol, _)| Some((symbol, ssa.source_variable_snapshot(symbol)?)))
                .collect(),
        ),
    }
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
pub(crate) fn terminator_read_vars(
    terminator: Option<&Terminator>,
    config: tcl_lexer::LexerConfig,
) -> Vec<(String, UseClass)> {
    match terminator {
        Some(Terminator::Branch { condition, .. }) => condition
            .vars_element_qualified_with_config(config)
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
                    crate::var_refs::scan_var_ref_forms_braced_with_config(v, config)
                        .into_iter()
                        // A `${…}` read's content is a literal name — `${$n}`
                        // reads the variable called `$n` — so its `$` must
                        // survive canonicalisation.
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
                    e.vars_element_qualified_with_config(config)
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
        key,
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
    config: tcl_lexer::LexerConfig,
) {
    let Some(block_id) = ssa.block_id(&block.name) else {
        return;
    };
    for (&symbol, &version) in &block.exit_versions {
        if version != 0 && ssa.source_variable_symbol(symbol).is_some() {
            add_use(
                chains,
                entry_name,
                (symbol, version),
                UseSite {
                    block: block.name.clone(),
                    kind: UseKind::Terminator,
                    statement_index: -1,
                    variable: symbol,
                    phi_version: 0,
                    class: UseClass::Quoted,
                },
            );
        }
    }
    let view = crate::ssa::SsaSourceView::at_terminator(ssa, block_id);
    let mut covered = std::collections::HashSet::new();
    if let Some(tokens) = view.source_tokens() {
        for access in &tokens.variable_accesses {
            if let Some(dialect) = access.variable_context.invocation_dialect {
                covered.insert(
                    tcl_syntax::naming::var_reference_for_style(
                        &access.original_spelling,
                        dialect.lexer_grammar.braced_var,
                    )
                    .to_owned(),
                );
            }
            let Some(read) = view.read_reference(&access.source, &access.original_spelling) else {
                continue;
            };
            let name = read.symbol;
            let keys = if let Some(version) = read.version {
                vec![(name, version)]
            } else {
                // A nested store without a represented value version still
                // depends on every represented producer of this binding.
                chains
                    .keys()
                    .filter(|(cell, _)| cell == &name)
                    .copied()
                    .collect::<Vec<_>>()
            };
            for key in keys {
                add_use(
                    chains,
                    entry_name,
                    key,
                    UseSite {
                        block: block.name.clone(),
                        kind: UseKind::Terminator,
                        statement_index: -1,
                        variable: key.0,
                        phi_version: 0,
                        class: UseClass::Substituted,
                    },
                );
            }
        }
    }
    for (var_name, class) in terminator_read_vars(cfg_block.terminator.as_ref(), config) {
        if covered.contains(&var_name) {
            continue;
        }
        let Some(symbol) = ssa.var_symbol_at_terminator(block_id, &var_name) else {
            continue;
        };
        let Some(var_name) = ssa.cell_key(symbol).cloned() else {
            continue;
        };
        let fanned: Vec<VariableCellKey> = ssa
            .cell_keys()
            .iter()
            .filter(|key| key.is_member_of(&var_name))
            .cloned()
            .collect();
        for name in std::iter::once(var_name).chain(fanned) {
            let version = ssa
                .cell_symbol(&name)
                .and_then(|s| block.exit_versions.get(&s))
                .copied()
                .unwrap_or(0);
            let Some(symbol) = ssa.cell_symbol(&name) else {
                continue;
            };
            let key = (symbol, version);
            add_use(
                chains,
                entry_name,
                key,
                UseSite {
                    block: block.name.clone(),
                    kind: UseKind::Terminator,
                    statement_index: -1,
                    variable: symbol,
                    phi_version: 0,
                    class,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::interprocedural::test_body_identity::fixture_id;

    use super::*;
    use crate::Statement;
    use crate::cfg::BlockId;
    use crate::ssa::{Phi, SsaBlock, SsaStatement};
    use std::collections::HashMap;
    use tcl_lexer::Span;

    #[test]
    fn original_source_chains_refuse_same_symbols_from_another_source_image() {
        let source = "proc p {} {global g; set g 1}\np";
        let (_, first) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        let (_, second) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        let (symbol, identity) = first
            .top_level
            .ssa
            .source_variable_symbols()
            .next()
            .expect("authentic source MAY definition");
        assert_eq!(
            second.top_level.ssa.source_variable_symbol(symbol),
            Some(identity)
        );
        assert!(
            first
                .top_level
                .def_use
                .source_variable_symbol(&first.top_level.ssa, symbol)
                .is_some()
        );
        assert!(
            first
                .top_level
                .def_use
                .source_variable_symbol(&second.top_level.ssa, symbol)
                .is_none(),
            "same coordinate and typed source key cannot donate an old source graph"
        );
        assert!(
            first
                .top_level
                .def_use
                .cell_key(&first.top_level.ssa, symbol)
                .is_none()
        );
    }

    #[test]
    fn distinct_native_key_data_does_not_become_a_presentation_lookup() {
        use crate::command_binding::SourceNamespaceKey;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };
        let interpreter = NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        };
        let key = |token| VariableCellKey::Namespace {
            identity: SourceNamespaceKey::Native(NativeNamespaceContext {
                interpreter,
                token,
                path: tcl_core_types::ByteNamespacePath::from_segments(["same"]),
            }),
            simple: "x".into(),
        };
        let original = key(1);
        let replacement = key(2);
        assert_ne!(original, replacement);
        assert_ne!(
            original.compatibility_name(),
            replacement.compatibility_name()
        );
        let mut ssa = empty_ssa("f", "entry");
        let first = ssa.intern_var("written_x");
        let second = ssa.intern_var("written_y");
        let mut result = DefUseResult::default();
        for symbol in [first, second] {
            result.chains.insert(
                (symbol, 1),
                DefUseChain {
                    key: (symbol, 1),
                    definition: DefSite {
                        block: "entry".into(),
                        kind: DefKind::Statement,
                        statement_index: 0,
                    },
                    uses: Vec::new(),
                },
            );
        }
        let facts = DefUseFacts {
            result: Arc::new(result),
            cells: Arc::new(HashMap::from([(first, original), (second, replacement)])),
            sources: Arc::new(HashMap::new()),
        };
        assert_eq!(facts.chains.len(), 2);
        assert!(facts.chain_for(first, 1).is_some());
        assert!(facts.chain_for(second, 1).is_some());
        assert!(facts.cell_key(&ssa, first).is_none());
        assert!(facts.cell_key(&ssa, second).is_none());
        assert!(
            facts
                .chain_for_cell(&ssa, ssa.cell_key(first).expect("original location"), 1)
                .is_none()
        );
    }

    #[test]
    fn typed_chains_cannot_be_queried_by_native_presentation_text() {
        let mut ssa = empty_ssa("f", "entry");
        let entry = bid(&ssa, "entry");
        let statement = assign_stmt(&mut ssa, "written_x", "1", 1);
        ssa.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(statement);
        let symbol = ssa.var_symbol("written_x").unwrap();
        let facts = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        assert_eq!(facts.chain_for(symbol, 1).unwrap().key, (symbol, 1));
        assert!(
            facts
                .chain_for_cell(&ssa, ssa.cell_key(symbol).expect("original location"), 1)
                .is_some()
        );

        let mut foreign = empty_ssa("g", "entry");
        let foreign_symbol = foreign.intern_var("::same::x");
        assert_eq!(symbol, foreign_symbol);
        assert!(facts.cell_key(&foreign, foreign_symbol).is_none());
        assert!(
            facts
                .chain_for_cell(
                    &foreign,
                    foreign.cell_key(foreign_symbol).expect("foreign location"),
                    1
                )
                .is_none()
        );
        let copied = facts.clone();
        assert!(Arc::ptr_eq(&facts, &copied));
        assert_eq!(copied.cell_key(&ssa, symbol), ssa.cell_key(symbol));
    }

    #[test]
    fn return_reads_preserve_both_producers_across_alias_retargeting() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source =
            "proc p {} {set x OLD; set y NEW; upvar 0 x a; return \"$a[upvar 0 y a]$a\"}; p";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body");
        for name in ["x", "y"] {
            let symbol = function.ssa.var_symbol(name).expect("unambiguous producer");
            let cell = function.ssa.cell_key(symbol).expect("original location");
            assert!(
                function
                    .def_use
                    .chains
                    .iter()
                    .any(
                        |((key, _), chain)| function.def_use.cell_key(&function.ssa, *key)
                            == Some(cell)
                            && chain
                                .uses
                                .iter()
                                .any(|site| site.kind == UseKind::Terminator)
                    ),
                "{name}: {:?}",
                function.def_use
            );
        }
    }

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
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
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
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn empty_function_has_no_chains() {
        let ssa = empty_ssa("f", "entry");
        let r = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        assert_eq!(r.total_defs(), 0);
        assert_eq!(r.total_uses(), 0);
    }

    #[test]
    fn single_def_is_dead() {
        let mut ssa = empty_ssa("f", "entry");
        let entry = bid(&ssa, "entry");
        let s = assign_stmt(&mut ssa, "x", "1", 1);
        ssa.blocks.get_mut(&entry).unwrap().statements.push(s);
        let r = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        assert_eq!(r.total_defs(), 1);
        assert!(r.is_dead(ssa.var_symbol("x").unwrap(), 1));
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
        let r = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        assert!(!r.is_dead(ssa.var_symbol("x").unwrap(), 1));
        assert_eq!(r.uses_of(ssa.var_symbol("x").unwrap(), 1).len(), 1);
        assert_eq!(
            r.uses_of(ssa.var_symbol("x").unwrap(), 1)[0].kind,
            UseKind::Operand
        );
        assert!(r.is_dead(ssa.var_symbol("y").unwrap(), 1));
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
        let r = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        // x@3 is the phi def.
        let phi_chain = r
            .chain_for(ssa.var_symbol("x").unwrap(), 3)
            .expect("phi def chain");
        assert_eq!(phi_chain.definition.kind, DefKind::Phi);
        // x@1 and x@2 each have one phi-incoming use.
        assert_eq!(r.uses_of(ssa.var_symbol("x").unwrap(), 1).len(), 1);
        assert_eq!(
            r.uses_of(ssa.var_symbol("x").unwrap(), 1)[0].kind,
            UseKind::PhiIncoming
        );
        assert_eq!(
            r.uses_of(ssa.var_symbol("x").unwrap(), 2)[0].kind,
            UseKind::PhiIncoming
        );
        assert!(
            r.chain_for(ssa.var_symbol("x").unwrap(), 1)
                .unwrap()
                .has_phi_use()
        );
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
        let r = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        let mut defs = r.reaching_defs(ssa.var_symbol("x").unwrap());
        defs.sort_by_key(|(_, v)| *v);
        assert_eq!(
            defs,
            vec![
                (ssa.var_symbol("x").unwrap(), 1),
                (ssa.var_symbol("x").unwrap(), 2),
                (ssa.var_symbol("x").unwrap(), 3)
            ]
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
        let r = build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
        let chain = r
            .chain_for(ssa.var_symbol("x").unwrap(), 0)
            .expect("synthesised param chain");
        assert_eq!(chain.definition.kind, DefKind::Parameter);
        assert_eq!(chain.uses.len(), 1);
    }

    #[test]
    fn return_raw_scan_uses_the_exact_jim_config() {
        let terminator = Terminator::Return {
            expr_base: None,
            tokens: None,
            value: Some("$($a)".to_owned()),
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        };
        let jim = terminator_read_vars(
            Some(&terminator),
            tcl_lexer::LexerConfig::for_dialect("jim"),
        );
        let tcl = terminator_read_vars(
            Some(&terminator),
            tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
        );

        assert!(
            jim.iter().any(|(name, _)| name == "a"),
            "Jim's $(...) expression sugar exposes its inner variable: {jim:?}"
        );
        assert!(
            !tcl.iter().any(|(name, _)| name == "a"),
            "the Tcl grammar reads $(...) as a different variable form: {tcl:?}"
        );
    }
}
