//! Global Value Numbering (GVN).
//!
//! Detects redundant computations by canonicalising pure expression
//! invocations to SSA-qualified [`ExprKey`] tuples and looking them
//! up in a dominator-tree-scoped hash table. A match means the same
//! expression was computed at a dominating definition; the new
//! occurrence can be replaced with the earlier result.
//!
//! Ported from `core/compiler/gvn.py` in four strips:
//! - **C26a** (this file) — value-table types and the scoped
//!   lookup table.
//! - **C26b** — canonicalisation helpers and diagnostic message
//!   builders.
//! - **C26c** — statement-level helpers: purity classifier, cmd-
//!   tokens extractor, per-statement pure-expression occurrence
//!   collector.
//! - **C26d** — `find_redundancies` driver that walks the
//!   dominator tree and reports full/partial redundancies.

#![allow(clippy::implicit_hasher, clippy::format_push_string)]

use std::collections::HashMap;

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, Traits};

use std::collections::HashSet;

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Statement;
use crate::side_effects::{classify_side_effects, EffectRegion};
use crate::ssa::{SsaFunction, SsaStatement};

// ---------------------------------------------------------------------------
// Expression-key alias (C26a)
// ---------------------------------------------------------------------------

/// Canonical identity for a computed expression.
///
/// A call to `cmd arg1 arg2 …` becomes `["call", "cmd", arg1, arg2, …]`
/// after variable references have been rewritten to their SSA-
/// versioned form (see `canonicalise_word` in C26b). Two occurrences
/// that produce the same `ExprKey` are known to compute the same
/// value under the current SSA.
pub type ExprKey = Vec<String>;

// ---------------------------------------------------------------------------
// Redundant-computation diagnostic (C26a)
// ---------------------------------------------------------------------------

/// A computation that re-evaluates an already-available expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RedundantComputation {
    /// Span of the duplicate computation.
    pub span: Span,
    /// Span of the first computation that produced the same value.
    pub first_span: Span,
    /// Human-readable expression text for diagnostic messages.
    pub expression_text: String,
    /// Diagnostic code (e.g. `"O105"` for full redundancy,
    /// `"O106"` for partial, `"O107"` for loop-invariant).
    pub code: String,
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
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            span,
            first_span,
            expression_text: expression_text.into(),
            code: code.into(),
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Value-table entries (C26a)
// ---------------------------------------------------------------------------

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
/// Produced by the per-statement collector in C26c and consumed by
/// the fixed-point walk in C26d.
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

// ---------------------------------------------------------------------------
// Dominator-tree-scoped value table (C26a)
// ---------------------------------------------------------------------------

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
    scopes: Vec<HashMap<ExprKey, ValueEntry>>,
}

impl ScopedValueTable {
    /// Build a table with a single empty root scope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new empty scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
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
    /// at the same key is replaced (matching Python behaviour).
    pub fn insert(&mut self, entry: ValueEntry) {
        let key = entry.key.clone();
        self.scopes
            .last_mut()
            .expect("root scope present")
            .insert(key, entry);
    }

