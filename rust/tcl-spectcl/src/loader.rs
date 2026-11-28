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

//! `SpecTcl` — the `.tclspec` spec-pack loader.
//!
//! **One loader**, in two halves that are not two ways of doing the same
//! thing:
//!
//! - **[`evaluate_pack`]** (in [`eval`]) — design E's evaluation loader: the
//!   pack runs as a sandboxed, deterministic, budgeted Tcl program whose
//!   registration words *capture* [`Stmt`]s rather than interpreting them.
//!   It is the only door from `.tclspec` text to a [`Pack`], for every
//!   consumer — the LSP's discovery and reload path (through
//!   [`crate::cache::evaluate_pack_cached`]), the Spec Studio, the `tcl` CLI
//!   and the MCP server.
//! - **The row readers in this module** — [`apply_pack_stmt`],
//!   [`apply_command_stmt`], [`apply_subcommand_stmt`],
//!   [`command_from_parts`], [`subcommand_from_parts`] and the property
//!   readers under them. They are the `SpecTcl` *vocabulary*: what a word
//!   means once a registration has been captured. Every registration reaches
//!   them, whether the file spelled it literally or a `foreach` computed it.
//!
//! There is no second front end. A pack whose statements are all static
//! vocabulary never reaches the interpreter at all — the evaluation loader's
//! static fast path captures them straight from the CST — but that is one
//! loader taking a shortcut through its own capture layer, not a parse-only
//! twin: the same staging, the same replay, the same readers.
//!
//! Segmentation uses the same
//! [`build_document`](tcl_source::cst::build::build_document) /
//! [`segments_from_document`](tcl_source::cst::segment::segments_from_document)
//! pair every static scan in the toolchain uses — and the readers turn the
//! declarations into live [`CommandSpec`]s, which the Spec Studio then seeds a
//! draft from through the ordinary `draft::from_command_spec`. Loading a pack
//! and browsing a shipped command therefore produce drafts by the *same* code,
//! which is what makes the per-port equivalence gate in
//! `tcl-spec-studio-it/tests/it_3/spectcl_ports.rs` meaningful.
//!
//! The frozen syntax is `docs/design/spec-dsl-examples/README.md`; the
//! architecture around it is `docs/design/registry/spec-packs.md`.
//!
//! ## What the loader does, and does not, do
//!
//! - **Never evaluates a hook body.** Registration runs; hook bodies do not.
//!   They are carried as text ([`HookDecl`]) together with their family and that family's declared
//!   metadata — emitter verbs, what silence means, and whether the family may
//!   only run on an all-literal call. Running them in the `tcl-vm` sandbox is
//!   later work, so every pack-declared hook is installed as an **abstaining**
//!   function pointer: the conservative answer for its family, and the same
//!   answer an erroring body is required to give.
//! - **Vocabulary tolerance.** An unknown property word, an unknown flag on a
//!   known row, an unknown trait / role / colour / dialect / hook name is
//!   *dropped with a notice* ([`Notice`]) and the rest of the pack loads. A
//!   pack never fails to load because the server is older than it.
//! - **`&'static` by leaking.** A `CommandSpec` is a `&'static`-shaped record,
//!   because the shipped ones live in `.rodata`. A loaded pack is likewise
//!   installed for the process's life, so its strings and slices are leaked
//!   once at load rather than reference-counted.
//!
//! ## Descriptor fields
//!
//! Ten properties take a block. Where the block is plain data the loader
//! builds the real descriptor ([`FrameEffectSpec`], [`EventRequires`],
//! [`CaseListSpec`], [`DefinitionBodyGrammar`], …). Where a descriptor also
//! carries a resolver function pointer, the plain-data part is built and the
//! resolver abstains — exactly the split
//! `docs/design/spec-dsl-examples/README.md` draws under "What a pack cannot
//! author". Either way the studio draft records such a field the way it
//! records the shipped one: as a key whose *defining Rust expression* could
//! not be recovered.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::sync::{LazyLock, Mutex};

use tcl_dialect::TclVersion;
#[cfg(test)]
use tcl_lexer::LeadingBom;
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::clause_grammar::ClauseGrammarSpec;
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{ArgValue, OptionValueOutcome, ScriptTiming};
use tcl_registry::literal_validation::LiteralArgumentValidation;
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::presentation::ArgPresentation;
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_registry::state_transition::StateTransitions;
use tcl_registry::{CommandPrefixArguments, InvocationArguments};
use tcl_source::spectcl::declared_pack_major as declared_major;
pub use tcl_source::spectcl::pack_source_lexer_config;

use tcl_dialect::model::SpecSurface;

