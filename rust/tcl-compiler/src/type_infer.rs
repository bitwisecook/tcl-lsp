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

//! Type propagation over the SSA graph.
//!
//! Computes a `TypeLattice` for every SSA value by iterating
//! to a fixed point over the CFG in SCCP-executable-block order.
//!
//! The pass is intentionally conservative: it only assigns a `Known`
//! type when the assignment is unambiguous (constant, `incr`, pure var
//! ref, or command with a declared `return_type`). Everything else
//! remains `Unknown` or widens to `Overdefined`.
//!
//! ## Inputs
//!
//! - `cfg` — control-flow graph.
//! - `ssa` — SSA form with phi nodes and statement use/def maps.
//! - `sccp` — SCCP result providing `executable_blocks` and
//!   `executable_edges` so unreachable branches don't widen types.
//! - `registry` — command registry for return-type look-ups.
//!
//! ## Output
//!
//! A `HashMap<ValueKey, TypeLattice>` mapping each `(name, version)`
//! pair to its inferred type. Values not present in the map are
//! implicitly `Unknown`.

use std::collections::HashMap;
use std::collections::HashSet;

use tcl_dialect::NumberSyntax;
use tcl_registry::{CommandRegistry, TclType};

use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::sccp::SccpResult;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
#[cfg(test)]
use crate::types::{Elements, TypeShape};
use crate::types::{TypeKind, TypeLattice, type_join};
use crate::var_resolve::VariableCellKey;

mod caller_safe_factories;
mod container_elements;
mod definition_types;
#[cfg(test)]
mod expression_types;
mod literals;
mod word_types;

use caller_safe_factories::has_only_caller_safe_factories;
#[cfg(test)]
use caller_safe_factories::{
    caller_safe_factory_invocation, empty_source_class,
    substitutions_have_only_caller_safe_factories,
};
#[cfg(test)]
use definition_types::elements_of_def_typing;
use definition_types::{DefTyping, evaluate_type_def};
#[cfg(test)]
use expression_types::infer_expr_type;
#[cfg(test)]
use literals::expr_literal_type;
#[cfg(test)]
use literals::literal_type;
pub use literals::{OriginalLiteralArgumentContents, original_literal_argument_contents};
use word_types::{
    lowered_expression_result_type, retained_contents_type, type_of_read_contents, value_word_type,
};

/// Explicit standalone numeric policy for compatibility helpers.
/// Actual function typing uses the independent retained source grammar in
/// [`TypePropagationMetadata`]. No ambient numeric state participates.
#[must_use]
fn numbers_of(registry: &CommandRegistry) -> NumberSyntax {
    registry.numbers()
}

/// Function namespace label for compatibility constructor advice.
fn function_namespace(qname: &str) -> String {
    tcl_syntax::naming::reported_qname_namespace(
        qname,
        tcl_syntax::naming::ReportedQNameNamespacePurpose::LiteralPrefix,
    )
}

/// Compatibility constructor advice in shared current/global candidate order.
/// The class roster supplies no competing-command, path or lifetime proof.
fn constructor_object_type<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> TypeLattice {
    let is_ctor_spelling = args
        .first()
        .is_some_and(|word| registry.is_possible_class_construction_word(word));
    if is_ctor_spelling
        && let Some(class) =
            crate::naming::resolve_command_with::<&str, _>(namespace, &[], command, |candidate| {
                known_classes.contains(candidate)
            })
    {
        return TypeLattice::object_of(class);
    }
    TypeLattice::overdefined()
}

/// Return the type produced by a known command's return value.
///
/// Checks the command spec's `return_type` field, with subcommand
/// support.  ``pub(crate)`` rather than ``pub`` so the helper
/// stays an internal API surface — only the analyser-side
/// W307 / W308 emitter consumes it today.
#[must_use]
pub(crate) fn return_type_for_command<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> TypeLattice {
    let Some(spec) = registry.get(command) else {
        // Not a registered built-in — recognise a TclOO / snit constructor
        // (`Foo new` / `Foo create x` / `Foo %AUTO%` / `Widget .path`) whose
        // head names a known class, typing it `OBJECT(::ns::Foo)` — but not
        // `object_of` from the `new` spelling alone.
        return constructor_object_type(registry, command, args, known_classes, namespace);
    };

    // Resolved against this call, not just the command: `regexp -inline`
    // returns the matched substrings as a list where a bare `regexp` returns
    // a match count.  Subcommand dispatch and the per-form
    // refinement both live in `return_type_for_call`, so the taint sanitiser
    // test and the shimmer byte-array check cannot disagree with this.
    match spec.return_type_for_call(args) {
        Some(t) => TypeLattice::of(t),
        None => TypeLattice::overdefined(),
    }
    // NB: a registry naming-factory (`struct::graph ?name?`) is deliberately
    // *not* typed `OBJECT(class)` in the SSA lattice here.  The W307/W308
    // object-dispatch checks aggregate `fu.types` object-insensitively across
    // procs (`var_command::aggregate_object_types`), so lattice-typing a
    // factory result would leak a handle's class from one proc to a same-named
    // untyped var in another (regressing FP-OBJ-04).  The factory-return
    // provenance lives instead in `object_types::object_handle_classes` (a
    // highlight/callback-only, imprecision-tolerant map), which harvests these
    // factories syntactically without feeding the diagnostic aggregate.
}

