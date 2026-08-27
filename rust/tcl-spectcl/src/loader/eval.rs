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

//! The **evaluation loader** — design E's executable registration
//! (`docs/design/spectcl-design-e-deep-dive.md` §1), and since the
//! `one-loader` lane the *only* way a `.tclspec` source becomes a [`Pack`].
//!
//! A pack file is evaluated as a Tcl program in the sandboxed, budgeted,
//! deterministic `tcl-vm` (`tcl_spec_hooks::pack_eval`). The registration
//! vocabulary — `speclib`, `command`, `subcommand`, `option`, `available`,
//! and every other statement word the vocabulary defines — is installed as
//! host commands, so general Tcl (`proc`, `foreach`, `set`, `if`, string and
//! list operations) works between them and a pack can template its
//! registrations.
//!
//! ## Capture, then replay through the vocabulary's own readers
//!
//! The host commands do **not** interpret their arguments. Each invocation
//! is captured as a [`Stmt`] — the shape a segmented source statement has —
//! staged in evaluation order, and, once the program has run to completion,
//! replayed through the row readers ([`apply_pack_stmt`],
//! [`apply_command_stmt`], [`apply_subcommand_stmt`],
//! [`command_from_parts`], [`subcommand_from_parts`]). Those readers are the
//! `SpecTcl` vocabulary itself, not a second front end: they are what
//! *every* registration passes through, whether it was written literally in
//! the file or computed by a `foreach`. A templated pack therefore produces
//! exactly what its hand-unrolled twin would, which is the equivalence the
//! gate in `tests/eval_loader.rs` enforces.
//!
//! Replaying after evaluation (rather than during) preserves the two-pass
//! rule: pack-level `values`/`descriptor`/`hook`/`default` tables are all
//! read before any `command` is built, so a command may reference a table
//! declared after it.
//!
//! ## The contracts this module enforces
//!
//! - **Determinism** (§1.2): the sandbox whitelist plus
//!   [`pack_eval::denied_axis`] — an evaluated pack cannot reach a clock,
//!   file, socket, process, environment variable, or the event loop, and a
//!   dispatch that tries fails the load with a notice naming the axis.
//!   Budgets bound steps, wall clock, and value size; a blown budget fails
//!   the load naming its axis.
//! - **Transactional registration** (§1.2): registrations accumulate in the
//!   staging structure; any hard error — a Tcl error, a denial, a blown
//!   budget, a provenance violation — discards the whole pack
//!   ([`Pack::load_error`]): a pack registers wholly or not at all.
//!   Vocabulary-*class* degradation still applies to unknown registration
//!   words: an unresolved word that is not a sandbox denial is captured and
//!   replayed through the ordinary unknown-word path, so it classifies
//!   under §6.1 (forward direction only) instead of raising a Tcl
//!   `invalid command name` error.
//! - **E-R1**: `available?` answers against the **union of the pack's
//!   declared support** (its `default available`/`default dialects` rows as
//!   evaluated so far); any use marks the pack
//!   [`Pack::target_dependent`], adds a `target-dependent registration`
//!   notice, and excludes the snapshot from
//!   [`crate::cache::evaluate_pack_cached`]'s memoisation.
//! - **E-R2**: provenance gates what a registration call may touch. For an
//!   untrusted tier (workspace or Spec Studio override), a `command`
//!   claiming a compiled name with `-override`, a `dialect` block (compiled
//!   dialect axes), or an `environment` block claiming a reserved compiled
//!   name fails the load with an error naming the provenance class —
//!   reusing the same reserved-name check `environment_block` performs.
//! - **Snapshot caching seam**: [`EvalSnapshotKey`] = (content hash,
//!   [`crate::VOCABULARY_VERSION`], [`LOADER_EVAL_VERSION`], tier). This
//!   module computes the identity and nothing more: both storage tiers live
//!   in [`crate::cache`], which is the one door production code loads a pack
//!   through. Target-dependent packs (E-R1) are never stored.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use tcl_dialect::DialectSet;
use tcl_dialect::model::environment::Provenance;
use tcl_spec_hooks::pack_eval::{
    self, PackEvalConfig, PackEvalCtx, PackEvalFailure, UnknownHandler, WordHandler,
};

use super::{
    CommandAcc, CommandSpec, FileBom, LoadError, Log, Notice, Pack, PackTables, Stmt, SubAcc,
    SubCommand, VocabularyClass, Word, apply_command_stmt, apply_pack_stmt, apply_subcommand_stmt,
    available, block, check_vocabulary_version, command_from_parts, empty_pack, environment_block,
    finish_newer_words, pack_statements, parse_dialects, statements, subcommand_from_parts,
};
use crate::discovery::Tier;
use crate::export::{Registration, synth_word};

/// The evaluation loader's own version, part of the snapshot cache key: a
/// change to how evaluation captures or replays invalidates every cached
/// evaluated snapshot exactly once, independent of the vocabulary version.
pub const LOADER_EVAL_VERSION: u32 = 1;

/// How to evaluate a pack: the provenance tier gating registrations (E-R2)
/// and the sandbox budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvalOptions {
    /// The tier the pack loads from. [`Tier::Workspace`] and
    /// [`Tier::StudioOverride`] are the untrusted provenance classes: their
    /// registrations may not touch reserved compiled names or compiled
    /// dialect axes.
    pub tier: Tier,
    /// The budgets evaluation runs under.
    pub config: PackEvalConfig,
    /// Whether the static fast path may short-circuit the interpreter for a
    /// body whose statements are all static vocabulary (see [`run_body`]).
    ///
    /// `true` in production, always. It is switchable only so the gate in
    /// `tests/eval_loader.rs` can load every shipped pack **both** ways and
    /// assert the snapshots are byte-identical — which is what makes the
    /// shortcut provably an optimisation rather than a second reading of the
    /// file. It is not part of the snapshot identity for exactly that reason:
    /// the two routes cannot produce different answers.
    pub static_fast_path: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            tier: Tier::Bundled,
            config: PackEvalConfig::default(),
            static_fast_path: true,
        }
    }
}

/// The identity an evaluated snapshot caches under (deep dive §1.1): the
/// file bytes, the vocabulary, the evaluation loader build, and the
/// provenance tier (which gates what the evaluation was allowed to
/// register, so it is part of the answer's identity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvalSnapshotKey {
    /// xxh3 of the pack source bytes.
    pub content_hash: u64,
    /// [`crate::VOCABULARY_VERSION`] at evaluation time.
    pub vocabulary: &'static str,
    /// [`LOADER_EVAL_VERSION`] at evaluation time.
    pub loader_eval_version: u32,
    /// The provenance tier the pack evaluated under.
    pub tier: Tier,
}

/// The snapshot cache key for one source under one set of options.
#[must_use]
pub fn eval_snapshot_key(source: &str, options: &EvalOptions) -> EvalSnapshotKey {
    EvalSnapshotKey {
        content_hash: xxhash_rust::xxh3::xxh3_64(source.as_bytes()),
        vocabulary: crate::VOCABULARY_VERSION,
        loader_eval_version: LOADER_EVAL_VERSION,
        tier: options.tier,
    }
}

