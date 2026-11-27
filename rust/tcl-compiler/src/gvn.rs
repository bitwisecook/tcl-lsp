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

//! Global Value Numbering (GVN).
//!
//! Detects redundant computations by canonicalising pure expression
//! invocations to SSA-qualified [`ExprKey`] tuples and looking them
//! up in a dominator-tree-scoped hash table. A match means the same
//! expression was computed at a dominating definition; the new
//! occurrence can be replaced with the earlier result.

use crate::command_binding::SourceOriginLexical;
use std::collections::HashMap;
use tcl_core_types::DiagCode;

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, ResultStability, StateTransitionKnowledge, Traits};

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::ir::Statement;
use crate::side_effects::{EffectRegion, classify_side_effects};
use crate::ssa::{SsaFunction, SsaStatement};
use tcl_source::words::NodeId;
use tcl_source::words::Provenance;
use tcl_source::words::SourceSite;

// Expression-key alias

/// Canonical identity for a computed expression.
///
/// A call to `cmd arg1 arg2 …` becomes `["call", "cmd", arg1, arg2, …]`
/// after variable references have been rewritten to their SSA-
/// versioned form (see `canonicalise_word`). Two occurrences
/// that produce the same `ExprKey` are known to compute the same
/// value under the current SSA.
pub type ExprKey = Vec<String>;

// Redundant-computation diagnostic

/// A computation that re-evaluates an already-available expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RedundantComputation {
    /// Span of the duplicate computation.
    pub span: Span,
    /// Span of the first computation that produced the same value.
    pub first_span: Span,
    /// Human-readable expression text for diagnostic messages.
    pub expression_text: String,
    /// Diagnostic code: `"O105"` for full **and** partial redundancy
    /// (GVN/CSE + GVN-PRE), `"O106"` for loop-invariant code motion.
    /// Matches the canonical KCS codes (full /
    /// partial both default to O105; loop-invariant sets O106).
    /// `O107` is *unreachable dead code* and is owned by the
    /// elimination pass, never emitted here.
    pub code: DiagCode,
    /// Formatted diagnostic message.
    pub message: String,
}

impl RedundantComputation {
    /// Minimal constructor used by the driver and tests.
    #[must_use]
    pub fn new(
        span: Span,
        first_span: Span,
        expression_text: impl Into<String>,
        code: DiagCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            span,
            first_span,
            expression_text: expression_text.into(),
            code,
            message: message.into(),
        }
    }
}

// Value-table entries

/// A single entry in a [`ScopedValueTable`] scope. Carries the
/// block / statement coordinates and the rendered expression text
/// so later occurrences can point back at where the value was
/// first computed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueEntry {
    /// Canonical key (same as the map key; stored here for ease of
    /// programmatic inspection).
    pub key: ExprKey,
    /// CFG block containing the first computation.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Source span of the first computation.
    pub span: Span,
    /// Rendered expression text (`cmd arg1 arg2 …`).
    pub expression_text: String,
}

/// One pure-expression occurrence observed in a statement stream.
///
/// Produced by the per-statement collector and consumed by
/// the fixed-point walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprOccurrence {
    /// Canonical key.
    pub key: ExprKey,
    /// Source span.
    pub span: Span,
    /// Rendered expression text.
    pub expression_text: String,
    /// CFG block containing the occurrence.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Variable names referenced by the expression (for loop-
    /// invariance detection).
    pub variable_uses: Vec<String>,
}

// Dominator-tree-scoped value table

/// Stack of `ExprKey → ValueEntry` maps, one per scope.
///
/// The outermost scope always exists (index 0). Each
/// `push_scope` / `pop_scope` pair brackets the processing of a
/// dominator-tree subtree so that entries introduced on one path
/// don't leak into its siblings. Lookups search from the innermost
/// scope outward; `kill_all` discards everything (used on barrier
/// or impure-call statements).
#[derive(Debug, Default)]
pub struct ScopedValueTable {
    scopes: Vec<FxHashMap<ExprKey, ValueEntry>>,
}

impl ScopedValueTable {
    /// Build a table with a single empty root scope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: vec![FxHashMap::default()],
        }
    }

    /// Push a new empty scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(FxHashMap::default());
    }

    /// Pop the innermost scope, keeping the root scope in place.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Look `key` up from the innermost scope outward.
    #[must_use]
    pub fn lookup(&self, key: &ExprKey) -> Option<&ValueEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.get(key) {
                return Some(entry);
            }
        }
        None
    }

    /// Insert an entry in the innermost scope. An existing entry
    /// at the same key is replaced.
    pub fn insert(&mut self, entry: ValueEntry) {
        let key = entry.key.clone();
        self.scopes
            .last_mut()
            .expect("root scope present")
            .insert(key, entry);
    }

    /// Drop every tracked entry. Used on barrier / impure-call
    /// statements where no previously-tracked value can be trusted.
    ///
    /// Clears each scope's entries *in place* while preserving the scope
    /// stack's depth. Collapsing the stack to a single root (the previous
    /// behaviour) breaks the push/pop discipline: a subsequent insert would
    /// land in that lone root scope, which `pop_scope` never removes (its
    /// `len > 1` guard), so an occurrence recorded on one dominator-tree path
    /// after a kill would leak into a sibling path and mis-fire O105
    /// ("computed again with the same arguments" on a path that never computed
    /// it). Keeping the depth intact means post-kill entries are still dropped
    /// at their own scope's `pop_scope`.
    pub fn kill_all(&mut self) {
        for scope in &mut self.scopes {
            scope.clear();
        }
    }

    /// Number of scopes currently on the stack. Primarily exposed
    /// for tests.
    #[must_use]
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Total entries across all scopes. Primarily exposed for
    /// tests — the driver does not use this.
    #[must_use]
    pub fn total_entries(&self) -> usize {
        self.scopes.iter().map(HashMap::len).sum()
    }
}

// Canonicalisation helpers

/// Render the variable versions of a standalone substitution template for
/// display. Production value equality uses the original grouped words, their
/// lexical parts and separate SSA version fields.
#[must_use]
pub fn canonicalise_word<S: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<crate::ssa::Symbol, u32, S>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> String {
    use std::fmt::Write as _;
    use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags};
    if uses.is_empty() {
        return text.to_owned();
    }
    let Ok(end) = u32::try_from(text.len()) else {
        return text.to_owned();
    };
    let Ok(arena) = ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(text.as_bytes()),
        tcl_lexer::Span::new(0, end),
        SubstFlags::default(),
        config,
    ) else {
        return text.to_owned();
    };
    let mut variables = Vec::new();
    let mut pending = vec![arena.root()];
    while let Some(list) = pending.pop() {
        for component in arena.list(list) {
            let ExecutablePart::Variable { name, index } = component.part else {
                continue;
            };
            pending.extend(index);
            let spelling = &text[name.as_range()];
            if let Some(version) = ssa
                .var_symbol(spelling)
                .and_then(|symbol| uses.get(&symbol))
            {
                let end = if index.is_some() {
                    name.end()
                } else {
                    component.span.end()
                };
                variables.push((
                    component.span.start() as usize,
                    end as usize,
                    spelling,
                    version,
                ));
            }
        }
    }
    variables.sort_unstable_by_key(|&(start, ..)| start);
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, name, version) in variables {
        output.push_str(&text[cursor..start]);
        let _ = write!(output, "${name}@{version}");
        cursor = end;
    }
    output.push_str(&text[cursor..]);
    output
}