mod available;
mod dialect_block;
mod environment_block;
mod eval;
mod reference;
mod semantics;
mod surface_roster;
mod vocabulary_class;

pub use dialect_block::{PackDialect, PackDialectAxis};
pub(crate) use environment_block::reserved_name_for as reserved_environment_name_for;
pub use environment_block::{PackCore, PackEnvironment, PackEnvironmentTier};
pub(crate) use eval::static_stmt;
pub use eval::{
    EvalOptions, EvalSnapshotKey, LOADER_EVAL_VERSION, eval_snapshot_key, evaluate_pack,
    evaluate_pack_in, evaluate_pack_with, pack_file_hash, provenance_violation,
};
pub(crate) use reference::derive_implementations;
pub use surface_roster::{PackRosterName, PackSurfaceRoster, family_named};
pub use vocabulary_class::VocabularyClass;

mod block_descriptors;
mod body_rows;
mod command;
mod notices;
mod pack_scope;
mod statement_reader;
mod subcommand;
mod value_parsers;

pub use block_descriptors::{SHIPPED_DEFINITION_BODIES, roles_from_manufacturers};
pub use command::hook_path;
use command::{CommandAcc, apply_command_stmt, command_from_parts};
use notices::quotable;
pub use notices::{NewerVocabulary, Notice};
pub(crate) use pack_scope::{
    INCLUDE_DEPTH_LIMIT, IncludeSourceObservation, include_name, uses_include,
};
use pack_scope::{
    apply_pack_stmt, check_vocabulary_version, finish_newer_words, finish_pack_cores,
    read_dialect_rows, read_environment_rows,
};
pub use statement_reader::speclib_version_span;
pub(crate) use statement_reader::{FileBom, Stmt, Word, block, pack_statements, statements};
use statement_reader::{body_statements, source_config};
use subcommand::{SubAcc, apply_subcommand_stmt, subcommand_from_parts};
pub(crate) use value_parsers::list_words;
use value_parsers::{enum_by_name, next_text, parse_dialects};
pub use value_parsers::{semantic_operation_spelling, semantic_operations};

/// Accumulates notices under a movable context label.
#[derive(Debug, Default)]
struct Log {
    context: String,
    source: Option<std::sync::Arc<str>>,
    /// Row subjects of `source` by line, indexed on first use.
    rows:
        Option<std::collections::BTreeMap<u32, tcl_lsp_core::code_actions::SpecPackNoticeSubject>>,
    notices: Vec<Notice>,
    /// Sites that used vocabulary newer than 1.0, as
    /// `(context, line, word, first vocabulary that has it)`.
    ///
    /// Drained by the loader into per-site notices for every site whose
    /// word is newer than the pack's declared vocabulary — the words still
    /// load (additions never gate), but a loader speaking only the declared
    /// vocabulary drops them silently, and the declaration is the pack's only
    /// way to say it needs them.
    ///
    /// Carrying the introducing version per site is what lets one mechanism
    /// serve every future vocabulary: a 1.2 word under a 1.1 declaration is
    /// the same defect as a 1.1 word under 1.0, and neither is a defect under
    /// a declaration at or above it.
    newer_words: Vec<(String, u32, String, &'static str)>,
    /// Whether the pack declares a vocabulary this build postdates, which
    /// is the only direction §6.1's fail-closed classes apply in.
    forward_vocabulary: bool,
    /// Whether an assistance-class unknown word was seen in the spec being
    /// read. Reset by [`Log::begin_spec`].
    assistance_unknown: bool,
    /// Whether a semantic-class unknown word was seen in the spec being
    /// read. Reset by [`Log::begin_spec`].
    semantic_unknown: bool,
    /// The command whose body is being read, for the scope a subcommand's
    /// `semantics`, `evaluate` and `facts` ids are spelled under.
    command: String,
}

struct RejectedWordSubject {
    original: Option<tcl_source::spectcl::OriginalPackSourceWord>,
    word: String,
    kind: tcl_lsp_core::code_actions::SpecPackNoticeKind,
}

// Leaking — a loaded pack lives as long as the process, like a shipped spec

/// Leak `text` as `&'static str`, **interned**: the same string is leaked once
/// for the life of the process however many pack generations contain it.
///
/// A loaded pack's data must be `&'static` because that is what a
/// `CommandSpec` is made of, and a leak can never be handed back. Without
/// interning, every reload leaked a fresh copy of *everything* — each edit of
/// one summary re-leaked every command name, synopsis, option detail and
/// keyword in the pack. Since the overwhelming majority of a pack is
/// byte-identical across an edit, interning collapses the marginal cost of a
/// reload to the strings that actually changed.
///
/// This bounds the growth; it does not end it. Retiring a whole generation
/// once no registry snapshot references it needs the registry's `'static`
/// specs to become refcounted, which is a change to its public type and not
/// one to smuggle in here.
pub(crate) fn leak_str(text: &str) -> &'static str {
    static INTERNED: LazyLock<Mutex<HashSet<&'static str>>> =
        LazyLock::new(|| Mutex::new(HashSet::new()));