    /// Drop every tracked entry. Used on barrier / impure-call
    /// statements where no previously-tracked value can be trusted.
    pub fn kill_all(&mut self) {
        self.scopes = vec![HashMap::new()];
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

// ---------------------------------------------------------------------------
// Canonicalisation helpers (C26b)
// ---------------------------------------------------------------------------

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
///
/// Ported from `gvn.py::_canonicalise_word` — the Rust version
/// corrects the `${x}` re-matching quirk of the Python `.replace`
/// chain while preserving the observable result on inputs that do
/// not already contain `@` sigils.
#[must_use]
pub fn canonicalise_word(text: &str, uses: &HashMap<String, u32>) -> String {
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
            // `${name}` — find the closing brace.
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j < bytes.len() {
                let name = &text[start..j];
                if let Some(ver) = uses.get(name) {
                    out.push_str(&format!("${name}@{ver}"));
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
        if let Some(ver) = uses.get(name) {
            out.push_str(&format!("${name}@{ver}"));
        } else {
            out.push_str(&text[i..j]);
        }
        i = j;
    }
    out
}

/// Build the canonical [`ExprKey`] for a pure-command invocation:
/// `["call", command, canonicalised_arg1, canonicalised_arg2, …]`.
///
/// Ported from `gvn.py::_build_call_key`.
#[must_use]
pub fn build_call_key(command: &str, args: &[String], uses: &HashMap<String, u32>) -> ExprKey {
    let mut parts: ExprKey = Vec::with_capacity(2 + args.len());
    parts.push("call".into());
    parts.push(command.to_owned());
    for arg in args {
        parts.push(canonicalise_word(arg, uses));
    }
    parts
}

/// Render a command invocation as human-readable text for
/// diagnostic messages. Matches `gvn.py::_format_expression_text`.
#[must_use]
pub fn format_expression_text(command: &str, args: &[String]) -> String {
    if args.is_empty() {
        return command.to_owned();
    }
    let mut out = String::with_capacity(
        command.len() + args.iter().map(|s| s.len() + 1).sum::<usize>(),
    );
    out.push_str(command);
    out.push(' ');
    out.push_str(&args.join(" "));
    out
}

// ---------------------------------------------------------------------------
// Diagnostic messages (C26b)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Statement-level helpers (C26c)
// ---------------------------------------------------------------------------

/// Return `true` if the command invocation is pure for GVN
/// purposes — i.e. has no observable side effects.
///
/// Bridges to [`classify_side_effects`] from C23d. An optional
/// dialect (`Some("irules")` / `Some("tcl")`) threads through to
/// the classifier.
#[must_use]
pub fn is_pure_command(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&str>,
) -> bool {
    let effect = classify_side_effects(registry, command, args, dialect, None);
    effect.pure
}

/// Return `true` if a redundant use of `command` is worth
/// flagging. Built-ins marked `CSE_CANDIDATE` qualify; user-proc
/// redundancy (interprocedural) is deferred.
#[must_use]
pub fn is_worth_reporting(registry: &CommandRegistry, command: &str) -> bool {
    if let Some(spec) = registry.get(command) {
        return spec.traits.contains(Traits::CSE_CANDIDATE);
    }
    false
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
    dialect: Option<&str>,
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

/// Collect pure-expression occurrences produced by a single SSA
/// statement.
///
/// Covers two cases:
///
/// - The top-level `Statement::Call` itself, when the command is
///   pure and marked `CSE_CANDIDATE`.
/// - **C26e1**: embedded `[cmd args…]` command substitutions
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
    dialect: Option<&str>,
) -> Vec<ExprOccurrence> {
    let mut out: Vec<ExprOccurrence> = Vec::new();

    // Top-level pure Call.
    if let Statement::Call {
        command,
        args,
        span,
        ..
    } = &stmt_ssa.statement
    {
        if is_pure_command(registry, command, args, dialect)
            && is_worth_reporting(registry, command)
        {
            out.push(ExprOccurrence {
                key: build_call_key(command, args, &stmt_ssa.uses),
                span: *span,
                expression_text: format_expression_text(command, args),
                block: block_name.to_owned(),
                statement_index,
                variable_uses: stmt_ssa.uses.keys().cloned().collect(),
            });
        }
    }

    // Embedded command substitutions inside argument/value text.
    let texts_to_scan: Vec<&str> = match &stmt_ssa.statement {
        Statement::Call { args, .. } => args.iter().map(String::as_str).collect(),
        Statement::AssignValue { value, .. } => vec![value.as_str()],
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
                key: build_call_key(&cmd, &args, &stmt_ssa.uses),
                span,
                expression_text: format_expression_text(&cmd, &args),
                block: block_name.to_owned(),
                statement_index,
                variable_uses: stmt_ssa.uses.keys().cloned().collect(),
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
/// Ported behaviour-equivalent with `gvn.py::_find_cmd_tokens_in_text`
/// plus `_parse_cmd_token`: we emit one pair per `[…]` region,
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
                while i < bytes.len()
                    && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r')
                {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    i += 1;
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

// ---------------------------------------------------------------------------
// Find-redundancies driver (C26d)
// ---------------------------------------------------------------------------

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
/// Focused subset of `gvn.py::_gvn_walk_function`:
///
/// - Treats every CFG block as executable (no SCCP unreachability
///   filter). Callers that want SCCP pruning can pre-filter the
///   CFG.
/// - Detects *full* redundancy (same expression computed twice on
///   the same path) with the `"O105"` code and the
///   [`full_redundancy_message`] text.
/// - Partial-redundancy (`O106`) and loop-invariant (`O107`)
///   detection are deferred to follow-up strips — they each need
///   substantially more walker state.
///
/// Returns the diagnostics in the order the walker encounters
/// them.
/// Iterative dominator-tree walk step. `Enter` pushes a fresh
/// scope and processes the block; `Leave` pops the scope when all
/// dominator children have been visited.
enum WalkStep<'a> {
    Enter(&'a str),
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
/// Focused subset of `gvn.py::_gvn_walk_function`:
///
/// - Treats every CFG block as executable (no SCCP unreachability
///   filter). Callers that want SCCP pruning can pre-filter the
///   CFG.
/// - Detects *full* redundancy (same expression computed twice on
///   the same path) with the `"O105"` code and the
///   [`full_redundancy_message`] text.
/// - Partial-redundancy (`O106`) and loop-invariant (`O107`)
///   detection are deferred to follow-up strips — they each need
///   substantially more walker state.
///
/// Returns the diagnostics in the order the walker encounters
/// them.
#[must_use]
pub fn find_redundancies(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    dialect: Option<&str>,
) -> Vec<RedundantComputation> {
    let mut table = ScopedValueTable::new();
    let mut results: Vec<RedundantComputation> = Vec::new();
    if !cfg.blocks.contains_key(&ssa.entry) {
        return results;
    }
    let mut stack: Vec<WalkStep> = vec![WalkStep::Enter(ssa.entry.as_str())];

    while let Some(step) = stack.pop() {
        match step {
            WalkStep::Leave => {
                table.pop_scope();
            }
            WalkStep::Enter(bn) => {
                table.push_scope();
                stack.push(WalkStep::Leave);

                let Some(cfg_block) = cfg.blocks.get(bn) else {
                    continue;
                };
                let Some(ssa_block) = ssa.blocks.get(bn) else {
                    continue;
                };
                let stmt_count =
                    std::cmp::min(cfg_block.statements.len(), ssa_block.statements.len());

                for idx in 0..stmt_count {
                    let ir_stmt = &cfg_block.statements[idx];
                    let ssa_stmt = &ssa_block.statements[idx];

                    if statement_writes_state(registry, ir_stmt, dialect) {
                        table.kill_all();
                        continue;
                    }

                    let occurrences =
                        statement_occurrences(registry, ssa_stmt, bn, idx, dialect);
                    for occ in occurrences {
                        if let Some(existing) = table.lookup(&occ.key) {
                            let text = existing.expression_text.clone();
                            results.push(RedundantComputation {
                                span: occ.span,
                                first_span: existing.span,
                                expression_text: text.clone(),
                                code: "O105".into(),
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
                if let Some(children) = ssa.dominator_tree.get(bn) {
                    // Store borrowed references; the `children`
                    // Vec's strings outlive the walk.
                    for child in children.iter().rev() {
                        stack.push(WalkStep::Enter(child.as_str()));
                    }
                }
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Loop-invariant detection (C26e2)
// ---------------------------------------------------------------------------

/// True when `ancestor` dominates `node` in `ssa.idom`.
fn dominates(ssa: &SsaFunction, ancestor: &str, node: &str) -> bool {
    if ancestor == node {
        return true;
    }
    let mut curr = node.to_owned();
    loop {
        match ssa.idom.get(&curr) {
            Some(Some(parent)) => {
                if parent == ancestor {
                    return true;
                }
                curr = parent.clone();
            }
            _ => return false,
        }
    }
}

/// Enumerate block names reachable from `entry` via CFG edges.
fn reachable_from(cfg: &CfgFunction, entry: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut stack = vec![entry.to_owned()];
    while let Some(name) = stack.pop() {
        if !out.insert(name.clone()) {
            continue;
        }
        if let Some(block) = cfg.blocks.get(&name) {
            if let Some(term) = &block.terminator {
                match term {
                    Terminator::Goto { target, .. } => stack.push(target.clone()),
                    Terminator::Branch {
                        true_target,
                        false_target,
                        ..
                    } => {
                        stack.push(true_target.clone());
                        stack.push(false_target.clone());
                    }
                    Terminator::Return { .. } => {}
                }
            }
        }
    }
    out
}

/// Natural loop: all blocks on any path from `latch` back to
/// `header` that doesn't leave the loop via a non-predecessor
/// edge. Walks predecessors starting from `latch`, stopping at
/// the header.
fn natural_loop_blocks(
    cfg: &CfgFunction,
    header: &str,
    latch: &str,
    executable: &HashSet<String>,
) -> HashSet<String> {
    let preds = cfg.predecessors();
    let mut blocks: HashSet<String> = HashSet::new();
    blocks.insert(header.to_owned());
    blocks.insert(latch.to_owned());
    let mut work = vec![latch.to_owned()];
    while let Some(node) = work.pop() {
        if let Some(ps) = preds.get(&node) {
            for p in ps {
                if !executable.contains(p) || blocks.contains(p) {
                    continue;
                }
                blocks.insert(p.clone());
                if p != header {
                    work.push(p.clone());
                }
            }
        }
    }
    blocks
}

/// All variable names defined inside `loop_blocks` (phi LHS +
/// statement defs).
fn loop_defined_variables(ssa: &SsaFunction, loop_blocks: &HashSet<String>) -> HashSet<String> {
    let mut defs = HashSet::new();
    for bn in loop_blocks {
        if let Some(block) = ssa.blocks.get(bn) {
            for phi in &block.phis {
                defs.insert(phi.name.clone());
            }
            for stmt in &block.statements {
                for name in stmt.defs.keys() {
                    defs.insert(name.clone());
                }
            }
        }
    }
    defs
}

/// Detect loop-invariant pure computations.
///
/// Returns a [`RedundantComputation`] with code `"O107"` for each
/// pure-expression occurrence inside a loop whose variable
/// references are all defined *outside* the loop (so the
/// computation produces the same value on every iteration).
///
/// Matches the Python LICM-style hint at
/// `gvn.py::_find_loop_invariants`, with a simplified occurrence
/// walk that reuses [`statement_occurrences`].
#[must_use]
pub fn find_loop_invariants(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    dialect: Option<&str>,
) -> Vec<RedundantComputation> {
    let executable = reachable_from(cfg, &ssa.entry);
    let mut results: Vec<RedundantComputation> = Vec::new();

    // Collect unique header → loop_blocks pairs via back-edge
    // detection: edge tail → succ where succ dominates tail.
    let mut header_to_blocks: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
    for tail in &executable {
        let Some(block) = cfg.blocks.get(tail) else {
            continue;
        };
        let successors: Vec<String> = match &block.terminator {
            Some(Terminator::Goto { target, .. }) => vec![target.clone()],
            Some(Terminator::Branch {
                true_target,
                false_target,
                ..
            }) => vec![true_target.clone(), false_target.clone()],
            _ => continue,
        };
        for succ in successors {
            if !executable.contains(&succ) {
                continue;
            }
            if !dominates(ssa, &succ, tail) {
                continue;
            }
            let blocks = natural_loop_blocks(cfg, &succ, tail, &executable);
            header_to_blocks
                .entry(succ)
                .and_modify(|e| e.extend(blocks.iter().cloned()))
                .or_insert(blocks);
        }
    }

    for (header, loop_blocks) in &header_to_blocks {
        let defined = loop_defined_variables(ssa, loop_blocks);
        for bn in loop_blocks {
            if bn == header {
                // Skip header-only scans (contains the loop test).
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };
            for (idx, stmt_ssa) in ssa_block.statements.iter().enumerate() {
                // Purity gate — loop-invariance only makes sense
                // for computations that don't otherwise touch
                // state each iteration.
                if statement_writes_state(registry, &stmt_ssa.statement, dialect) {
                    continue;
                }
                let occurrences =
                    statement_occurrences(registry, stmt_ssa, bn, idx, dialect);
                for occ in occurrences {
                    if occ
                        .variable_uses
                        .iter()
                        .any(|name| defined.contains(name))
                    {
                        continue;
                    }
                    let text = occ.expression_text.clone();
                    results.push(RedundantComputation {
                        span: occ.span,
                        first_span: occ.span,
                        expression_text: text.clone(),
                        code: "O107".into(),
                        message: loop_invariant_message(&text),
                    });
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(t.scope_depth(), 1);
        assert_eq!(t.total_entries(), 0);
    }

    #[test]
    fn redundant_computation_constructor() {
        let r = RedundantComputation::new(
            Span::new(10, 20),
            Span::new(0, 5),
            "llength $x",
            "O105",
            "message",
        );
        assert_eq!(r.expression_text, "llength $x");
        assert_eq!(r.code, "O105");
        assert_eq!(r.span.start(), 10);
        assert_eq!(r.first_span.end(), 5);
    }

    // -- C26b: canonicalisation + messages --

    #[test]
    fn canonicalise_empty_uses_returns_input() {
        let uses = HashMap::new();
        assert_eq!(canonicalise_word("foo", &uses), "foo");
    }

    #[test]
    fn canonicalise_replaces_bare_and_braced() {
        let mut uses = HashMap::new();
        uses.insert("x".to_string(), 3);
        assert_eq!(canonicalise_word("$x", &uses), "$x@3");
        assert_eq!(canonicalise_word("${x}", &uses), "$x@3");
    }

    #[test]
    fn canonicalise_sorts_by_name_length_desc() {
        // `$longname` must be replaced before `$long` so the
        // longer name is not partially matched.
        let mut uses = HashMap::new();
        uses.insert("long".to_string(), 1);
        uses.insert("longname".to_string(), 2);
        let out = canonicalise_word("$longname$long", &uses);
        assert_eq!(out, "$longname@2$long@1");
    }

    #[test]
    fn canonicalise_ignores_unmentioned_variables() {
        let uses = HashMap::new();
        assert_eq!(canonicalise_word("$x", &uses), "$x");
    }

    #[test]
    fn build_call_key_for_pure_command() {
        let mut uses = HashMap::new();
        uses.insert("x".to_string(), 3);
        let args = vec!["$x".into(), "literal".into()];
        let key = build_call_key("llength", &args, &uses);
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

    // -- C26c: statement-level helpers --

    fn call_stmt(cmd: &str, args: &[&str]) -> Statement {
        Statement::Call {
            span: Span::new(0, 0),
            command: cmd.into(),
            args: args.iter().map(|s| (*s).into()).collect(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
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
    fn statement_writes_state_true_for_barrier() {
        let registry = CommandRegistry::build_default();
        let barrier = Statement::Barrier {
            span: Span::new(0, 0),
            reason: "eval".into(),
            command: "eval".into(),
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
        };
        let occurrences = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None);
        assert!(occurrences.is_empty());
    }

    #[test]
    fn statement_occurrences_emits_nothing_for_non_call_stmts() {
        let registry = CommandRegistry::build_default();
        let stmt_ssa = SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: "x".into(),
                value: "1".into(),
            },
            uses: HashMap::new(),
            defs: HashMap::new(),
        };
        assert!(statement_occurrences(&registry, &stmt_ssa, "entry", 0, None).is_empty());
    }

    #[test]
    fn is_worth_reporting_unknown_command_is_false() {
        let registry = CommandRegistry::build_default();
        assert!(!is_worth_reporting(&registry, "__nonexistent"));
    }

    // -- C26d: find_redundancies driver --

    use crate::cfg::{Block, Function, Terminator};
    use crate::ssa::{SsaBlock, SsaStatement};
    use std::collections::HashMap as Map;

    fn llength_call() -> Statement {
        Statement::Call {
            span: Span::new(0, 0),
            command: "llength".into(),
            args: vec!["$x".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        }
    }

    fn ssa_stmt_for(stmt: Statement, uses_x_ver: Option<u32>) -> SsaStatement {
        let mut uses = Map::new();
        if let Some(v) = uses_x_ver {
            uses.insert("x".to_string(), v);
        }
        SsaStatement {
            statement: stmt,
            uses,
            defs: Map::new(),
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
    fn find_redundancies_detects_same_block_duplicate() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        let entry_blk = cfg.blocks.get_mut("entry").unwrap();
        entry_blk.statements.push(llength_call());
        entry_blk.statements.push(llength_call());
        entry_blk.terminator = Some(Terminator::Return {
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
        ssa.dominator_tree.insert("entry".into(), Vec::new());
        let mut ssa_entry = empty_ssa_block("entry");
        ssa_entry.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa_entry.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa.blocks.insert("entry".into(), ssa_entry);

        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].code, "O105");
        assert!(results[0].expression_text.contains("llength"));
    }

    #[test]
    fn find_redundancies_ignores_different_ssa_version() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        let entry_blk = cfg.blocks.get_mut("entry").unwrap();
        entry_blk.statements.push(llength_call());
        entry_blk.statements.push(llength_call());
        entry_blk.terminator = Some(Terminator::Return {
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
        ssa.dominator_tree.insert("entry".into(), Vec::new());
        let mut ssa_entry = empty_ssa_block("entry");
        // Same expression but different SSA versions of $x → no
        // redundancy.
        ssa_entry.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa_entry.statements.push(ssa_stmt_for(llength_call(), Some(2)));
        ssa.blocks.insert("entry".into(), ssa_entry);

        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert!(results.is_empty());
    }

    #[test]
    fn find_redundancies_global_write_invalidates_cache() {
        let registry = CommandRegistry::build_default();
        // entry: llength $x; set ::g 1; llength $x
        let mut cfg = Function::new("::top", "entry");
        let entry_blk = cfg.blocks.get_mut("entry").unwrap();
        entry_blk.statements.push(llength_call());
        entry_blk.statements.push(Statement::Call {
            span: Span::new(0, 0),
            command: "set".into(),
            args: vec!["::g".into(), "1".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        });
        entry_blk.statements.push(llength_call());
        entry_blk.terminator = Some(Terminator::Return {
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
        ssa.dominator_tree.insert("entry".into(), Vec::new());
        let mut ssa_entry = empty_ssa_block("entry");
        ssa_entry.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa_entry
            .statements
            .push(ssa_stmt_for(cfg.blocks["entry"].statements[1].clone(), None));
        ssa_entry.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa.blocks.insert("entry".into(), ssa_entry);

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
        cfg.blocks.insert("child".into(), Block::new("child"));
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Goto {
            target: "child".into(),
            span: None,
        });
        cfg.blocks
            .get_mut("child")
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut("child").unwrap().terminator = Some(Terminator::Return {
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
        ssa.dominator_tree.insert("entry".into(), vec!["child".into()]);
        ssa.dominator_tree.insert("child".into(), Vec::new());
        let mut e = empty_ssa_block("entry");
        e.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        let mut c = empty_ssa_block("child");
        c.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa.blocks.insert("entry".into(), e);
        ssa.blocks.insert("child".into(), c);

        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert_eq!(results.len(), 1);
    }

    // -- C26e1: embedded command-substitution scanning --

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
        assert_eq!(args, vec!["{a b c}".to_string(), "\"d e\"".into(), "f".into()]);
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
                args: vec!["x".into(), "[llength $y]".into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
            },
            uses: HashMap::new(),
            defs: HashMap::new(),
        };
        let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None);
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
                value: "[llength $y]".into(),
                value_needs_backsubst: false,
                tokens: None,
            },
            uses: HashMap::new(),
            defs: HashMap::new(),
        };
        let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None);
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].expression_text, "llength $y");
    }

    #[test]
    fn find_redundancies_empty_for_no_blocks() {
        let registry = CommandRegistry::build_default();
        let cfg = Function::new("::top", "entry");
        let ssa = SsaFunction {
            name: "::top".into(),
            entry: "nonexistent".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        let results = find_redundancies(&registry, &cfg, &ssa, None);
        assert!(results.is_empty());
    }

    // -- C26e2: loop-invariant detection --

    #[test]
    fn find_loop_invariants_detects_hoistable_llength() {
        let registry = CommandRegistry::build_default();
        // header: branch on $i < 10 → body → header (back edge)
        // body: llength $x (invariant w.r.t. loop-defined $i)
        let mut cfg = Function::new("::top", "header");
        cfg.blocks.insert("body".into(), Block::new("body"));
        cfg.blocks.insert("exit".into(), Block::new("exit"));
        cfg.blocks.get_mut("header").unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            true_target: "body".into(),
            false_target: "exit".into(),
            span: None,
        });
        cfg.blocks
            .get_mut("body")
            .unwrap()
            .statements
            .push(llength_call());
        cfg.blocks.get_mut("body").unwrap().terminator = Some(Terminator::Goto {
            target: "header".into(),
            span: None,
        });
        cfg.blocks.get_mut("exit").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "header".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        // header dominates body and exit.
        ssa.idom.insert("header".into(), None);
        ssa.idom.insert("body".into(), Some("header".into()));
        ssa.idom.insert("exit".into(), Some("header".into()));
        ssa.dominator_tree
            .insert("header".into(), vec!["body".into(), "exit".into()]);
        ssa.dominator_tree.insert("body".into(), Vec::new());
        ssa.dominator_tree.insert("exit".into(), Vec::new());

        let h = empty_ssa_block("header");
        let mut b = empty_ssa_block("body");
        // $x's SSA version comes from outside the loop (x@1).
        b.statements.push(ssa_stmt_for(llength_call(), Some(1)));
        ssa.blocks.insert("header".into(), h);
        ssa.blocks.insert("body".into(), b);
        ssa.blocks.insert("exit".into(), empty_ssa_block("exit"));

        let results = find_loop_invariants(&registry, &cfg, &ssa, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].code, "O107");
        assert!(results[0].expression_text.contains("llength"));
    }

    #[test]
    fn find_loop_invariants_skips_loop_defined_var() {
        let registry = CommandRegistry::build_default();
        // As above, but `$i` is defined inside the loop — llength
        // uses `$i` → not loop-invariant.
        let mut cfg = Function::new("::top", "header");
        cfg.blocks.insert("body".into(), Block::new("body"));
        cfg.blocks.insert("exit".into(), Block::new("exit"));
        cfg.blocks.get_mut("header").unwrap().terminator = Some(Terminator::Branch {
            condition: crate::expr_ast::ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            true_target: "body".into(),
            false_target: "exit".into(),
            span: None,
        });
        let llength_on_i = Statement::Call {
            span: Span::new(0, 0),
            command: "llength".into(),
            args: vec!["$i".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };
        cfg.blocks
            .get_mut("body")
            .unwrap()
            .statements
            .push(llength_on_i.clone());
        cfg.blocks.get_mut("body").unwrap().terminator = Some(Terminator::Goto {
            target: "header".into(),
            span: None,
        });
        cfg.blocks.get_mut("exit").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "header".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        ssa.idom.insert("header".into(), None);
        ssa.idom.insert("body".into(), Some("header".into()));
        ssa.idom.insert("exit".into(), Some("header".into()));

        let h = empty_ssa_block("header");
        let mut b = empty_ssa_block("body");
        // `$i` defined in body.
        let mut defs = Map::new();
        defs.insert("i".to_string(), 1);
        b.statements.push(SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: "i".into(),
                value: "0".into(),
            },
            uses: Map::new(),
            defs,
        });
        // llength $i — uses map tracks `i`, not `x`.
        let mut uses_i = Map::new();
        uses_i.insert("i".to_string(), 1);
        b.statements.push(SsaStatement {
            statement: llength_on_i,
            uses: uses_i,
            defs: Map::new(),
        });
        ssa.blocks.insert("header".into(), h);
        ssa.blocks.insert("body".into(), b);
        ssa.blocks.insert("exit".into(), empty_ssa_block("exit"));

        let results = find_loop_invariants(&registry, &cfg, &ssa, None);
        assert!(results.is_empty());
    }

    #[test]
    fn find_loop_invariants_no_loops_empty() {
        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
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
        ssa.idom.insert("entry".into(), None);
        ssa.blocks.insert("entry".into(), empty_ssa_block("entry"));
        assert!(find_loop_invariants(&registry, &cfg, &ssa, None).is_empty());
    }

    #[test]
    fn dominates_trivial_and_chain() {
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: Map::new(),
            idom: Map::new(),
            dominance_frontier: Map::new(),
            dominator_tree: Map::new(),
        };
        ssa.idom.insert("entry".into(), None);
        ssa.idom.insert("a".into(), Some("entry".into()));
        ssa.idom.insert("b".into(), Some("a".into()));
        assert!(dominates(&ssa, "entry", "b"));
        assert!(dominates(&ssa, "a", "b"));
        assert!(!dominates(&ssa, "b", "a"));
        assert!(dominates(&ssa, "b", "b"));
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
}