/// Evaluate a `.tclspec` source as a Tcl program (design E) with default
/// options: bundled-tier trust, default budgets.
#[must_use]
pub fn evaluate_pack(source: &str) -> Pack {
    evaluate_pack_with(source, &EvalOptions::default())
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// Every statement word that is a plain **row** at some scope: captured
/// verbatim and replayed through the scope's own reader, which is also what
/// sorts a word used at the wrong scope into the same unknown-property
/// unknown-property notice.
///
/// `speclib`, `command`, `subcommand`, and `available?` are not here — each
/// has its own handler with body evaluation or an answer.
const ROW_WORDS: &[&str] = &[
    // pack scope
    "values",
    "descriptor",
    "hook",
    "default",
    "display_name",
    "file_extension",
    "ambient_package",
    "provides",
    "co_provides",
    "environment",
    "dialect",
    // command scope
    "dialects",
    "available",
    "traits",
    "arity",
    "required_package",
    "tk_geometry",
    "tcllib_package",
    "implementation_namespace",
    "introduced_version",
    "deprecated_version",
    "retired_version",
    "warn_missing_import",
    "is_namespace_exported",
    "unsafe_command",
    "excluded_events",
    "safe_on_uninit",
    "deprecated_replacement",
    "deprecated_replacement_drop_in",
    "xc_translatable",
    "arg",
    "repeat",
    "reserved_trailing_words",
    "assigns_variable_at",
    "creates_instance_at",
    "defines_command_at",
    "body_arg_implicit_args",
    "body_kind",
    "allow_unknown_subcommands",
    "dynamic_surface",
    "unknown_members",
    "prefix_matching",
    "self_receiver_words",
    "return_type",
    "var_write_typing",
    "inferred_storage_type",
    "byte_array_effect",
    "byte_array_payload",
    "pattern_type",
    "format_string_type",
    "return_elements",
    "var_elements_effect",
    "representation_effect",
    "default_form_first_word",
    "defines_symbol",
    "deprecation_fix",
    "setter_constraint",
    "hover",
    "form",
    "side_effect",
    "command_table_effect",
    "frame_effect",
    "world_effects",
    "state_transitions",
    // The ratified §6.2 words, at the scopes their model fields live on:
    // `result_stability` at both command and subcommand scope, the rest at
    // command scope. Captured here so an evaluated pack registers them
    // through the same row readers a CST-loaded one does.
    "result_stability",
    "side_switch_target",
    "event_handler_priority",
    "data_collection",
    "event_requirement_form",
    "body_scope",
    "taint_source",
    "taint_transform",
    "taint_double_encode_colour",
    "taint_sink_safe_colour",
    "taint_output_sink",
    "taint_log_sink",
    "taint_output_sink_subcommands",
    "taint_interp_eval_subcommands",
    "taint_network_sink_args",
    "taint_code_sink_args",
    "callback_taint_inputs",
    "credential_options",
    "sensitive_headers",
    "event_requires",
    "option",
    "option_conflict",
    "option_requires",
    "option_requires_one_of",
    "option_forbids",
    "option_placement",
    "constraints",
    "versioned_arg_value",
    "case_list",
    "definition_body",
    "manufacturer",
    "clause_grammar",
    "binds_handle",
    "object_class",
    "oo_context_fact",
    "lowering_hook",
    "codegen_hook",
    "inline_codegen_hook",
    "analyser_hook",
    "semantic_operation",
    "arg_role_resolver",
    "command_prefix_resolver",
    "script_timing_resolver",
    "const_fold",
    "const_fold_versioned",
    "taint_sink_gate",
    "context_gate",
    "literal_argument_validator",
    "clause_shape_check",
    // subcommand scope
    "detail",
    "synopsis",
    "pure",
    "mutator",
    "destructive",
    "returns_path",
    "is_unescape",
    "loop_list_header",
    "creates_scope_alias",
    "arg_values_accept_prefix",
    "cfg_rewrite_name",
    "min_abbrev",
    "max_leading_option_words",
    "credential_arg",
    "sub_subcommand",
];

/// How the current body is being driven: the interpreter when there is one,
/// and whether the capture layer may shortcut past it.
///
/// `ctx: None` is the **file-level fast path**: a pack whose every statement,
/// at every level, is static vocabulary is captured straight from the CST and
/// never builds a `tcl-vm` interpreter at all. The moment a body needs real
/// evaluation the drive gives up with [`NEEDS_INTERPRETER`] and the whole
/// load restarts on the interpreter path — one attempt, no partial state,
/// and the segmentations the attempt did are memoised for the retry.
struct Drive<'a, 'b> {
    ctx: Option<&'a mut PackEvalCtx<'b>>,
    /// [`EvalOptions::static_fast_path`], carried here rather than on the
    /// staging state because it is a property of *how* a body is driven.
    fast_path: bool,
}

impl<'b> Drive<'_, 'b> {
    /// The same drive, borrowed for a shorter life — what a loop over
    /// statements needs to hand the interpreter down more than once.
    fn reborrow(&mut self) -> Drive<'_, 'b> {
        Drive {
            ctx: self.ctx.as_deref_mut(),
            fast_path: self.fast_path,
        }
    }
}

/// A drive with the interpreter behind it — what a host command's handler
/// hands on when it recurses into a body. Reaching a handler at all means
/// the interpreter route is running, and the fast path stays available for
/// the nested bodies that qualify.
fn interpreted<'a, 'b>(ctx: &'a mut PackEvalCtx<'b>) -> Drive<'a, 'b> {
    Drive {
        ctx: Some(ctx),
        fast_path: true,
    }
}

/// The sentinel [`run_body`] returns when a body must really evaluate and no
/// interpreter was built. Never surfaces to a caller: [`evaluate_pack_in`]
/// turns it into the interpreter run.
const NEEDS_INTERPRETER: &str = "\u{0}spectcl: this pack needs the interpreter";

/// One captured statement inside a `command` or `subcommand` body.
#[derive(Debug)]
enum Node {
    /// A plain row, replayed through the scope's statement reader.
    Row(Stmt),
    /// A `subcommand NAME { … }` whose body was itself evaluated.
    Sub(StagedSub),
}

#[derive(Debug)]
struct StagedSub {
    name: String,
    line: u32,
    body: Vec<Node>,
}

/// One `command` declaration captured at pack scope.
#[derive(Debug)]
struct StagedCommand {
    name: String,
    overrides: bool,
    line: u32,
    /// `None` when no `{ … }` body word was found — the
    /// brace-on-the-next-line mistake, whose notice the replay raises.
    body: Option<Vec<Node>>,
}

/// One captured top-level statement.
#[derive(Debug)]
enum PackNode {
    Row(Stmt),
    Command(StagedCommand),
}

/// What scope the evaluation is currently capturing into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Command,
    Subcommand,
}

/// One body block as the file spells it, for line-exact evaluation.
///
/// Evaluation receives argument *values*, and Tcl's one substitution inside
/// braces — backslash-newline collapse — removes physical newlines from
/// them, so a statement captured after a continuation would attribute to
/// the wrong line. The fix is to evaluate the **verbatim** body text from
/// the CST whenever an invocation corresponds to a source statement: same
/// semantics (the lexer processes the continuation either way), physical
/// lines preserved. A templated invocation matches no source statement and
/// falls back to the evaluated value, where lines are best-effort.
#[derive(Debug)]
struct VerbatimBody {
    /// Absolute line of the declaring statement, the lookup key.
    stmt_line: u32,
    /// Every byte between the body word's braces.
    text: String,
    /// The body word's own line — the base line of the nested unit,
    /// exactly what [`block`] numbers a nested body from.
    body_line: u32,
}