    let mut interned = match INTERNED.lock() {
        Ok(interned) => interned,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(existing) = interned.get(text) {
        return existing;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    interned.insert(leaked);
    leaked
}

fn leak_slice<T>(items: Vec<T>) -> &'static [T] {
    if items.is_empty() {
        return &[];
    }
    Box::leak(items.into_boxed_slice())
}

fn leak_strs(items: &[String]) -> &'static [&'static str] {
    leak_slice(items.iter().map(|s| leak_str(s)).collect())
}

fn leak_one<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

// Hook declarations: bodies are carried as text for the hook host.

/// Which hook family a body belongs to.
///
/// The family is what decides a body's emitter verbs, what its *silence*
/// means, and whether it may run at all against a call carrying a dynamic
/// word. Those are the three things `README.md`'s "When a hook runs" and
/// "Outputs: the emitter protocol" pin normatively, and they live on the
/// **registry's** type ([`tcl_registry::pack_hooks::HookFamily`]) rather than
/// on a loader-local copy: the registry owns the vocabulary, and the hook host
/// that actually runs a body reads the same enum the loader classified it
/// with, so the two cannot drift into disagreeing about what silence means.
pub use tcl_registry::pack_hooks::HookFamily;

/// The whitelist the hook host evaluates a body against.
///
/// Recorded here so the loader can report a body reaching for
/// something it will never be given. Deliberately absent: `open`, `exec`,
/// `source`, `socket`, `after`, `interp`, `uplevel`, `upvar`, `trace`,
/// `namespace`, `proc`, `rename`, `info`, `subst`.
pub const SANDBOX_COMMANDS: &[&str] = &[
    "set", "expr", "if", "while", "for", "foreach", "switch", "return", "break", "continue",
    "incr", "lappend", "lassign", "list", "lindex", "llength", "lrange", "lreplace", "lsearch",
    "lsort", "join", "split", "string", "format", "scan", "regexp", "regsub", "dict", "binary",
];

/// The general Tcl the pack-evaluation host gives a pack program beyond
/// [`SANDBOX_COMMANDS`], recorded here so the static reader knows a statement
/// it heads must really evaluate, with or without the host linked.
pub const PACK_EVAL_EXTRA_COMMANDS: &[&str] = &[
    "proc",
    "apply",
    "eval",
    "namespace",
    "variable",
    "global",
    "concat",
    "append",
    "unset",
    "catch",
    "error",
    "throw",
    "try",
    "lmap",
    "linsert",
    "lrepeat",
    "lreverse",
    "lset",
    "lpop",
    "ledit",
    "lseq",
    "array",
    "const",
    "fpclassify",
];

/// Commands the pack sandbox deliberately denies, each with the determinism
/// axis its absence protects. An unresolved dispatch of one of these is a
/// **sandbox denial** — a hard, transactional load failure — where an
/// unresolved word outside this table is unknown *vocabulary* and follows the
/// loader's §6.1 classification instead.
const DENIED_COMMANDS: &[(&str, &str)] = &[
    ("clock", "clock/time"),
    ("time", "clock/time"),
    ("after", "event loop"),
    ("update", "event loop"),
    ("vwait", "event loop"),
    ("open", "file/channel access"),
    ("close", "file/channel access"),
    ("read", "file/channel access"),
    ("gets", "file/channel access"),
    ("puts", "file/channel access"),
    ("seek", "file/channel access"),
    ("tell", "file/channel access"),
    ("eof", "file/channel access"),
    ("flush", "file/channel access"),
    ("fblocked", "file/channel access"),
    ("fconfigure", "file/channel access"),
    ("file", "file/channel access"),
    ("glob", "file/channel access"),
    ("cd", "file/channel access"),
    ("pwd", "file/channel access"),
    ("source", "file/channel access"),
    ("socket", "network"),
    ("exec", "process execution"),
    ("exit", "process control"),
    ("package", "package loading"),
    ("auto_load", "package loading"),
    ("auto_import", "package loading"),
    ("encoding", "host environment"),
    ("interp", "interpreter escape"),
    ("info", "host introspection"),
    ("uplevel", "sandbox containment"),
    ("upvar", "sandbox containment"),
    ("trace", "sandbox containment"),
    ("rename", "sandbox containment"),
    ("subst", "sandbox containment"),
    ("tailcall", "sandbox containment"),
    ("coroutine", "sandbox containment"),
    ("yield", "sandbox containment"),
    ("yieldto", "sandbox containment"),
    // Swept from the sandbox, so `expr {rand()}` reaches `unknown` under its
    // math-function command name, as a missing function does in C Tcl.
    ("tcl::mathfunc::rand", "randomness"),
    ("tcl::mathfunc::srand", "randomness"),
];