/// True when `command` (with `args`) creates a scope alias — see
/// [`crate::var_scoping::is_scope_alias_call`], the shared recogniser.
///
/// Such a statement imports an externally-determined variable whose intrep
/// lives in another scope, so its def must widen to `Overdefined` rather
/// than take the command's nominal (`String`) return type — otherwise a
/// use-site / merge shimmer check fires on a nominally-`String`-typed
/// alias.
#[cfg(test)]
fn is_scope_alias_call(registry: &CommandRegistry, command: &str, args: &[String]) -> bool {
    crate::var_scoping::is_scope_alias_call(registry, command, args)
}

/// Widening advice from the same original selected invocation. Missing or
/// foreign metadata supplies no effect-free alias declaration or nominal type.
fn original_scope_alias_call(
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    statement: &Statement,
) -> Option<bool> {
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    let Statement::Call {
        command,
        canonical_command,
        args,
        tokens,
        ..
    } = statement
    else {
        return Some(false);
    };
    // Explicit standalone source compatibility carries no Native receiver.
    if metadata.is_standalone()
        && tokens
            .as_ref()
            .is_none_or(|tokens| tokens.source_binding.is_none())
    {
        return Some(crate::var_scoping::is_scope_alias_call(
            registry,
            canonical_command.as_deref().unwrap_or(command),
            args,
        ));
    }
    let tokens = tokens.as_ref()?;
    // The element stores of a proved iteration write its loop variables in
    // this frame; they never link a local to another frame's cell.
    if matches!(
        tokens.synthetic,
        Some(crate::ir::SyntheticMarker::IterationBindings(_))
    ) {
        return Some(false);
    }
    let binding = tokens.source_binding.as_ref()?;
    let purpose = binding.original_invocation_source_purpose(tokens, metadata, registry)?;
    let selected = match purpose {
        crate::command_binding::OriginalInvocationSourcePurpose::Logical => {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        }
        crate::command_binding::OriginalInvocationSourcePurpose::Execution
        | crate::command_binding::OriginalInvocationSourcePurpose::Standalone
        | crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation => {
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                registry,
                Some(metadata),
                tokens,
            )
        }
    }?;
    Some(
        selected
            .facts
            .traits
            .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS),
    )
}

/// Retain executable type provenance only when no widening fact owns the key.
fn type_version(
    ssa: &SsaFunction,
    symbol: Symbol,
    version: u32,
    types: &HashMap<ValueKey, TypeLattice>,
) -> u32 {
    if types.contains_key(&(symbol, version)) {
        version
    } else {
        ssa.binding_version(symbol, version)
    }
}

/// Conditional normal result types keyed by original implementation allocation.
pub(crate) type NormalProcedureResultTypes =
    HashMap<crate::command_binding::CommandAllocation, TypeLattice>;

/// Read-only word-shape context at one retained program point.
struct WordTypingCtx<'a, S: std::hash::BuildHasher> {
    preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    /// Reached math-function dispatches of the same source inventory.
    math: &'a [crate::command_binding::SourceMathInvocation],
    tokens: Option<&'a crate::ir::CommandTokens>,
    source: crate::ssa::SsaSourceView<'a>,
    uses: &'a HashMap<Symbol, u32>,
    /// The definitions of the statement being typed, when one is: a name it
    /// defines for the first time has no reaching spelling at its own point.
    defs: Option<&'a HashMap<Symbol, u32>>,
    /// Complete retained availability; this supplies no executed handler.
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    /// Exact lexical and list policy of the original source.
    config: tcl_lexer::LexerConfig,
    types: &'a HashMap<ValueKey, TypeLattice>,
    /// SCCP constants at this point — purity evidence (a numeric-typed
    /// literal is still a pure string) and constant list/index values for
    /// element facts.
    values: &'a HashMap<ValueKey, LatticeValue>,
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    ssa: &'a SsaFunction,
    /// The actual source's numeric grammar, independently of the command store.
    numbers: NumberSyntax,
    /// Active provenance write points; cycles cannot establish a value type.
    active_writes: &'a [(BlockId, usize)],
    normal_results: Option<&'a NormalProcedureResultTypes>,
}