/// Value equality for an original grouped invocation. An SSA display spelling
/// can coincide with literal source bytes, so byte strings and version numbers
/// occupy distinct fields. A nested evaluation requires its own effect proof.
fn original_call_key(
    command: &str,
    statement: &SsaStatement,
    ssa: &SsaFunction,
    world: &[String],
) -> Option<ExprKey> {
    use tcl_lexer::word_parts::ExecutablePart;
    let tokens = statement.statement.tokens()?;
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let site = binding.invocation_site()?;
    let original = site
        .source
        .original_lexical_command(tokens.words(), site.offset, config)?;
    let words = original.native_words();
    let mut key = vec![
        "call".to_owned(),
        command.to_owned(),
        words.len().to_string(),
    ];
    for word in words {
        if word.group().expand {
            return None;
        }
        let arena = word.executable_parts();
        let mut reads = Vec::new();
        for component in arena.list(arena.root()) {
            match component.part {
                ExecutablePart::Text(_) => {}
                ExecutablePart::Variable { name, index: None } => {
                    let name =
                        std::str::from_utf8(word.image().bytes().get(name.as_range())?).ok()?;
                    let symbol = ssa.var_symbol(name)?;
                    reads.push(statement.uses.get(&symbol)?);
                }
                ExecutablePart::Variable { index: Some(_), .. }
                | ExecutablePart::Command { .. }
                | ExecutablePart::Expression { .. }
                | ExecutablePart::ParseError(_) => return None,
            }
        }
        key.push(std::str::from_utf8(word.written_bytes()).ok()?.to_owned());
        key.push(reads.len().to_string());
        key.extend(reads.into_iter().map(u32::to_string));
    }
    key.extend_from_slice(world);
    Some(key)
}

/// Build the canonical [`ExprKey`] for a pure-command invocation:
/// `["call", command, canonicalised_arg1, canonicalised_arg2, …]`.
#[must_use]
pub fn build_call_key<S: std::hash::BuildHasher>(
    command: &str,
    args: &[String],
    uses: &HashMap<crate::ssa::Symbol, u32, S>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> ExprKey {
    let mut parts: ExprKey = Vec::with_capacity(2 + args.len());
    parts.push("call".into());
    parts.push(command.to_owned());
    for arg in args {
        parts.push(canonicalise_word(arg, uses, ssa, config));
    }
    parts
}

/// Render a command invocation as human-readable text for
/// diagnostic messages.
#[must_use]
pub fn format_expression_text(command: &str, args: &[String]) -> String {
    if args.is_empty() {
        return command.to_owned();
    }
    let mut out =
        String::with_capacity(command.len() + args.iter().map(|s| s.len() + 1).sum::<usize>());
    out.push_str(command);
    out.push(' ');
    out.push_str(&args.join(" "));
    out
}

// Diagnostic messages

/// Message shown when a pure expression is computed twice on the
/// same control-flow path.
#[must_use]
pub fn full_redundancy_message(expression_text: &str) -> String {
    format!(
        "'{expression_text}' computed again with the same arguments. \
        Consider storing the result in a local variable."
    )
}

/// Message shown when a pure expression is computed on some but
/// not all paths into a merge point.
#[must_use]
pub fn partial_redundancy_message(expression_text: &str) -> String {
    format!(
        "'{expression_text}' is partially redundant across control-flow \
        paths. Consider hoisting it before the branch."
    )
}

/// Message shown when a pure expression is loop-invariant.
#[must_use]
pub fn loop_invariant_message(expression_text: &str) -> String {
    format!(
        "'{expression_text}' is loop-invariant and re-computed on each \
        iteration. Consider hoisting it before the loop."
    )
}

// Statement-level helpers

/// Return `true` if the command invocation is pure for GVN
/// purposes — i.e. has no observable side effects.
///
/// Bridges to [`classify_side_effects`]. An optional
/// dialect (`Some(tcl_dialect::DialectProfile::irules())` /
/// `Some(DialectProfile::plain_tcl())`) threads through to
/// the classifier.
#[must_use]
pub fn is_pure_command(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    if !classify_side_effects(registry, command, args, dialect, None).pure {
        return false;
    }
    // O106: an outer pure command whose args read from
    // an impure command substitution is NOT loop-invariant — its value
    // depends on the inner side effect's per-call return (e.g.
    // `expr [incr counter]`). Recurse into every nested `[…]` and
    // require each to be pure too.
    arg_substitutions_pure(
        args,
        tcl_lexer::LexerConfig::for_profile(dialect),
        |c, a| classify_side_effects(registry, c, a, dialect, None).pure,
    )
}

/// Scan `args` for embedded `[cmd …]` command substitutions and return
/// `true` iff every nested command satisfies `check`. Braced regions
/// are skipped (no substitution inside `{…}`). Shared by the purity
/// gates so an outer pure command wrapping an impure inner subst is
/// rejected as loop-invariant (O106).
fn arg_substitutions_pure(
    args: &[String],
    config: tcl_lexer::LexerConfig,
    check: impl Fn(&str, &[String]) -> bool,
) -> bool {
    args.iter().all(|arg| {
        !arg.contains('[')
            || scan_bracketed_commands_with_config(arg, config)
                .is_some_and(|commands| commands.iter().all(|(cmd, inner)| check(cmd, inner)))
    })
}

/// Trace-aware purity gate.
///
/// Same purity classification as [`is_pure_command`] but additionally
/// returns `false` for any command targeted by an active execution
/// trace (`trace add execution NAME enter|leave HANDLER`) AND for
/// every command when [`crate::ir::Module::has_dynamic_trace`] is set.
///
/// Calls to a command with an active execution trace are never pure
/// because the trace body composes side-effects in.
/// GVN, partial-redundancy, and loop-invariant passes thread the
/// module's trace facts through this gate so optimisations don't
/// silently delete a second invocation that the trace re-armed.
#[must_use]
pub fn is_pure_command_with_traces(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    traced_commands: &std::collections::BTreeSet<String>,
    has_dynamic_trace: bool,
) -> bool {
    // This compatibility facade receives an explicitly authored written name;
    // production original-token gates below consume selected descriptor keys.
    !crate::command_binding::trace_registrations::command_may_be_traced(
        traced_commands,
        has_dynamic_trace,
        command,
    ) && is_pure_command(registry, command, args, dialect)
}

/// Purity of this original selected invocation and its exact argument inventory.
/// Complete availability and closed original argument evaluation are separate
/// premises. Missing metadata or child receipts cannot establish pure effects.
pub(crate) fn is_pure_tokens_with_metadata_context(
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
    traced_commands: &std::collections::BTreeSet<String>,
    has_dynamic_trace: bool,
) -> bool {
    let Some(metadata) = metadata.filter(|context| context.matches_registry(registry)) else {
        return false;
    };
    let Some(invocation) =
        crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )
    else {
        return false;
    };
    let facts = &invocation.facts;
    if has_dynamic_trace
        || crate::registry_invocation::original_invocation_has_execution_trace(
            registry,
            metadata,
            tokens,
            facts.descriptor_key(),
            traced_commands,
        )
        || !facts.is_pure()
        || !facts.effects.accesses().is_empty()
        || facts.effects.requires_world_barrier()
        || facts.effects.legacy().command_table_mutation
        || !facts.effects.legacy().frame_effects.is_empty()
        || !facts.effects.legacy().side_effects.is_empty()
        || !matches!(&facts.state_transitions, StateTransitionKnowledge::Declared(transitions) if transitions.facts().is_empty())
    {
        return false;
    }
    let Some(calls) = crate::word_subst::checked_lifted_calls(
        tokens,
        tokens.native_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile())),
    ) else {
        return false;
    };
    // This shared gate requires every original variable read and nested result
    // to have closed evaluation. It does not promote a nominal pure label.
    crate::const_subst::ConstSubstCtx {
        registry,
        resolution_namespace: "::",
        namespace_context: None,
        version: None,
        defining_class: None,
        trusts: &|_| false,
        lookup_var: &|_| None,
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
    }
    .retained_token_arguments_with_metadata_context(
        tokens,
        crate::const_subst::RetainedCalls::new(&calls)
            .with_traces(traced_commands, has_dynamic_trace),
        metadata,
    )
    .is_some()
}