/// The determinism axis `name` is denied on, when it is a denied command
/// rather than unknown vocabulary.
#[must_use]
pub(crate) fn denied_axis(name: &str) -> Option<&'static str> {
    // This classifier accepts written vocabulary spelling. Actual unknown
    // dispatch attempts command lookup before it reaches this classifier.
    let written = tcl_syntax::naming::canonical_written_command(name);
    let name = tcl_syntax::naming::unroot_rooted_key(&written).unwrap_or(&written);
    if name.starts_with("thread::") || name.starts_with("tpool::") || name.starts_with("tsv::") {
        return Some("threading");
    }
    DENIED_COMMANDS
        .iter()
        .find(|(denied, _)| *denied == name)
        .map(|(_, axis)| *axis)
}

/// Where a hook's behaviour comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookSource {
    /// A pure Tcl body with a proc-shaped signature, carried verbatim.
    Body {
        /// The declared parameter list, conventionally `words ctx`.
        params: Vec<String>,
        /// Every byte between the body braces.
        body: String,
        /// What `-inputs {…}` declared this body reads.
        ///
        /// The hook host turns this into the registry's cacheability rule
        /// ([`HookInputs::shape_only`]): a hook that declares only shape
        /// inputs is answered from the shape-keyed cache at native speed, and
        /// one that declares nothing (the default) is fully legal, always
        /// correct, and uncacheable — `docs/design/registry/spec-packs.md`'s
        /// "granularity is not restricted; consequences are documented".
        inputs: HookInputs,
    },
    /// `-native ID` — the engine, by name.
    Native {
        /// `command::hook` for a per-command implementation, or the bare
        /// catalogue variant for a shared one.
        id: String,
    },
    /// A closed keyword the loader derives from data the spec already
    /// declares (`from-manufacturers`, `from-frame-effect`, `clause_grammar`).
    Derived {
        /// The keyword as written.
        keyword: String,
    },
}

/// What a hook is attached to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOwner {
    /// A property on the command itself.
    Command,
    /// A property on one of its subcommands.
    Subcommand(String),
    /// The `-arity-hook` flag of one option row.
    Option {
        /// The owning subcommand, when the option is not a command-level one.
        subcommand: Option<String>,
        /// The option word as written.
        option: String,
    },
}

/// One declared hook: what it is attached to, which field, which family, the
/// text or name that supplies it, and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookDecl {
    /// What the hook hangs off.
    pub owner: HookOwner,
    /// The schema key it fills.
    pub field: &'static str,
    /// The family, which fixes the emitter protocol.
    pub family: HookFamily,
    /// Body text, native id, or derivation keyword.
    pub source: HookSource,
    /// The line of the row that declares it: the hook property, the
    /// `option` row carrying an `-arity-hook`, a `state_transitions`
    /// block's `resolver` row, the `evaluate` statement a declared
    /// implementation came from, or the `clause_grammar` a derivation reads.
    /// A notice about the hook itself — a dormant body in an untrusted
    /// workspace ([`crate::hooks::dormant_hooks`]) — goes here, in the file
    /// the merge records on the command.
    pub line: u32,
    /// The source subject of that row, issued by the loader when the row's
    /// head word is a plain literal alone on its line.
    pub row: Option<tcl_lsp_core::code_actions::SpecPackNoticeSubject>,
}

// The abstaining implementations installed for every pack-declared hook until
// the sandbox lands. Each is its family's documented silence, which is also
// what an erroring body is required to produce.

fn abstain_arg_roles(_args: &[&str]) -> Vec<(u8, ArgRole)> {
    Vec::new()
}

fn abstain_command_prefixes(_args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    Vec::new()
}

fn abstain_script_timings(_args: tcl_registry::InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    Vec::new()
}

fn abstain_const_fold(_args: &[&str]) -> Option<String> {
    None
}

fn abstain_const_fold_versioned(_args: &[&str], _version: Option<TclVersion>) -> Option<String> {
    None
}

/// Silence keeps the security finding alive.
fn sink_applies(_args: &[&str]) -> bool {
    true
}

fn allow_context(
    _args: tcl_registry::InvocationArguments<'_>,
    _in_event_body: bool,
) -> Option<&'static str> {
    None
}