/// The verbatim skeleton of the file: the first `speclib`'s body, each
/// `command` body inside it, each `subcommand` body inside those — and
/// every **statement** at those levels, keyed by line, so a captured row
/// whose invocation corresponds to a source statement is staged with the
/// CST's own words (verbatim text, real per-word braced-ness and lines)
/// rather than the evaluated values.
///
/// ## Built in two layers, the second on demand
///
/// The `speclib` body is needed by every load: the file's top level always
/// runs through the interpreter (that is where the `speclib` command is
/// dispatched from), and its body must be handed on verbatim so the static
/// fast path can take it.
///
/// The rest — the per-declaration bodies and the line-keyed statement
/// table — is needed **only when a body actually reaches the interpreter**.
/// A body the fast path drives captures its statements straight from the
/// CST and never asks this index anything. Building the full skeleton
/// eagerly therefore cost every declarative pack a second and third walk of
/// its own statements plus a clone of each into a hash table, which on a
/// 20k-line pack is most of the load: it is built lazily instead, so the
/// pack that does not template pays nothing for the machinery that makes
/// templating line-exact.
struct VerbatimIndex {
    /// The file's own text, to build [`Skeleton`] from if it is ever asked
    /// for.
    source: Rc<str>,
    speclib: Option<VerbatimBody>,
    /// The declaration skeleton, built on first use.
    skeleton: Option<Skeleton>,
}

impl Default for VerbatimIndex {
    fn default() -> Self {
        Self {
            source: Rc::from(""),
            speclib: None,
            skeleton: None,
        }
    }
}

/// The on-demand half of [`VerbatimIndex`].
#[derive(Default)]
struct Skeleton {
    /// Keyed by (statement word, declared name).
    bodies: HashMap<(&'static str, String), Vec<VerbatimBody>>,
    /// Every declarative-skeleton statement, keyed by its line.
    rows: HashMap<u32, Vec<Stmt>>,
}

impl VerbatimIndex {
    /// The cheap layer: the first `speclib` block's body.
    fn of(source: &str) -> Self {
        let mut index = Self {
            source: Rc::from(source),
            ..Self::default()
        };
        let top = pack_statements(source);
        let Some(speclib) = top.into_iter().find(|stmt| stmt.word_text(0) == "speclib") else {
            return index;
        };
        let Some(body) = speclib.arg(3).filter(|word| word.braced) else {
            return index;
        };
        index.speclib = Some(VerbatimBody {
            stmt_line: speclib.line,
            text: body.text.clone(),
            body_line: body.line,
        });
        index
    }

    /// The declaration skeleton, built on first ask.
    fn skeleton(&mut self) -> &mut Skeleton {
        if self.skeleton.is_none() {
            self.skeleton = Some(Skeleton::of(&self.source));
        }
        self.skeleton
            .as_mut()
            .unwrap_or_else(|| unreachable!("just built"))
    }

    /// Take the verbatim body for a `word NAME …` invocation captured at
    /// `line`, when the file has exactly that statement there.
    fn take(&mut self, word: &'static str, name: &str, line: u32) -> Option<VerbatimBody> {
        let entries = self.skeleton().bodies.get_mut(&(word, name.to_owned()))?;
        let at = entries.iter().position(|body| body.stmt_line == line)?;
        Some(entries.remove(at))
    }

    /// The verbatim statement matching a captured `word arg…` invocation at
    /// `line`, when the file has one of the same shape there. A templated
    /// invocation (a loop body, a helper proc) attributes to a line whose
    /// source statement has a different head word or width, so it misses
    /// and keeps its evaluated capture.
    fn row(&mut self, word: &str, arg_count: usize, line: u32) -> Option<Stmt> {
        self.skeleton()
            .rows
            .get(&line)?
            .iter()
            .find(|stmt| stmt.word_text(0) == word && stmt.words.len() == arg_count + 1)
            .cloned()
    }
}

impl Skeleton {
    fn of(source: &str) -> Self {
        let mut index = Self::default();
        let top = pack_statements(source);
        index.record_rows(&top);
        let Some(speclib) = top.into_iter().find(|stmt| stmt.word_text(0) == "speclib") else {
            return index;
        };
        let Some(body) = speclib.arg(3).filter(|word| word.braced) else {
            return index;
        };
        let body_stmts = block(body);
        index.record_rows(&body_stmts);
        for stmt in &body_stmts {
            if stmt.word_text(0) != "command" {
                continue;
            }
            let Some(command_body) = stmt.words.iter().skip(2).find(|word| word.braced) else {
                continue;
            };
            let command_stmts = block(command_body);
            index.record_rows(&command_stmts);
            for sub in &command_stmts {
                if sub.word_text(0) != "subcommand" {
                    continue;
                }
                let Some(sub_body) = sub.arg(2).filter(|word| word.braced) else {
                    continue;
                };
                index.record_rows(&block(sub_body));
                index.record("subcommand", sub.word_text(1), sub, sub_body);
            }
            index.record("command", stmt.word_text(1), stmt, command_body);
        }
        index
    }

    fn record_rows(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.rows.entry(stmt.line).or_default().push(stmt.clone());
        }
    }

    fn record(&mut self, word: &'static str, name: &str, stmt: &Stmt, body: &Word) {
        self.bodies
            .entry((word, name.to_owned()))
            .or_default()
            .push(VerbatimBody {
                stmt_line: stmt.line,
                text: body.text.clone(),
                body_line: body.line,
            });
    }
}

/// The `speclib` header as captured.
#[derive(Debug)]
struct SpeclibDecl {
    name: String,
    version: String,
    line: u32,
    had_body: bool,
}

/// Everything the evaluation stages before replay.
#[derive(Default)]
struct State {
    pack_nodes: Vec<PackNode>,
    /// Open capture scopes, innermost last, each with its collected nodes.
    scopes: Vec<(ScopeKind, Vec<Node>)>,
    speclib: Option<SpeclibDecl>,
    /// `speclib` saw a braced word where its name belongs — the CST
    /// loader's issue-#1638 refusal, replayed as "nothing loaded".
    refused_braced_name: bool,
    /// Notices the evaluation itself produced (extra `speclib` blocks,
    /// E-R1 target-dependence), appended after the replay's notices.
    eval_notices: Vec<Notice>,
    /// Whether `available?` was used (E-R1).
    target_dependent: bool,
    /// The absolute base line of each unit being evaluated, innermost
    /// last; a captured statement's absolute line is `base + relative - 1`.
    base_lines: Vec<u32>,
    /// The union of the pack's declared support so far (`default
    /// available` / `default dialects`), the set `available?` answers
    /// against. `None` = nothing declared = everything.
    declared_dialects: Option<DialectSet>,
    /// The file's declarative skeleton, verbatim, for line-exact body
    /// evaluation. Its second layer builds only if a body reaches the
    /// interpreter.
    verbatim: VerbatimIndex,
    /// The include context this evaluation resolves `include` rows
    /// through, when the caller gave one (`evaluate_pack_in`).
    include: Option<Rc<super::IncludeContext>>,
    /// Content hashes on the current inclusion path, the root source
    /// first — the determinism contract's cycle key.
    include_stack: Vec<u64>,
    /// Whether an included fragment is currently being evaluated: the
    /// verbatim index describes the *root* file, so line-keyed lookups
    /// must not fire inside an included one.
    in_include: bool,
    /// Sites that used vocabulary the capture layer (not the replay's
    /// readers) consumed — today only `include` — replayed into the
    /// vocabulary-consistency log so a word the capture layer ate still
    /// gets its per-site "newer than declared" notice.
    newer_word_sites: Vec<(u32, &'static str)>,
}

impl State {
    fn absolute_line(&self, relative: u32) -> u32 {
        self.base_lines
            .last()
            .copied()
            .unwrap_or(1)
            .saturating_add(relative.saturating_sub(1))
    }