impl<S: std::hash::BuildHasher> WordTypingCtx<'_, S> {
    fn is_variable_word(&self, spelling: &str) -> bool {
        tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), self.config)
            .is_ok_and(|reference| reference.is_some())
    }

    fn variable_key(&self, spelling: &str) -> Option<VariableCellKey> {
        self.source
            .symbol(spelling)
            .or_else(|| self.defined_symbol(spelling))
            .and_then(|symbol| self.ssa.cell_key(symbol).cloned())
    }

    /// The one symbol the statement being typed defines under `spelling`.
    fn defined_symbol(&self, spelling: &str) -> Option<Symbol> {
        let mut named = self
            .defs?
            .keys()
            .filter(|symbol| self.ssa.var_name(**symbol) == spelling);
        let symbol = *named.next()?;
        named.next().is_none().then_some(symbol)
    }

    fn variable_type(&self, spelling: &str) -> Option<TypeLattice> {
        if self.source.is_positioned() {
            let read = self.source.read_spelling(spelling)?;
            if let Some(version) = read.version {
                let version = type_version(self.ssa, read.symbol, version, self.types);
                return self.types.get(&(read.symbol, version)).cloned();
            }
            return self
                .source
                .read_spelling_contents(spelling, self.registry)
                .map(|contents| type_of_read_contents(self, contents));
        }
        let symbol = self.source.symbol(spelling)?;
        let version = type_version(self.ssa, symbol, *self.uses.get(&symbol)?, self.types);
        (version != 0)
            .then(|| self.types.get(&(symbol, version)).cloned())
            .flatten()
    }

    /// Expression types are keyed by original occurrence, independently of
    /// base-name dependency labels and conflicting same-spelling reads.
    fn expression_variable_types(
        &self,
        expression: &ExprNode,
        expression_base: Option<u32>,
    ) -> HashMap<(String, u32, u32), TypeLattice> {
        expression
            .variable_nodes()
            .into_iter()
            .filter_map(|node| {
                let ExprNode::Var {
                    text, start, end, ..
                } = node
                else {
                    return None;
                };
                let value = self
                    .source
                    .read_expression_variable(node, expression_base)
                    .and_then(|read| {
                        let version =
                            type_version(self.ssa, read.symbol, read.version?, self.types);
                        self.types.get(&(read.symbol, version)).cloned()
                    })
                    .or_else(|| {
                        self.source
                            .read_expression_variable_contents(node, expression_base, self.registry)
                            .map(|contents| type_of_read_contents(self, contents))
                    })?;
                Some(((text.clone(), *start, *end), value))
            })
            .collect()
    }

    /// Written substitutions select their original read; decoded names never
    /// pass through a second substitution decoder.
    fn written_variable_type(
        &self,
        spelling: &str,
        retained: Option<&tcl_source::words::WordExpr>,
    ) -> Option<TypeLattice> {
        if let Some(word) = retained {
            return Some(retained_contents_type(self, word));
        }
        let mut selected = None;
        for word in self
            .tokens?
            .words()
            .iter()
            .filter(|word| written_variable_word_matches(word, spelling))
        {
            let value = retained_contents_type(self, word);
            if selected.as_ref().is_some_and(|previous| *previous != value) {
                return None;
            }
            selected = Some(value);
        }
        selected
    }

    /// Contents read by an admitted native named-cell update. The operand is
    /// an evaluated variable name, rather than a substitution from argv.
    fn named_cell_type(&self, name: &str) -> Option<TypeLattice> {
        let symbol = self.source.symbol(name)?;
        let version = type_version(self.ssa, symbol, *self.uses.get(&symbol)?, self.types);
        if version == 0 {
            return None;
        }
        self.types.get(&(symbol, version)).cloned()
    }

    fn named_cell_constant(&self, name: &str) -> Option<&LatticeValue> {
        let symbol = self.source.symbol(name)?;
        self.values.get(&(symbol, *self.uses.get(&symbol)?))
    }

    fn with_tokens<'b>(&'b self, tokens: &'b crate::ir::CommandTokens) -> WordTypingCtx<'b, S> {
        WordTypingCtx {
            preparations: self.preparations,
            math: self.math,
            tokens: Some(tokens),
            source: self.source,
            uses: self.uses,
            defs: self.defs,
            context: self.context,
            config: self.config,
            types: self.types,
            values: self.values,
            registry: self.registry,
            known_classes: self.known_classes,
            namespace: self.namespace,
            ssa: self.ssa,
            numbers: self.numbers,
            active_writes: self.active_writes,
            normal_results: self.normal_results,
        }
    }

    /// Constant contents from the original written substitution reads.
    fn constant_of_word(&self, spelling: &str) -> Option<&LatticeValue> {
        let mut selected = None;
        for word in self
            .tokens?
            .words()
            .iter()
            .filter(|word| written_variable_word_matches(word, spelling))
        {
            let read = self.source.read_word(word)?;
            let value = self.values.get(&(read.symbol, read.version?))?;
            if selected.is_some_and(|previous| previous != value) {
                return None;
            }
            selected = Some(value);
        }
        selected
    }
}