/// Return `true` if a redundant use of `command` is worth
/// flagging. Built-ins marked `CSE_CANDIDATE` qualify; user-proc
/// redundancy (interprocedural) is not flagged.
#[must_use]
pub fn is_worth_reporting(registry: &CommandRegistry, command: &str) -> bool {
    if let Some(spec) = registry.get(command) {
        return spec.traits.contains(Traits::CSE_CANDIDATE);
    }
    false
}

/// Exact source-span key used only while the legacy CFG/SSA surface does not
/// retain semantic [`NodeId`]s.
///
/// The semantic executable IR itself is keyed by a stable node identity.  This
/// compatibility bridge records that identity, then permits an exact source
/// span lookup only when it selects one unambiguous source-provenance node.
/// A transformed, opaque, or ambiguous site fails closed instead of guessing
/// which registry invocation supplied its facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SemanticSourceKey {
    start: u32,
    end: u32,
}

impl From<tcl_lexer::Span> for SemanticSourceKey {
    fn from(span: tcl_lexer::Span) -> Self {
        Self {
            start: span.start(),
            end: span.end(),
        }
    }
}

/// One semantic invocation projected into GVN's narrow legality question.
///
/// `node` remains the authoritative identity for the executable semantic
/// graph.  The current legacy SSA has no node field, so [`GvnSemanticFacts`]
/// uses `source` solely as an exact, ambiguity-checked compatibility lookup.
#[derive(Debug, Clone)]
struct GvnSemanticSite {
    node: NodeId,
    source: SourceSite,
    eligibility: GvnSiteEligibility,
}

/// Whether the semantic registry facts permit GVN to reuse a direct call.
#[derive(Debug, Clone, PartialEq, Eq)]
enum GvnSiteEligibility {
    /// A closed registry proof plus a complete live-dispatch site proof
    /// allow a stable CSE candidate at this site.
    Eligible {
        canonical_command: String,
        /// World-state version fragments the expression key must include:
        /// the site's dispatch-dependency tracks plus any registry-declared
        /// versioned-world result domains.
        world_key: Vec<String>,
    },
    /// The site is known but any missing proof makes GVN abstain.
    Declined,
}

/// Result of matching one legacy statement to executable semantic facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GvnSemanticLookup<'a> {
    /// The source-bearing semantic sidecar has no matching invocation.
    Unmapped,
    /// A uniquely matched invocation is eligible, supplying its canonical
    /// registry command identity and world-version key fragments.
    Eligible {
        command: &'a str,
        world_key: &'a [String],
    },
    /// The source span was mapped, but was ambiguous or lacks a complete
    /// semantic/world proof. GVN must not use the legacy classifier here.
    Declined,
}

/// Authority used to classify GVN occurrences.
///
/// The public CFG/SSA API remains explicitly legacy. Production `FunctionUnit`
/// analysis uses only exact common-semantic facts and fails closed when the
/// sidecar or an individual source-site mapping is unavailable.
#[derive(Debug, Clone, Copy)]
enum GvnLegalitySource<'a> {
    Legacy,
    Common(Option<&'a GvnSemanticFacts>),
}

/// Target-neutral GVN legality facts derived from one function's semantic
/// sidecar.
///
/// This is intentionally a consumer of [`crate::semantic_analysis::SemanticAnalysisBundle`],
/// not a second command resolver.  Its only source fallback is exact span
/// matching to a retained [`NodeId`] and direct source provenance; the bridge
/// can disappear once the scalar CFG owns the same node IDs.
#[derive(Debug, Clone, Default)]
struct GvnSemanticFacts {
    sites: FxHashMap<SemanticSourceKey, Vec<GvnSemanticSite>>,
}

impl GvnSemanticFacts {
    /// Build legality facts when a source-bearing executable sidecar exists.
    ///
    /// `None` means the function could not expose executable source sites (no
    /// source retained, incompatible source shape, or no selected dialect).
    /// Production callers must treat that as unavailable proof, never as
    /// permission to invoke the legacy command-string classifier.
    #[must_use]
    fn from_function(
        function: &crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let availability = function.semantic_facts.executable();
        let executable = availability.function()?;
        // The dispatch-stability contents analysis is meaningful only when
        // the world-state graph itself was buildable; a world-state decline
        // fails every site closed exactly as before.
        let proofs = availability.world_state_ssa().is_some().then(|| {
            crate::dispatch_proof::analyse_reuse_dispatch_stability(
                executable,
                function.semantic_facts.dispatch_entry_assumption(),
                registry,
                function.invocation_metadata_context(registry),
            )
        });
        let mut facts = Self {
            sites: FxHashMap::default(),
        };
        for (block_index, block) in executable.blocks.iter().enumerate() {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                let crate::executable_ir::ExecutableInstruction::Invoke(invoke) = instruction
                else {
                    continue;
                };
                let key = SemanticSourceKey::from(invoke.source.span);
                let site_proof = proofs.as_ref().and_then(|proofs| {
                    proofs.site(
                        u32::try_from(block_index).unwrap_or(u32::MAX),
                        u32::try_from(instruction_index).unwrap_or(u32::MAX),
                    )
                });
                let eligibility = match &invoke.resolution {
                    crate::executable_ir::InvocationResolution::Resolved(resolved) => {
                        gvn_site_eligibility(resolved, site_proof)
                    }
                    crate::executable_ir::InvocationResolution::Unresolved(_) => {
                        GvnSiteEligibility::Declined
                    }
                };
                facts.sites.entry(key).or_default().push(GvnSemanticSite {
                    node: invoke.node.clone(),
                    source: invoke.source.clone(),
                    eligibility,
                });
            }
        }
        Some(facts)
    }

    /// Look up a direct legacy statement through the explicit compatibility
    /// boundary. Duplicate span matches are declined even if their text looks
    /// alike: node identity, not source spelling, is authoritative.
    #[must_use]
    fn lookup(&self, span: tcl_lexer::Span) -> GvnSemanticLookup<'_> {
        let Some(sites) = self.sites.get(&SemanticSourceKey::from(span)) else {
            return GvnSemanticLookup::Unmapped;
        };
        let [site] = sites.as_slice() else {
            return GvnSemanticLookup::Declined;
        };
        if !matches!(&site.source.provenance, Provenance::Source) || site.node.path().is_empty() {
            return GvnSemanticLookup::Declined;
        }
        match &site.eligibility {
            GvnSiteEligibility::Eligible {
                canonical_command,
                world_key,
            } => GvnSemanticLookup::Eligible {
                command: canonical_command,
                world_key,
            },
            GvnSiteEligibility::Declined => GvnSemanticLookup::Declined,
        }
    }
}

/// Static registry gates shared by every GVN candidate class.
///
/// An empty effect vector on its own is not enough: transition coverage can
/// remove a write from the ordinary footprint.  Reuse requires the explicit
/// pure/CSE registry traits, a closed and empty transition set, no direct
/// state accesses, and no callback barrier.  Result stability is judged by
/// the per-class predicates below; live dispatch stability is a separate
/// per-site proof ([`crate::dispatch_proof::SiteDispatchProof`]).
fn resolved_invocation_static_gvn_gates(resolved: &tcl_registry::InvocationFacts) -> bool {
    // Effect-free, not pure: a versioned-world result is effect-free with a
    // result that depends on the world; the referentially transparent gate
    // checks its own stability.
    if !(resolved.is_effect_free() && resolved.traits.contains(Traits::CSE_CANDIDATE)) {
        return false;
    }
    if !resolved.effects.accesses().is_empty() || resolved.effects.requires_world_barrier() {
        return false;
    }
    if resolved.effects.legacy().command_table_mutation
        || !resolved.effects.legacy().frame_effects.is_empty()
        || !resolved.effects.legacy().side_effects.is_empty()
    {
        return false;
    }
    matches!(
        &resolved.state_transitions,
        StateTransitionKnowledge::Declared(transitions) if transitions.facts().is_empty()
    )
}