fn accept_clause_shape(
    _args: tcl_registry::InvocationArguments<'_>,
) -> Option<tcl_registry::ClauseShapeIssue> {
    None
}

fn literals_valid(_args: InvocationArguments<'_>) -> LiteralArgumentValidation {
    LiteralArgumentValidation::Valid
}

/// The `constraints` placeholder a declared-but-unbound hook carries: no
/// report, which is exactly what a pack with no `constraints` hook answers.
fn no_constraint_reports(
    _facts: &tcl_registry::spec::OptionFacts<'_>,
) -> Vec<tcl_registry::spec::ConstraintReport> {
    Vec::new()
}

/// The `state_transitions` resolver placeholder a declared-but-unbound body
/// carries: no transitions, the family's silence.
fn abstain_state_transitions(_args: InvocationArguments<'_>) -> StateTransitions {
    StateTransitions::default()
}

/// Silence still consumes one word — `consume 0` is a report, not an
/// abstention.
fn consume_one_word(_args: &[&str], _start: usize) -> OptionValueOutcome {
    OptionValueOutcome {
        words: 1,
        invalid: None,
    }
}

// The loaded pack

/// One command a pack declares.
#[derive(Debug, Clone)]
pub struct PackCommand {
    /// The live spec, ready to install in a registry.
    pub spec: &'static CommandSpec,
    /// Whether the declaration claimed a shipped name with `-override`.
    pub overrides_shipped: bool,
    /// Every hook the command (or one of its subcommands / options) declares.
    pub hooks: Vec<HookDecl>,
    /// The `clause_grammar` the command declares, when it has one — the same
    /// descriptor its spec carries. The registry's walk
    /// ([`ClauseGrammarSpec::walk`]) derives both the argument roles and the
    /// clause-shape defect from it.
    pub clause_grammar: Option<&'static ClauseGrammarSpec>,
    /// Whether an assistance-class word this build does not speak was
    /// dropped from the spec (§6.1).
    ///
    /// The command is still known — its name completes, its hover shows —
    /// but the capability the dropped word configures must answer
    /// `Unknown` rather than confidently, because a shape or value set the
    /// author stated was not read.
    pub degraded: bool,
    /// The line the `command` statement was declared on.
    ///
    /// Carried so a collision notice can point at the declaration that lost
    /// rather than at the file's first line. The loader
    /// knows the line; the *file* is added later by the merge, which is the
    /// only layer that knows which file a command came from.
    pub line: u32,
    /// The source subject of the `command` row, issued by the loader.
    pub row: Option<tcl_lsp_core::code_actions::SpecPackNoticeSubject>,
    /// The file this command was declared in.
    ///
    /// Empty as the loader builds it — [`crate::loader`] parses one source at a
    /// time and is never told its path. [`crate::pack::load`] fills it in
    /// during the merge, so every command in a [`crate::MergedPack`] carries
    /// exact `(file, line)` attribution even when the pack spans many files.
    pub file: std::path::PathBuf,
    /// The content hash of the source the command was declared in: the xxh3
    /// of the pack file's bytes — the value its [`EvalSnapshotKey`] interns —
    /// folded with every fragment an `include` row brought in, so an edit
    /// to either moves it. The pack-fact stamp a specialised site records
    /// is built from it.
    pub content_hash: u64,
    /// How far the package that ships the declaring file sits from the
    /// workspace root ([`crate::discovery::PackFile::dependency_tier`]);
    /// `None` as the loader builds it, and for a command no package ships.
    /// The merge fills it in beside [`Self::file`], the two facts about the
    /// file only it knows, and the capability gate reads it
    /// ([`crate::stamps`]).
    pub dependency_tier: Option<tcl_dialect::model::DependencyTier>,
    /// The text a `tcl-body {-package-source PATH}` backing pointed at,
    /// read from the package that ships the declaring file when the load had
    /// a store to read it through ([`crate::package_sources`]); `None` as the
    /// loader builds it and for every other backing. A `PackText` body is the
    /// spec's own and is not copied here.
    pub reference_text: Option<std::sync::Arc<str>>,
}