fn written_variable_word_matches(word: &tcl_source::words::WordExpr, spelling: &str) -> bool {
    word.sole_variable_substitution()
        .is_some_and(|(original, _)| original == spelling || word.legacy_text() == spelling)
}

/// Run type propagation with an explicitly supplied standalone command store.
/// Actual function/unit consumers use [`propagate_types_with_metadata_context`]
/// with their retained source metadata and configuration.
///
/// Returns a map from `(variable_name, ssa_version)` to inferred
/// `TypeLattice`. Values absent from the map are implicitly `Unknown`.
///
/// Every def of a name [`crate::ssa::SsaSourceView::externally_mutable_by`] considers
/// externally mutable — fully namespace-qualified, aliased via `global`/
/// `variable`/`upvar`/traced *within this function* ([`crate::var_observability::
/// analyse_var_observability`]), named by `extra_global_escaping` (the
/// whole-module `global`-declaration scan for the *top-level* unit — see
/// [`crate::var_observability::scan_module_global_names`]), or traced
/// *anywhere in the module* (`trace_facts`) — is forced `Overdefined` here,
/// unless the source owner retains the exact successful store's value in the
/// same physical cell after callbacks. That receipt types this definition;
/// later reads still require their own reached contents proof.
/// *anywhere in the module* (`trace_facts`), or written by a callback script
/// of the module — is forced `Overdefined` here,
/// reusing the exact predicate [`crate::sccp::sccp_with_extra_escaping`] and
/// [`crate::optimiser::propagation`]'s O102 load-forwarding already apply to
/// their own (separate) lattices, rather than re-deriving a third,
/// potentially-divergent notion of "externally mutable" for this one. A
/// `set`-only view of such a name's literal types is not sound: a callee's
/// `global NAME; set NAME …`, a top-level name no procedure declares
/// `global` itself but another's `global NAME` can still reach, or a write
/// trace's callback, can all change what the name actually holds — reporting
/// a shimmer purely from the visible literals could be a false positive (or
/// miss the real one). `extra_global_escaping` is empty and `trace_facts` is
/// [`crate::compilation_unit::ModuleTraceFacts::none()`] for the
/// module-context-free callers ([`Self`] unit tests, isolated single-function
/// rebuilds with no module to scan).
#[must_use]
pub fn propagate_types<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    extra_global_escaping: &HashSet<String, S>,
    trace_facts: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> HashMap<ValueKey, TypeLattice> {
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile)
        .map(Into::into);
    propagate_types_with_metadata_context(
        cfg,
        ssa,
        sccp,
        TypePropagationMetadata {
            registry,
            context,
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            numbers: numbers_of(registry),
        },
        known_classes,
        extra_global_escaping,
        trace_facts,
    )
}

/// Exact source metadata and lexical/numeric grammar used by the type solver.
#[derive(Clone, Copy)]
pub struct TypePropagationMetadata<'a> {
    /// Actual command store used by this graph.
    pub registry: &'a CommandRegistry,
    /// Supplied complete availability, independent of an executing interpreter.
    pub context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    /// Exact original source lexical and list policy.
    pub config: tcl_lexer::LexerConfig,
    /// Exact source numeric grammar.
    pub numbers: NumberSyntax,
}

impl<'a> TypePropagationMetadata<'a> {
    /// Borrow all typing premises from one genuine function input.
    /// Missing input, a foreign store or inconsistent source policy declines.
    #[must_use]
    pub fn for_function(
        function: &'a crate::compilation_unit::FunctionUnit,
        registry: &'a CommandRegistry,
    ) -> Option<Self> {
        let input = function.source_metadata_input()?;
        let context = function.invocation_metadata_context(registry)?;
        let config = function.source_lexer_config();
        if config.normalized() != input.lexer_config().normalized() {
            return None;
        }
        Some(Self {
            registry,
            context: Some(context),
            config,
            numbers: input.unit_profile().grammar.numbers,
        })
    }
}