/// Return whether one resolved registry invocation is a static GVN candidate
/// whose result depends only on its evaluated argument values.
pub(crate) fn resolved_invocation_is_gvn_candidate(
    resolved: &tcl_registry::InvocationFacts,
) -> bool {
    resolved.result_stability == ResultStability::ReferentiallyTransparent
        && resolved_invocation_static_gvn_gates(resolved)
}

/// Return whether one resolved registry invocation is a static GVN candidate
/// whose result additionally reads registry-declared versioned world-state
/// domains.
///
/// Such a call is reusable only when the expression key includes the
/// world-state versions of every declared domain — which the semantic path
/// supplies from the dispatch-stability analysis.  Volatile and undeclared
/// results are never candidates.
pub(crate) fn resolved_invocation_is_versioned_world_gvn_candidate(
    resolved: &tcl_registry::InvocationFacts,
) -> bool {
    matches!(
        resolved.result_stability,
        ResultStability::ReadsVersionedWorld(_)
    ) && resolved_invocation_static_gvn_gates(resolved)
}

/// Judge one resolved invocation against its live dispatch-stability proof.
///
/// Fails closed without a site proof (world-state graph unavailable, the
/// analysis abandoned, or the site unreached).  A complete proof must cover
/// every registry-declared dispatch dependency and admit every operand word;
/// the returned eligibility then carries the world-version key fragments for
/// the dependency tracks plus any versioned-world result domains.
fn gvn_site_eligibility(
    resolved: &tcl_registry::InvocationFacts,
    site: Option<&crate::dispatch_proof::SiteDispatchProof>,
) -> GvnSiteEligibility {
    let Some(site) = site else {
        return GvnSiteEligibility::Declined;
    };
    let result_reusable = resolved_invocation_is_gvn_candidate(resolved)
        || resolved_invocation_is_versioned_world_gvn_candidate(resolved);
    if !result_reusable || !site.satisfies(resolved.dispatch_dependencies) {
        return GvnSiteEligibility::Declined;
    }
    GvnSiteEligibility::Eligible {
        canonical_command: resolved.canonical_command.clone(),
        world_key: site.world_key(
            resolved.dispatch_dependencies,
            resolved.result_stability.versioned_world_dependencies(),
        ),
    }
}

/// Return `true` when the statement's effects must invalidate
/// value numbering.
///
/// [`Statement::Barrier`] always invalidates. [`Statement::Call`]
/// invalidates when its side-effect profile writes any
/// [`EffectRegion`] other than `NONE`. Other statements (pure
/// assigns, returns) do not invalidate.
#[must_use]
pub fn statement_writes_state(
    registry: &CommandRegistry,
    stmt: &Statement,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    match stmt {
        Statement::Barrier { .. } | Statement::NativeCall { .. } => true,
        Statement::Call { command, args, .. } => {
            let effect = classify_side_effects(registry, command, args, dialect, None);
            let (_reads, writes) = effect.to_effect_regions();
            writes != EffectRegion::NONE
        }
        _ => false,
    }
}

/// GVN's state-kill gate under an explicit legality source.
///
/// Common-semantic mode never reclassifies from command text. A denied,
/// unavailable, or unmapped call kills all available expressions; an eligible
/// site has already proved that neither its effect footprint nor its
/// transition/world-state facts mutate state.
fn statement_writes_state_for_gvn(
    registry: &CommandRegistry,
    stmt: &Statement,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    legality: GvnLegalitySource<'_>,
) -> bool {
    match legality {
        GvnLegalitySource::Legacy => statement_writes_state(registry, stmt, dialect),
        GvnLegalitySource::Common(semantic) => match stmt {
            Statement::Barrier { .. } | Statement::NativeCall { .. } => true,
            Statement::Call { span, .. } => {
                match semantic.map_or(GvnSemanticLookup::Unmapped, |facts| facts.lookup(*span)) {
                    GvnSemanticLookup::Eligible { .. } => false,
                    GvnSemanticLookup::Declined | GvnSemanticLookup::Unmapped => true,
                }
            }
            _ => false,
        },
    }
}

/// Collect pure-expression occurrences produced by a single SSA
/// statement.
///
/// Covers two cases:
///
/// - The top-level `Statement::Call` itself, when the command is
///   pure and marked `CSE_CANDIDATE`.
/// - Embedded `[cmd args…]` command substitutions
///   inside `Statement::Call` argument text and
///   `Statement::AssignValue` value text. Each nested
///   substitution is parsed via
///   [`scan_bracketed_commands`] and, when its extracted command
///   is pure + CSE-candidate, recorded as its own occurrence.
#[must_use]
pub fn statement_occurrences(
    registry: &CommandRegistry,
    stmt_ssa: &SsaStatement,
    block_name: &str,
    statement_index: usize,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    ssa: &SsaFunction,
) -> Vec<ExprOccurrence> {
    statement_occurrences_for_gvn(
        registry,
        stmt_ssa,
        block_name,
        statement_index,
        dialect,
        ssa,
        GvnLegalitySource::Legacy,
    )
}

/// Semantic-sidecar-aware occurrence collector used by the production
/// function-level GVN entry point.
fn statement_occurrences_for_gvn(
    registry: &CommandRegistry,
    stmt_ssa: &SsaStatement,
    block_name: &str,
    statement_index: usize,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    ssa: &SsaFunction,
    legality: GvnLegalitySource<'_>,
) -> Vec<ExprOccurrence> {
    let legacy_config = || {
        if let Some(tokens) = stmt_ssa.statement.tokens()
            && let Some(binding) = &tokens.source_binding
        {
            return binding.original_lexer_config_for_tokens(tokens);
        }
        dialect
            .or_else(|| registry.profile())
            .map(|profile| tcl_lexer::LexerConfig::for_profile(Some(profile)))
    };
    let mut out: Vec<ExprOccurrence> = Vec::new();
    let variable_uses = || -> Vec<String> {
        stmt_ssa
            .uses
            .keys()
            .map(|&s| ssa.var_name(s).to_owned())
            .collect()
    };

    // Top-level pure Call.
    if let Statement::Call {
        command,
        args,
        span,
        ..
    } = &stmt_ssa.statement
    {
        let eligible: Option<(&str, &[String])> = match legality {
            GvnLegalitySource::Legacy => (is_pure_command(registry, command, args, dialect)
                && is_worth_reporting(registry, command))
            .then_some((command.as_str(), &[] as &[String])),
            GvnLegalitySource::Common(Some(semantic)) => match semantic.lookup(*span) {
                GvnSemanticLookup::Eligible { command, world_key } => Some((command, world_key)),
                GvnSemanticLookup::Declined | GvnSemanticLookup::Unmapped => None,
            },
            GvnLegalitySource::Common(None) => None,
        };
        let key = eligible.and_then(|(canonical_command, world_key)| match legality {
            GvnLegalitySource::Legacy => Some(build_call_key(
                canonical_command,
                args,
                &stmt_ssa.uses,
                ssa,
                legacy_config()?,
            )),
            GvnLegalitySource::Common(_) => {
                original_call_key(canonical_command, stmt_ssa, ssa, world_key)
            }
        });
        if let Some(key) = key {
            out.push(ExprOccurrence {
                key,
                span: *span,
                expression_text: format_expression_text(command, args),
                block: block_name.to_owned(),
                statement_index,
                variable_uses: variable_uses(),
            });
        }
    }

    // Embedded command substitutions inside argument/value text.
    let texts_to_scan: Vec<&str> = match (legality, &stmt_ssa.statement) {
        (GvnLegalitySource::Legacy, Statement::Call { args, .. }) => {
            args.iter().map(String::as_str).collect()
        }
        (GvnLegalitySource::Legacy, Statement::AssignValue { value, .. }) => {
            vec![value.as_str()]
        }
        _ => Vec::new(),
    };
    let span = stmt_ssa.statement.span();
    for text in texts_to_scan {
        let Some(config) = legacy_config() else {
            break;
        };
        for (cmd, args) in scan_bracketed_commands_with_config(text, config).unwrap_or_default() {
            if !is_pure_command(registry, &cmd, &args, dialect) {
                continue;
            }
            if !is_worth_reporting(registry, &cmd) {
                continue;
            }
            out.push(ExprOccurrence {
                key: build_call_key(&cmd, &args, &stmt_ssa.uses, ssa, config),
                span,
                expression_text: format_expression_text(&cmd, &args),
                block: block_name.to_owned(),
                statement_index,
                variable_uses: variable_uses(),
            });
        }
    }

    out
}