/// A loaded `.tclspec` pack.
#[derive(Debug, Clone)]
pub struct Pack {
    /// The pack name from `speclib <pack-name> <dsl-version> { … }`.
    pub name: String,
    /// The **DSL vocabulary** version, not the library's.
    pub dsl_version: String,
    /// The pack's human-readable name (`display_name {IEEE 1801 UPF}`),
    /// for editor surfaces that show a library rather than a file.
    pub display_name: Option<String>,
    /// The file extensions this pack's language is written under
    /// (`file_extension upf -name {Unified Power Format} -dialect …`),
    /// in declaration order.
    pub file_extensions: Vec<FileExtension>,
    /// The package trains this pack declares it describes
    /// (`provides upf ?VERSION…?`, `SpecTcl` 2.0 §6.2), in declaration
    /// order. Commands with no availability default of their own default
    /// their provider (`required_package`) to the first `provides` name.
    pub provides: Vec<PackProvides>,
    /// `co_provides` relations (`SpecTcl` 2.0): predicated
    /// "requiring NAME routes to this pack's package" declarations,
    /// carried as data — the loader-alias mechanics that consume them are
    /// later wire-up, and carrying them is what keeps an older
    /// build from silently flattening a predicated relation.
    pub co_provides: Vec<CoProvides>,
    /// Packages this pack declares **ambient** in its dialect, with the
    /// version the runtime provides (`ambient_package Tk 8.6`), in
    /// declaration order.
    ///
    /// A package that comes *with* the dialect is never `package require`d in
    /// the documents that use it, so nothing else could give its commands a
    /// version floor — every per-release gate on them would go unchecked.
    /// This is the pack-authored twin of an ambient
    /// [`tcl_dialect::LibraryPin`], and the axis that lets a package be
    /// modelled as a pack without having to be compiled into `tcl-dialect`
    /// first.
    pub ambient_packages: Vec<AmbientPackage>,
    /// The interpreter-provided globals this pack declares with
    /// `special_var`, in declaration order — the pack-authored rows of the
    /// special-variable registry, installed through
    /// [`tcl_registry::CommandRegistry::insert_special_var`].
    pub special_vars: Vec<PackSpecialVar>,
    /// The `environment NAME { … }` blocks the pack declares (`SpecTcl`
    /// 2.0, §6.2), in declaration order, with the rejected ones dropped.
    pub environments: Vec<PackEnvironment>,
    /// The `dialect NAME { … }` blocks the pack declares (`SpecTcl` 2.0,
    /// §6.2 owner directive), in declaration order, with the rejected ones
    /// dropped.
    pub dialects: Vec<PackDialect>,
    /// The `include from SOURCE into TARGET { … }` rows the pack declares
    /// (`SpecTcl` 2.0 §6.2, design **Q6**), in declaration order, with the
    /// rejected ones dropped.
    ///
    /// Each row enumerates part of one reimplementing family's inherited
    /// command surface; several rows for the same pair accumulate, which
    /// is how a roster with per-release windows is written without
    /// repeating the pair. See [`surface_roster`].
    pub surface_rosters: Vec<PackSurfaceRoster>,
    /// The commands, in declaration order.
    pub commands: Vec<PackCommand>,
    /// Why the whole pack failed closed, when it did (§6.1: an unsupported
    /// `speclib` **major**; under evaluation also a broken transaction).
    /// `Some` always comes with an empty `commands` and one explaining
    /// notice — this is a load error wearing the never-panicking shape the
    /// rest of the loader has.
    pub load_error: Option<LoadError>,
    /// Whether the pack used `available?` during evaluation (design E-R1):
    /// its surface depends on the analysis target, so its snapshot must not
    /// be cached per (content, vocabulary) alone. `false` for every pack
    /// that does not ask — which is every pack shipped today.
    pub target_dependent: bool,
    /// Every registration call the load made, in the order it made them —
    /// the **canonical subset** of the snapshot (design E-R11).
    ///
    /// What the *program* registered — which for a straight-line declarative
    /// pack is the file's own statements, and for a templated one is the
    /// expansion. [`crate::export::export_pack`] renders it
    /// back as canonical source, so an export cannot lose a word the loader
    /// read — including the ones no `CommandSpec` field holds (a value
    /// table, an inline descriptor, a hook body).
    pub registrations: Vec<crate::export::Registration>,
    /// Everything dropped on the way in.
    pub notices: Vec<Notice>,
}

/// Why a pack loaded nothing at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The `speclib` word named a vocabulary major past this loader
    /// (§6.1). The string is the declared version, verbatim.
    UnsupportedMajor(String),
    /// Evaluation (design E, §1.2) raised an uncaught error — a Tcl error,
    /// a compile failure, or an engine crash. Registration is transactional,
    /// so nothing loaded.
    EvaluationFailed(String),
    /// Evaluation outran its budget on the named axis (§1.2): `command
    /// steps`, `wall clock`, or `value size`.
    BudgetExhausted(&'static str),
    /// Evaluation reached for a command the determinism contract denies
    /// (§1.2). The string is the denial, naming the command and its axis.
    Determinism(String),
    /// A registration touched something the pack's provenance tier may not
    /// (design E-R2). The string names the registration and the tier.
    Provenance(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMajor(declared) => write!(
                f,
                "SpecTcl vocabulary {declared} is a major this loader does not support"
            ),
            Self::EvaluationFailed(message) => {
                write!(f, "pack evaluation failed: {message}")
            }
            Self::BudgetExhausted(axis) => {
                write!(f, "pack evaluation exhausted its budget on the {axis} axis")
            }
            Self::Determinism(message) | Self::Provenance(message) => {
                write!(f, "{message}")
            }
        }
    }
}

