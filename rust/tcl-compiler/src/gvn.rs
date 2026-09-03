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

use std::collections::HashMap;
use tcl_core_types::DiagCode;

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, ResultStability, StateTransitionKnowledge, Traits};

use crate::cfg::{BlockId, CfgModule, Function as CfgFunction, Terminator};
use crate::ir::{NodeId, Provenance, SourceSite, Statement};
use crate::side_effects::{EffectRegion, classify_side_effects};
use crate::ssa::{SsaFunction, SsaStatement};

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

/// Rewrite `$var` / `${var}` references in `text` to their
/// SSA-versioned canonical form `$var@N`.
///
/// Scans `text` left-to-right and rewrites each variable reference
/// exactly once — avoiding the re-matching trap where a naive
/// `.replace` on `${x}` produces `$x@3@3` because the emitted
/// `$x@3` contains a second `$x` substring.
///
/// A variable reference begins at `$`:
/// - `${name}` — braced form; `name` is everything up to the
///   closing `}`.
/// - `$name` — bare form; `name` matches the Tcl identifier
///   grammar (`[A-Za-z0-9_:]+`).
///
/// Names that are not present in `uses` are left unchanged.
#[must_use]
pub fn canonicalise_word<S: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<crate::ssa::Symbol, u32, S>,
    ssa: &SsaFunction,
    braced_var: tcl_dialect::BracedVarStyle,
) -> String {
    use std::fmt::Write as _;
    if uses.is_empty() {
        return text.to_owned();
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            out.push(char::from(bytes[i]));
            i += 1;
            continue;
        }
        // At `$` — inspect the next char.
        if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // `${name}` — the name starts just past the `${`, and its closer
            // comes from the shared owner under this document's release rule.
            // A name this misreads is not merely unversioned: the reference is
            // then copied through verbatim, so two occurrences at *different*
            // SSA versions produce the same key and GVN treats them as
            // redundant — an unsound CSE, not a lost one (issue #1604).
            let start = i + 2;
            if let tcl_lexer::BracedVarEnd::Closed(j) =
                tcl_lexer::braced_var_name_end(bytes, start, braced_var)
            {
                let name = &text[start..j];
                if let Some(ver) = ssa.var_symbol(name).and_then(|s| uses.get(&s)) {
                    let _ = write!(out, "${name}@{ver}");
                } else {
                    out.push_str(&text[i..=j]);
                }
                i = j + 1;
                continue;
            }
            // No closing brace — treat as a bare `$` and move on.
            out.push('$');
            i += 1;
            continue;
        }
        // Bare `$name` — scan identifier characters.
        let start = i + 1;
        let mut j = start;
        while j < bytes.len() {
            let b = bytes[j];
            let is_ident = b.is_ascii_alphanumeric() || b == b'_' || b == b':';
            if !is_ident {
                break;
            }
            j += 1;
        }
        if j == start {
            // Lone `$` with no name — pass through.
            out.push('$');
            i += 1;
            continue;
        }
        let name = &text[start..j];
        if let Some(ver) = ssa.var_symbol(name).and_then(|s| uses.get(&s)) {
            let _ = write!(out, "${name}@{ver}");
        } else {
            out.push_str(&text[i..j]);
        }
        i = j;
    }
    out
}