    fn push_node(&mut self, node: Node) {
        if let Some((_, nodes)) = self.scopes.last_mut() {
            nodes.push(node);
        } else if let Node::Row(stmt) = node {
            self.pack_nodes.push(PackNode::Row(stmt));
        }
    }

    /// A captured invocation as a [`Stmt`]: the file's verbatim statement
    /// when the invocation corresponds to one, the evaluated values
    /// otherwise. Inside an included fragment the verbatim index (which
    /// describes the root file) is not consulted, so a line-number
    /// coincidence cannot substitute the wrong statement.
    fn captured(&mut self, word: &str, args: &[String], line: u32) -> Stmt {
        if self.in_include {
            return capture_stmt(word, args, line);
        }
        self.verbatim
            .row(word, args.len(), line)
            .unwrap_or_else(|| capture_stmt(word, args, line))
    }
}

/// Whether an argument's text can only have been written as a `{ … }` block
/// (or quoted equivalent): the evaluation loses the CST's per-word
/// braced-ness, so it is reconstructed from the one property that matters
/// to every consumer — a name is one whitespace-free word, a block is not.
fn blockish(text: &str) -> bool {
    text.is_empty() || text.contains(char::is_whitespace)
}

/// Rebuild a [`Stmt`] — the shape a segmented source statement has — from
/// a captured invocation.
fn capture_stmt(word: &str, args: &[String], line: u32) -> Stmt {
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(Word {
        text: word.to_owned(),
        braced: blockish(word),
        line,
    });
    for arg in args {
        words.push(Word {
            text: arg.clone(),
            braced: blockish(arg),
            line,
        });
    }
    Stmt { words, line }
}

fn eval_notice(context: &str, line: u32, class: VocabularyClass, message: String) -> Notice {
    Notice {
        context: context.to_owned(),
        line,
        message,
        class,
    }
}

/// The handler for one plain row word.
fn row_handler(word: &'static str, state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let mut st = state.borrow_mut();
        let line = st.absolute_line(ctx.line());
        if word == "default" {
            track_declared_support(&mut st, args, line);
        }
        let stmt = st.captured(word, args, line);
        st.push_node(Node::Row(stmt));
        Ok(None)
    })
}

/// Keep the running union of declared support for `available?` (E-R1).
fn track_declared_support(state: &mut State, args: &[String], line: u32) {
    let mut scratch = Log::default();
    match args.first().map(String::as_str) {
        Some("dialects") => {
            if let Some(set) = parse_dialects(args.get(1).map_or("", |a| a), line, &mut scratch) {
                state.declared_dialects = Some(set);
            }
        }
        Some("available") => {
            let availability = available::from_texts(&args[1..], line, &mut scratch);
            if let Some(set) = availability.dialects {
                state.declared_dialects = Some(set);
            }
        }
        _ => {}
    }
}

/// The `speclib` handler: header capture, extra-block reporting, and body
/// evaluation. A thin adapter over [`stage_speclib`], which the static
/// driver reaches with the file's own words instead of evaluated values.
fn speclib_handler(state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        stage_speclib(interpreted(ctx), &state, args, line)?;
        Ok(None)
    })
}

/// Stage one `speclib NAME VERSION { … }` declaration and run its body.
///
/// Shared by the host command and the static driver, so the file-level
/// shortcut and the interpreter cannot drift on what a `speclib` header
/// means. `args` are the statement's words after `speclib`, as values.
fn stage_speclib(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    args: &[String],
    line: u32,
) -> Result<(), String> {
    // Only a top-level `speclib` is the pack header; one inside any
    // evaluated body is an ordinary (unknown) row.
    let nested = {
        let st = state.borrow();
        !st.scopes.is_empty() || !st.base_lines.is_empty()
    };
    if nested {
        let mut st = state.borrow_mut();
        let stmt = st.captured("speclib", args, line);
        st.push_node(Node::Row(stmt));
        return Ok(());
    }
    if state.borrow().speclib.is_some() || state.borrow().refused_braced_name {
        let extra_name = args.first().cloned().unwrap_or_default();
        let commands = args.get(2).map_or(0, |body| {
            statements(body, 1, FileBom::Content)
                .iter()
                .filter(|s| s.word_text(0) == "command")
                .count()
        });
        state.borrow_mut().eval_notices.push(eval_notice(
            "pack",
            line,
            VocabularyClass::Presentation,
            format!(
                "a second `speclib` block (`{extra_name}`) in one file is not loaded; \
                 only the first pack in a file is read, so its {commands} command(s) \
                 are dropped — move it to its own `.tclspec`"
            ),
        ));
        return Ok(());
    }
    let name = args.first().cloned().unwrap_or_default();
    if blockish(&name) && !name.is_empty() {
        let mut st = state.borrow_mut();
        st.refused_braced_name = true;
        st.eval_notices.push(eval_notice(
            "pack",
            line,
            VocabularyClass::Presentation,
            "`speclib` needs a name and a vocabulary version before its body \
             block (`speclib NAME 1.1 { … }`); nothing loaded"
                .to_owned(),
        ));
        return Ok(());
    }
    let version = args.get(1).cloned().unwrap_or_default();
    let body = args.get(2).cloned();
    {
        let mut st = state.borrow_mut();
        st.speclib = Some(SpeclibDecl {
            name,
            version: version.clone(),
            line,
            had_body: body.is_some(),
        });
    }
    // An unsupported major fails the pack closed (§6.1): the body is
    // not even evaluated, matching "nothing is loaded". The replay
    // re-runs the check to produce the notice.
    let mut scratch = Log::default();
    if check_vocabulary_version(&version, line, &mut scratch) {
        return Ok(());
    }
    if let Some(body) = body {
        // Prefer the file's verbatim body so continuation collapse
        // cannot shift line attribution (see `VerbatimBody`).
        let verbatim = {
            let mut st = state.borrow_mut();
            st.verbatim
                .speclib
                .take_if(|candidate| candidate.stmt_line == line)
        };
        let block = match verbatim {
            Some(vb) => BodyText {
                text: vb.text,
                base: vb.body_line,
                verbatim: true,
            },
            None => BodyText {
                text: body,
                base: line,
                verbatim: false,
            },
        };
        state.borrow_mut().base_lines.push(block.base);
        let outcome = run_body(drive, state, &block);
        state.borrow_mut().base_lines.pop();
        outcome?;
    }
    Ok(())
}