/// One `provides NAME ?VERSION…?` row (`SpecTcl` 2.0 §6.2): a package
/// train this pack describes. Several versions name parallel majors; no
/// version at all declares the train without pinning any release, which
/// is what `tcl spec upgrade --infer-provides` writes when it hoists a
/// bare `required_package` default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackProvides {
    /// The package name, as `package require` would spell it.
    pub name: &'static str,
    /// The declared train versions, possibly empty.
    pub versions: Vec<&'static str>,
    /// The declaring line, for notices and editors.
    pub line: u32,
}

/// One `co_provides NAME ?-requires-exact PACKAGE? ?-when PREDICATE?`
/// row (`SpecTcl` 2.0): loading this pack's package
/// co-provides `NAME`; requiring `NAME` requires the named package at
/// the exact loaded version; all of it under an optional build
/// predicate. Data only — see [`Pack::co_provides`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoProvides {
    /// The co-provided name (`Tk` for the lowercase-`tk` loader).
    pub name: &'static str,
    /// The package whose exact loaded version a requirement of
    /// [`Self::name`] resolves through, when stated.
    pub requires_exact: Option<&'static str>,
    /// The build predicate the relation holds under, verbatim, when
    /// stated (`without TK_NO_DEPRECATED`).
    pub when: Option<&'static str>,
    /// The declaring line, for notices and editors.
    pub line: u32,
}

/// One `ambient_package NAME VERSION` row: a package the pack's dialect
/// provides without a `package require`, and the version it provides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbientPackage {
    /// The package name, as `package require` would spell it.
    pub name: &'static str,
    /// The version the runtime provides — a floor, not an exact release.
    pub version: &'static str,
    /// The declaring line, for notices and editors.
    pub line: u32,
}

/// One `special_var NAME -kind K -access A -origin O ?-dialects {…}?
/// ?-startup B?` row: a global the pack's dialect provides, as the
/// special-variable registry states one.
#[derive(Debug, Clone, Copy)]
pub struct PackSpecialVar {
    /// The row, built as a shipped [`tcl_registry::SpecialVarSpec`] is.
    pub spec: &'static tcl_registry::SpecialVarSpec,
    /// The declaring line, for notices and editors.
    pub line: u32,
}

/// One `file_extension` row: an extension the pack's language is written
/// under, with an optional human-readable name and an optional dialect the
/// server routes files of this extension to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileExtension {
    /// The extension, lower-case, without the leading dot (`upf`).
    pub extension: String,
    /// Human-readable name for editor pickers (`Unified Power Format`).
    pub display_name: Option<String>,
    /// The canonical dialect-profile name files of this extension detect
    /// as; validated against the profile catalogue at load.
    pub dialect: Option<&'static str>,
    /// The declaring line, for notices and editors.
    pub line: u32,
    /// The file this row was declared in.
    ///
    /// Exactly the arrangement [`PackCommand::file`] uses, and for the same
    /// reason: empty as the loader builds it, filled in by
    /// [`crate::pack::load`] during the merge. A logical pack can span several
    /// files, so the row's own path is the only thing that attributes a
    /// collision notice to the file the author must edit — attaching it to the
    /// merged pack's *first* file publishes a squiggle against a line that
    /// belongs to a different document.
    pub file: std::path::PathBuf,
}

impl Pack {
    /// The command of that name, if the pack declares one.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<&PackCommand> {
        self.commands.iter().find(|c| c.spec.name == name)
    }
}

/// A pack with nothing in it yet — where every load starts.
fn empty_pack() -> Pack {
    Pack {
        name: String::new(),
        display_name: None,
        file_extensions: Vec::new(),
        provides: Vec::new(),
        co_provides: Vec::new(),
        ambient_packages: Vec::new(),
        special_vars: Vec::new(),
        environments: Vec::new(),
        dialects: Vec::new(),
        surface_rosters: Vec::new(),
        dsl_version: String::new(),
        commands: Vec::new(),
        load_error: None,
        target_dependent: false,
        registrations: Vec::new(),
        notices: Vec::new(),
    }
}