fn seed_clobbered_types<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
) -> HashMap<ValueKey, TypeLattice> {
    let mut types: HashMap<ValueKey, TypeLattice> = HashMap::new();
    let caller_safe_factories = has_only_caller_safe_factories(cfg, metadata, known_classes);
    for (block, markers) in &ssa.value_clobbers {
        if sccp.executable_blocks.contains(block) {
            for versions in markers.values() {
                for (&symbol, &(_, fresh)) in versions {
                    if !caller_safe_factories {
                        types.insert((symbol, fresh), TypeLattice::overdefined());
                    }
                }
            }
        }
    }
    types
}

/// Infer source values under retained metadata without recreating availability.
#[must_use]
pub fn propagate_types_with_metadata_context<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
    extra_global_escaping: &HashSet<String, S>,
    trace_facts: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> HashMap<ValueKey, TypeLattice> {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return HashMap::new();
    };
    let preds = cfg.predecessors();
    let order = crate::sccp::cfg_order(cfg);
    let mut escaping = crate::var_observability::analyse_var_observability_with_metadata_context(
        cfg,
        registry,
        Some(context),
    )
    .escaping_var_names();
    if !extra_global_escaping.is_empty() {
        escaping.extend(extra_global_escaping.iter().cloned());
    }
    escaping.extend(trace_facts.traced_variables.iter().cloned());
    escaping.extend(trace_facts.deferred_writes.names.iter().cloned());
    // Constructor heads written `[Foo new]` inside this function resolve
    // relative names against the function's own namespace.
    let namespace = function_namespace(&cfg.name);
    // Every nested query borrows the same complete source availability and
    // lexical policy; neither is reconstructed from the registry profile.
    let ctx = StatementTypingCtx {
        preparations: &cfg.expression_preparations,
        math: &cfg.implicit_math_invocations,
        ssa,
        context: Some(context),
        config: metadata.config,
        registry,
        known_classes,
        namespace: &namespace,
        values: &sccp.values,
        folded: &sccp.folded_types,
        escaping: &escaping,
        has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace
            || trace_facts.deferred_writes.any,
        numbers: metadata.numbers,
    };

    let mut types = seed_clobbered_types(cfg, ssa, sccp, metadata, known_classes);
    let mut changed = true;
    while changed {
        changed = false;
        for bn in &order {
            if !sccp.executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };

            // Phi nodes at non-entry blocks.
            if *bn != cfg.entry {
                let mut exec_preds: Vec<BlockId> = preds.get(bn).map_or_default(|ps| {
                    ps.iter()
                        .filter(|p| sccp.executable_edges.contains(&(**p, *bn)))
                        .copied()
                        .collect()
                });
                // `predecessors()` yields a `HashSet`, so the fold order below is
                // nondeterministic — and `type_join` is *not* order-independent
                // for a 3+-way shimmer merge (it records only a `(from, to)`
                // pair), so an unsorted fold makes the S101 message name
                // different types run-to-run.  Sort for a stable join order.
                exec_preds.sort_unstable();

                for phi in &ssa_block.phis {
                    if exec_preds.is_empty() {
                        continue;
                    }
                    let mut phi_type = TypeLattice::unknown();
                    for pred in &exec_preds {
                        let ver = type_version(
                            ssa,
                            phi.name,
                            phi.incoming.get(pred).copied().unwrap_or(0),
                            &types,
                        );
                        // A version-0 incoming is the entry / live-in root (a
                        // proc parameter, global, or other caller-supplied
                        // value). Its runtime type is unknown at compile time,
                        // so it joins in as OVERDEFINED — never skipped.
                        // Skipping it would let a phi that merges a live-in
                        // with a defined-arm type collapse to that arm's type,
                        // so a conditionally-assigned parameter
                        // (`proc p {c x} { if {$c} { set x 5 } … $x }`) would
                        // be typed solely from the assigned arm and the S101 /
                        // W307 / W308 consumers would report facts false for
                        // the not-taken path. Mirrors `sccp_process_phis`.
                        let t = if ver == 0 {
                            TypeLattice::overdefined()
                        } else {
                            types
                                .get(&(phi.name, ver))
                                .cloned()
                                .unwrap_or_else(TypeLattice::unknown)
                        };
                        phi_type = type_join(&phi_type, &t);
                    }
                    let key = (phi.name, phi.version);
                    let old = types
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(TypeLattice::unknown);
                    let merged = type_join(&old, &phi_type);
                    if merged != old {
                        types.insert(key, merged);
                        changed = true;
                    }
                }
            }

            // Statements, under the type refinements in force at the block.
            if type_refined_statements(&mut types, (sccp, *bn), ssa_block, &ctx) {
                changed = true;
            }
        }
    }

    types
}