/// The `command` handler: head capture plus body evaluation in a fresh
/// command scope.
fn command_handler(state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        if !state.borrow().scopes.is_empty() {
            let mut st = state.borrow_mut();
            let stmt = st.captured("command", args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        let name = args.first().cloned().unwrap_or_default();
        let overrides = args.iter().any(|arg| arg == "-override");
        // The body is the first braced word after the name; evaluation
        // loses per-word braced-ness, so the captured equivalent is the
        // first blockish argument.
        let body = args.iter().skip(1).find(|arg| blockish(arg)).cloned();
        let Some(body) = body else {
            state
                .borrow_mut()
                .pack_nodes
                .push(PackNode::Command(StagedCommand {
                    name,
                    overrides,
                    line,
                    body: None,
                }));
            return Ok(None);
        };
        let body = {
            let mut st = state.borrow_mut();
            match st.verbatim.take("command", &name, line) {
                Some(vb) => BodyText {
                    text: vb.text,
                    base: vb.body_line,
                    verbatim: true,
                },
                None => BodyText {
                    text: body,
                    base: line,
                    verbatim: false,
                },
            }
        };
        stage_command(interpreted(ctx), &state, name, overrides, line, &body)?;
        Ok(None)
    })
}

/// One body block ready to run: its text, the base line of its unit, and
/// whether the text is the file's verbatim bytes (which is what licenses
/// the static fast path).
struct BodyText {
    text: String,
    base: u32,
    verbatim: bool,
}

/// Evaluate one `command` body in a fresh command scope and stage the
/// declaration. Shared by the host command and the static driver.
fn stage_command(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    name: String,
    overrides: bool,
    line: u32,
    body: &BodyText,
) -> Result<(), String> {
    {
        let mut st = state.borrow_mut();
        st.scopes.push((ScopeKind::Command, Vec::new()));
        st.base_lines.push(body.base);
    }
    let outcome = run_body(drive, state, body);
    let nodes = {
        let mut st = state.borrow_mut();
        st.base_lines.pop();
        st.scopes.pop().map(|(_, nodes)| nodes).unwrap_or_default()
    };
    outcome?;
    state
        .borrow_mut()
        .pack_nodes
        .push(PackNode::Command(StagedCommand {
            name,
            overrides,
            line,
            body: Some(nodes),
        }));
    Ok(())
}

/// The `subcommand` handler: valid only directly inside a `command` body,
/// where its own body evaluates in a subcommand scope; anywhere else it is
/// a plain row, which replays to the ordinary unknown-property notice.
fn subcommand_handler(state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        let in_command = matches!(state.borrow().scopes.last(), Some((ScopeKind::Command, _)));
        if !in_command {
            let mut st = state.borrow_mut();
            let stmt = st.captured("subcommand", args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        let name = args.first().cloned().unwrap_or_default();
        // The body is exactly the third word; a `subcommand` without one
        // is silently dropped.
        let Some(body) = args.get(1).cloned() else {
            return Ok(None);
        };
        let body = {
            let mut st = state.borrow_mut();
            match st.verbatim.take("subcommand", &name, line) {
                Some(vb) => BodyText {
                    text: vb.text,
                    base: vb.body_line,
                    verbatim: true,
                },
                None => BodyText {
                    text: body,
                    base: line,
                    verbatim: false,
                },
            }
        };
        stage_subcommand(interpreted(ctx), &state, name, line, &body)?;
        Ok(None)
    })
}

/// Evaluate one `subcommand` body in a fresh subcommand scope and stage the
/// declaration into the owning command. Shared by the host command and the
/// static driver.
fn stage_subcommand(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    name: String,
    line: u32,
    body: &BodyText,
) -> Result<(), String> {
    {
        let mut st = state.borrow_mut();
        st.scopes.push((ScopeKind::Subcommand, Vec::new()));
        st.base_lines.push(body.base);
    }
    let outcome = run_body(drive, state, body);
    let nodes = {
        let mut st = state.borrow_mut();
        st.base_lines.pop();
        st.scopes.pop().map(|(_, nodes)| nodes).unwrap_or_default()
    };
    outcome?;
    state.borrow_mut().push_node(Node::Sub(StagedSub {
        name,
        line,
        body: nodes,
    }));
    Ok(())
}

/// The `include` handler (2.0, Q6): valid only as a literal pack-scope
/// row; the resolved fragment evaluates in place with provenance
/// inherited, under the determinism contract's content-hash cycle key.
fn include_handler(state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        let at_pack = state.borrow().scopes.is_empty();
        if !at_pack {
            // Inside a command/subcommand body `include` is not pack
            // vocabulary; captured as a row, it replays to the unknown
            // word path.
            let mut st = state.borrow_mut();
            let stmt = st.captured("include", args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        stage_include(interpreted(ctx), &state, &words, line)?;
        Ok(None)
    })
}