/// The `SpecTcl` **vocabulary** versions this loader reads, newest last.
///
/// Minor releases add words without changing the meaning of earlier words.
/// The redesign's §6.1 compatibility contract therefore keeps one parser for
/// every word ever ratified: an older pack loads to the same surface, while a
/// pack declaring a newer minor loads maximally and reports words this build
/// does not know. Only an unsupported major fails closed (see
/// [`check_vocabulary_version`]).
pub const KNOWN_VOCABULARY_VERSIONS: &[&str] =
    &["1", "1.0", "1.1", "1.2", "2", "2.0", "2.1", "2.2", "2.3"];

/// The newest vocabulary this loader speaks, for the notice below.
pub const NEWEST_VOCABULARY_VERSION: &str = "2.3";

/// The newest `speclib` **major** this loader supports.
///
/// §6.1: an unsupported major fails the pack closed. An unknown *minor*
/// within a supported major keeps loading maximally, because a minor is only
/// ever additive — but a major says the meaning of words this loader thinks it
/// knows has changed, and reading them anyway is exactly the "stronger,
/// safer-looking result" the contract forbids.
const NEWEST_SUPPORTED_MAJOR: u32 = tcl_source::spectcl::NEWEST_SUPPORTED_MAJOR;

// `include` — pack-file inclusion under the determinism contract (2.0, Q6)

/// Resolves `include NAME` rows to source text.
///
/// The resolver is the whole IO surface: the loader itself never opens a
/// file, so "no IO beyond the pack search path" is enforced by handing the
/// loader a resolver that reaches nothing else — [`IncludeContext::for_file`]
/// reads only sibling files of the including pack, which is inside the
/// search path the discovery layer already walks.
pub struct IncludeContext {
    resolver: IncludeResolver,
    /// How many times an evaluation has asked the resolver. A load that never
    /// asked depends on its source alone, so its result can be memoised.
    reads: std::cell::Cell<u32>,
}

/// The resolver an [`IncludeContext`] wraps: include name in, source text
/// (or a one-line reason) out.
type IncludeResolver = Box<dyn Fn(&str) -> Result<String, String>>;

impl std::fmt::Debug for IncludeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncludeContext").finish_non_exhaustive()
    }
}

// Pack-level tables

/// The pack-wide availability / identity defaults a command inherits.
#[derive(Debug, Default, Clone)]
struct PackDefaults {
    surface: Option<&'static [SpecSurface]>,
    required_package: Option<&'static str>,
    /// The fallback provider `provides` declares (§6.2): applied only
    /// when no explicit `default required_package` (or `default
    /// available {package …}`) names one.
    provides_package: Option<&'static str>,
    tcllib_package: Option<&'static str>,
    introduced_version: Option<&'static str>,
    deprecated_version: Option<&'static str>,
    retired_version: Option<&'static str>,
    warn_missing_import: Option<bool>,
    is_namespace_exported: Option<bool>,
}

#[derive(Debug, Default)]
struct PackTables {
    values: BTreeMap<String, Vec<ArgValue>>,
    /// Descriptor blocks by `(property key, name)`, kept as their statements
    /// so each property's own reader interprets them.
    descriptors: BTreeMap<(String, String), Vec<Stmt>>,
    /// Shared hook bodies by name.
    hooks: BTreeMap<String, (Vec<String>, String)>,
    defaults: PackDefaults,
}

/// One `arg` row as authored.
///
/// A record per row rather than six parallel vectors, so a column added here
/// has one place to be projected from ([`ArgRows::seal`]) rather than six
/// accumulator sites to keep in step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct ArgRow {
    index: u8,
    role: Option<ArgRole>,
    type_hint: Option<ArgTypeHint>,
    values: &'static [ArgValue],
    closed: bool,
    presentation: Option<ArgPresentation>,
    appends: Option<tcl_registry::arg_role::AppendedArity>,
    domain: Option<tcl_registry::hover::IntegerDomain>,
}

/// The seven parallel per-argument slices a command or subcommand stores.
#[derive(Debug, Default)]
struct ArgSlices {
    roles: Vec<(u8, ArgRole)>,
    types: Vec<(u8, ArgTypeHint)>,
    values: Vec<(u8, &'static [ArgValue])>,
    closed: Vec<u8>,
    presentation: Vec<(u8, ArgPresentation)>,
    prefixes: Vec<(u8, tcl_registry::arg_role::AppendedArity)>,
    domains: Vec<(u8, tcl_registry::hover::IntegerDomain)>,
}

/// The `arg` rows a command or subcommand body declared, in source order.
#[derive(Debug, Default)]
struct ArgRows {
    rows: Vec<ArgRow>,
}

/// What separates the steps of a pack hook's identity (`-native
/// lsort::sub::const_fold`): owning command, then member, then field.
pub use tcl_registry::native_argument_role_hook::HOOK_PATH_SEPARATOR;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod authored_construction_hint_controls;