/// Scan `text` for top-level `[command args…]` substitutions,
/// returning each matched `(command, args)` pair.
///
/// Handles nested brackets (e.g. `[foo [bar] baz]` extracts both
/// the outer `foo [bar] baz` and the inner `bar`). Brace-quoted
/// regions `{…}` are treated as opaque so commands inside them
/// aren't accidentally matched. Strings inside `"…"` quotes are
/// scanned normally because Tcl performs command substitution
/// through quotes.
///
/// Emits one pair per `[…]` region,
/// with nested regions emitted in depth-first order (outer first,
/// then each nested inner).
#[must_use]
pub fn scan_bracketed_commands(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<(String, Vec<String>)> {
    scan_bracketed_commands_with_config(text, config).unwrap_or_default()
}

fn scan_bracketed_commands_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(String, Vec<String>)>> {
    use tcl_lexer::word_parts::ExecutablePart;
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        tcl_lexer::SourceImage::document(text),
        tcl_lexer::Span::new(0, u32::try_from(text.len()).ok()?),
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    let mut pending = vec![(plan, false)];
    let mut commands = Vec::new();
    while let Some((plan, nested)) = pending.pop() {
        if plan.fatal_tail.is_some() {
            return None;
        }
        let mut children = Vec::new();
        for command in &plan.commands {
            if nested {
                let (head, args) = command.words.split_first()?;
                let head = String::from_utf8(
                    tcl_syntax::word_rules::original_static_word_unicode_value(head)?,
                )
                .ok()?;
                let args = args
                    .iter()
                    .map(|word| {
                        std::str::from_utf8(word.written_bytes())
                            .ok()
                            .map(str::to_owned)
                    })
                    .collect::<Option<Vec<_>>>()?;
                commands.push((head, args));
            }
            for word in &command.words {
                let parts = word.executable_parts();
                let mut bodies = Vec::new();
                for component in parts.all_parts() {
                    match component.part {
                        ExecutablePart::Command { body } => bodies.push(body),
                        ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => {
                            return None;
                        }
                        _ => {}
                    }
                }
                bodies.sort_unstable_by_key(|span| span.start());
                for body in bodies {
                    children.push((
                        plan.reborrow_command_substitution_plan(word, body).ok()?,
                        true,
                    ));
                }
            }
        }
        pending.extend(children.into_iter().rev());
    }
    Some(commands)
}

// Find-redundancies driver

/// Walk `cfg` / `ssa` in dominator-tree preorder and return a
/// [`RedundantComputation`] diagnostic for every pure-expression
/// occurrence that replays a dominating occurrence on the same
/// path.
///
/// Uses an iterative traversal (explicit `(block, phase)` work
/// stack) so deeply nested dominator trees don't risk blowing the
/// Rust call stack. Each block pushes a new scope on entry,
/// processes its statements, visits dominator-tree children, and
/// pops the scope on exit.
///
/// Behaviour:
///
/// - Treats every CFG block as executable (no SCCP unreachability
///   filter). Callers that want SCCP pruning can pre-filter the
///   CFG.
/// - Detects *full* redundancy (same expression computed twice on
///   the same path) with the `"O105"` code and the
///   [`full_redundancy_message`] text.
/// - Partial-redundancy (`O106`) and loop-invariant (`O107`)
///   detection are not handled here — they each need
///   substantially more walker state.
///
/// Returns the diagnostics in the order the walker encounters
/// them.
/// Iterative dominator-tree walk step. `Enter` pushes a fresh
/// scope and processes the block; `Leave` pops the scope when all
/// dominator children have been visited.
enum WalkStep {
    Enter(BlockId),
    Leave,
}

/// Walk `cfg` / `ssa` in dominator-tree preorder and return a
/// [`RedundantComputation`] diagnostic for every pure-expression
/// occurrence that replays a dominating occurrence on the same
/// path.
///
/// Uses an iterative traversal (explicit `(block, phase)` work
/// stack) so deeply nested dominator trees don't risk blowing the
/// Rust call stack. Each block pushes a new scope on entry,
/// processes its statements, visits dominator-tree children, and
/// pops the scope on exit.
///
/// Behaviour:
///
/// - Treats every CFG block as executable (no SCCP unreachability
///   filter). Callers that want SCCP pruning can pre-filter the
///   CFG.
/// - Detects *full* redundancy (same expression computed twice on
///   the same path) with the `"O105"` code and the
///   [`full_redundancy_message`] text.
/// - Partial-redundancy (`O106`) and loop-invariant (`O107`)
///   detection are not handled here — they each need
///   substantially more walker state.
///
/// Returns the diagnostics in the order the walker encounters
/// them.
#[must_use]
pub fn find_redundancies(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    find_redundancies_with_semantic_facts(registry, cfg, ssa, dialect, GvnLegalitySource::Legacy)
}

/// GVN implementation shared by the legacy CFG-only entry point and the
/// production function-level entry point that has semantic sidecar facts.
fn find_redundancies_with_semantic_facts(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    legality: GvnLegalitySource<'_>,
) -> Vec<RedundantComputation> {
    let mut table = ScopedValueTable::new();
    let mut results: Vec<RedundantComputation> = Vec::new();
    if !cfg.blocks.contains_key(&ssa.entry) {
        return results;
    }
    let mut stack: Vec<WalkStep> = vec![WalkStep::Enter(ssa.entry)];

    while let Some(step) = stack.pop() {
        match step {
            WalkStep::Leave => {
                table.pop_scope();
            }
            WalkStep::Enter(bn) => {
                table.push_scope();
                stack.push(WalkStep::Leave);

                let Some(cfg_block) = cfg.blocks.get(&bn) else {
                    continue;
                };
                let Some(ssa_block) = ssa.blocks.get(&bn) else {
                    continue;
                };
                let block_name = ssa.block_name(bn);
                let stmt_count =
                    std::cmp::min(cfg_block.statements.len(), ssa_block.statements.len());

                for idx in 0..stmt_count {
                    let ir_stmt = &cfg_block.statements[idx];
                    let ssa_stmt = &ssa_block.statements[idx];

                    if statement_writes_state_for_gvn(registry, ir_stmt, dialect, legality) {
                        table.kill_all();
                        continue;
                    }

                    let occurrences = statement_occurrences_for_gvn(
                        registry, ssa_stmt, block_name, idx, dialect, ssa, legality,
                    );
                    for occ in occurrences {
                        if let Some(existing) = table.lookup(&occ.key) {
                            let text = existing.expression_text.clone();
                            results.push(RedundantComputation {
                                span: occ.span,
                                first_span: existing.span,
                                expression_text: text.clone(),
                                code: DiagCode::O105,
                                message: full_redundancy_message(&text),
                            });
                            continue;
                        }
                        table.insert(ValueEntry {
                            key: occ.key.clone(),
                            block: occ.block.clone(),
                            statement_index: occ.statement_index,
                            span: occ.span,
                            expression_text: occ.expression_text.clone(),
                        });
                    }
                }

                // Visit dominator-tree children. Push in reverse so
                // they're popped in left-to-right order.
                if let Some(children) = ssa.dominator_tree.get(&bn) {
                    for child in children.iter().rev() {
                        stack.push(WalkStep::Enter(*child));
                    }
                }
            }
        }
    }

    results
}

/// Run full-redundancy GVN using one function's target-neutral semantic
/// analysis sidecar when it has source-faithful executable facts.
///
/// The production path never invokes the legacy command-string classifier. An
/// unavailable sidecar and every unresolved, ambiguous, declined, or unmapped
/// source site fail closed and cannot become a CSE occurrence.
#[must_use]
pub fn find_redundancies_for_function(
    registry: &CommandRegistry,
    function: &crate::compilation_unit::FunctionUnit,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let semantic = GvnSemanticFacts::from_function(function, registry);
    find_redundancies_with_semantic_facts(
        registry,
        &function.cfg,
        &function.ssa,
        dialect,
        GvnLegalitySource::Common(semantic.as_ref()),
    )
}

// Loop-invariant detection

/// Adopt the compiler's single "is this block executable" fact into GVN's
/// hasher.
///
/// The owner is [`crate::sccp::SccpResult::executable_blocks`] — the
/// optimistic executable-block set SCCP accumulates while it folds constant
/// branches (`sccp.rs:199`). The deletion side already reads exactly that
/// set: `optimiser/elimination.rs`'s `unreachable_blocks` turns it into the
/// O107 deletion, and O112's constant-condition deletions
/// (`optimiser/structure_elimination.rs`) are the structured-IR view of the
/// same constant facts. `crate::loops::build_loop_forest` takes it as a
/// parameter for the same reason.
///
/// Code motion must consult the same fact rather than raw CFG reachability:
/// a plain terminator-successor walk still reaches a block behind a
/// constant-false branch, so PRE and LICM would offer a hoist *into* a
/// region the deletion passes simultaneously offer to remove.
/// SCCP is optimistic, so a block it never marked executable is provably
/// dead under those same facts — pruning to the set is the sound direction.
fn adopt_executable_blocks<S: std::hash::BuildHasher>(
    executable: &std::collections::HashSet<BlockId, S>,
) -> FxHashSet<BlockId> {
    executable.iter().copied().collect()
}

/// Natural loop: all blocks on any path from `latch` back to
/// `header` that doesn't leave the loop via a non-predecessor
/// edge. Walks predecessors starting from `latch`, stopping at
/// the header.
///
/// Mirrors `crate::loops::natural_loop_blocks` (`loops.rs:98-122`) and now
/// takes the same executable set, so the two agree on which blocks a loop
/// body contains. The bodies stay separate only because the back-edge
/// search around this one uses `SsaFunction::dominator_intervals` (one O(V)
/// numbering, O(1) per query) where `crate::loops::dominates`
/// (`loops.rs:64-73`) walks the idom chain per query, which is quadratic.
fn natural_loop_blocks(
    cfg: &CfgFunction,
    header: BlockId,
    latch: BlockId,
    executable: &FxHashSet<BlockId>,
) -> FxHashSet<BlockId> {
    let preds = cfg.predecessors();
    let mut blocks: FxHashSet<BlockId> = FxHashSet::default();
    blocks.insert(header);
    blocks.insert(latch);
    let mut work = vec![latch];
    while let Some(node) = work.pop() {
        if let Some(ps) = preds.get(&node) {
            for p in ps {
                if !executable.contains(p) || blocks.contains(p) {
                    continue;
                }
                blocks.insert(*p);
                if *p != header {
                    work.push(*p);
                }
            }
        }
    }
    blocks
}

/// All variable names defined inside `loop_blocks` (phi LHS +
/// statement defs).
fn loop_defined_variables(
    ssa: &SsaFunction,
    loop_blocks: &FxHashSet<BlockId>,
) -> FxHashSet<String> {
    let mut defs = FxHashSet::default();
    for bn in loop_blocks {
        if let Some(block) = ssa.blocks.get(bn) {
            for phi in &block.phis {
                defs.insert(ssa.var_name(phi.name).to_owned());
            }
            for stmt in &block.statements {
                for &sym in stmt.defs.keys() {
                    defs.insert(ssa.var_name(sym).to_owned());
                }
            }
        }
    }
    defs
}

/// Detect loop-invariant pure computations.
///
/// Returns a [`RedundantComputation`] with code `"O106"` for each
/// pure-expression occurrence inside a loop whose variable
/// references are all defined *outside* the loop (so the
/// computation produces the same value on every iteration).
///
/// Uses a simplified occurrence
/// walk that reuses [`statement_occurrences`].
///
/// `executable` is the caller's [`crate::sccp::SccpResult::executable_blocks`]
/// — see [`adopt_executable_blocks`]. Blocks outside it are provably dead and
/// never receive a hoist offer.
#[must_use]
pub fn find_loop_invariants<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &std::collections::HashSet<BlockId, S>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    find_loop_invariants_with_legality(
        registry,
        cfg,
        ssa,
        &adopt_executable_blocks(executable),
        dialect,
        GvnLegalitySource::Legacy,
    )
}