/// Resolve and evaluate one `include` row. Shared by the host command and
/// the static driver, so the two evaluation paths cannot drift.
fn stage_include(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    words: &[&str],
    line: u32,
) -> Result<(), String> {
    // The vocabulary-consistency log's eval-side record: `include` is 2.0
    // vocabulary, and the replay turns this into the same per-site notice
    // the replay's readers log for a row they read themselves.
    state.borrow_mut().newer_word_sites.push((line, "include"));
    let name = match super::include_name(words, line) {
        Ok(name) => name,
        Err(notice) => {
            state.borrow_mut().eval_notices.push(notice);
            return Ok(());
        }
    };
    let resolved = {
        let st = state.borrow();
        st.include.clone().map(|context| context.resolve(&name))
    };
    let Some(resolved) = resolved else {
        let notice = eval_notice(
            "pack",
            line,
            VocabularyClass::Semantic,
            format!(
                "`include {name}` needs a pack search path and this load was given \
                 none; the row is dropped and its declarations are not loaded"
            ),
        );
        state.borrow_mut().eval_notices.push(notice);
        return Ok(());
    };
    let text = match resolved {
        Ok(text) => text,
        Err(error) => {
            let notice = eval_notice(
                "pack",
                line,
                VocabularyClass::Semantic,
                format!(
                    "`include {name}` did not resolve ({error}); the row is dropped \
                     and its declarations are not loaded"
                ),
            );
            state.borrow_mut().eval_notices.push(notice);
            return Ok(());
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned();
    let hash = xxhash_rust::xxh3::xxh3_64(text.as_bytes());
    {
        let mut st = state.borrow_mut();
        if st.include_stack.contains(&hash) {
            let notice = eval_notice(
                "pack",
                line,
                VocabularyClass::Semantic,
                format!(
                    "`include {name}` closes an include cycle (its content is already \
                     on the inclusion path); the row is dropped"
                ),
            );
            st.eval_notices.push(notice);
            return Ok(());
        }
        if st.include_stack.len() >= super::INCLUDE_DEPTH_LIMIT {
            let notice = eval_notice(
                "pack",
                line,
                VocabularyClass::Semantic,
                format!(
                    "`include {name}` exceeds the include depth limit ({}); the row \
                     is dropped",
                    super::INCLUDE_DEPTH_LIMIT
                ),
            );
            st.eval_notices.push(notice);
            return Ok(());
        }
        st.include_stack.push(hash);
        st.base_lines.push(1);
    }
    let was_in_include = {
        let mut st = state.borrow_mut();
        std::mem::replace(&mut st.in_include, true)
    };
    let body = BodyText {
        text,
        base: 1,
        verbatim: true,
    };
    let outcome = run_body(drive, state, &body);
    {
        let mut st = state.borrow_mut();
        st.in_include = was_in_include;
        st.base_lines.pop();
        st.include_stack.pop();
    }
    outcome
}

/// Run one body: when it is the file's verbatim text and every statement at
/// this level is static vocabulary, capture the statements directly —
/// exactly what evaluating them would do, minus the interpreter — and hand
/// anything else to the interpreter whole. The fast path is what keeps a
/// 20k-line declarative pack from paying a 20k-line bytecode compilation
/// for an evaluation that could only ever capture rows.
fn run_body(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    body: &BodyText,
) -> Result<(), String> {
    if body.verbatim && drive.fast_path {
        let stmts = statements(&body.text, body.base, FileBom::Content);
        if stmts.iter().all(static_stmt) {
            return drive_static(drive, state, &stmts);
        }
    }
    match drive.ctx {
        Some(ctx) => ctx.eval_body(&body.text),
        None => Err(NEEDS_INTERPRETER.to_owned()),
    }
}

/// General Tcl whose dispatch reads or writes interpreter state — a
/// statement headed by one of these must really evaluate.
fn is_general_tcl(word: &str) -> bool {
    tcl_spec_hooks::SANDBOX_COMMANDS.contains(&word)
        || pack_eval::PACK_EVAL_EXTRA_COMMANDS.contains(&word)
        || word.starts_with("tcl::")
        || word.starts_with("::")
}

/// Whether `stmt` is a **static** vocabulary row at its own level: its
/// evaluation could only capture it verbatim. Substitution in any unbraced
/// word, a general-Tcl or denied head word, and `available?` all force the
/// interpreter path.
fn static_stmt(stmt: &Stmt) -> bool {
    let head = stmt.word_text(0);
    if head.is_empty()
        || head == "available?"
        || is_general_tcl(head)
        || pack_eval::denied_axis(head).is_some()
    {
        return false;
    }
    stmt.words.iter().all(|word| {
        word.braced
            || !(word.text.contains('$') || word.text.contains('[') || word.text.contains('\\'))
    })
}

/// Capture a run of static statements without the interpreter, recursing
/// into `command`/`subcommand` bodies through the same fast-or-evaluate
/// decision.
fn drive_static(
    mut drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    stmts: &[Stmt],
) -> Result<(), String> {
    for stmt in stmts {
        let at_pack = state.borrow().scopes.is_empty();
        let in_command = matches!(state.borrow().scopes.last(), Some((ScopeKind::Command, _)));
        match stmt.word_text(0) {
            "speclib" => {
                let args: Vec<String> = stmt.words.iter().skip(1).map(|w| w.text.clone()).collect();
                stage_speclib(drive.reborrow(), state, &args, stmt.line)?;
            }
            "command" if at_pack => {
                let name = stmt.word_text(1).to_owned();
                let overrides = stmt.words.iter().any(|w| w.text == "-override");
                let Some(body) = stmt.words.iter().skip(2).find(|w| w.braced) else {
                    state
                        .borrow_mut()
                        .pack_nodes
                        .push(PackNode::Command(StagedCommand {
                            name,
                            overrides,
                            line: stmt.line,
                            body: None,
                        }));
                    continue;
                };
                let block = BodyText {
                    text: body.text.clone(),
                    base: body.line,
                    verbatim: true,
                };
                stage_command(drive.reborrow(), state, name, overrides, stmt.line, &block)?;
            }
            "subcommand" if in_command => {
                let name = stmt.word_text(1).to_owned();
                let Some(body) = stmt.arg(2) else {
                    // A `subcommand` with no body word is silently
                    // dropped, on this route as on the interpreter's.
                    continue;
                };
                let block = BodyText {
                    text: body.text.clone(),
                    base: body.line,
                    verbatim: true,
                };
                stage_subcommand(drive.reborrow(), state, name, stmt.line, &block)?;
            }
            "default" => {
                let args: Vec<String> = stmt.words.iter().skip(1).map(|w| w.text.clone()).collect();
                let mut st = state.borrow_mut();
                track_declared_support(&mut st, &args, stmt.line);
                st.push_node(Node::Row(stmt.clone()));
            }
            "include" if at_pack => {
                let words: Vec<&str> = stmt.words.iter().skip(1).map(|w| w.text.as_str()).collect();
                stage_include(drive.reborrow(), state, &words, stmt.line)?;
            }
            _ => state.borrow_mut().push_node(Node::Row(stmt.clone())),
        }
    }
    Ok(())
}

/// The `available?` query (E-R1): answers against the union of the pack's
/// declared support and downgrades the pack's cacheability class.
fn available_handler(state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let mut st = state.borrow_mut();
        let line = st.absolute_line(ctx.line());
        let mut scratch = Log::default();
        let requirement = available::from_texts(args, line, &mut scratch);
        let required = requirement.dialects.unwrap_or_else(DialectSet::all);
        let declared = st.declared_dialects.unwrap_or_else(DialectSet::all);
        let answer = required.intersects(declared);
        if !st.target_dependent {
            st.target_dependent = true;
        }
        st.eval_notices.push(eval_notice(
            "pack",
            line,
            VocabularyClass::Presentation,
            format!(
                "target-dependent registration: `available? {}` evaluates against the \
                 union of the pack's declared support, so the pack's snapshot depends \
                 on the analysis target and is excluded from caching (design E-R1); \
                 prefer an `-available` row on the registration itself",
                args.join(" ")
            ),
        ));
        Ok(Some(if answer { "1" } else { "0" }.to_owned()))
    })
}

/// The unresolved-dispatch handler: a sandbox denial is a hard determinism
/// error; anything else is unknown vocabulary, captured so the replay
/// classifies it under §6.1.
fn unknown_handler(state: &Rc<RefCell<State>>) -> UnknownHandler {
    let state = Rc::clone(state);
    Rc::new(
        move |ctx: &mut PackEvalCtx<'_>, name: &str, args: &[String]| {
            if let Some(axis) = pack_eval::denied_axis(name) {
                return Err(format!(
                    "pack evaluation is deterministic: `{name}` is denied by the \
                     pack sandbox (determinism axis: {axis}) and no partial \
                     registration is kept (design E §1.2)"
                ));
            }
            let mut st = state.borrow_mut();
            let line = st.absolute_line(ctx.line());
            let stmt = st.captured(name, args, line);
            st.push_node(Node::Row(stmt));
            Ok(None)
        },
    )
}

// ---------------------------------------------------------------------------
// Evaluate
// ---------------------------------------------------------------------------

/// Evaluate a `.tclspec` source as a Tcl program (design E).
///
/// Never panics; every failure — including a blown budget or a determinism
/// denial — comes back as a [`Pack`] with [`Pack::load_error`] set, empty
/// commands, and one explaining notice, because registration is
/// transactional.
#[must_use]
pub fn evaluate_pack_with(source: &str, options: &EvalOptions) -> Pack {
    evaluate_pack_in(source, options, None)
}