/// Type one block's statements under the type refinements in force there
/// ([`types_at`]): a version a `string is` test or a numeric `==` proved a
/// type of reads as that type in the block, and as its own past it. Whether
/// a type moved.
fn type_refined_statements<S: std::hash::BuildHasher>(
    types: &mut HashMap<ValueKey, TypeLattice>,
    (sccp, block): (&SccpResult, BlockId),
    ssa_block: &crate::ssa::SsaBlock,
    ctx: &StatementTypingCtx<'_, S>,
) -> bool {
    let narrowed: Vec<(ValueKey, Option<TypeLattice>)> = types_at(types, sccp, block)
        .into_iter()
        .map(|(key, ty)| (key, types.insert(key, ty)))
        .collect();
    let changed = type_infer_process_statements(types, block, ssa_block, ctx);
    for (key, before) in narrowed {
        match before {
            Some(ty) => {
                types.insert(key, ty);
            }
            None => {
                types.remove(&key);
            }
        }
    }
    changed
}

/// The type each version a type refinement in force at `block` names holds
/// there ([`SccpResult::refinements_in`]): the type the test proved, or the
/// version's own where that is the proved one or narrower
/// ([`refined_type`]), the refinements of one version applied in turn. The
/// test's own conversion is why a type holds: `string is integer -strict
/// $x` and `$x == 1` leave `x` with a numeric representation whatever it
/// had.
fn types_at(
    types: &HashMap<ValueKey, TypeLattice>,
    sccp: &SccpResult,
    block: BlockId,
) -> HashMap<ValueKey, TypeLattice> {
    let mut narrowed: HashMap<ValueKey, TypeLattice> = HashMap::new();
    for refinement in sccp.refinements_in(block) {
        let tcl_registry::value_transfer::FactView::Domain(
            tcl_registry::value_transfer::DomainFact::Type {
                intrep: Some(proved),
                ..
            },
        ) = &refinement.fact
        else {
            continue;
        };
        // A version-0 key is what the frame entered with, a parameter's
        // argument among them, which no definition types: the live-in
        // root, as a φ reads it.
        let own = narrowed
            .get(&refinement.key)
            .or_else(|| types.get(&refinement.key))
            .cloned()
            .unwrap_or_else(|| {
                if refinement.key.1 == 0 {
                    TypeLattice::overdefined()
                } else {
                    TypeLattice::unknown()
                }
            });
        let refined = refined_type(&own, *proved);
        if refined != own {
            narrowed.insert(refinement.key, refined);
        }
    }
    narrowed
}

/// Per block, the type each version a type refinement in force there names
/// holds there ([`types_at`]), over the function's settled `types`: what a
/// use in the block reads, where it differs from the version's own type.
#[must_use]
pub(crate) fn types_in_force(
    types: &HashMap<ValueKey, TypeLattice>,
    sccp: &SccpResult,
) -> HashMap<(BlockId, ValueKey), TypeLattice> {
    sccp.refinements_at
        .keys()
        .flat_map(|&block| {
            types_at(types, sccp, block)
                .into_iter()
                .map(move |(key, ty)| ((block, key), ty))
        })
        .collect()
}

/// What a version typed `own` is typed where a refinement proved `proved`:
/// its own type where that is the proved one or narrower (an integer where
/// a number is proved), the proved type otherwise, and the optimistic
/// bottom for a version not typed yet.
fn refined_type(own: &TypeLattice, proved: TclType) -> TypeLattice {
    let within = |ty: TclType| {
        ty == proved || (proved == TclType::Numeric && matches!(ty, TclType::Int | TclType::Double))
    };
    match own.kind() {
        TypeKind::Unknown => own.clone(),
        TypeKind::Known if own.tcl_type().is_some_and(within) => own.clone(),
        _ => TypeLattice::of(proved),
    }
}

/// Shared, read-only context for [`type_infer_process_statements`].
struct StatementTypingCtx<'a, S: std::hash::BuildHasher> {
    preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    /// Reached math-function dispatches of the same source inventory.
    math: &'a [crate::command_binding::SourceMathInvocation],
    ssa: &'a SsaFunction,
    /// Same retained metadata as each original word query.
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    /// SCCP constants — purity evidence and constant list/index values for
    /// the element-inference helpers (see [`WordTypingCtx::values`]).
    values: &'a HashMap<ValueKey, LatticeValue>,
    /// Types produced by the selected shared route, independent of source-name lookup.
    folded: &'a HashMap<ValueKey, crate::value_transfer::FoldedType>,
    /// Names [`crate::ssa::SsaSourceView::externally_mutable_by`] should treat as
    /// unconditionally aliased/escaping (per-function `analyse_var_observability`
    /// union'd with the caller's whole-module `extra_global_escaping` and
    /// `trace_facts.traced_variables`).
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
    /// The actual source's numeric-literal grammar.
    numbers: NumberSyntax,
}