/// Run loop-invariance detection using one function's dispatch-stability
/// proofs.
///
/// Hoisting a call out of a loop changes how many times it is dispatched, so
/// it needs the same live-dispatch proof as reusing one: a command whose
/// binding, traces, or interpreter policy may change inside the loop is not
/// invariant merely because its arguments are.
///
/// Executability comes from the function's own SCCP result, the same fact
/// the deletion passes read, so a hoist is never offered into a block O112 /
/// O107 offer to delete.
#[must_use]
pub fn find_loop_invariants_for_function(
    registry: &CommandRegistry,
    function: &crate::compilation_unit::FunctionUnit,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let semantic = GvnSemanticFacts::from_function(function, registry);
    find_loop_invariants_with_legality(
        registry,
        &function.cfg,
        &function.ssa,
        &adopt_executable_blocks(&function.sccp.executable_blocks),
        dialect,
        GvnLegalitySource::Common(semantic.as_ref()),
    )
}

fn find_loop_invariants_with_legality(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &FxHashSet<BlockId>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    legality: GvnLegalitySource<'_>,
) -> Vec<RedundantComputation> {
    let mut results: Vec<RedundantComputation> = Vec::new();
    // One O(V) dominator-tree numbering serves every dominance query below.
    // Walking the idom chain per query instead made this pass O(V²) on a wide
    // CFG — a flat N-branch dispatch chain is one idom chain N blocks deep.
    let dom = ssa.dominator_intervals();

    // Collect unique header → loop_blocks pairs via back-edge
    // detection: edge tail → succ where succ dominates tail. The
    // back-edge tails (latches) are tracked per header for the
    // latch-dominance gate below.
    let mut header_to_blocks: FxHashMap<BlockId, FxHashSet<BlockId>> = FxHashMap::default();
    let mut header_to_latches: FxHashMap<BlockId, FxHashSet<BlockId>> = FxHashMap::default();
    for tail in executable {
        let Some(block) = cfg.blocks.get(tail) else {
            continue;
        };
        let successors: Vec<BlockId> = match &block.terminator {
            Some(Terminator::Goto { target, .. }) => vec![*target],
            Some(Terminator::Branch {
                true_target,
                false_target,
                ..
            }) => vec![*true_target, *false_target],
            _ => continue,
        };
        for succ in successors {
            if !executable.contains(&succ) {
                continue;
            }
            if !dom.dominates(succ, *tail) {
                continue;
            }
            let blocks = natural_loop_blocks(cfg, succ, *tail, executable);
            header_to_blocks
                .entry(succ)
                .and_modify(|e| e.extend(blocks.iter().copied()))
                .or_insert(blocks);
            header_to_latches.entry(succ).or_default().insert(*tail);
        }
    }

    for (header, loop_blocks) in &header_to_blocks {
        let defined = loop_defined_variables(ssa, loop_blocks);
        let latches = &header_to_latches[header];
        for bn in loop_blocks {
            // The loop guard is always carried by a block's `Branch`/`Goto`
            // *terminator*, never by a `Statement`, so scanning a block's
            // statements never hoists the test condition. The header must be
            // scanned too: in a bottom-test loop (`for`/`while` lowered with
            // the test in the latch) the header block holds real body
            // statements, and skipping it would miss every invariant there
            // (FP-OPT-03). The latch-dominance gate below still enforces
            // "runs every iteration".
            // Latch-dominance "runs every iteration" gate: an invariant in
            // a block that does not dominate every back-edge tail runs only
            // on some iterations (it sits behind a branch inside the loop),
            // so hoisting it changes *when* it runs. Only blocks that
            // dominate every latch are guaranteed to execute each iteration.
            if !latches.iter().all(|latch| dom.dominates(*bn, *latch)) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };
            let block_name = ssa.block_name(*bn);
            for (idx, stmt_ssa) in ssa_block.statements.iter().enumerate() {
                // Purity gate — loop-invariance only makes sense
                // for computations that don't otherwise touch
                // state each iteration.
                if statement_writes_state_for_gvn(registry, &stmt_ssa.statement, dialect, legality)
                {
                    continue;
                }
                let occurrences = statement_occurrences_for_gvn(
                    registry, stmt_ssa, block_name, idx, dialect, ssa, legality,
                );
                for occ in occurrences {
                    if occ.variable_uses.iter().any(|name| defined.contains(name)) {
                        continue;
                    }
                    let text = occ.expression_text.clone();
                    results.push(RedundantComputation {
                        span: occ.span,
                        first_span: occ.span,
                        expression_text: text.clone(),
                        code: DiagCode::O106,
                        message: loop_invariant_message(&text),
                    });
                }
            }
        }
    }

    results
}