/// [`evaluate_pack_with`] with an [`super::IncludeContext`] resolving
/// literal pack-scope `include` rows. A computed include (an
/// interpreter-path invocation inside a scope, or a substituted name) stays
/// refused under the determinism contract.
#[must_use]
pub fn evaluate_pack_in(
    source: &str,
    options: &EvalOptions,
    include: Option<Rc<super::IncludeContext>>,
) -> Pack {
    // The file entry point treats a leading byte-order mark as a prologue
    // (issue #1635), exactly as `pack_statements` does.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);

    // The file-level fast path: a wholly declarative pack — which is what
    // every shipped pack is — captures its registrations straight from the
    // CST and never builds an interpreter. Handing a 1.2 MB `.tclspec` to
    // the VM costs a lex and a bytecode compilation of the whole file before
    // the first registration word is even dispatched, and that is most of a
    // large pack's load time. Nothing about the answer changes: the same
    // staging, through the same `drive_static`, into the same replay.
    if options.static_fast_path
        && let Some(state) = drive_file_statically(source, include.clone())
    {
        return replay(state, options);
    }

    let state = new_state(source, include);
    let mut vocabulary: Vec<(&str, WordHandler)> = Vec::with_capacity(ROW_WORDS.len() + 5);
    for &word in ROW_WORDS {
        vocabulary.push((word, row_handler(word, &state)));
    }
    vocabulary.push(("speclib", speclib_handler(&state)));
    vocabulary.push(("command", command_handler(&state)));
    vocabulary.push(("subcommand", subcommand_handler(&state)));
    vocabulary.push(("available?", available_handler(&state)));
    vocabulary.push(("include", include_handler(&state)));
    let unknown = unknown_handler(&state);

    let outcome = pack_eval::run_pack_program(source, &vocabulary, &unknown, &options.config);
    drop(vocabulary);
    drop(unknown);
    let state = Rc::try_unwrap(state)
        .map(RefCell::into_inner)
        .unwrap_or_default();

    match outcome {
        Ok(()) => replay(state, options),
        Err(failure) => failed_pack(&state, &failure),
    }
}

/// The staging state one load starts from.
fn new_state(source: &str, include: Option<Rc<super::IncludeContext>>) -> Rc<RefCell<State>> {
    let state = Rc::new(RefCell::new(State::default()));
    {
        let mut st = state.borrow_mut();
        st.verbatim = VerbatimIndex::of(source);
        st.include = include;
        st.include_stack = vec![xxhash_rust::xxh3::xxh3_64(source.as_bytes())];
    }
    state
}

/// Drive the whole file through the static capture layer, with no
/// interpreter behind it.
///
/// `None` means some body — at any depth — was not static vocabulary, so the
/// load must run for real. The attempt is abandoned whole: the partially
/// built state is dropped and [`evaluate_pack_in`] starts again on the
/// interpreter path, which is why this cannot leave half a pack behind. The
/// cost of a failed attempt is one `static_stmt` pass over statements the
/// segmentation memo then hands straight back to the retry.
fn drive_file_statically(
    source: &str,
    include: Option<Rc<super::IncludeContext>>,
) -> Option<State> {
    let state = new_state(source, include);
    // `FileBom::Skip`: this is the file entry point, so a leading mark is a
    // prologue rather than part of the first word.
    let top = pack_statements(source);
    if !top.iter().all(static_stmt) {
        return None;
    }
    drive_static(
        Drive {
            ctx: None,
            fast_path: true,
        },
        &state,
        &top,
    )
    .ok()?;
    Rc::try_unwrap(state).ok().map(RefCell::into_inner)
}

/// A pack whose evaluation broke: transactional discard, one explaining
/// notice, and the reason as a typed [`LoadError`].
fn failed_pack(state: &State, failure: &PackEvalFailure) -> Pack {
    let mut pack = empty_pack();
    if let Some(speclib) = &state.speclib {
        pack.name.clone_from(&speclib.name);
        pack.dsl_version.clone_from(&speclib.version);
    }
    pack.target_dependent = state.target_dependent;
    let error = match failure {
        PackEvalFailure::Budget(axis) => LoadError::BudgetExhausted(axis),
        PackEvalFailure::Script(message)
            if message.contains("pack evaluation is deterministic:") =>
        {
            LoadError::Determinism(message.clone())
        }
        PackEvalFailure::Script(message)
            if message.contains("tcl::mathfunc::rand")
                || message.contains("tcl::mathfunc::srand") =>
        {
            LoadError::Determinism(format!(
                "pack evaluation is deterministic: `rand()`/`srand()` are denied \
                 (determinism axis: randomness) — {message}"
            ))
        }
        PackEvalFailure::Script(message) | PackEvalFailure::Compile(message) => {
            LoadError::EvaluationFailed(message.clone())
        }
        PackEvalFailure::Panic(payload) => {
            LoadError::EvaluationFailed(format!("the evaluation engine crashed: {payload}"))
        }
    };
    pack.notices.push(Notice {
        context: "pack".to_owned(),
        line: state.speclib.as_ref().map_or(1, |s| s.line),
        class: VocabularyClass::Semantic,
        message: format!(
            "{error}; registration is transactional, so nothing from this pack is \
             loaded (design E §1.2)"
        ),
    });
    pack.load_error = Some(error);
    pack
}

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

/// Whether this tier's registrations are gated by E-R2's untrusted rules.
///
/// The class is the one [`super::PackEnvironmentTier::provenance`] already
/// maps a tier to, so the loader and the registration layer cannot disagree
/// about what a tier means — before this was derived, the loader called
/// `Tier::Workspace` untrusted while the environment model called the same
/// tier [`Provenance::WorkspaceTrusted`].
///
/// Redesign §6.4 keys the workspace half on the **editor's Workspace Trust
/// state**, not on where the file was discovered: a *trusted* workspace pack
/// may `-override` a shipped command — that is the collision policy
/// [`crate::install`] implements, tests, and reports through
/// [`crate::pack::collision_notices`] — and only an *untrusted* workspace
/// needs "explicit trusted opt-in". Nothing on the discovery path is told
/// the trust state yet (redesign open item 19), so the untrusted class is
/// reachable today through the live Spec Studio override tier; the day the
/// editor's trust state is plumbed, it arrives as a tier whose provenance is
/// [`Provenance::WorkspaceUntrusted`] and this predicate already answers for
/// it.
fn untrusted(tier: Tier) -> bool {
    matches!(
        super::PackEnvironmentTier::of(tier).provenance(),
        Provenance::WorkspaceUntrusted | Provenance::StudioOverride | Provenance::Document
    )
}

/// The compiled command surface a workspace pack may not shadow: the
/// permissive all-Tcl view, the same registry the collision policy
/// consults.
fn compiled_command_exists(name: &str) -> bool {
    crate::environment::lenient_store().get(name).is_some()
}

/// The first E-R2 violation in a pack's registration record **as if** the
/// pack were untrusted at `tier`: the line it was declared on, and a
/// notice-ready message naming the provenance class.
///
/// Deliberately unconditional — it answers the hypothetical, so `tier` only
/// supplies the class the message names. That is what the caller wants:
/// reading the **record** rather than the evaluator's own staging lets the
/// verdict be asked of a snapshot that has already loaded, which is how an
/// authoring tool (`spectcl_check`, the Spec Studio's
/// `untrusted_tier_refusal`) tells its user "this loads for you, and would
/// be refused from an untrusted workspace" without evaluating the pack a
/// second time. The load's own gate is in [`replay`], under [`untrusted`].
#[must_use]
pub fn provenance_violation(pack: &Pack, tier: Tier) -> Option<(u32, String)> {
    provenance_violation_in(&pack.registrations, tier)
}