/// Build the canonical [`ExprKey`] for a pure-command invocation:
/// `["call", command, canonicalised_arg1, canonicalised_arg2, …]`.
#[must_use]
pub fn build_call_key<S: std::hash::BuildHasher>(
    command: &str,
    args: &[String],
    uses: &HashMap<crate::ssa::Symbol, u32, S>,
    ssa: &SsaFunction,
    braced_var: tcl_dialect::BracedVarStyle,
) -> ExprKey {
    let mut parts: ExprKey = Vec::with_capacity(2 + args.len());
    parts.push("call".into());
    parts.push(command.to_owned());
    for arg in args {
        parts.push(canonicalise_word(arg, uses, ssa, braced_var));
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

// Intra-module pure-proc analysis

/// Set of procedure names proven pure by the intra-module
/// analysis. Consumed by [`is_pure_with_procs`] /
/// [`is_worth_reporting_with_procs`] so GVN treats user procs
/// the same way as built-in CSE candidates.
#[derive(Debug, Clone, Default)]
pub struct PureProcs {
    /// Fully-qualified names of procs with no observable side
    /// effects.
    pub names: FxHashSet<String>,
}

impl PureProcs {
    /// Empty set — no user procs are considered pure.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True when `name` was classified pure.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// Number of pure procs in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// True if no procs were classified pure.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Analyse every procedure in `cfg_module` and return the set of
/// procs with no observable side effects.
///
/// A proc is pure when every statement in every block is pure:
///
/// - `Statement::Barrier` — never pure.
/// - `Statement::Call { command, args }` — pure when
///   `classify_side_effects(...).pure` is true, or when `command`
///   resolves to a proc name already in the "pure" set.
/// - Pure IR nodes (`AssignConst`, `AssignValue`, `AssignExpr`,
///   `Incr`, `ExprEval`, `Return`) — pure.
/// - Structured IR (`If`/`For`/`While`/`Foreach`/`Switch`/`Catch`/
///   `Try`) — not present in the post-CFG flat form; the CFG
///   builder flattens them into blocks.
///
/// Uses fixed-point iteration: starts by assuming every user proc
/// is pure, then iteratively removes any proc with an impure
/// statement until the set is stable. This resolves mutual
/// recursion correctly: a pair of procs that only call each other
/// and do nothing else remains pure.
#[must_use]
pub fn find_pure_procs(
    registry: &CommandRegistry,
    cfg_module: &CfgModule,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> PureProcs {
    let mut pure: FxHashSet<String> = cfg_module.procedures.keys().cloned().collect();

    let mut changed = true;
    while changed {
        changed = false;
        let candidates: Vec<String> = pure.iter().cloned().collect();
        for name in candidates {
            let Some(cfg) = cfg_module.procedures.get(&name) else {
                continue;
            };
            if !function_body_is_pure(registry, cfg, &pure, dialect) {
                pure.remove(&name);
                changed = true;
            }
        }
    }

    PureProcs { names: pure }
}

/// Return `true` when every statement in `cfg` is pure under the
/// current working `pure` set of proc names.
fn function_body_is_pure(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    pure: &FxHashSet<String>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if !statement_is_pure(registry, stmt, pure, dialect) {
                return false;
            }
        }
    }
    true
}

/// Purity predicate for one IR statement, consulting both the
/// registry and the intra-module pure-proc set.
fn statement_is_pure(
    registry: &CommandRegistry,
    stmt: &Statement,
    pure: &FxHashSet<String>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    match stmt {
        Statement::Call {
            command,
            args,
            defs,
            ..
        } => {
            // Variable-defining calls (e.g. `set`, `incr`,
            // `append`, `lappend`, the synthetic foreach header)
            // are only acceptable when the defined variables are
            // all proc-local. The simple heuristic: `defs` that
            // look like bare names (no `::` prefix) are safe; a
            // qualified name is a global write which is impure.
            for def in defs {
                if def.starts_with("::") {
                    return false;
                }
            }
            // Recognised pure call?
            if is_pure_with_procs_core(registry, pure, command, args, dialect) {
                return true;
            }
            // Variable-writing built-ins (like `set x 1`) land
            // here — classify_side_effects returns pure=false even
            // though the effect is proc-local. Accept the call
            // when writes are to proc-local state only, i.e. the
            // to_effect_regions write set is NONE.
            let effect = classify_side_effects(registry, command, args, dialect, None);
            let (_reads, writes) = effect.to_effect_regions();
            writes == EffectRegion::NONE
        }
        // IR nodes that never have side effects of their own.
        Statement::AssignConst { .. }
        | Statement::AssignValue { .. }
        | Statement::AssignExpr { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Return { .. } => true,
        // Statement::Barrier and any structured constructs that
        // reach here (they should have been flattened by the CFG
        // builder) are treated conservatively.
        _ => false,
    }
}

fn is_pure_with_procs_core(
    registry: &CommandRegistry,
    pure: &FxHashSet<String>,
    cmd: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    let outer_pure = pure.contains(cmd)
        // Try the common `::` prefix form too — some lowerings yield
        // fully-qualified command words.
        || match cmd.strip_prefix("::") {
            Some(stripped) => pure.contains(stripped),
            None => pure.contains(&format!("::{cmd}")),
        }
        || classify_side_effects(registry, cmd, args, dialect, None).pure;
    // O106: an outer-pure command whose args read from an impure inner
    // substitution is not loop-invariant — recurse into nested `[…]`.
    outer_pure
        && arg_substitutions_pure(args, |c, a| {
            is_pure_with_procs_core(registry, pure, c, a, dialect)
        })
}

/// Purity predicate for GVN that consults the intra-module
/// pure-proc set in addition to registry metadata.
#[must_use]
pub fn is_pure_with_procs(
    registry: &CommandRegistry,
    pure_procs: &PureProcs,
    cmd: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    is_pure_with_procs_core(registry, &pure_procs.names, cmd, args, dialect)
}

/// Extended worth-reporting check: user procs proved pure by
/// intra-module analysis count as CSE candidates even if they
/// don't carry the `CSE_CANDIDATE` registry trait.
#[must_use]
pub fn is_worth_reporting_with_procs(
    registry: &CommandRegistry,
    pure_procs: &PureProcs,
    cmd: &str,
) -> bool {
    if pure_procs.contains(cmd) {
        return true;
    }
    if let Some(stripped) = cmd.strip_prefix("::") {
        if pure_procs.contains(stripped) {
            return true;
        }
    } else {
        let prefixed = format!("::{cmd}");
        if pure_procs.contains(&prefixed) {
            return true;
        }
    }
    is_worth_reporting(registry, cmd)
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
    arg_substitutions_pure(args, |c, a| is_pure_command(registry, c, a, dialect))
}

/// Scan `args` for embedded `[cmd …]` command substitutions and return
/// `true` iff every nested command satisfies `check`. Braced regions
/// are skipped (no substitution inside `{…}`). Shared by the purity
/// gates so an outer pure command wrapping an impure inner subst is
/// rejected as loop-invariant (O106).
fn arg_substitutions_pure(args: &[String], check: impl Fn(&str, &[String]) -> bool) -> bool {
    args.iter().all(|arg| {
        !arg.contains('[')
            || scan_bracketed_commands(arg)
                .iter()
                .all(|(cmd, inner)| check(cmd, inner))
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
    if has_dynamic_trace {
        return false;
    }
    if traced_commands.contains(command.trim_start_matches("::")) {
        return false;
    }
    is_pure_command(registry, command, args, dialect)
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
    fn from_function(function: &crate::compilation_unit::FunctionUnit) -> Option<Self> {
        let availability = function.semantic_facts.executable();
        let executable = availability.function()?;
        // The dispatch-stability contents analysis is meaningful only when
        // the world-state graph itself was buildable; a world-state decline
        // fails every site closed exactly as before.
        let proofs = availability.world_state_ssa().is_some().then(|| {
            crate::dispatch_proof::analyse_dispatch_stability(
                executable,
                function.semantic_facts.dispatch_entry_assumption(),
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
    if !resolved
        .traits
        .contains(Traits::PURE.union(Traits::CSE_CANDIDATE))
    {
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
        Statement::Barrier { .. } => true,
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
            Statement::Barrier { .. } => true,
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
        if let Some((canonical_command, world_key)) = eligible {
            let mut key = build_call_key(
                canonical_command,
                args,
                &stmt_ssa.uses,
                ssa,
                tcl_dialect::BracedVarStyle::of_profile(dialect),
            );
            // Two occurrences may only match when the world state their
            // dispatch and result depend on carries the same versions.
            key.extend(world_key.iter().cloned());
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
        for (cmd, args) in scan_bracketed_commands(text) {
            if !is_pure_command(registry, &cmd, &args, dialect) {
                continue;
            }
            if !is_worth_reporting(registry, &cmd) {
                continue;
            }
            out.push(ExprOccurrence {
                key: build_call_key(
                    &cmd,
                    &args,
                    &stmt_ssa.uses,
                    ssa,
                    tcl_dialect::BracedVarStyle::of_profile(dialect),
                ),
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
pub fn scan_bracketed_commands(text: &str) -> Vec<(String, Vec<String>)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // Skip braced region entirely.
                let mut depth = 1i32;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'\\' if i + 1 < bytes.len() => i += 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'[' => {
                let start = i + 1;
                let mut depth = 1i32;
                let mut j = start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    j += 1;
                }
                if depth == 0 && j < bytes.len() {
                    let inner = &text[start..j];
                    if let Some(pair) = split_cmd_text(inner) {
                        out.push(pair);
                    }
                    // Also scan the inner content for further
                    // nested substitutions.
                    out.extend(scan_bracketed_commands(inner));
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
            }
            _ => i += 1,
        }
    }
    out
}

/// Split `"cmd arg1 arg2 ..."` into `(cmd, args)`.
///
/// Whitespace-separated at the top level; brace and quote regions
/// are kept as a single word (delimiters preserved so downstream
/// canonicalisation still sees variable references). Returns
/// `None` for empty / whitespace-only input.
fn split_cmd_text(text: &str) -> Option<(String, Vec<String>)> {
    let bytes = text.as_bytes();
    let mut words: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        match bytes[i] {
            b'{' => {
                let mut depth = 1i32;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'\\' if i + 1 < bytes.len() => i += 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            }
            _ => {
                while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => {
                            i += 2;
                        }
                        // A `[…]` command substitution or `{…}` brace region
                        // inside a word keeps its internal whitespace — skip the
                        // balanced region so `x[incr i]y` / `[format %d [incr
                        // i]]` stay a single word (else the nested `[incr i]`
                        // splits and its impurity is lost to the LICM purity
                        // check).
                        b'[' | b'{' => {
                            let (open, close) = if bytes[i] == b'[' {
                                (b'[', b']')
                            } else {
                                (b'{', b'}')
                            };
                            let mut depth = 1i32;
                            i += 1;
                            while i < bytes.len() && depth > 0 {
                                match bytes[i] {
                                    b'\\' if i + 1 < bytes.len() => {
                                        i += 1;
                                    }
                                    c if c == open => depth += 1,
                                    c if c == close => depth -= 1,
                                    _ => {}
                                }
                                i += 1;
                            }
                        }
                        _ => i += 1,
                    }
                }
            }
        }
        words.push(text[start..i].to_owned());
    }
    if words.is_empty() {
        return None;
    }
    let cmd = words.remove(0);
    Some((cmd, words))
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
    let semantic = GvnSemanticFacts::from_function(function);
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
/// set: `optimiser/elimination.rs:392` turns it into `unreachable_blocks`
/// for the O107 deletion, and O112's constant-condition deletions
/// (`optimiser/structure_elimination.rs`) are the structured-IR view of the
/// same constant facts. `crate::loops::build_loop_forest` takes it as a
/// parameter for the same reason.
///
/// Code motion must consult the same fact rather than raw CFG reachability:
/// a plain terminator-successor walk still reaches a block behind a
/// constant-false branch, so PRE and LICM would offer a hoist *into* a
/// region the deletion passes simultaneously offer to remove (issue #1385).
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
/// (`loops.rs:64-73`) walks the idom chain per query — the quadratic issue
/// #1250 removed from this pass.
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
/// O107 offer to delete (issue #1385).
#[must_use]
pub fn find_loop_invariants_for_function(
    registry: &CommandRegistry,
    function: &crate::compilation_unit::FunctionUnit,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let semantic = GvnSemanticFacts::from_function(function);
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
    // CFG — a flat N-branch dispatch chain is one idom chain N blocks deep
    // (issue #1250).
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
/// told to reuse (issue #1385).
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
            let pred_set: Vec<BlockId> = preds
                .get(bn)
                .map(|s| {
                    s.iter()
                        .filter(|p| executable.contains(*p))
                        .copied()
                        .collect()
                })
                .unwrap_or_default();
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
/// the deletion passes read (issue #1385).
#[must_use]
pub fn find_partial_redundancies_for_function(
    registry: &CommandRegistry,
    function: &crate::compilation_unit::FunctionUnit,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<RedundantComputation> {
    let semantic = GvnSemanticFacts::from_function(function);
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
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    /// The default document's `${…}` close rule — these unit tests build no
    /// dialect profile, so they exercise the same style the lexer would use.
    const STYLE: tcl_dialect::BracedVarStyle = tcl_dialect::BracedVarStyle::Tcl9Nesting;

    use super::*;
    use crate::semantic_analysis::ExecutableAnalysisAvailability;

    #[test]
    fn function_level_gvn_proves_llength_dispatch_and_reports_o105() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            "llength {a b}\nllength {a b}",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let function = &cu.top_level;

        assert!(
            !find_redundancies(
                &registry,
                &function.cfg,
                &function.ssa,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
                )
            )
            .is_empty(),
            "the legacy classifier recognises the registry pure trait"
        );
        let semantic = find_redundancies_for_function(
            &registry,
            function,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        );
        assert_eq!(
            semantic.len(),
            1,
            "a complete site proof over a pristine entry world enables stable-call CSE"
        );
        assert_eq!(semantic[0].code, DiagCode::O105);
    }

    #[test]
    fn function_level_gvn_declines_without_a_selected_semantic_dialect() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "llength {a b}\nllength {a b}",
            &registry,
            false,
        );
        let function = &cu.top_level;
        let legacy = find_redundancies(&registry, &function.cfg, &function.ssa, None);
        let sidecar = find_redundancies_for_function(&registry, function, None);
        assert!(!legacy.is_empty());
        assert!(
            sidecar.is_empty(),
            "production GVN must not fall back to command strings when semantic facts are unavailable"
        );
    }

    #[test]
    fn function_level_gvn_maps_calls_inside_a_structured_conditional_body() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            "if {$flag} {\nllength {a b}\nllength {a b}\n}",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let function = &cu.top_level;
        assert!(
            !find_redundancies(
                &registry,
                &function.cfg,
                &function.ssa,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
                )
            )
            .is_empty(),
            "the explicit legacy API still sees the nested string-classified calls"
        );
        assert!(
            !find_redundancies_for_function(
                &registry,
                function,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
                )
            )
            .is_empty(),
            "an `if` body is now executable-IR blocks, so its two identical pure \
             calls have exact invocation mappings and the function-level API \
             agrees with the legacy CFG API"
        );
    }

    #[test]
    fn function_level_gvn_proves_exact_mapped_closed_pure_call() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "test::pure_cse",
            traits: Traits::PURE | Traits::CSE_CANDIDATE,
            ..tcl_registry::CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
        });
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            "test::pure_cse value\ntest::pure_cse value",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        assert!(matches!(
            cu.top_level.semantic_facts.executable(),
            ExecutableAnalysisAvailability::Available(_)
        ));
        assert_eq!(
            find_redundancies_for_function(
                &registry,
                &cu.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
                )
            )
            .len(),
            1,
            "a closed registry contract plus a complete site proof reports the replay"
        );
    }

    #[test]
    fn function_level_gvn_proves_other_stable_commands() {
        let registry = CommandRegistry::build_default();
        for source in [
            "format %s x\nformat %s x",
            "lindex {a b} 0\nlindex {a b} 0",
            "lreverse {a b}\nlreverse {a b}",
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
                source,
                &registry,
                false,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            );
            assert_eq!(
                find_redundancies_for_function(
                    &registry,
                    &cu.top_level,
                    Some(
                        tcl_registry::model::ingress::resolve_environment("tcl9.0")
                            .analyser_profile()
                    )
                )
                .len(),
                1,
                "closed stable registry category with a live dispatch proof reuses: {source}"
            );
        }
    }

    #[test]
    fn static_llength_candidate_requires_coverage_of_registry_dispatch_dependencies() {
        use crate::dispatch_proof::{SiteDispatchProof, TrackVersion, WorldTrack};

        let registry = CommandRegistry::build_default();
        let resolved = registry
            .resolve_invocation(
                "llength",
                &["a b"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("llength resolves")
            .facts();

        assert!(resolved_invocation_is_gvn_candidate(&resolved));
        assert!(!resolved.dispatch_dependencies.is_empty());

        // No site proof at all (world graph unavailable or unreached).
        assert_eq!(
            gvn_site_eligibility(&resolved, None),
            GvnSiteEligibility::Declined
        );

        let complete = SiteDispatchProof {
            covers: resolved.dispatch_dependencies,
            versions: [TrackVersion::Entry; WorldTrack::ALL.len()],
            operand_words_admissible: true,
        };
        assert!(matches!(
            gvn_site_eligibility(&resolved, Some(&complete)),
            GvnSiteEligibility::Eligible { .. }
        ));

        // Any uncovered dependency domain fails the site closed.
        let partial = SiteDispatchProof {
            covers: tcl_registry::DispatchDependencies::NONE,
            versions: [TrackVersion::Entry; WorldTrack::ALL.len()],
            operand_words_admissible: true,
        };
        assert_eq!(
            gvn_site_eligibility(&resolved, Some(&partial)),
            GvnSiteEligibility::Declined
        );

        // Inadmissible operand words fail the site closed even with covered
        // dispatch domains.
        let inadmissible = SiteDispatchProof {
            operand_words_admissible: false,
            ..complete
        };
        assert_eq!(
            gvn_site_eligibility(&resolved, Some(&inadmissible)),
            GvnSiteEligibility::Declined
        );
    }

    #[test]
    fn live_execution_trace_keeps_o105_suppressed() {
        let registry = CommandRegistry::build_default();
        // The handler is deliberately never defined here. `proc` declares a
        // script-kind callback barrier that clobbers every domain on its own,
        // so defining `observe` would suppress the report for a reason that
        // has nothing to do with the trace, and this test would pass while
        // proving nothing. A trace prefix need not name a defined command.
        let source = "trace add execution llength enter observe\n\
                      llength {a b}\n\
                      llength {a b}";
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );

        assert!(matches!(
            cu.top_level.semantic_facts.executable(),
            ExecutableAnalysisAvailability::Available(_)
        ));
        assert!(
            find_redundancies_for_function(
                &registry,
                &cu.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .is_empty(),
            "an execution-traced command is observably invoked twice"
        );

        // Control: with the registration removed, the same two calls report.
        let untraced = crate::compilation_unit::CompilationUnit::build_for_profile(
            "llength {a b}\nllength {a b}",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert_eq!(
            find_redundancies_for_function(
                &registry,
                &untraced.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .len(),
            1,
            "the trace registration must be the only reason the report is withheld"
        );
    }

    #[test]
    fn function_level_gvn_declines_volatile_result_despite_closed_effects() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "test::volatile_cse",
            traits: Traits::PURE | Traits::CSE_CANDIDATE,
            result_stability: Some(ResultStability::Volatile),
            world_effects: Some(tcl_registry::WorldEffectDescriptor::EMPTY),
            state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            "test::volatile_cse\ntest::volatile_cse",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );

        assert!(
            find_redundancies_for_function(
                &registry,
                &cu.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .is_empty(),
            "read-only volatile results must not become common subexpressions"
        );
    }

    #[test]
    fn function_level_gvn_declines_clock_seconds_and_versioned_clock_context() {
        let registry = CommandRegistry::build_default();
        for source in [
            "clock seconds\nclock seconds",
            "clock add 0 1 day\nclock add 0 1 day",
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
                source,
                &registry,
                false,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            );
            assert!(
                find_redundancies_for_function(
                    &registry,
                    &cu.top_level,
                    Some(
                        tcl_registry::model::ingress::resolve_environment("tcl9.0")
                            .analyser_profile()
                    )
                )
                .is_empty(),
                "clock result dependencies must make production GVN abstain: {source}"
            );
        }
    }

    fn entry(key: &[&str], block: &str, idx: usize) -> ValueEntry {
        let key_owned: ExprKey = key.iter().map(|s| (*s).into()).collect();
        ValueEntry {
            key: key_owned.clone(),
            block: block.into(),
            statement_index: idx,
            span: Span::new(0, 0),
            expression_text: key_owned.join(" "),
        }
    }

    #[test]
    fn scoped_table_root_scope_always_present() {
        let t = ScopedValueTable::new();
        assert_eq!(t.scope_depth(), 1);
        assert!(t.lookup(&vec!["call".into(), "foo".into()]).is_none());
    }

    #[test]
    fn insert_and_lookup_in_root() {
        let mut t = ScopedValueTable::new();
        let e = entry(&["call", "llength", "$x@1"], "entry", 2);
        t.insert(e.clone());
        let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
        assert_eq!(t.lookup(&key), Some(&e));
    }

    #[test]
    fn pop_scope_discards_inner_entries_only() {
        let mut t = ScopedValueTable::new();
        t.insert(entry(&["call", "root"], "b", 0));
        t.push_scope();
        t.insert(entry(&["call", "inner"], "b", 1));
        assert_eq!(t.total_entries(), 2);
        t.pop_scope();
        let root_key: ExprKey = vec!["call".into(), "root".into()];
        let inner_key: ExprKey = vec!["call".into(), "inner".into()];
        assert!(t.lookup(&root_key).is_some());
        assert!(t.lookup(&inner_key).is_none());
    }

    #[test]
    fn pop_scope_preserves_root_scope() {
        let mut t = ScopedValueTable::new();
        t.pop_scope(); // Should be a no-op; root always survives.
        t.pop_scope();
        assert_eq!(t.scope_depth(), 1);
    }

    #[test]
    fn lookup_shadows_outer_scope_first() {
        let mut t = ScopedValueTable::new();
        let key: ExprKey = vec!["call".into(), "f".into()];
        t.insert(entry(&["call", "f"], "outer", 0));
        t.push_scope();
        t.insert(entry(&["call", "f"], "inner", 5));
        assert_eq!(t.lookup(&key).unwrap().block, "inner");
        t.pop_scope();
        assert_eq!(t.lookup(&key).unwrap().block, "outer");
    }

    #[test]
    fn kill_all_drops_every_scope() {
        let mut t = ScopedValueTable::new();
        t.insert(entry(&["call", "a"], "b", 0));
        t.push_scope();
        t.insert(entry(&["call", "b"], "b", 1));
        t.kill_all();
        // `kill_all` drops every entry but preserves the scope-stack depth, so
        // the push/pop discipline survives. Collapsing the
        // stack would strand later inserts in a never-popped root scope.
        assert_eq!(t.scope_depth(), 2);
        assert_eq!(t.total_entries(), 0);
    }

    #[test]
    fn kill_all_mid_branch_does_not_leak_to_sibling() {
        // replicated at the table level exactly as the
        // dominator walk drives it: enter block, enter then-arm, a state write
        // kills, record an occurrence, leave the then-arm, enter the else-arm.
        // The then-arm's occurrence must not be visible in the sibling.
        let key: ExprKey = vec!["call".into(), "llength".into(), "$lst".into()];
        let mut t = ScopedValueTable::new(); // root, depth 1
        t.push_scope(); // enter condition block, depth 2
        t.push_scope(); // enter then-arm, depth 3
        t.kill_all(); // a state write invalidates everything
        t.insert(entry(&["call", "llength", "$lst"], "then", 0));
        t.pop_scope(); // leave then-arm → depth 2, entry dropped
        t.push_scope(); // enter else-arm, depth 3
        assert!(
            t.lookup(&key).is_none(),
            "then-arm occurrence leaked into the else-arm after kill_all",
        );
    }

    #[test]
    fn redundant_computation_constructor() {
        let r = RedundantComputation::new(
            Span::new(10, 20),
            Span::new(0, 5),
            "llength $x",
            DiagCode::O105,
            "message",
        );
        assert_eq!(r.expression_text, "llength $x");
        assert_eq!(r.code, DiagCode::O105);
        assert_eq!(r.span.start(), 10);
        assert_eq!(r.first_span.end(), 5);
    }

    // -- canonicalisation + messages --

    /// A bare SSA shell (no blocks) for tests that only need the variable
    /// interner to resolve canonicalisation names.
    fn bare_ssa() -> SsaFunction {
        SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
    }

    #[test]
    fn canonicalise_empty_uses_returns_input() {
        let ssa = bare_ssa();
        let uses = HashMap::new();
        assert_eq!(canonicalise_word("foo", &uses, &ssa, STYLE), "foo");
    }

    #[test]
    fn canonicalise_replaces_bare_and_braced() {
        let mut ssa = bare_ssa();
        let mut uses = HashMap::new();
        uses.insert(ssa.intern_var("x"), 3);
        assert_eq!(canonicalise_word("$x", &uses, &ssa, STYLE), "$x@3");
        assert_eq!(canonicalise_word("${x}", &uses, &ssa, STYLE), "$x@3");
    }

    /// Issue #1604 — the `${…}` closer follows the release rule, so a
    /// nested-brace name is *versioned* rather than copied through verbatim.
    ///
    /// The verbatim copy is the unsound half: two occurrences of `${a{b}c}` at
    /// different SSA versions both render to the same literal text, so their
    /// `ExprKey`s match and GVN reports the second as redundant — a CSE that
    /// reuses a stale value. Oracle: `set {a{b}c} 7; subst {${a{b}c}}` is `7`
    /// on tclsh 9.0.4 and `can't read "a{b"` on 8.6.16.
    #[test]
    fn canonicalise_braced_var_follows_the_release_rule() {
        use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
        let mut ssa = bare_ssa();
        let mut uses = HashMap::new();
        let nested = ssa.intern_var("a{b}c");
        let first = ssa.intern_var("a{b");
        uses.insert(nested, 4);
        uses.insert(first, 9);

        // 9.x: the whole name is `a{b}c`, so the reference carries its version.
        assert_eq!(
            canonicalise_word("${a{b}c}", &uses, &ssa, Tcl9Nesting),
            "$a{b}c@4"
        );
        // 8.x: the name ends at the first `}` and `c}` is ordinary word text.
        assert_eq!(
            canonicalise_word("${a{b}c}", &uses, &ssa, FirstClose),
            "$a{b@9c}"
        );

        // Two versions of the same nested name must not canonicalise alike —
        // the property the unsound CSE turned on.
        let mut other = HashMap::new();
        other.insert(nested, 5);
        assert_ne!(
            canonicalise_word("${a{b}c}", &uses, &ssa, Tcl9Nesting),
            canonicalise_word("${a{b}c}", &other, &ssa, Tcl9Nesting),
        );
    }

    #[test]
    fn canonicalise_sorts_by_name_length_desc() {
        // `$longname` must be replaced before `$long` so the
        // longer name is not partially matched.
        let mut ssa = bare_ssa();
        let mut uses = HashMap::new();
        uses.insert(ssa.intern_var("long"), 1);
        uses.insert(ssa.intern_var("longname"), 2);
        let out = canonicalise_word("$longname$long", &uses, &ssa, STYLE);
        assert_eq!(out, "$longname@2$long@1");
    }

    #[test]
    fn canonicalise_ignores_unmentioned_variables() {
        let ssa = bare_ssa();
        let uses = HashMap::new();
        assert_eq!(canonicalise_word("$x", &uses, &ssa, STYLE), "$x");
    }

    #[test]
    fn build_call_key_for_pure_command() {
        let mut ssa = bare_ssa();
        let mut uses = HashMap::new();
        uses.insert(ssa.intern_var("x"), 3);
        let args = vec!["$x".into(), "literal".into()];
        let key = build_call_key("llength", &args, &uses, &ssa, STYLE);
        assert_eq!(
            key,
            vec![
                "call".to_string(),
                "llength".into(),
                "$x@3".into(),
                "literal".into()
            ]
        );
    }

    #[test]
    fn format_expression_text_no_args() {
        let args: Vec<String> = Vec::new();
        assert_eq!(format_expression_text("clock", &args), "clock");
    }

    #[test]
    fn format_expression_text_with_args() {
        let args: Vec<String> = vec!["$x".into(), "literal".into()];
        assert_eq!(
            format_expression_text("llength", &args),
            "llength $x literal"
        );
    }

    #[test]
    fn message_builders_include_expression_text() {
        assert!(full_redundancy_message("llength $x").contains("llength $x"));
        assert!(full_redundancy_message("llength $x").contains("local variable"));
        assert!(partial_redundancy_message("dict get $d k").contains("partially redundant"));
        assert!(loop_invariant_message("expr {$x + 1}").contains("loop-invariant"));
    }

    // -- statement-level helpers --

    fn call_stmt(cmd: &str, args: &[&str]) -> Statement {
        Statement::Call {
            span: Span::new(0, 0),
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

    #[test]
    fn is_pure_command_picks_up_registry_pure_trait() {
        let registry = CommandRegistry::build_default();
        // `expr` is marked PURE / PURE_EVALUATION in the registry.
        assert!(is_pure_command(
            &registry,
            "expr",
            &["{1 + 2}".into()],
            None
        ));
        // `set` is variable-assigning — not pure.
        assert!(!is_pure_command(
            &registry,
            "set",
            &["x".into(), "1".into()],
            None
        ));
    }

    #[test]
    fn is_pure_command_recurses_into_impure_substitution() {
        // O106: `expr` is pure, but an arg that embeds
        // an impure `[incr c]` substitution makes the whole invocation
        // impure — its value depends on the per-call side effect, so it
        // must not be hoisted as loop-invariant.
        let registry = CommandRegistry::build_default();
        assert!(
            !is_pure_command(&registry, "expr", &["[incr c]".into()], None),
            "outer pure `expr` wrapping impure `[incr c]` must be impure",
        );
        // A pure nested substitution keeps the outer call pure.
        assert!(
            is_pure_command(&registry, "expr", &["[expr {1 + 1}]".into()], None),
            "outer pure `expr` wrapping a pure subst stays pure",
        );
        // Deeper nesting: the impurity is two levels down.
        assert!(
            !is_pure_command(&registry, "expr", &["[expr [incr c]]".into()], None),
            "nested impure subst at any depth must propagate",
        );
    }

    /// A pure command (`expr`) gates to non-pure once its
    /// canonical name appears in `traced_commands`.
    #[test]
    fn is_pure_command_with_traces_traced_command_not_pure() {
        let registry = CommandRegistry::build_default();
        let mut traced = std::collections::BTreeSet::new();
        traced.insert("foo".to_string());
        // `foo` is unknown to the registry but in the trace set —
        // never pure.
        assert!(!is_pure_command_with_traces(
            &registry,
            "foo",
            &["arg".into()],
            None,
            &traced,
            false,
        ));
    }

    /// `has_dynamic_trace=true` widens the gate to *every*
    /// command (even ones the registry marks PURE).
    #[test]
    fn is_pure_command_with_traces_dynamic_trace_inhibits_all() {
        let registry = CommandRegistry::build_default();
        let traced = std::collections::BTreeSet::new();
        // `expr` is PURE per the registry, but `has_dynamic_trace=true`
        // means any call could have a trace registered at runtime.
        assert!(!is_pure_command_with_traces(
            &registry,
            "expr",
            &["{1 + 2}".into()],
            None,
            &traced,
            true,
        ));
    }

    /// Regression — untraced pure commands stay pure.
    #[test]
    fn is_pure_command_with_traces_untraced_command_still_pure() {
        let registry = CommandRegistry::build_default();
        let traced = std::collections::BTreeSet::new();
        assert!(is_pure_command_with_traces(
            &registry,
            "expr",
            &["{1 + 2}".into()],
            None,
            &traced,
            false,
        ));
    }

    /// `::ns::foo` and `ns::foo` look up identically in the
    /// trace set (mirrors `populate_trace_facts`'s `::`-strip).
    #[test]
    fn is_pure_command_with_traces_qualified_name_canonicalised() {
        let registry = CommandRegistry::build_default();
        let mut traced = std::collections::BTreeSet::new();
        traced.insert("ns::foo".to_string());
        assert!(!is_pure_command_with_traces(
            &registry,
            "::ns::foo",
            &[],
            None,
            &traced,
            false,
        ));
        assert!(!is_pure_command_with_traces(
            &registry,
            "ns::foo",
            &[],
            None,
            &traced,
            false,
        ));
    }

    #[test]
    fn statement_writes_state_true_for_barrier() {
        let registry = CommandRegistry::build_default();
        let barrier = Statement::Barrier {
            span: Span::new(0, 0),
            reason: "eval".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["script".into()],
            tokens: None,
        };
        assert!(statement_writes_state(&registry, &barrier, None));
    }

    #[test]
    fn statement_writes_state_true_for_global_write() {
        let registry = CommandRegistry::build_default();
        // Global-scope `set` writes GLOBAL_STATE — must invalidate.
        let set_global = call_stmt("set", &["::foo::bar", "1"]);
        assert!(statement_writes_state(&registry, &set_global, None));
    }

    #[test]
    fn statement_writes_state_false_for_proc_local_set() {
        let registry = CommandRegistry::build_default();
        // Proc-local `set x 1` only writes the local — no region.
        let set_local = call_stmt("set", &["x", "1"]);
        assert!(!statement_writes_state(&registry, &set_local, None));
    }

    #[test]
    fn statement_writes_state_false_for_pure_expr() {
        let registry = CommandRegistry::build_default();
        let expr_stmt = call_stmt("expr", &["{1 + 2}"]);
        assert!(!statement_writes_state(&registry, &expr_stmt, None));
    }

    #[test]
    fn statement_occurrences_skips_impure_commands() {
        let registry = CommandRegistry::build_default();
        // `set` is impure — statement_occurrences emits nothing.
        let stmt_ssa = SsaStatement {
            statement: call_stmt("set", &["x", "1"]),
            uses: HashMap::new(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let occurrences =
            statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
        assert!(occurrences.is_empty());
    }

    #[test]
    fn statement_occurrences_emits_nothing_for_non_call_stmts() {
        let registry = CommandRegistry::build_default();
        let stmt_ssa = SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            uses: HashMap::new(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        assert!(
            statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa()).is_empty()
        );
    }

    #[test]
    fn is_worth_reporting_unknown_command_is_false() {
        let registry = CommandRegistry::build_default();
        assert!(!is_worth_reporting(&registry, "__nonexistent"));
    }

    // -- find_redundancies driver --

    use crate::cfg::{Block, Function, Terminator};
    use crate::ssa::{SsaBlock, SsaStatement};
    use std::collections::HashMap as Map;

    /// The [`BlockId`] a block name was interned to in `cfg`.
    fn id_of(cfg: &Function, name: &str) -> BlockId {
        cfg.block_id(name).expect("interned")
    }

    fn llength_call() -> Statement {
        llength_call_at(0, 0)
    }

    fn llength_call_at(start: u32, end: u32) -> Statement {
        Statement::Call {
            span: Span::new(start, end),
            command: "llength".into(),
            canonical_command: None,
            args: vec!["$x".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    fn ssa_stmt_for(
        ssa: &mut SsaFunction,
        stmt: Statement,
        uses_x_ver: Option<u32>,
    ) -> SsaStatement {
        let mut uses = Map::new();
        if let Some(v) = uses_x_ver {
            uses.insert(ssa.intern_var("x"), v);
        }
        SsaStatement {
            statement: stmt,
            uses,
            defs: Map::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
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

    /// Every block of a hand-built CFG, as the executable set. These tests
    /// exercise the loop / availability machinery on CFGs with no SCCP run,
    /// so they hand it the whole CFG — the pre-#1385 premise, stated at the
    /// call site instead of derived inside the pass.
    fn all_blocks(cfg: &Function) -> std::collections::HashSet<BlockId> {
        cfg.blocks.keys().copied().collect()
    }

    #[test]
    fn find_redundancies_detects_same_block_duplicate() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
        entry_blk.statements.push(llength_call());
        entry_blk.statements.push(llength_call());
        entry_blk.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.dominator_tree.insert(cfg.entry, Vec::new());
        let mut ssa_entry = empty_ssa_block("entry");
        ssa_entry
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa_entry
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa.blocks.insert(cfg.entry, ssa_entry);

        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].code, DiagCode::O105);
        assert!(results[0].expression_text.contains("llength"));
    }

    #[test]
    fn find_redundancies_ignores_different_ssa_version() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
        entry_blk.statements.push(llength_call());
        entry_blk.statements.push(llength_call());
        entry_blk.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.dominator_tree.insert(cfg.entry, Vec::new());
        let mut ssa_entry = empty_ssa_block("entry");
        // Same expression but different SSA versions of $x → no
        // redundancy.
        ssa_entry
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa_entry
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(2)));
        ssa.blocks.insert(cfg.entry, ssa_entry);

        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert!(results.is_empty());
    }

    #[test]
    fn find_redundancies_global_write_invalidates_cache() {
        let registry = CommandRegistry::build_default();
        // entry: llength $x; set ::g 1; llength $x
        let mut cfg = Function::new("::top", "entry");
        let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
        entry_blk.statements.push(llength_call());
        entry_blk.statements.push(Statement::Call {
            span: Span::new(0, 0),
            command: "set".into(),
            canonical_command: None,
            args: vec!["::g".into(), "1".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });
        entry_blk.statements.push(llength_call());
        entry_blk.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.dominator_tree.insert(cfg.entry, Vec::new());
        let mut ssa_entry = empty_ssa_block("entry");
        ssa_entry
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa_entry.statements.push(ssa_stmt_for(
            &mut ssa,
            cfg.blocks[&cfg.entry].statements[1].clone(),
            None,
        ));
        ssa_entry
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa.blocks.insert(cfg.entry, ssa_entry);

        // The global write should invalidate — no redundancy reported.
        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert!(results.is_empty());
    }

    #[test]
    fn find_redundancies_descends_dominator_tree() {
        let registry = CommandRegistry::build_default();
        // entry: llength $x
        // dom_child: llength $x   (should trigger)
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let child = cfg.intern_block("child");
        cfg.blocks.insert(child, Block::new("child"));
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Goto {
            target: child,
            span: None,
        });
        cfg.blocks
            .get_mut(&child)
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut(&child).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.dominator_tree.insert(entry, vec![child]);
        ssa.dominator_tree.insert(child, Vec::new());
        let mut e = empty_ssa_block("entry");
        e.statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        let mut c = empty_ssa_block("child");
        c.statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa.blocks.insert(entry, e);
        ssa.blocks.insert(child, c);

        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert_eq!(results.len(), 1);
    }

    // -- embedded command-substitution scanning --

    #[test]
    fn scan_bracketed_commands_simple() {
        let out = scan_bracketed_commands("[llength $x]");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "llength");
        assert_eq!(out[0].1, vec!["$x".to_string()]);
    }

    #[test]
    fn scan_bracketed_commands_nested() {
        let out = scan_bracketed_commands("[set v [lindex $x 0]]");
        // Outer pair + inner pair.
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "set");
        assert_eq!(out[1].0, "lindex");
    }

    #[test]
    fn scan_bracketed_commands_skips_braced_regions() {
        let out = scan_bracketed_commands("{[not a command]}");
        assert!(out.is_empty());
    }

    #[test]
    fn scan_bracketed_commands_handles_backslash_escape() {
        // `\[` shouldn't start a region.
        let out = scan_bracketed_commands("hello \\[world]");
        assert!(out.is_empty());
    }

    #[test]
    fn split_cmd_text_braced_and_quoted_words() {
        let (cmd, args) = split_cmd_text("list {a b c} \"d e\" f").unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(
            args,
            vec!["{a b c}".to_string(), "\"d e\"".into(), "f".into()]
        );
    }

    #[test]
    fn statement_occurrences_includes_embedded_pure_cmd() {
        let registry = CommandRegistry::build_default();
        // set x [llength $y] — an impure `set` with an embedded
        // pure `llength`. The top-level Call is skipped (impure),
        // but the embedded `llength` should be reported.
        let stmt_ssa = SsaStatement {
            statement: Statement::Call {
                span: Span::new(0, 0),
                command: "set".into(),
                canonical_command: None,
                args: vec!["x".into(), "[llength $y]".into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            },
            uses: HashMap::new(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].expression_text, "llength $y");
    }

    #[test]
    fn statement_occurrences_includes_embedded_in_assign_value() {
        let registry = CommandRegistry::build_default();
        let stmt_ssa = SsaStatement {
            statement: Statement::AssignValue {
                span: Span::new(0, 0),
                name: "len".into(),
                name_braced: false,
                value: "[llength $y]".into(),
                value_needs_backsubst: false,
                tokens: None,
            },
            uses: HashMap::new(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].expression_text, "llength $y");
    }

    #[test]
    fn find_redundancies_empty_for_no_blocks() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        // Intern a name with no matching block so the SSA entry is absent
        // from `cfg.blocks` and the walk early-returns.
        let nonexistent = cfg.intern_block("nonexistent");
        let ssa = SsaFunction::trivial(cfg.name.clone(), nonexistent, cfg.block_names().to_vec());
        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert!(results.is_empty());
    }

    // -- loop-invariant detection --

    #[test]
    fn find_loop_invariants_detects_hoistable_llength() {
        let registry = CommandRegistry::build_default();
        // header: branch on $i < 10 → body → header (back edge)
        // body: llength $x (invariant w.r.t. loop-defined $i)
        let mut cfg = Function::new("::top", "header");
        let header = cfg.entry;
        let body = cfg.intern_block("body");
        let exit = cfg.intern_block("exit");
        cfg.blocks.insert(body, Block::new("body"));
        cfg.blocks.insert(exit, Block::new("exit"));
        cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            true_target: body,
            false_target: exit,
            span: None,
            condition_base: None,
        });
        cfg.blocks
            .get_mut(&body)
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
            target: header,
            span: None,
        });
        cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        // header dominates body and exit.
        ssa.idom.insert(header, None);
        ssa.idom.insert(body, Some(header));
        ssa.idom.insert(exit, Some(header));
        ssa.dominator_tree.insert(header, vec![body, exit]);
        ssa.dominator_tree.insert(body, Vec::new());
        ssa.dominator_tree.insert(exit, Vec::new());

        let h = empty_ssa_block("header");
        let mut b = empty_ssa_block("body");
        // $x's SSA version comes from outside the loop (x@1).
        b.statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa.blocks.insert(header, h);
        ssa.blocks.insert(body, b);
        ssa.blocks.insert(exit, empty_ssa_block("exit"));

        let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
        assert_eq!(results.len(), 1);
        // Canonical: loop-invariant code motion is O106 (not O107,
        // which is unreachable dead code).
        assert_eq!(results[0].code, DiagCode::O106);
        assert!(results[0].expression_text.contains("llength"));
    }

    #[test]
    fn find_loop_invariants_skips_branch_that_does_not_dominate_latch() {
        // Loop:  header → cond → {then, latch}; then → latch; latch → header.
        // The invariant `llength $x` sits in `then`, which does NOT dominate
        // the latch (cond can reach the latch directly), so it runs only on
        // some iterations — the latch-dominance gate must suppress O106.
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "header");
        let header = cfg.entry;
        for b in ["cond", "then", "latch", "exit"] {
            let id = cfg.intern_block(b);
            cfg.blocks.insert(id, Block::new(b));
        }
        let cond = id_of(&cfg, "cond");
        let then = id_of(&cfg, "then");
        let latch = id_of(&cfg, "latch");
        let exit = id_of(&cfg, "exit");
        cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            true_target: cond,
            false_target: exit,
            span: None,
            condition_base: None,
        });
        cfg.blocks.get_mut(&cond).unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            true_target: then,
            false_target: latch,
            span: None,
            condition_base: None,
        });
        cfg.blocks
            .get_mut(&then)
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
            target: latch,
            span: None,
        });
        cfg.blocks.get_mut(&latch).unwrap().terminator = Some(Terminator::Goto {
            target: header,
            span: None,
        });
        cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(header, None);
        ssa.idom.insert(cond, Some(header));
        ssa.idom.insert(then, Some(cond));
        // The latch is reachable from `cond` both directly and via `then`,
        // so its immediate dominator is `cond`, not `then`.
        ssa.idom.insert(latch, Some(cond));
        ssa.idom.insert(exit, Some(header));
        ssa.dominator_tree.insert(header, vec![cond, exit]);
        ssa.dominator_tree.insert(cond, vec![then, latch]);
        for b in [then, latch, exit] {
            ssa.dominator_tree.insert(b, Vec::new());
        }

        ssa.blocks.insert(header, empty_ssa_block("header"));
        ssa.blocks.insert(cond, empty_ssa_block("cond"));
        let mut then_b = empty_ssa_block("then");
        then_b
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa.blocks.insert(then, then_b);
        ssa.blocks.insert(latch, empty_ssa_block("latch"));
        ssa.blocks.insert(exit, empty_ssa_block("exit"));

        let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
        assert!(
            results.is_empty(),
            "invariant behind an in-loop branch must not be hoisted, got {results:?}",
        );
    }

    #[test]
    fn find_loop_invariants_skips_loop_defined_var() {
        let registry = CommandRegistry::build_default();
        // As above, but `$i` is defined inside the loop — llength
        // uses `$i` → not loop-invariant.
        let mut cfg = Function::new("::top", "header");
        let header = cfg.entry;
        let body = cfg.intern_block("body");
        let exit = cfg.intern_block("exit");
        cfg.blocks.insert(body, Block::new("body"));
        cfg.blocks.insert(exit, Block::new("exit"));
        cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            true_target: body,
            false_target: exit,
            span: None,
            condition_base: None,
        });
        let llength_on_i = Statement::Call {
            span: Span::new(0, 0),
            command: "llength".into(),
            canonical_command: None,
            args: vec!["$i".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        cfg.blocks
            .get_mut(&body)
            .unwrap()
            .statements
            .push(llength_on_i.clone());
        cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
            target: header,
            span: None,
        });
        cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(header, None);
        ssa.idom.insert(body, Some(header));
        ssa.idom.insert(exit, Some(header));

        let h = empty_ssa_block("header");
        let mut b = empty_ssa_block("body");
        // `$i` defined in body.
        let mut defs = Map::new();
        defs.insert(ssa.intern_var("i"), 1);
        b.statements.push(SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            },
            uses: Map::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        });
        // llength $i — uses map tracks `i`, not `x`.
        let mut uses_i = Map::new();
        uses_i.insert(ssa.intern_var("i"), 1);
        b.statements.push(SsaStatement {
            statement: llength_on_i,
            uses: uses_i,
            defs: Map::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        });
        ssa.blocks.insert(header, h);
        ssa.blocks.insert(body, b);
        ssa.blocks.insert(exit, empty_ssa_block("exit"));

        let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
        assert!(results.is_empty());
    }

    #[test]
    fn find_loop_invariants_no_loops_empty() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(cfg.entry, None);
        ssa.blocks.insert(cfg.entry, empty_ssa_block("entry"));
        assert!(find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None).is_empty());
    }

    // -- partial-redundancy detection --

    #[test]
    fn find_partial_redundancies_if_diamond() {
        // Diamond:
        //   entry → tt (computes llength $x) → join
        //         → ff → join (computes llength $x again)
        // `llength $x` is may-available at join (came via tt), but
        // not must-available (ff didn't compute it). On the join
        // occurrence, report O106 — consider hoisting before the
        // branch.
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let tt = cfg.intern_block("tt");
        let ff = cfg.intern_block("ff");
        let join = cfg.intern_block("join");
        cfg.blocks.insert(tt, Block::new("tt"));
        cfg.blocks.insert(ff, Block::new("ff"));
        cfg.blocks.insert(join, Block::new("join"));
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Var {
                text: "$c".into(),
                name: "c".into(),
                start: 0,
                end: 2,
            },
            true_target: tt,
            false_target: ff,
            span: None,
            condition_base: None,
        });
        cfg.blocks
            .get_mut(&tt)
            .unwrap()
            .statements
            .push(llength_call_at(100, 110));
        cfg.blocks.get_mut(&tt).unwrap().terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        cfg.blocks.get_mut(&ff).unwrap().terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        cfg.blocks
            .get_mut(&join)
            .unwrap()
            .statements
            .push(llength_call_at(200, 210));
        cfg.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(entry, None);
        ssa.idom.insert(tt, Some(entry));
        ssa.idom.insert(ff, Some(entry));
        ssa.idom.insert(join, Some(entry));

        let entry_b = empty_ssa_block("entry");
        let mut tt_b = empty_ssa_block("tt");
        tt_b.statements
            .push(ssa_stmt_for(&mut ssa, llength_call_at(100, 110), Some(1)));
        let ff_b = empty_ssa_block("ff");
        let mut join_b = empty_ssa_block("join");
        join_b
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call_at(200, 210), Some(1)));
        ssa.blocks.insert(entry, entry_b);
        ssa.blocks.insert(tt, tt_b);
        ssa.blocks.insert(ff, ff_b);
        ssa.blocks.insert(join, join_b);

        let results = find_partial_redundancies(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
        assert_eq!(results.len(), 1);
        // Canonical: partial redundancy (GVN-PRE) shares O105 with
        // full redundancy — the loop-invariant code O106 was wrong
        // here.
        assert_eq!(results[0].code, DiagCode::O105);
        assert_eq!(results[0].span.start(), 200);
        assert_eq!(results[0].first_span.start(), 100);
    }

    #[test]
    fn find_partial_redundancies_none_on_linear_chain() {
        let registry = CommandRegistry::build_default();
        // Single-block function with one llength — no partial
        // redundancy possible.
        let mut cfg = Function::new("::top", "entry");
        cfg.blocks
            .get_mut(&cfg.entry)
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(cfg.entry, None);
        let mut entry_b = empty_ssa_block("entry");
        entry_b
            .statements
            .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
        ssa.blocks.insert(cfg.entry, entry_b);

        assert!(
            find_partial_redundancies(&registry, &cfg, &ssa, &all_blocks(&cfg), None).is_empty()
        );
    }

    #[test]
    fn transfer_occurrence_keys_kill_clears_state() {
        let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
        let occ = ExprOccurrence {
            key: key.clone(),
            span: Span::new(0, 0),
            expression_text: String::new(),
            block: String::new(),
            statement_index: 0,
            variable_uses: Vec::new(),
        };
        let events = vec![OccurrenceEvent::Occur(occ), OccurrenceEvent::Kill];
        let out = transfer_occurrence_keys(&events, FxHashSet::default());
        assert!(!out.contains(&key));
    }

    // -- intra-module pure-proc analysis --

    fn module_with_procs(procs: Vec<(&str, Vec<Statement>)>) -> CfgModule {
        let mut cfg_module = CfgModule {
            top_level: Function::new("::top", "entry"),
            procedures: Map::new(),
        };
        for (name, stmts) in procs {
            let mut cfg = Function::new(name, "entry");
            let entry = cfg.entry;
            for s in stmts {
                cfg.blocks.get_mut(&entry).unwrap().statements.push(s);
            }
            cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            });
            cfg_module.procedures.insert(name.into(), cfg);
        }
        cfg_module
    }

    #[test]
    fn find_pure_procs_identifies_empty_body() {
        let registry = CommandRegistry::build_default();
        let m = module_with_procs(vec![("::empty", Vec::new())]);
        let result = find_pure_procs(&registry, &m, None);
        assert!(result.contains("::empty"));
    }

    #[test]
    fn find_pure_procs_identifies_pure_body() {
        let registry = CommandRegistry::build_default();
        let m = module_with_procs(vec![(
            "::pure",
            vec![
                Statement::AssignConst {
                    span: Span::new(0, 0),
                    name: "x".into(),
                    name_braced: false,
                    value: "1".into(),
                    value_span: None,
                },
                Statement::Return {
                    span: Span::new(0, 0),
                    value: Some("$x".into()),
                    expr: None,
                    braced: false,
                },
            ],
        )]);
        let result = find_pure_procs(&registry, &m, None);
        assert!(result.contains("::pure"));
    }

    #[test]
    fn find_pure_procs_rejects_body_with_barrier() {
        let registry = CommandRegistry::build_default();
        let m = module_with_procs(vec![(
            "::impure",
            vec![Statement::Barrier {
                span: Span::new(0, 0),
                reason: "eval".into(),
                command: "eval".into(),
                canonical_command: None,
                args: vec!["script".into()],
                tokens: None,
            }],
        )]);
        let result = find_pure_procs(&registry, &m, None);
        assert!(!result.contains("::impure"));
    }

    #[test]
    fn find_pure_procs_rejects_global_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_procs(vec![(
            "::writes_global",
            vec![Statement::Call {
                span: Span::new(0, 0),
                command: "set".into(),
                canonical_command: None,
                args: vec!["::g".into(), "1".into()],
                defs: vec!["::g".into()],
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }],
        )]);
        let result = find_pure_procs(&registry, &m, None);
        assert!(!result.contains("::writes_global"));
    }

    #[test]
    fn find_pure_procs_mutual_recursion_stays_pure() {
        let registry = CommandRegistry::build_default();
        let call_b = Statement::Call {
            span: Span::new(0, 0),
            command: "::b".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let call_a = Statement::Call {
            span: Span::new(0, 0),
            command: "::a".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let m = module_with_procs(vec![("::a", vec![call_b]), ("::b", vec![call_a])]);
        let result = find_pure_procs(&registry, &m, None);
        assert!(result.contains("::a"));
        assert!(result.contains("::b"));
    }

    #[test]
    fn find_pure_procs_impurity_propagates_to_callers() {
        let registry = CommandRegistry::build_default();
        let barrier = Statement::Barrier {
            span: Span::new(0, 0),
            reason: "eval".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["$x".into()],
            tokens: None,
        };
        let call_tainted = Statement::Call {
            span: Span::new(0, 0),
            command: "::tainted".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let m = module_with_procs(vec![
            ("::tainted", vec![barrier]),
            ("::wraps", vec![call_tainted]),
        ]);
        let result = find_pure_procs(&registry, &m, None);
        assert!(!result.contains("::tainted"));
        assert!(!result.contains("::wraps"));
    }

    #[test]
    fn is_pure_with_procs_accepts_user_proc() {
        let registry = CommandRegistry::build_default();
        let mut pp = PureProcs::new();
        pp.names.insert("::pure_helper".into());
        assert!(is_pure_with_procs(
            &registry,
            &pp,
            "::pure_helper",
            &[],
            None
        ));
        assert!(is_pure_with_procs(&registry, &pp, "pure_helper", &[], None));
        assert!(is_pure_with_procs(
            &registry,
            &pp,
            "expr",
            &["{1+2}".into()],
            None
        ));
        assert!(!is_pure_with_procs(
            &registry,
            &pp,
            "set",
            &["x".into(), "1".into()],
            None
        ));
    }

    #[test]
    fn is_worth_reporting_with_procs_accepts_pure_user_proc() {
        let registry = CommandRegistry::build_default();
        let mut pp = PureProcs::new();
        pp.names.insert("::pure_helper".into());
        assert!(is_worth_reporting_with_procs(
            &registry,
            &pp,
            "::pure_helper"
        ));
        assert!(is_worth_reporting_with_procs(&registry, &pp, "llength"));
        assert!(!is_worth_reporting_with_procs(
            &registry,
            &pp,
            "nonexistent_cmd"
        ));
    }

    #[test]
    fn dominates_trivial_and_chain() {
        // Intern the chain through a CFG so the SSA shares its BlockIds.
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let a = cfg.intern_block("a");
        let b = cfg.intern_block("b");
        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(entry, None);
        ssa.idom.insert(a, Some(entry));
        ssa.idom.insert(b, Some(a));
        let dom = ssa.dominator_intervals();
        assert!(dom.dominates(entry, b));
        assert!(dom.dominates(a, b));
        assert!(!dom.dominates(b, a));
        assert!(dom.dominates(b, b));
    }

    #[test]
    fn dominator_intervals_answer_siblings_and_unreachable() {
        // Issue #1250: the interval index must give the same answers the idom
        // chain walk gave — including "no", which is the case an interval
        // scheme can get wrong if the numbering is off by one.
        //
        //   entry → a → { b, c },  d unreachable (no idom entry)
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let a = cfg.intern_block("a");
        let b = cfg.intern_block("b");
        let c = cfg.intern_block("c");
        let d = cfg.intern_block("d");
        let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        ssa.idom.insert(entry, None);
        ssa.idom.insert(a, Some(entry));
        ssa.idom.insert(b, Some(a));
        ssa.idom.insert(c, Some(a));
        let dom = ssa.dominator_intervals();
        for node in [a, b, c] {
            assert!(dom.dominates(entry, node), "entry must dominate {node:?}");
            assert!(dom.dominates(node, node), "{node:?} dominates itself");
        }
        assert!(dom.dominates(a, b) && dom.dominates(a, c));
        assert!(!dom.dominates(b, c) && !dom.dominates(c, b));
        assert!(!dom.dominates(b, a) && !dom.dominates(c, entry));
        // A block outside the dominator forest dominates only itself.
        assert!(dom.dominates(d, d));
        assert!(!dom.dominates(entry, d) && !dom.dominates(d, entry));
    }

    #[test]
    fn expr_occurrence_carries_variable_uses() {
        let occ = ExprOccurrence {
            key: vec!["call".into(), "llength".into(), "$x@1".into()],
            span: Span::new(0, 5),
            expression_text: "llength $x".into(),
            block: "entry".into(),
            statement_index: 0,
            variable_uses: vec!["x".into()],
        };
        assert_eq!(occ.variable_uses, vec!["x".to_string()]);
    }

    /// Exercise the explicitly legacy CFG-only classifier. Production
    /// [`FunctionUnit`](crate::compilation_unit::FunctionUnit) GVN deliberately
    /// does not scan embedded source strings when no exact semantic node exists.
    fn legacy_redundancies(src: &str) -> Vec<String> {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut out = Vec::new();
        for function in cu.analysable_functions() {
            out.extend(
                find_redundancies(&registry, &function.cfg, &function.ssa, None)
                    .into_iter()
                    .map(|redundancy| redundancy.expression_text),
            );
        }
        out.sort();
        out
    }

    #[test]
    fn detects_redundant_ensemble_subcommand_cse() {
        // `string length $a` is pure via its *subcommand* purity — the
        // second computation is redundant. Before subcommand-aware
        // side-effect classification this was missed (the ensemble call
        // looked like an unknown-write).
        assert_eq!(
            legacy_redundancies(
                "proc f {a} { set x [string length $a]\n set y [string length $a]\n return [expr {$x + $y}] }"
            ),
            vec!["string length $a".to_string()],
        );
        // Likewise `dict get`.
        assert_eq!(
            legacy_redundancies(
                "proc f {d} { set a [dict get $d k]\n set b [dict get $d k]\n return [list $a $b] }"
            ),
            vec!["dict get $d k".to_string()],
        );
    }

    #[test]
    fn mutating_ensemble_subcommand_is_not_cse() {
        // `dict set` mutates — never a redundant-computation candidate.
        assert!(
            legacy_redundancies("proc f {d} { dict set d k 1\n dict set d k 1\n return $d }")
                .is_empty()
        );
    }

    #[test]
    fn kill_all_does_not_leak_across_sibling_branches() {
        // A state-writing statement (a global write here) in
        // the then-arm calls `kill_all`. That must NOT collapse the scope
        // stack, or the then-arm's `[llength $lst]` occurrence strands in the
        // never-popped root scope and the else-arm's identical computation is
        // falsely reported redundant. The two arms are mutually exclusive
        // paths — nothing is redundant.
        assert!(
            legacy_redundancies(
                "proc f {c lst} { if {$c} { set ::g 1\n set a [llength $lst] } else { set b [llength $lst] }\n return 1 }"
            )
            .is_empty(),
            "llength on mutually-exclusive branches must not be flagged redundant",
        );
    }
    #[test]
    fn kill_all_preserves_within_path_detection() {
        // FP-guard for: the in-place clear must not disturb
        // ordinary same-path redundancy detection — two identical pure
        // computations with nothing between them are still redundant.
        assert_eq!(
            legacy_redundancies(
                "proc f {lst} { set a [llength $lst]\n set b [llength $lst]\n return [list $a $b] }"
            ),
            vec!["llength $lst".to_string()],
        );
    }

    /// Production function-level GVN over one Tcl 9.0 top-level script.
    ///
    /// Every acceptance script below is straight-line top-level source, so the
    /// executable semantic sidecar must exist; a decline there would make an
    /// empty result meaningless.
    fn stable_call_findings(registry: &CommandRegistry, source: &str) -> Vec<RedundantComputation> {
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert!(
            matches!(
                cu.top_level.semantic_facts.executable(),
                ExecutableAnalysisAvailability::Available(_)
            ),
            "executable semantic facts must be available for: {source}"
        );
        find_redundancies_for_function(
            registry,
            &cu.top_level,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()),
        )
    }

    /// How many stable-call reuse findings one top-level script produces.
    fn stable_call_reports(registry: &CommandRegistry, source: &str) -> usize {
        let findings = stable_call_findings(registry, source);
        assert!(
            findings
                .iter()
                .all(|finding| finding.code == DiagCode::O105),
            "stable-call reuse must only ever report O105: {source}"
        );
        findings.len()
    }

    /// Two repeated `llength` calls with no interposed statement.
    const PLAIN_REPEATED_LLENGTH: &str = "llength {a b}\nllength {a b}";

    #[test]
    fn literal_execution_trace_removal_restores_stable_call_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "trace add execution llength enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            0,
            "control: the live enter trace observes both invocations"
        );
        assert_eq!(
            stable_call_reports(
                &registry,
                "trace add execution llength enter observe\n\
                 trace remove execution llength enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            1,
            "an exactly matching literal removal retires the registration on the success edge"
        );
    }

    #[test]
    fn dynamic_execution_trace_removal_abstains() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "set h observe\n\
                 trace add execution llength enter observe\n\
                 trace remove execution llength enter $h\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            0,
            "a dynamic removal prefix cannot be proven to match the live registration"
        );
        assert_eq!(
            stable_call_reports(
                &registry,
                "set h observe\n\
                 trace add execution llength enter observe\n\
                 trace remove execution llength enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            1,
            "control: the identical script with a literal removal prefix proves retirement"
        );
    }

    #[test]
    fn command_rename_trace_on_the_candidate_suppresses_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "trace add command llength rename observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            0,
            "a rename/delete trace on the candidate is a live command-trace observer"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the command trace restores reuse"
        );
    }

    #[test]
    fn step_execution_trace_on_an_unrelated_command_suppresses_every_site() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "trace add execution observe enterstep observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            0,
            "a step trace observes commands nested inside its target, so it is a global hazard"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the step trace restores reuse"
        );
    }

    #[test]
    fn non_step_execution_trace_on_an_unrelated_command_keeps_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "trace add execution observe enter observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            1,
            "an enter trace only observes its own target, so the candidate stays stable"
        );
    }

    #[test]
    fn renaming_the_candidate_command_suppresses_reuse() {
        let registry = CommandRegistry::build_default();
        // `rename` declares a trace-only current-interpreter callback barrier,
        // which the contents lattice discharges here because no trace is live.
        // What is left is the `CommandBinding(Move)` transition, which records
        // both subjects — so the candidate fails on CommandBinding alone.
        assert_eq!(
            stable_call_reports(
                &registry,
                "rename llength llen\nllength {a b}\nllength {a b}",
            ),
            0,
            "the candidate's binding is moved between the two invocations"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the rename restores reuse"
        );
    }

    #[test]
    fn renaming_an_unrelated_command_keeps_reuse_per_subject() {
        let registry = CommandRegistry::build_default();
        // The per-domain, per-subject contract: the identity ledger records
        // only `lreverse` and `lrev`, so a differently-named candidate keeps a
        // complete CommandBinding proof.
        assert_eq!(
            stable_call_reports(
                &registry,
                "rename lreverse lrev\nllength {a b}\nllength {a b}",
            ),
            1,
            "renaming another command cannot disturb the candidate's binding"
        );
        assert_eq!(
            stable_call_reports(
                &registry,
                "rename llength llen\nllength {a b}\nllength {a b}",
            ),
            0,
            "contrast: renaming the candidate itself does suppress"
        );
    }

    #[test]
    fn aliasing_over_the_candidate_command_suppresses_reuse() {
        let registry = CommandRegistry::build_default();
        // Two independent gates fail here: the `Alias` transition records the
        // candidate's own name, and the invocation's wildcard
        // interpreter-policy write marks current-interpreter policy unstable.
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp alias {} llength {} list\nllength {a b}\nllength {a b}",
            ),
            0,
            "an alias installed over the candidate rebinds the name being dispatched"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the alias restores reuse"
        );
    }

    #[test]
    fn aliasing_an_unrelated_name_abstains_on_the_declared_policy_write() {
        let registry = CommandRegistry::build_default();
        // Sound-conservative abstention rather than the per-subject report the
        // `Alias` transition alone would allow: `interp alias` additionally
        // declares an ordinary read-write of the whole current interpreter's
        // policy, and InterpreterPolicy is one of the candidate's irreducible
        // BASE dependencies.  Per-subject binding precision is proven instead
        // by `renaming_an_unrelated_command_keeps_reuse_per_subject`.
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp alias {} shortcut {} list\nllength {a b}\nllength {a b}",
            ),
            0,
            "the wildcard interpreter-policy write fails the candidate's policy proof"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the alias restores reuse"
        );
    }

    #[test]
    fn installing_an_alias_into_a_child_interpreter_abstains() {
        let registry = CommandRegistry::build_default();
        // `interp create child` alone keeps the site eligible (see
        // `creating_a_child_interpreter_only_invalidates_the_child_binding`),
        // so the topology change is not what suppresses.  The alias names
        // `child` as its source interpreter, which puts the transition outside
        // the current interpreter; the trace-only callback barrier therefore
        // cannot be discharged (foreign trace registrations are not tracked)
        // and clobbers every domain.  A conservative abstention, not a bug.
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp create child\n\
                 interp alias child foo {} list\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            0,
            "an alias reaching into a named child interpreter keeps its world barrier"
        );
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp create child\nllength {a b}\nllength {a b}",
            ),
            1,
            "control: dropping only the alias statement restores reuse"
        );
    }

    #[test]
    fn hiding_the_candidate_in_the_current_interpreter_suppresses_reuse() {
        let registry = CommandRegistry::build_default();
        // The empty interpreter path is Tcl's spelling of "this interpreter",
        // so the transition records the candidate's visible binding and marks
        // interpreter policy unstable — exactly the two domains llength needs.
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp hide {} llength\nllength {a b}\nllength {a b}",
            ),
            0,
            "hiding the candidate in the current interpreter unbinds the dispatched name"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the hide restores reuse"
        );
    }

    #[test]
    fn creating_a_safe_child_interpreter_keeps_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp create -safe jail\nllength {a b}\nllength {a b}",
            ),
            1,
            "a child interpreter's safety policy is not the current interpreter's policy"
        );
    }

    #[test]
    fn creating_a_child_interpreter_only_invalidates_the_child_binding() {
        let registry = CommandRegistry::build_default();
        // `interp create child` binds the command `child` in the current
        // interpreter and bumps interpreter topology.  The identity ledger
        // records only that subject, so an unrelated candidate stays provably
        // stable — the per-domain, per-subject invalidation contract.
        assert_eq!(
            stable_call_reports(
                &registry,
                "interp create child\nllength {a b}\nllength {a b}",
            ),
            1,
            "a fresh child-path binding cannot collide with the candidate's resolution"
        );
    }

    #[test]
    fn namespace_import_suppresses_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "namespace import ::tcl::mathfunc::*\nllength {a b}\nllength {a b}",
            ),
            0,
            "an import can shadow the candidate's resolution in the current namespace"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the import restores reuse"
        );
    }

    #[test]
    fn namespace_unknown_abstains_on_its_declared_lookup_and_policy_writes() {
        let registry = CommandRegistry::build_default();
        // llength's dispatch dependencies are the irreducible BASE set, which
        // excludes UnknownHandling, so the `SetUnknown` transition alone would
        // leave the site eligible.  The invocation additionally declares
        // ordinary wildcard footprint writes to NamespaceLookup and
        // InterpreterPolicy, and those two domains *are* in BASE — hence a
        // conservative abstention rather than the predicted report.
        assert_eq!(
            stable_call_reports(
                &registry,
                "namespace unknown handler\nllength {a b}\nllength {a b}",
            ),
            0,
            "the invocation's wildcard lookup and policy writes widen two BASE domains"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: removing the unknown-handler statement restores reuse"
        );
    }

    #[test]
    fn eval_source_and_package_barriers_suppress_reuse() {
        let registry = CommandRegistry::build_default();
        for barrier in ["eval {set x 1}", "source lib.tcl", "package require json"] {
            assert_eq!(
                stable_call_reports(
                    &registry,
                    &format!("{barrier}\nllength {{a b}}\nllength {{a b}}"),
                ),
                0,
                "an arbitrary-script barrier widens every dispatch domain: {barrier}"
            );
        }
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: without a barrier statement the same pair reuses"
        );
    }

    #[test]
    fn variable_read_trace_on_an_argument_variable_suppresses_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "set items {a b}\n\
                 trace add variable items read observe\n\
                 llength $items\n\
                 llength $items",
            ),
            0,
            "reading the traced variable runs arbitrary Tcl, so the operand word is inadmissible"
        );
        assert_eq!(
            stable_call_reports(&registry, "set items {a b}\nllength $items\nllength $items",),
            1,
            "control: with no variable trace the same argument reads are admissible"
        );
    }

    #[test]
    fn variable_trace_removal_restores_argument_variable_reuse() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(
                &registry,
                "set items {a b}\n\
                 trace add variable items read observe\n\
                 trace remove variable items read observe\n\
                 llength $items\n\
                 llength $items",
            ),
            1,
            "an exactly matching removal retires the read trace and re-admits the operand word"
        );
    }

    #[test]
    fn variable_write_trace_on_another_variable_keeps_reuse() {
        let registry = CommandRegistry::build_default();
        // The traced variable is written before the trace is installed and
        // never touched afterwards; the candidate's operands are literals, so
        // nothing can fire the trace between the two invocations.
        assert_eq!(
            stable_call_reports(
                &registry,
                "set other 1\n\
                 trace add variable other write observe\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            1,
            "a write trace on an untouched variable is not a hazard for literal operands"
        );
    }

    #[test]
    fn procedure_units_abstain_under_the_unknown_world_entry_assumption() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            "proc p {items} {\nllength $items\nllength $items\n}",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let procedure = cu.procedures.get("::p").expect("the proc unit is built");
        assert!(
            !find_redundancies(
                &registry,
                &procedure.cfg,
                &procedure.ssa,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .is_empty(),
            "the legacy classifier still recognises the repeated pure call"
        );
        assert!(
            find_redundancies_for_function(
                &registry,
                procedure,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .is_empty(),
            "a proc body runs after arbitrary interposed history, so every site proof fails closed"
        );
    }

    #[test]
    fn repeated_stable_calls_in_a_while_body_are_mapped() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            "while {$go} {\nllength {a b}\nllength {a b}\n}",
            &registry,
            false,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert!(
            !find_redundancies(
                &registry,
                &cu.top_level.cfg,
                &cu.top_level.ssa,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .is_empty(),
            "the explicit legacy API still sees the nested string-classified calls"
        );
        assert!(
            !find_redundancies_for_function(
                &registry,
                &cu.top_level,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
                )
            )
            .is_empty(),
            "a loop body is now executable-IR blocks with a real back edge, so \
             the two identical pure calls in one iteration are mapped"
        );
    }

    #[test]
    fn a_variable_store_write_between_candidates_keeps_reuse() {
        let registry = CommandRegistry::build_default();
        // The variable store is versioned but is not a dispatch domain, so an
        // untraced `set` between the two calls neither widens a ledger nor
        // changes a key fragment llength depends on.
        assert_eq!(
            stable_call_reports(&registry, "llength {a b}\nset x 1\nllength {a b}"),
            1,
            "an untraced variable write is not a dispatch-stability hazard"
        );
    }

    #[test]
    fn command_substitution_in_an_operand_word_fails_closed() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            stable_call_reports(&registry, "llength [list a b]\nllength [list a b]"),
            0,
            "a nested command substitution is an inadmissible operand word and a world barrier"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: the same calls with literal operands reuse"
        );
    }

    #[test]
    fn argument_expansion_fails_closed() {
        let registry = CommandRegistry::build_default();
        // Every dispatch domain is still proven stable here; expansion is
        // declined purely on operand-word admissibility, because it changes
        // the argv shape the registry facts were resolved for.
        assert_eq!(
            stable_call_reports(&registry, "set l {a b}\nllength {*}$l\nllength {*}$l"),
            0,
            "an expanded word is never an admissible operand"
        );
    }

    #[test]
    fn versioned_world_results_key_on_the_read_domain_version() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "test::versioned",
            traits: Traits::PURE | Traits::CSE_CANDIDATE,
            result_stability: Some(ResultStability::ReadsVersionedWorld(&[
                tcl_registry::WorldStateDomain::VariableStore,
            ])),
            world_effects: Some(tcl_registry::WorldEffectDescriptor::EMPTY),
            state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
            ..tcl_registry::CommandSpec::DEFAULT
        });

        let versioned = registry
            .resolve_invocation(
                "test::versioned",
                &["x"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("the synthetic spec resolves")
            .facts();
        assert!(resolved_invocation_is_versioned_world_gvn_candidate(
            &versioned
        ));
        assert!(!resolved_invocation_is_gvn_candidate(&versioned));

        let llength = registry
            .resolve_invocation(
                "llength",
                &["a b"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("llength resolves")
            .facts();
        assert!(!resolved_invocation_is_versioned_world_gvn_candidate(
            &llength
        ));
        assert!(resolved_invocation_is_gvn_candidate(&llength));

        assert_eq!(
            stable_call_reports(&registry, "test::versioned x\ntest::versioned x"),
            1,
            "the declared VariableStore version is equal at both sites"
        );
        assert_eq!(
            stable_call_reports(&registry, "test::versioned x\nset y 1\ntest::versioned x",),
            0,
            "an interposed variable write bumps the read domain, so the two keys differ"
        );
    }

    #[test]
    fn oo_class_creation_suppresses_reuse_through_its_constructor_barrier() {
        let registry = CommandRegistry::build_default();
        // llength's dispatch dependencies are BASE and exclude ObjectDispatch,
        // so the `oo::define` object-dispatch transition is not what suppresses
        // here.  Both `oo::class create` and `oo::define` declare a callback
        // world barrier — a constructor and a definition script run arbitrary
        // Tcl — and that barrier clobbers every domain.  `oo::class create`
        // alone is already enough.
        assert_eq!(
            stable_call_reports(
                &registry,
                "oo::class create C\n\
                 oo::define C method m {} {}\n\
                 llength {a b}\n\
                 llength {a b}",
            ),
            0,
            "the oo definition statements declare callback world barriers"
        );
        assert_eq!(
            stable_call_reports(
                &registry,
                "oo::class create C\nllength {a b}\nllength {a b}",
            ),
            0,
            "the class-creation barrier alone already widens every dispatch domain"
        );
        assert_eq!(
            stable_call_reports(&registry, PLAIN_REPEATED_LLENGTH),
            1,
            "control: without the oo statements the same pair reuses"
        );
    }
}