// Partial-redundancy detection

/// One event in a block's occurrence stream: either a pure
/// occurrence that *adds* a key to availability, or `None` — a
/// kill event produced by a mutator / barrier that clears all
/// previously-tracked keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OccurrenceEvent {
    /// An occurrence is available after this point.
    Occur(ExprOccurrence),
    /// Kill: clear all availability (barrier/impure call).
    Kill,
}

/// Collect per-block occurrence events for a function. Each block
/// contributes a Vec of events in statement order:
///
/// - For a statement that `statement_writes_state` returns true
///   for, emit `OccurrenceEvent::Kill`.
/// - For every pure + CSE-candidate occurrence reported by
///   `statement_occurrences`, emit `OccurrenceEvent::Occur(occ)`.
///
/// Blocks outside `executable` (see [`adopt_executable_blocks`]) contribute
/// nothing: an occurrence SCCP proved unreachable is not a real earlier
/// computation, so it must not become the hoist target a later occurrence is
/// told to reuse.
#[must_use]
pub fn collect_function_occurrence_events<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &std::collections::HashSet<BlockId, S>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (
    FxHashMap<BlockId, Vec<OccurrenceEvent>>,
    Vec<ExprOccurrence>,
) {
    collect_function_occurrence_events_with_legality(
        registry,
        cfg,
        ssa,
        &adopt_executable_blocks(executable),
        dialect,
        GvnLegalitySource::Legacy,
    )
}

/// Occurrence collection under an explicit legality source.
fn collect_function_occurrence_events_with_legality(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &FxHashSet<BlockId>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    legality: GvnLegalitySource<'_>,
) -> (
    FxHashMap<BlockId, Vec<OccurrenceEvent>>,
    Vec<ExprOccurrence>,
) {
    let mut events_by_block: FxHashMap<BlockId, Vec<OccurrenceEvent>> = FxHashMap::default();
    let mut all_occurrences: Vec<ExprOccurrence> = Vec::new();

    for (bn, cfg_block) in &cfg.blocks {
        if !executable.contains(bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(bn) else {
            continue;
        };
        let stmt_count = cfg_block.statements.len().min(ssa_block.statements.len());
        let block_name = cfg.block_name(*bn);
        let mut events: Vec<OccurrenceEvent> = Vec::new();
        for idx in 0..stmt_count {
            let ir_stmt = &cfg_block.statements[idx];
            let ssa_stmt = &ssa_block.statements[idx];
            if statement_writes_state_for_gvn(registry, ir_stmt, dialect, legality) {
                events.push(OccurrenceEvent::Kill);
                continue;
            }
            for occ in statement_occurrences_for_gvn(
                registry, ssa_stmt, block_name, idx, dialect, ssa, legality,
            ) {
                all_occurrences.push(occ.clone());
                events.push(OccurrenceEvent::Occur(occ));
            }
        }
        if !events.is_empty() {
            events_by_block.insert(*bn, events);
        }
    }

    (events_by_block, all_occurrences)
}

/// Apply a block's events to a starting set of available keys,
/// returning the resulting set at block exit.
fn transfer_occurrence_keys(
    events: &[OccurrenceEvent],
    mut state: FxHashSet<ExprKey>,
) -> FxHashSet<ExprKey> {
    for e in events {
        match e {
            OccurrenceEvent::Kill => state.clear(),
            OccurrenceEvent::Occur(occ) => {
                state.insert(occ.key.clone());
            }
        }
    }
    state
}

/// Detect path-partial redundancies via may/must availability
/// dataflow. Returns `RedundantComputation { code: "O105", … }`
/// for each occurrence that is *may-available* on entry (some
/// path has already computed it) but not *must-available* (so
/// hoisting before the merge point would save the computation
/// on the path where it hadn't yet been run).
/// Maps tracked by the partial-redundancy dataflow fixpoint.
type ExprKeySetMap = FxHashMap<BlockId, FxHashSet<ExprKey>>;

/// Run the partial-redundancy may/must-availability dataflow
/// fixpoint.  Returns `(may_in, may_out, must_in, must_out)` after
/// convergence.  Extracted from [`find_partial_redundancies`] to
/// keep the dispatcher under threshold.
fn run_partial_redundancy_fixpoint(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &FxHashSet<BlockId>,
    events_by_block: &FxHashMap<BlockId, Vec<OccurrenceEvent>>,
    universe: &FxHashSet<ExprKey>,
    order: &[BlockId],
) -> (ExprKeySetMap, ExprKeySetMap, ExprKeySetMap, ExprKeySetMap) {
    let preds = cfg.predecessors();
    let mut may_in: ExprKeySetMap = FxHashMap::default();
    let mut may_out: ExprKeySetMap = FxHashMap::default();
    let mut must_in: ExprKeySetMap = FxHashMap::default();
    let mut must_out: ExprKeySetMap = FxHashMap::default();
    for bn in executable {
        may_in.insert(*bn, FxHashSet::default());
        may_out.insert(*bn, FxHashSet::default());
        must_in.insert(
            *bn,
            if bn == &ssa.entry {
                FxHashSet::default()
            } else {
                universe.clone()
            },
        );
    }
    // Seed must_out from initial must_in + events.
    for bn in executable {
        let initial_must_in = must_in[bn].clone();
        let out = transfer_occurrence_keys(
            events_by_block.get(bn).map_or(&[][..], Vec::as_slice),
            initial_must_in,
        );
        must_out.insert(*bn, out);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for bn in order {
            if !executable.contains(bn) {
                continue;
            }
            let pred_set: Vec<BlockId> = preds.get(bn).map_or_default(|s| {
                s.iter()
                    .filter(|p| executable.contains(*p))
                    .copied()
                    .collect()
            });
            let (new_must_in, new_may_in) = if bn == &ssa.entry || pred_set.is_empty() {
                (FxHashSet::default(), FxHashSet::default())
            } else {
                let mut mi = must_out[&pred_set[0]].clone();
                for p in &pred_set[1..] {
                    mi = mi.intersection(&must_out[p]).cloned().collect();
                }
                let mut my: FxHashSet<ExprKey> = FxHashSet::default();
                for p in &pred_set {
                    my.extend(may_out[p].iter().cloned());
                }
                (mi, my)
            };
            let new_must_out = transfer_occurrence_keys(
                events_by_block.get(bn).map_or(&[][..], Vec::as_slice),
                new_must_in.clone(),
            );
            let new_may_out = transfer_occurrence_keys(
                events_by_block.get(bn).map_or(&[][..], Vec::as_slice),
                new_may_in.clone(),
            );
            if new_must_in != must_in[bn] {
                must_in.insert(*bn, new_must_in);
                changed = true;
            }
            if new_may_in != may_in[bn] {
                may_in.insert(*bn, new_may_in);
                changed = true;
            }
            if new_must_out != must_out[bn] {
                must_out.insert(*bn, new_must_out);
                changed = true;
            }
            if new_may_out != may_out[bn] {
                may_out.insert(*bn, new_may_out);
                changed = true;
            }
        }
    }
    (may_in, may_out, must_in, must_out)
}

/// Find partial redundancies — expressions that occur multiple
/// times where one occurrence dominates another along some path
/// but not on every path.  Uses the may/must-availability
/// bidirectional dataflow from
/// [`run_partial_redundancy_fixpoint`].
///
/// `executable` is the caller's [`crate::sccp::SccpResult::executable_blocks`]
/// — see [`adopt_executable_blocks`]. The hoist point a partial redundancy
/// implies must land in a block that survives deletion, so the availability
/// dataflow runs over that set and never over raw CFG reachability.
#[must_use]
pub fn find_partial_redundancies<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &std::collections::HashSet<BlockId, S>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    find_partial_redundancies_with_legality(
        registry,
        cfg,
        ssa,
        &adopt_executable_blocks(executable),
        dialect,
        GvnLegalitySource::Legacy,
    )
}