/// Evaluate each statement's defs for one block, forcing every def of a name
/// [`crate::ssa::SsaSourceView::externally_mutable_by`] considers externally mutable to
/// `Overdefined` unless its exact normal-store contents receipt survives.
/// Returns `true` if any lattice value changed. Extracted
/// from [`propagate_types`], mirroring [`crate::sccp::sccp_process_statements`]'s
/// shape for the (separate) constant-folding lattice.
fn type_infer_process_statements<S: std::hash::BuildHasher>(
    types: &mut HashMap<ValueKey, TypeLattice>,
    block: BlockId,
    ssa_block: &crate::ssa::SsaBlock,
    ctx: &StatementTypingCtx<'_, S>,
) -> bool {
    let mut changed = false;
    for (index, ssa_stmt) in ssa_block.statements.iter().enumerate() {
        let stmt = &ssa_stmt.statement;
        let source = crate::ssa::SsaSourceView::at_statement(ctx.ssa, block, index);
        // A barrier widens every def to OVERDEFINED (it may have mutated
        // them arbitrarily); a scope-alias declaration (`global`/`variable`/
        // `upvar`/`namespace upvar`) likewise widens its defs — the
        // imported variable's intrep is external and unknown.
        let inferred = match stmt {
            Statement::Barrier { .. } | Statement::NativeCall { .. } => {
                DefTyping::Uniform(TypeLattice::overdefined())
            }
            Statement::Call { defs, .. }
                if !defs.is_empty()
                    && original_scope_alias_call(ctx.registry, ctx.context, stmt)
                        != Some(false) =>
            {
                DefTyping::Uniform(TypeLattice::overdefined())
            }
            _ => {
                let word_ctx = WordTypingCtx {
                    preparations: ctx.preparations,
                    math: ctx.math,
                    tokens: stmt.tokens(),
                    source,
                    uses: &ssa_stmt.uses,
                    defs: Some(&ssa_stmt.defs),
                    context: ctx.context,
                    config: ctx.config,
                    types,
                    values: ctx.values,
                    registry: ctx.registry,
                    known_classes: ctx.known_classes,
                    namespace: ctx.namespace,
                    ssa: ctx.ssa,
                    numbers: ctx.numbers,
                    active_writes: &[],
                    normal_results: None,
                };
                evaluate_type_def(stmt, &word_ctx)
            }
        };
        for (&var, &ver) in &ssa_stmt.defs {
            let key = (var, ver);
            let old = types
                .get(&key)
                .cloned()
                .unwrap_or_else(TypeLattice::unknown);
            let cell = ctx.ssa.cell_key(var);
            let externally_mutable = source.externally_mutable_by(
                var,
                ctx.escaping,
                ctx.has_dynamic_variable_trace,
                ctx.registry,
            ) != Some(false);
            let def_type = if cell.is_none()
                || ssa_stmt.destruction_defs.contains(&var)
                || (externally_mutable
                    && !source.normal_store_contents_preserved(var, ctx.registry))
                || ctx.ssa.is_array_root_refresh_version(var, ver)
            {
                TypeLattice::overdefined()
            } else if ssa_stmt.may_defs.contains(&var) {
                // A synthetic may-def: the write may or may not have hit
                // this element, so its type is the JOIN of the prior
                // version's type (recorded as a use) and the written type
                // — `set arr($k) 9` over an INT `arr(a)` stays INT, over a
                // STRING one widens. No prior use (a base refresh from a
                // Call def) → Overdefined.
                match ssa_stmt.uses.get(&var) {
                    Some(prev_ver) => {
                        let prev = types
                            .get(&(var, *prev_ver))
                            .cloned()
                            .unwrap_or_else(TypeLattice::overdefined);
                        let written = match &inferred {
                            DefTyping::Uniform(t) => t.clone(),
                            DefTyping::PerDef(map) => cell
                                .and_then(|cell| map.get(cell))
                                .cloned()
                                .unwrap_or_else(TypeLattice::overdefined),
                        };
                        type_join(&prev, &written)
                    }
                    None => TypeLattice::overdefined(),
                }
            } else {
                let inferred = match &inferred {
                    DefTyping::Uniform(t) => t.clone(),
                    // Positional element typing: a def the map does not
                    // name widens to Overdefined.
                    DefTyping::PerDef(map) => cell
                        .and_then(|cell| map.get(cell))
                        .cloned()
                        .unwrap_or_else(TypeLattice::overdefined),
                };
                refined_by_folded_type(inferred, ctx.folded.get(&key))
            };
            let merged = type_join(&old, &def_type);
            if merged != old {
                types.insert(key, merged);
                changed = true;
            }
        }
    }
    changed
}

