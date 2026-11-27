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

//! The **evaluation loader** — executable registration
//! (`docs/design/registry/spec-packs.md` § *The format: a Tcl program, evaluated by
//! our own toolchain*), and the *only* way a `.tclspec` source becomes a
//! [`Pack`].
//!
//! A pack file is evaluated as a Tcl program in the sandboxed, budgeted,
//! deterministic `tcl-vm` (`tcl_spec_hooks::pack_eval`). The registration
//! vocabulary — `speclib`, `command`, `subcommand`, `option`, `available`,
//! and every other statement word the vocabulary defines — is installed as
//! host commands, so general Tcl (`proc`, `foreach`, `set`, `if`, string and
//! list operations) works between them and a pack can template its
//! registrations.
//!
//! The interpreter is linked with the `programmed-packs` feature. Without it
//! only the static fast path below runs, and a pack it cannot capture fails
//! its load with a [`LoadError::EvaluationFailed`] naming the feature.
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
//!   [`denied_axis`](super::denied_axis) — an evaluated pack cannot reach a clock,
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
//!   untrusted provenance (a workspace the editor has not trusted, or a Spec
//!   Studio override), a `command` claiming a compiled name with `-override`,
//!   a `dialect` block (compiled dialect axes), or an `environment` block
//!   claiming a reserved compiled name fails the load with an error naming
//!   the provenance class — reusing the same reserved-name check
//!   `environment_block` performs.
//! - **Snapshot caching seam**: [`EvalSnapshotKey`] = (content hash,
//!   [`crate::VOCABULARY_VERSION`], [`LOADER_EVAL_VERSION`], tier, trust). This
//!   module computes the identity and nothing more: both storage tiers live
//!   in [`crate::cache`], which is the one door production code loads a pack
//!   through. Target-dependent packs (E-R1) are never stored.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[cfg(feature = "programmed-packs")]
use tcl_spec_hooks::pack_eval::{self, PackEvalConfig, PackEvalCtx, PackEvalFailure, WordHandler};

use super::{LoadError, Notice, Pack, Stmt, VocabularyClass, Word, empty_pack};
use crate::discovery::Tier;
use tcl_dialect::model::{SpecSurface, WorkspaceTrust};

mod capture_handlers;
mod replay;
mod static_drive;
mod verbatim;

use capture_handlers::capture_stmt;
#[cfg(feature = "programmed-packs")]
use capture_handlers::{
    available_handler, block_handler, command_handler, include_handler, row_handler,
    speclib_handler, subcommand_handler, unknown_handler,
};
pub use replay::provenance_violation;
use replay::replay;
#[cfg(test)]
use static_drive::is_general_tcl;
pub(crate) use static_drive::static_stmt;
use static_drive::{drive_file_statically, substitution_free};

/// The evaluation loader's own version, part of the snapshot cache key: a
/// change to how evaluation captures or replays invalidates every cached
/// evaluated snapshot exactly once, independent of the vocabulary version.
pub const LOADER_EVAL_VERSION: u32 = 2;