fn provenance_violation_in(registrations: &[Registration], tier: Tier) -> Option<(u32, String)> {
    let class = tier.label();
    for reg in registrations {
        match reg.word() {
            "command" if reg.has_flag("-override") && compiled_command_exists(reg.arg(1)) => {
                return Some((
                    reg.line(),
                    format!(
                        "command `{}` declares `-override` for a compiled command \
                         name, but this pack loads from the {class} tier; an \
                         untrusted pack may not shadow compiled family names, so \
                         the pack is not loaded (design E-R2)",
                        reg.arg(1)
                    ),
                ));
            }
            "dialect" => {
                return Some((
                    reg.line(),
                    format!(
                        "`dialect {}` declares compiled dialect axes, but this pack \
                         loads from the {class} tier; an untrusted pack may not alter \
                         dialect axes, so the pack is not loaded (design E-R2)",
                        reg.arg(1)
                    ),
                ));
            }
            "environment" => {
                if let Some(reserved) = environment_block::reserved_name(reg.arg(1)) {
                    let verb = if reg.has_flag("-extend") {
                        // §6.4: altering a canonical environment — detection
                        // rows and placements included — needs a trusted
                        // tier, exactly as claiming its name does.
                        "extends"
                    } else {
                        "claims"
                    };
                    return Some((
                        reg.line(),
                        format!(
                            "`environment {}` {verb} `{reserved}`, a compiled \
                             environment name, and this pack loads from the {class} \
                             tier; an untrusted pack may not touch reserved names, so \
                             the pack is not loaded (design E-R2)",
                            reg.arg(1)
                        ),
                    ));
                }
            }
            _ => {}
        }
    }
    None
}

/// Turn a completed evaluation's staging into a [`Pack`] through the
/// vocabulary's row readers.
fn replay(state: State, options: &EvalOptions) -> Pack {
    let mut log = Log {
        context: "pack".to_owned(),
        ..Log::default()
    };
    let mut pack = empty_pack();
    pack.target_dependent = state.target_dependent;

    let Some(speclib) = &state.speclib else {
        if state.refused_braced_name {
            pack.notices.extend(state.eval_notices);
            return pack;
        }
        log.say(1, "no `speclib` declaration; nothing loaded");
        pack.notices = log.notices;
        pack.notices.extend(state.eval_notices);
        return pack;
    };

    pack.name.clone_from(&speclib.name);
    pack.dsl_version.clone_from(&speclib.version);
    log.forward_vocabulary = super::declared_major(&pack.dsl_version).is_some()
        && !super::KNOWN_VOCABULARY_VERSIONS.contains(&pack.dsl_version.as_str())
        && tcl_registry::version::compare(&pack.dsl_version, super::NEWEST_VOCABULARY_VERSION)
            .is_gt();
    if check_vocabulary_version(&pack.dsl_version, speclib.line, &mut log) {
        pack.load_error = Some(LoadError::UnsupportedMajor(pack.dsl_version.clone()));
        pack.notices = log.notices;
        return pack;
    }
    if !speclib.had_body && state.pack_nodes.is_empty() {
        log.say(speclib.line, "`speclib` has no body block");
        pack.notices = log.notices;
        pack.notices.extend(state.eval_notices);
        return pack;
    }

    // The canonical record (design E-R11): what the program registered, in
    // the order it registered it. For a straight-line pack this is the file's
    // own statements — which is what makes `export` a byte-stable round trip
    // there and the *expansion* for a templated pack. Built before the E-R2
    // gate, which reads it, and published on the pack only if the gate
    // passes: a discarded pack registered nothing.
    let registrations = record_nodes(&state.pack_nodes);

    // E-R2: provenance gates what the registrations may touch, and a
    // violation is transactional — the whole pack is discarded.
    if untrusted(options.tier)
        && let Some((line, message)) = provenance_violation_in(&registrations, options.tier)
    {
        let error = LoadError::Provenance(message.clone());
        pack.notices.push(Notice {
            context: "pack".to_owned(),
            line,
            class: VocabularyClass::Semantic,
            message,
        });
        pack.load_error = Some(error);
        return pack;
    }

    // Pass 1: pack-level rows, in evaluation order — tables first, since
    // commands are staged separately and built in pass 2.
    let mut tables = PackTables::default();
    for node in &state.pack_nodes {
        if let PackNode::Row(stmt) = node {
            let is_command = apply_pack_stmt(&mut pack, &mut tables, stmt, &mut log);
            debug_assert!(!is_command, "a raw `command` row cannot reach pack scope");
        }
    }

    // Pass 2: the staged commands, through the shared builder.
    for node in &state.pack_nodes {
        let PackNode::Command(staged) = node else {
            continue;
        };
        if staged.name.is_empty() {
            log.say(staged.line, "`command` with no name dropped");
            continue;
        }
        let Some(body) = &staged.body else {
            log.say(
                staged.line,
                format!(
                    "`command {}` has no `{{ … }}` body block; dropped \
                     (an opening brace must be on the same line as the command name)",
                    staged.name
                ),
            );
            continue;
        };
        let built = command_from_parts(
            &staged.name,
            staged.overrides,
            staged.line,
            &tables,
            &mut log,
            |spec, acc, log| fill_command_nodes(body, &tables, spec, acc, log),
        );
        if let Some(command) = built {
            pack.commands.push(command);
        }
    }

    pack.registrations = registrations;

    // Sites the capture layer consumed itself (`include`): logged here so
    // they get the same per-site vocabulary-consistency notices a row the
    // readers saw would.
    for (line, word) in &state.newer_word_sites {
        log.since(*line, word, "2.0");
    }
    super::finish_pack_cores(&mut pack, &mut log);
    finish_newer_words(&pack, &mut log);
    pack.notices = log.notices;
    pack.notices.extend(state.eval_notices);
    pack
}

/// The staged top-level nodes as canonical registrations.
fn record_nodes(nodes: &[PackNode]) -> Vec<Registration> {
    nodes
        .iter()
        .map(|node| match node {
            PackNode::Row(stmt) => Registration::row(stmt),
            PackNode::Command(staged) => {
                let mut head = vec![
                    synth_word("command", staged.line),
                    synth_word(&staged.name, staged.line),
                ];
                if staged.overrides {
                    head.push(synth_word("-override", staged.line));
                }
                match &staged.body {
                    // A `command` whose body word never arrived stays a row,
                    // so the reload reports the same missing-block notice.
                    None => Registration::row_words(head, staged.line),
                    Some(body) => Registration::block(head, record_body(body), staged.line),
                }
            }
        })
        .collect()
}

/// One command or subcommand body's staged nodes as registrations.
fn record_body(nodes: &[Node]) -> Vec<Registration> {
    nodes
        .iter()
        .map(|node| match node {
            Node::Row(stmt) => Registration::row(stmt),
            Node::Sub(sub) => Registration::block(
                vec![
                    synth_word("subcommand", sub.line),
                    synth_word(&sub.name, sub.line),
                ],
                record_body(&sub.body),
                sub.line,
            ),
        })
        .collect()
}

/// Replay one command body's captured nodes into the shared accumulator.
fn fill_command_nodes(
    nodes: &[Node],
    tables: &PackTables,
    spec: &mut CommandSpec,
    acc: &mut CommandAcc,
    log: &mut Log,
) {
    for node in nodes {
        match node {
            Node::Row(stmt) => apply_command_stmt(spec, acc, stmt, tables, log),
            Node::Sub(sub) => {
                let hooks = &mut acc.hooks;
                let built = subcommand_from_parts(
                    &sub.name,
                    "subcommand",
                    sub.line,
                    log,
                    |built: &mut SubCommand, sacc: &mut SubAcc, log: &mut Log| {
                        for inner in &sub.body {
                            if let Node::Row(stmt) = inner {
                                apply_subcommand_stmt(
                                    built, sacc, stmt, tables, hooks, &sub.name, log,
                                );
                            }
                        }
                    },
                );
                if let Some(subcommand) = built {
                    acc.subcommands.push(subcommand);
                }
            }
        }
    }
}