/// A definition's static type, refined by the folded type its evaluation
/// states: where the static typing knows nothing — a command with no
/// declared return type, a destructured target — the type the evaluation
/// proved for this call (its stated intrep, or the one the route
/// constructed) is the definition's. A known static type stands: it can
/// carry element facts the folded type does not, and the evaluation's own
/// type is the same fact for every shipped route.
fn refined_by_folded_type(
    inferred: TypeLattice,
    folded: Option<&crate::value_transfer::FoldedType>,
) -> TypeLattice {
    if inferred.kind() == TypeKind::Known {
        return inferred;
    }
    let proved = folded.and_then(|folded| {
        folded.intrep.or(match folded.representation {
            tcl_registry::value_transfer::RepresentationEvidence::Constructed(built) => Some(built),
            tcl_registry::value_transfer::RepresentationEvidence::Unknown => None,
        })
    });
    proved.map_or(inferred, TypeLattice::of)
}

/// Infer a function's overall return type by joining the result types
/// of every executable exit — explicit `Return` terminators *and*
/// fall-through exits.
///
/// `types` is the [`propagate_types`] result for the same function.
/// Value returns use the terminator's reaching SSA versions and retained
/// source invocation context. Expression-only returns currently join each
/// name's known versions conservatively.
///
/// A reachable block with no terminator is a *fall-through* exit:
/// control runs off the end of the body and Tcl returns the result of
/// the last command executed (e.g. the empty string of an
/// else-less `if`, or `set`'s value).  That result is not modelled
/// here, so a fall-through contributes `Overdefined` to the join
/// rather than being skipped — without this, a partial-return proc
/// like `if {$c} { return 1 }` would report an overconfident `Int`.
/// Returns `Unknown` only when the function has no executable exit at
/// all.
#[must_use]
pub(crate) fn infer_function_return_type<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    sccp: &SccpResult,
    types: &HashMap<ValueKey, TypeLattice>,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
    ssa: &SsaFunction,
) -> TypeLattice {
    infer_function_return_type_with_results(cfg, sccp, types, metadata, known_classes, ssa, None)
}

/// Conditional normal result inference with independently retained callee body
/// types. Only the actual positioned handler allocation can select a result.
pub(crate) fn infer_function_return_type_with_results<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    sccp: &SccpResult,
    types: &HashMap<ValueKey, TypeLattice>,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
    ssa: &SsaFunction,
    normal_results: Option<&NormalProcedureResultTypes>,
) -> TypeLattice {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return TypeLattice::unknown();
    };
    let namespace = function_namespace(&cfg.name);
    let numbers = metadata.numbers;
    let mut result: Option<TypeLattice> = None;
    for (bn, block) in &cfg.blocks {
        if !sccp.executable_blocks.contains(bn) {
            continue;
        }
        let t = match &block.terminator {
            Some(Terminator::Return {
                value,
                value_word,
                tokens,
                expr,
                expr_base,
                ..
            }) => {
                let Some(ssa_block) = ssa.blocks.get(bn) else {
                    continue;
                };
                let context = WordTypingCtx {
                    preparations: &cfg.expression_preparations,
                    math: &cfg.implicit_math_invocations,
                    tokens: tokens.as_deref(),
                    source: crate::ssa::SsaSourceView::at_terminator(ssa, *bn),
                    uses: &ssa_block.exit_versions,
                    defs: None,
                    context: Some(context),
                    config: metadata.config,
                    types,
                    values: &sccp.values,
                    registry,
                    known_classes,
                    namespace: &namespace,
                    ssa,
                    numbers,
                    active_writes: &[],
                    normal_results,
                };
                if let Some(expr) = expr {
                    lowered_expression_result_type(&context, tokens.as_deref(), expr, *expr_base)
                } else if let Some(value) = value {
                    value_word_type(&context, value, value_word.as_ref())
                } else {
                    // Bare `return` yields the empty string.
                    TypeLattice::of(TclType::String)
                }
            }
            // Fall-through exit (last-command result, not modelled).
            None => TypeLattice::overdefined(),
            // `Goto` / `Branch` have successors — not exits.
            Some(_) => continue,
        };
        result = Some(match result {
            Some(acc) => type_join(&acc, &t),
            None => t,
        });
    }

    result.unwrap_or_else(TypeLattice::unknown)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reported_namespace_tests;

#[cfg(test)]
mod scope_alias_metadata_tests;