/// How to evaluate a pack: the provenance tier and trust state gating
/// registrations (E-R2), and the sandbox budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvalOptions {
    /// The tier the pack loads from. Whether its registrations are gated by
    /// E-R2's untrusted rules — no reserved compiled name, no compiled
    /// dialect axis — is [`Provenance::is_untrusted`]'s answer about the
    /// provenance [`super::PackEnvironmentTier::provenance`] maps this tier
    /// and [`Self::trust`] to, never a property of the tier alone.
    ///
    /// That makes [`Tier::StudioOverride`] untrusted whatever the editor
    /// says, and [`Tier::Workspace`] untrusted exactly when the editor has
    /// not trusted the workspace ([`Provenance::WorkspaceUntrusted`]); a
    /// trusted workspace is [`Provenance::WorkspaceTrusted`] and may
    /// `-override` a shipped command. This comment once named both tiers
    /// untrusted outright, which the discovery location never decided
    /// (#2139).
    ///
    /// [`Provenance::is_untrusted`]: tcl_dialect::model::Provenance::is_untrusted
    /// [`Provenance::WorkspaceTrusted`]: tcl_dialect::model::Provenance::WorkspaceTrusted
    /// [`Provenance::WorkspaceUntrusted`]: tcl_dialect::model::Provenance::WorkspaceUntrusted
    pub tier: Tier,
    /// The editor's Workspace Trust state the pack loads under. Only the
    /// workspace tier reads it ([`Tier::trust_under`]); the default, a client
    /// that reports nothing, is trusted.
    pub trust: WorkspaceTrust,
    /// The budgets evaluation runs under.
    #[cfg(feature = "programmed-packs")]
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
    /// Whether `command` declarations are evaluated. `false` loads a pack's
    /// **manifest** — its environments, extension routes, ambient packages,
    /// rosters and special variables — and skips every command body, which is
    /// nearly all of a vendor library's text (`crate::demand`).
    pub commands: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            tier: Tier::Bundled,
            trust: WorkspaceTrust::Trusted,
            #[cfg(feature = "programmed-packs")]
            config: PackEvalConfig::default(),
            static_fast_path: true,
            commands: true,
        }
    }
}

/// The identity an evaluated snapshot caches under (deep dive §1.1): the
/// file bytes, the vocabulary, the evaluation loader build, and the
/// provenance — the tier and the trust state, which together gate what the
/// evaluation was allowed to register, so both are part of the answer's
/// identity.
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
    /// The trust state the tier read ([`Tier::trust_under`]): the editor's
    /// for a workspace pack, trusted for every other tier, so an untrusted
    /// workspace's snapshot never answers for a trusted one's.
    pub trust: WorkspaceTrust,
    /// [`EvalOptions::commands`]: a manifest is not the pack.
    pub commands: bool,
}

/// The snapshot cache key for one source under one set of options.
#[must_use]
pub fn eval_snapshot_key(source: &str, options: &EvalOptions) -> EvalSnapshotKey {
    EvalSnapshotKey {
        content_hash: content_hash(source),
        vocabulary: crate::VOCABULARY_VERSION,
        loader_eval_version: LOADER_EVAL_VERSION,
        tier: options.tier,
        trust: options.tier.trust_under(options.trust),
        commands: options.commands,
    }
}

/// Evaluate a `.tclspec` source as a Tcl program (design E) with default
/// options: bundled-tier trust, default budgets.
#[must_use]
pub fn evaluate_pack(source: &str) -> Pack {
    evaluate_pack_with(source, &EvalOptions::default())
}

// Capture

/// Every statement word that is a plain **row** at some scope: captured
/// verbatim and replayed through the scope's own reader, which is also what
/// sorts a word used at the wrong scope into the same unknown-property
/// unknown-property notice.
///
/// `speclib`, `command`, `subcommand`, and `available?` are not here — each
/// has its own handler with body evaluation or an answer.
#[cfg(feature = "programmed-packs")]
const ROW_WORDS: &[&str] = &[
    // pack scope
    "values",
    "descriptor",
    "hook",
    "default",
    "display_name",
    "file_extension",
    "ambient_package",
    "special_var",
    "provides",
    "co_provides",
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
    "source_deprecation_advice",
    "alias_of",
    "runtime_backing",
    "xc_translatable",
    "arg",
    "repeat",
    "reserved_trailing_words",
    "option_prefix_words",
    "assigns_variable_at",
    "creates_instance_at",
    "defines_command_at",
    "body_arg_implicit_args",
    "body_kind",
    "body_execution",
    "procedure_definition",
    "native_compilation",
    "successful_handler",
    "variable_receivers",
    "allow_unknown_subcommands",
    "dynamic_surface",
    "unknown_members",
    "prefix_matching",
    "self_receiver_words",
    "return_type",
    "completion",
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
    "refine",
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
    "irules_source_flow",
    "event_handler_priority",
    "data_collection",
    "event_requirement_form",
    "body_scope",
    "taint_source",
    "taint_transform",
    "taint_transform_when",
    "taint_double_encode_colour",
    "taint_sink_safe_colour",
    "taint_output_sink",
    "taint_log_sink",
    "taint_output_sink_subcommands",
    "taint_interp_eval_subcommands",
    "taint_network_sink_args",
    "taint_code_sink_args",
    "taint_numeric_coercion",
    "callback_taint_inputs",
    "channel_configuration",
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
    "arg_role_resolver_roles",
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
    "header_write",
    "destructive",
    "returns_path",
    "source_index_bounds",
    "source_path_operation",
    "source_configuration_reference",
    "is_unescape",
    "loop_list_header",
    "creates_scope_alias",
    "arg_values_accept_prefix", // Shared with command scope; both use the same flag reader.
    "cfg_rewrite_name",
    "min_abbrev",
    "max_leading_option_words",
    "credential_arg",
    "sub_subcommand",
];