/// Run partial-redundancy detection using one function's dispatch-stability
/// proofs.
///
/// This is the production entry point. Like [`find_redundancies_for_function`]
/// it never consults the legacy command-string classifier: an unavailable
/// sidecar and every unresolved, ambiguous, declined, or unmapped site fail
/// closed.
///
/// Executability comes from the function's own SCCP result, the same fact
/// the deletion passes read.
#[must_use]
pub fn find_partial_redundancies_for_function(
    registry: &CommandRegistry,
    function: &crate::compilation_unit::FunctionUnit,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let semantic = GvnSemanticFacts::from_function(function, registry);
    find_partial_redundancies_with_legality(
        registry,
        &function.cfg,
        &function.ssa,
        &adopt_executable_blocks(&function.sccp.executable_blocks),
        dialect,
        GvnLegalitySource::Common(semantic.as_ref()),
    )
}

fn find_partial_redundancies_with_legality(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &FxHashSet<BlockId>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    legality: GvnLegalitySource<'_>,
) -> Vec<RedundantComputation> {
    if !executable.contains(&ssa.entry) {
        return Vec::new();
    }
    let (events_by_block, all_occurrences) = collect_function_occurrence_events_with_legality(
        registry, cfg, ssa, executable, dialect, legality,
    );
    if all_occurrences.is_empty() {
        return Vec::new();
    }
    let universe: FxHashSet<ExprKey> = all_occurrences.iter().map(|o| o.key.clone()).collect();

    let mut order: Vec<BlockId> = cfg.reverse_postorder();
    let seen: FxHashSet<BlockId> = order.iter().copied().collect();
    for id in executable {
        if !seen.contains(id) {
            order.push(*id);
        }
    }

    let (may_in, _may_out, must_in, _must_out) =
        run_partial_redundancy_fixpoint(cfg, ssa, executable, &events_by_block, &universe, &order);

    // Build first-occurrence map by source offset.
    let mut sorted = all_occurrences.clone();
    sorted.sort_by_key(|o| o.span.start());
    let mut first_by_key: FxHashMap<ExprKey, ExprOccurrence> = FxHashMap::default();
    for occ in sorted {
        first_by_key.entry(occ.key.clone()).or_insert(occ);
    }
    let mut key_offsets: FxHashMap<ExprKey, FxHashSet<u32>> = FxHashMap::default();
    for occ in &all_occurrences {
        key_offsets
            .entry(occ.key.clone())
            .or_default()
            .insert(occ.span.start());
    }

    let mut results: Vec<RedundantComputation> = Vec::new();
    for bn in &order {
        if !executable.contains(bn) {
            continue;
        }
        let mut state_may = may_in[bn].clone();
        let mut state_must = must_in[bn].clone();
        for event in events_by_block.get(bn).map_or(&[][..], Vec::as_slice) {
            match event {
                OccurrenceEvent::Kill => {
                    state_may.clear();
                    state_must.clear();
                }
                OccurrenceEvent::Occur(occ) => {
                    let multi = key_offsets.get(&occ.key).is_some_and(|s| s.len() >= 2);
                    if multi
                        && state_may.contains(&occ.key)
                        && !state_must.contains(&occ.key)
                        && let Some(first) = first_by_key.get(&occ.key)
                        && first.span.start() != occ.span.start()
                    {
                        let text = first.expression_text.clone();
                        results.push(RedundantComputation {
                            span: occ.span,
                            first_span: first.span,
                            expression_text: text.clone(),
                            code: DiagCode::O105,
                            message: partial_redundancy_message(&text),
                        });
                    }
                    state_may.insert(occ.key.clone());
                    state_must.insert(occ.key.clone());
                }
            }
        }
    }
    results
}

// Compilation-unit-level convenience wrappers

/// Run [`find_redundancies`] across every function in the
/// compilation unit, concatenating the per-function findings in
/// iteration order.
///
/// Convenience wrapper for callers that already hold a
/// [`crate::compilation_unit::CompilationUnit`] and don't want to
/// re-implement the per-function loop.
#[must_use]
pub fn find_redundancies_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_redundancies_for_function(registry, fu, dialect));
    }
    out
}

/// Run [`find_partial_redundancies`] across every function in the
/// compilation unit. See [`find_redundancies_for_cu`] for the
/// motivation.
#[must_use]
pub fn find_partial_redundancies_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_partial_redundancies_for_function(
            registry, fu, dialect,
        ));
    }
    out
}

/// Run [`find_loop_invariants`] across every function in the
/// compilation unit. See [`find_redundancies_for_cu`] for the
/// motivation.
#[must_use]
pub fn find_loop_invariants_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_loop_invariants_for_function(registry, fu, dialect));
    }
    out
}

#[cfg(test)]
mod tests;