/// The interpreter's place in a [`Drive`] when none is linked: never built.
#[cfg(not(feature = "programmed-packs"))]
struct PackEvalCtx<'a>(std::convert::Infallible, std::marker::PhantomData<&'a ()>);

#[cfg(not(feature = "programmed-packs"))]
impl PackEvalCtx<'_> {
    fn eval_body(&mut self, _body: &str) -> Result<(), String> {
        match self.0 {}
    }
}

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
/// hands on when it recurses into a body.
///
/// `fast_path` is the *load's* setting, threaded through the handler rather
/// than assumed: a load that asked for no fast path must not get one back
/// for its nested bodies, which is exactly what the gate in
/// `tests/eval_loader.rs` relies on to compare the two routes.
#[cfg(feature = "programmed-packs")]
fn interpreted<'a, 'b>(ctx: &'a mut PackEvalCtx<'b>, fast_path: bool) -> Drive<'a, 'b> {
    Drive {
        ctx: Some(ctx),
        fast_path,
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

/// One `environment` / `dialect` declaration captured at pack scope, with
/// its body evaluated as a script.
///
/// A pack is a Tcl program, so these blocks are programs too: a version
/// shared by several environments is an ordinary variable substituted into
/// each `ambient` row, and a repetitive ladder is a `foreach`. Only the
/// rows the body registered are kept — the block's own
/// reader (`environment_block` / `dialect_block`) is still the single
/// owner of what a row means.
#[derive(Debug)]
struct StagedBlock {
    /// `environment` or `dialect`.
    word: &'static str,
    /// The declaration's own words — the block word, the name, and any
    /// flag — with the body word left out.
    head: Vec<Word>,
    line: u32,
    /// `None` when no `{ … }` body word was found, which the block reader
    /// reports as the brace-on-the-next-line mistake.
    body: Option<Vec<Node>>,
}

/// One captured top-level statement.
#[derive(Debug)]
enum PackNode {
    Row(Stmt),
    Command(StagedCommand),
    Block(StagedBlock),
}

/// What scope the evaluation is currently capturing into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Command,
    Subcommand,
    /// An `environment` or `dialect` body. Its rows are whatever the block
    /// reader accepts, so nothing is dispatched specially inside it — an
    /// unknown word is captured like any other row and the block reader
    /// decides, which is what makes those rows semantic-class by scope.
    Block,
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
    original: Option<tcl_source::spectcl::OriginalPackSourceWord>,
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

/// The `speclib` header as captured.
#[derive(Debug)]
struct SpeclibDecl {
    name: String,
    version: String,
    line: u32,
    had_body: bool,
}

/// Independently retained declared support and its unavailable-input obligation.
/// The unavailable flag never supplies an unstated surface default.
#[derive(Default)]
struct DeclaredSupport {
    surface: Option<&'static [SpecSurface]>,
    unavailable: bool,
}

/// Whether an evaluation stages `command` declarations ([`EvalOptions::commands`]).
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Commands {
    #[default]
    Staged,
    Skipped,
}

/// Everything the evaluation stages before replay.
#[derive(Default)]
struct State {
    /// [`EvalOptions::commands`]: whether `command` declarations stage anything.
    commands: Commands,
    pack_nodes: Vec<PackNode>,
    /// Open capture scopes, innermost last, each with its collected nodes.
    scopes: Vec<(ScopeKind, Vec<Node>)>,
    speclib: Option<SpeclibDecl>,
    /// `speclib` saw a braced word where its name belongs — the CST
    /// loader's refusal, replayed as "nothing loaded".
    refused_braced_name: bool,
    /// Notices the evaluation itself produced (extra `speclib` blocks,
    /// E-R1 target-dependence), appended after the replay's notices.
    eval_notices: Vec<Notice>,
    /// Whether `available?` was used (E-R1).
    target_dependent: bool,
    /// The first sandbox denial the evaluation raised. A denial is a hard
    /// determinism error even when the pack catches it, so the failure is
    /// classified from this record, never from the guest error text.
    #[cfg(feature = "programmed-packs")]
    determinism_denial: Option<String>,
    /// The absolute base line of each unit being evaluated, innermost
    /// last; a captured statement's absolute line is `base + relative - 1`.
    base_lines: Vec<u32>,
    // Source-only original container correspondence. None remains unavailable
    // through nested bodies; reader values and entered execution are independent.
    original_body_sources: Vec<Option<tcl_source::spectcl::OriginalPackSourceWord>>,
    /// The union of the pack's declared support so far (`default
    /// available` / `default dialects`), the set `available?` answers
    /// against. `None` = nothing declared = everything.
    declared_support: DeclaredSupport,
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
    /// The content hash of every fragment an `include` row evaluated, in
    /// inclusion order — what the commands' content hash folds in beside
    /// the root file's own ([`pack_content_hash`]).
    included: Vec<u64>,
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
    #[cfg(feature = "programmed-packs")]
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

    /// The file's own statement for a captured invocation, when the file
    /// has one of the same shape at that line. Inside an included fragment
    /// the verbatim index (which describes the root file) is not consulted,
    /// so a line-number coincidence cannot substitute the wrong statement.
    ///
    /// The source statement stands in only when it **substitutes nothing**.
    /// A row built from a variable or a command substitution has values the
    /// source text does not spell, and replaying `ambient Tk $tkver`
    /// verbatim would hand the reader the dollar sign instead of the
    /// version.
    fn source_stmt(&mut self, word: &str, args: &[String], line: u32) -> Option<Stmt> {
        if self.in_include {
            return None;
        }
        self.verbatim
            .row(word, args, line)
            .filter(substitution_free)
            .map(|mut row| {
                let parent = self.original_body_sources.last();
                let owned = match parent {
                    Some(None) => false,
                    Some(Some(parent)) => row.words.iter().all(|word| {
                        word.original_source
                            .as_ref()
                            .is_some_and(|original| original.matches_original_parent(Some(parent)))
                    }),
                    None => row.words.iter().all(|word| {
                        word.original_source
                            .as_ref()
                            .is_some_and(|original| original.matches_original_parent(None))
                    }),
                };
                if !owned {
                    for word in &mut row.words {
                        word.original_source = None;
                    }
                }
                row
            })
    }

    /// A captured invocation as a [`Stmt`]: the file's verbatim statement
    /// when the invocation corresponds to one, the evaluated values
    /// otherwise.
    fn captured(&mut self, word: &str, args: &[String], line: u32) -> Stmt {
        self.source_stmt(word, args, line)
            .unwrap_or_else(|| capture_stmt(word, args, line))
    }
}

/// The evaluated registration vocabulary's exact `NAME ?-override? BODY`
/// layout. These are already evaluated Unicode values, not original words or
/// a reusable source recipe. Unknown layouts cannot donate body geometry.
#[cfg(feature = "programmed-packs")]
struct EvaluatedCommandBody<'a> {
    text: &'a str,
    overrides: bool,
}

/// One body block ready to run: its text, the base line of its unit, and
/// whether the text is the file's verbatim bytes (which is what licenses
/// the static fast path).
struct BodyText {
    text: String,
    base: u32,
    verbatim: bool,
    original: Option<tcl_source::spectcl::OriginalPackSourceWord>,
}

// Evaluate

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

/// [`evaluate_pack_with`] with an [`super::IncludeContext`] resolving actual
/// checked pack-scope include invocations, including helper/computed calls on
/// the interpreter path. Resolution stays lazy and uses the same include-name,
/// content-cycle, depth and determinism guards. An include in a DSL command or
/// subcommand scope remains an ordinary unknown-property row.
#[must_use]
pub fn evaluate_pack_in(
    source: &str,
    options: &EvalOptions,
    include: Option<Rc<super::IncludeContext>>,
) -> Pack {
    // The file's own content hash, taken before the prologue is stripped:
    // the value its snapshot key interns (`eval_snapshot_key`).
    let root = content_hash(source);
    // The file entry point treats a leading byte-order mark as a prologue,
    // exactly as `pack_statements` does.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);

    // The file-level fast path: a wholly declarative pack — which is what
    // every shipped pack is — captures its registrations straight from the
    // CST and never builds an interpreter. Handing a 1.2 MB `.tclspec` to
    // the VM costs a lex and a bytecode compilation of the whole file before
    // the first registration word is even dispatched, and that is most of a
    // large pack's load time. Nothing about the answer changes: the same
    // staging, through the same `drive_static`, into the same replay.
    if options.static_fast_path
        && let Some(state) = drive_file_statically(source, include.clone(), options.commands)
    {
        let hash = pack_content_hash(root, &state.included);
        return with_content_hash(replay(state, *options), hash);
    }
    interpret(source, root, *options, include)
}

/// A pack the static reader could not capture, run in the VM.
#[cfg(feature = "programmed-packs")]
fn interpret(
    source: &str,
    root: u64,
    options: EvalOptions,
    include: Option<Rc<super::IncludeContext>>,
) -> Pack {
    let state = new_state(source, include, options.commands);
    let mut vocabulary: Vec<(&str, WordHandler)> = Vec::with_capacity(ROW_WORDS.len() + 5);
    for &word in ROW_WORDS {
        vocabulary.push((word, row_handler(word, &state)));
    }
    let fast_path = options.static_fast_path;
    vocabulary.push(("speclib", speclib_handler(&state, fast_path)));
    vocabulary.push(("command", command_handler(&state, fast_path)));
    vocabulary.push(("subcommand", subcommand_handler(&state, fast_path)));
    vocabulary.push(("available?", available_handler(&state)));
    vocabulary.push(("include", include_handler(&state, fast_path)));
    vocabulary.push((
        "environment",
        block_handler("environment", &state, fast_path),
    ));
    vocabulary.push(("dialect", block_handler("dialect", &state, fast_path)));
    let unknown = unknown_handler(&state);

    let outcome = pack_eval::run_pack_program(source, &vocabulary, &unknown, &options.config);
    drop(vocabulary);
    drop(unknown);
    let state = Rc::try_unwrap(state).map_or_default(RefCell::into_inner);

    match outcome {
        Ok(()) => {
            let hash = pack_content_hash(root, &state.included);
            with_content_hash(replay(state, options), hash)
        }
        Err(failure) => failed_pack(&state, &failure),
    }
}

/// Without the VM, a pack the static reader could not capture fails its load.
#[cfg(not(feature = "programmed-packs"))]
fn interpret(
    _source: &str,
    _root: u64,
    _options: EvalOptions,
    _include: Option<Rc<super::IncludeContext>>,
) -> Pack {
    let error = LoadError::EvaluationFailed(
        "the pack is a Tcl program and this build links no interpreter to run it \
         (`tcl-spectcl`'s `programmed-packs` feature)"
            .to_owned(),
    );
    let mut pack = empty_pack();
    pack.notices.push(Notice {
        subject: None,
        newer_vocabulary: None,
        context: "pack".to_owned(),
        line: 1,
        class: VocabularyClass::Semantic,
        message: format!("{error}; nothing from this pack is loaded"),
    });
    pack.load_error = Some(error);
    pack
}

/// xxh3 of a pack source's bytes — the one hashing rule the snapshot key,
/// the include-cycle key, and a pack-fact stamp all use.
#[must_use]
pub(crate) fn content_hash(source: &str) -> u64 {
    xxhash_rust::xxh3::xxh3_64(source.as_bytes())
}

/// The content hash the commands of the pack file at `path` carry
/// ([`tcl_runtime_api::PackFactStamp::content_hash`]), for the file's
/// `source`: the xxh3 of its bytes, folded with each fragment an `include`
/// row brought in. The one function the lockfile's pack hash and a compiled
/// unit's claims are both built from, so the two cannot be hashed by
/// different rules.
///
/// A file whose pack declares no command carries no hash of its own; its
/// fragments are then not read, and the value is the file's bytes alone.
#[must_use]
pub fn pack_file_hash(path: &std::path::Path, source: &str) -> u64 {
    let root = content_hash(source);
    if !super::uses_include(source) {
        return root;
    }
    let include = Rc::new(super::IncludeContext::for_file(path));
    evaluate_pack_in(source, &EvalOptions::default(), Some(include))
        .commands
        .first()
        .map_or(root, |command| command.content_hash)
}

/// The content hash a pack's commands carry: the root file's own when it
/// included nothing, otherwise one xxh3 over the root's hash and each
/// included fragment's in inclusion order, so an edit to an included file
/// moves it too.
fn pack_content_hash(root: u64, included: &[u64]) -> u64 {
    if included.is_empty() {
        return root;
    }
    let bytes: Vec<u8> = std::iter::once(root)
        .chain(included.iter().copied())
        .flat_map(u64::to_le_bytes)
        .collect();
    xxhash_rust::xxh3::xxh3_64(&bytes)
}

fn with_content_hash(mut pack: Pack, hash: u64) -> Pack {
    for command in &mut pack.commands {
        command.content_hash = hash;
    }
    pack
}

/// The staging state one load starts from.
fn new_state(
    source: &str,
    include: Option<Rc<super::IncludeContext>>,
    commands: bool,
) -> Rc<RefCell<State>> {
    let state = Rc::new(RefCell::new(State::default()));
    {
        let mut st = state.borrow_mut();
        st.commands = if commands {
            Commands::Staged
        } else {
            Commands::Skipped
        };
        st.verbatim = VerbatimIndex::of(source);
        st.include = include;
        st.include_stack = vec![content_hash(source)];
    }
    state
}

/// A pack whose evaluation broke: transactional discard, one explaining
/// notice, and the reason as a typed [`LoadError`].
#[cfg(feature = "programmed-packs")]
fn failed_pack(state: &State, failure: &PackEvalFailure) -> Pack {
    let mut pack = empty_pack();
    if let Some(speclib) = &state.speclib {
        pack.name.clone_from(&speclib.name);
        pack.dsl_version.clone_from(&speclib.version);
    }
    pack.target_dependent = state.target_dependent;
    let error = match (failure, &state.determinism_denial) {
        (PackEvalFailure::Budget(axis), _) => LoadError::BudgetExhausted(axis),
        (PackEvalFailure::Panic(payload), _) => {
            LoadError::EvaluationFailed(format!("the evaluation engine crashed: {payload}"))
        }
        (_, Some(denial)) => LoadError::Determinism(denial.clone()),
        (PackEvalFailure::Script(message) | PackEvalFailure::Compile(message), None) => {
            LoadError::EvaluationFailed(message.clone())
        }
        (PackEvalFailure::ScriptBytes(_) | PackEvalFailure::ExecutionRefusal(_), None) => {
            LoadError::EvaluationFailed(failure.to_string())
        }
    };
    pack.notices.push(Notice {
        subject: None,
        newer_vocabulary: None,
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

#[cfg(test)]
mod original_head_namespace_tests;

#[cfg(test)]
mod original_notice_body_capture_tests;

#[cfg(test)]
mod evaluated_source_correspondence_controls;

#[cfg(test)]
mod evaluated_command_layout_controls;
