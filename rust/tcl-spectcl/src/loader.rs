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
//! A spec pack is **one Tcl script, read from the CST and never executed**.
//! This module walks that tree — the same
//! [`build_document`](tcl_compiler::parsing::syntax::build::build_document) /
//! [`segments_from_document`](tcl_compiler::parsing::syntax::segment::segments_from_document)
//! pair every static scan in the toolchain uses — and turns the declarations
//! into live [`CommandSpec`]s, which the Spec Studio then seeds a draft from
//! through the ordinary `draft::from_command_spec`. Loading a pack and
//! browsing a shipped command therefore produce drafts by the *same* code,
//! which is what makes the per-port equivalence gate in
//! `tcl-spec-studio/tests/spectcl_ports.rs` meaningful.
//!
//! The frozen syntax is `docs/design/spec-dsl-examples/README.md`; the
//! architecture around it is `docs/design/spec-packs.md`.
//!
//! ## What the loader does, and does not, do
//!
//! - **Never evaluates.** Only the CST is read. Hook bodies are carried as
//!   text ([`HookDecl`]) together with their family and that family's declared
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

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_core_types::DiagCode;
use tcl_dialect::{DialectSet, TclVersion};
use tcl_lexer::{LexerConfig, SourceMap, TokenType};
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::arity::Arity;
use tcl_registry::body_kind::BodyKind;
use tcl_registry::byte_array_effect::ByteArrayEffect;
use tcl_registry::clause_shape::ClauseShapeError;
use tcl_registry::command_table::CommandTableEffect;
use tcl_registry::definer::{
    BuiltinMethodReceiver, BuiltinObjectMethod, DefinerFamily, DefinitionBodyGrammar,
    ManufacturerMethod, MemberBodyCommand, MemberKind, MemberRefKind, MemberRetraction, MemberSpec,
    MemberVisibility, SlotOp, SlotSpec,
};
use tcl_registry::deprecation::{DeprecationFixHook, DeprecationFixSafety};
use tcl_registry::events::EventRequires;
use tcl_registry::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevelWord};
use tcl_registry::handle_binding::{
    HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
use tcl_registry::hooks::{
    AnalyserHookId, ArgTypeHint, CodegenHookId, InlineCodegenHookId, LoweringHookId,
};
use tcl_registry::hover::{
    ArgValue, FormKind, FormSpec, HoverSnippet, IntegerDomain, OptionArg, OptionArity, OptionSpec,
    OptionValue, OptionValueOutcome,
};
use tcl_registry::intrinsic::IntrinsicId;
use tcl_registry::literal_validation::LiteralArgumentValidation;
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::patterns::{FormatType, PatternType};
use tcl_registry::presentation::ArgPresentation;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::side_effects::{ConnectionSide, SideEffect, SideEffectTarget, StorageType};
use tcl_registry::spec::{
    BytePayloadSpec, CaseListSpec, CommandSpec, DefaultFormFirstWord, SubCommand, SubSubCommand,
};
use tcl_registry::symbol_def::{DefinedSymbolKind, SymbolDef};
use tcl_registry::taint::SetterConstraint;
use tcl_registry::traits::Traits;
use tcl_registry::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use tcl_registry::world_effect::WorldEffectDescriptor;
use tcl_registry::{CommandPrefixArguments, InvocationArguments};

use crate::catalogue;

// ---------------------------------------------------------------------------
// Notices
// ---------------------------------------------------------------------------

/// One thing the loader dropped, with enough context to fix the pack.
///
/// Every notice is a *degradation*, never a failure: the declaration it names
/// is discarded and the rest of the pack loads. That is what makes "new server
/// + old pack" and "old server + new pack" both work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Where in the pack the notice arose (`"command lsort"`,
    /// `"command string / subcommand is"`, `"pack"`).
    pub context: String,
    /// 1-based source line of the offending statement.
    pub line: u32,
    /// What was dropped, and why.
    pub message: String,
}

impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.context, self.line, self.message)
    }
}

/// Accumulates notices under a movable context label.
#[derive(Debug, Default)]
struct Log {
    context: String,
    notices: Vec<Notice>,
}

impl Log {
    fn say(&mut self, line: u32, message: impl Into<String>) {
        self.notices.push(Notice {
            context: self.context.clone(),
            line,
            message: message.into(),
        });
    }

    /// Run `body` with `context` pushed, restoring the previous label after.
    fn scoped<T>(&mut self, context: String, body: impl FnOnce(&mut Self) -> T) -> T {
        let previous = std::mem::replace(&mut self.context, context);
        let out = body(self);
        self.context = previous;
        out
    }

    fn unknown_property(&mut self, stmt: &Stmt) {
        let word = stmt.word_text(0);
        self.say(stmt.line, format!("unknown property `{word}` dropped"));
    }

    fn unknown_flag(&mut self, row: &str, line: u32, flag: &str) {
        self.say(
            line,
            format!("unknown flag `{}` on `{row}` dropped", quotable(flag)),
        );
    }
}

/// The longest a word quoted inside a notice may be before it is elided.
const QUOTABLE_WIDTH: usize = 40;

/// A word as a notice can quote it: one physical line, bounded length.
///
/// An unknown flag is *usually* a short word, but a flag the loader does not
/// know consumes no value, so the braced word that followed it is read as a
/// flag in its own right on the next turn — and that word can be a twenty-line
/// block (`apave.tclspec`'s invented `-repeats { row { … } }`). Quoting it
/// verbatim would put newlines inside a notice message, which the notice's
/// consumers cannot carry: `tests/spec_corpus_baseline.txt` is one
/// tab-separated record per line, and an editor's problems pane is one line
/// per diagnostic.
fn quotable(word: &str) -> String {
    // The first line of a braced word is usually empty — the brace is followed
    // by a newline — so the first line with content is what identifies it.
    let head = word
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let elided = head.len() < word.trim().len();
    if head.chars().count() <= QUOTABLE_WIDTH {
        return if elided {
            format!("{head}…")
        } else {
            head.to_owned()
        };
    }
    let cut: String = head.chars().take(QUOTABLE_WIDTH).collect();
    format!("{cut}…")
}

// ---------------------------------------------------------------------------
// The statement reader — CST in, words out
// ---------------------------------------------------------------------------

/// One word of a statement, with the text the CST resolved for it.
///
/// For a braced word `text` is **every byte between the braces**, verbatim:
/// the loader never reflows, dedents, or joins. That is what lets a ported
/// `description` compare byte for byte against the `&'static str` it came
/// from.
#[derive(Debug, Clone)]
pub(crate) struct Word {
    pub(crate) text: String,
    /// Whether the word was written `{…}` — the one thing that distinguishes
    /// an inline descriptor block from a descriptor named by a bare word.
    pub(crate) braced: bool,
    pub(crate) line: u32,
}

/// One `word word…` declaration.
#[derive(Debug, Clone)]
pub(crate) struct Stmt {
    pub(crate) words: Vec<Word>,
    pub(crate) line: u32,
}

impl Stmt {
    fn word_text(&self, i: usize) -> &str {
        self.words.get(i).map_or("", |w| w.text.as_str())
    }

    fn arg(&self, i: usize) -> Option<&Word> {
        self.words.get(i)
    }

    /// The words after the statement's own name word.
    fn tail(&self) -> &[Word] {
        self.words.get(1..).unwrap_or(&[])
    }
}

/// Segment `source` into statements, numbering lines from `base_line`.
///
/// Comments, `;` separators, and line continuations are handled by the lexer,
/// so the loader inherits exactly the Tcl an author already knows — including
/// the trap that `#` only starts a comment where a command word would start.
///
/// Segmentation is a **pure function of `(source, base_line)`**, which is what
/// lets [`crate::cache`] memoise it: every level of a pack — the file, the
/// `speclib` body, each `command` body, each descriptor block — reaches the
/// CST through this one door, so memoising here captures the whole tree
/// without the loader's readers knowing a cache exists. The memo is a no-op
/// unless a caller installed one.
fn statements(source: &str, base_line: u32) -> Vec<Stmt> {
    if let Some(hit) = crate::cache::memo_get(source, base_line) {
        return hit;
    }
    let parsed = segment(source, base_line);
    crate::cache::memo_put(source, base_line, &parsed);
    parsed
}

/// The uncached segmentation — [`statements`] minus the memo.
fn segment(source: &str, base_line: u32) -> Vec<Stmt> {
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(source, LexerConfig::default());
    let mut line_starts: Vec<usize> = vec![0];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(offset + 1);
        }
    }
    let line_of = |offset: usize| -> u32 {
        let index = match line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        base_line + u32::try_from(index).unwrap_or(u32::MAX)
    };

    segments_from_document(document, &source_map)
        .into_iter()
        .map(|segment| {
            let words: Vec<Word> = segment
                .texts
                .iter()
                .zip(segment.argv.iter())
                .map(|(text, token)| Word {
                    text: text.clone(),
                    braced: token.kind == TokenType::Str && !token.in_quote,
                    line: line_of(token.span.start() as usize),
                })
                .collect();
            let line = words.first().map_or(base_line, |w| w.line);
            Stmt { words, line }
        })
        .collect()
}

/// The statements inside a braced block word.
fn block(word: &Word) -> Vec<Stmt> {
    statements(&word.text, word.line)
}

/// Segment a whole pack source — the loader's entry into [`statements`], and
/// the one [`crate::cache`] keys its on-disk entry from.
pub(crate) fn pack_statements(source: &str) -> Vec<Stmt> {
    statements(source, 1)
}

// ---------------------------------------------------------------------------
// Leaking — a loaded pack lives as long as the process, like a shipped spec
// ---------------------------------------------------------------------------

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
fn leak_str(text: &str) -> &'static str {
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

// ---------------------------------------------------------------------------
// Hook bodies — carried as text, never run
// ---------------------------------------------------------------------------

/// Which of the nine hook families a body belongs to.
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

/// The whitelist a hook body is evaluated against once the sandbox lands.
///
/// Recorded here so the loader can already report a body reaching for
/// something it will never be given. Deliberately absent: `open`, `exec`,
/// `source`, `socket`, `after`, `interp`, `uplevel`, `upvar`, `trace`,
/// `namespace`, `proc`, `rename`, `info`, `subst`.
pub const SANDBOX_COMMANDS: &[&str] = &[
    "set", "expr", "if", "while", "for", "foreach", "switch", "return", "break", "continue",
    "incr", "lappend", "lassign", "list", "lindex", "llength", "lrange", "lreplace", "lsearch",
    "lsort", "join", "split", "string", "format", "scan", "regexp", "regsub", "dict", "binary",
];

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
        /// correct, and uncacheable — `docs/design/spec-packs.md`'s
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

/// One declared hook: what it is attached to, which field, which family, and
/// the text or name that supplies it.
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

fn allow_context(_args: &[&str], _in_event_body: bool) -> Option<&'static str> {
    None
}

fn accept_clause_shape(_args: &[&str]) -> Option<ClauseShapeError> {
    None
}

fn literals_valid(_args: InvocationArguments<'_>) -> LiteralArgumentValidation {
    LiteralArgumentValidation::Valid
}

/// Silence still consumes one word — `consume 0` is a report, not an
/// abstention.
fn consume_one_word(_args: &[&str], _start: usize) -> OptionValueOutcome {
    OptionValueOutcome {
        words: 1,
        invalid: None,
    }
}

// ---------------------------------------------------------------------------
// The loaded pack
// ---------------------------------------------------------------------------

/// One command a pack declares.
#[derive(Debug, Clone)]
pub struct PackCommand {
    /// The live spec, ready to install in a registry.
    pub spec: &'static CommandSpec,
    /// Whether the declaration claimed a shipped name with `-override`.
    pub overrides_shipped: bool,
    /// Every hook the command (or one of its subcommands / options) declares.
    pub hooks: Vec<HookDecl>,
    /// The `clause_grammar` the command declares, when it has one. Both hook
    /// behaviours are derived from it by [`ClauseGrammar::walk`].
    pub clause_grammar: Option<ClauseGrammar>,
}

/// A loaded `.tclspec` pack.
#[derive(Debug, Clone)]
pub struct Pack {
    /// The pack name from `speclib <pack-name> <dsl-version> { … }`.
    pub name: String,
    /// The **DSL vocabulary** version, not the library's.
    pub dsl_version: String,
    /// The commands, in declaration order.
    pub commands: Vec<PackCommand>,
    /// Everything dropped on the way in.
    pub notices: Vec<Notice>,
}

impl Pack {
    /// The command of that name, if the pack declares one.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<&PackCommand> {
        self.commands.iter().find(|c| c.spec.name == name)
    }
}

/// Load a `.tclspec` source.
///
/// Never fails: a pack with no `speclib` wrapper, or with declarations this
/// server does not know, loads as much as it can and reports the rest through
/// [`Pack::notices`].
#[must_use]
pub fn load_pack(source: &str) -> Pack {
    let mut log = Log {
        context: "pack".to_owned(),
        notices: Vec::new(),
    };
    let mut pack = Pack {
        name: String::new(),
        dsl_version: String::new(),
        commands: Vec::new(),
        notices: Vec::new(),
    };

    let top = pack_statements(source);
    let Some(speclib) = top.iter().find(|s| s.word_text(0) == "speclib") else {
        log.say(1, "no `speclib` declaration; nothing loaded");
        pack.notices = log.notices;
        return pack;
    };
    for stray in top.iter().filter(|s| s.word_text(0) != "speclib") {
        log.unknown_property(stray);
    }
    speclib.word_text(1).clone_into(&mut pack.name);
    speclib.word_text(2).clone_into(&mut pack.dsl_version);
    let Some(body) = speclib.arg(3) else {
        log.say(speclib.line, "`speclib` has no body block");
        pack.notices = log.notices;
        return pack;
    };

    let mut tables = PackTables::default();
    let mut declarations: Vec<Stmt> = Vec::new();
    // Two passes: pack-level tables and descriptors first, so a `command`
    // may reference one declared after it.
    for stmt in block(body) {
        match stmt.word_text(0) {
            "values" => tables.add_values(&stmt, &mut log),
            "descriptor" => tables.add_descriptor(&stmt, &mut log),
            "hook" => tables.add_hook(&stmt, &mut log),
            "default" => tables.add_default(&stmt, &mut log),
            "command" => declarations.push(stmt),
            _ => log.unknown_property(&stmt),
        }
    }

    for stmt in declarations {
        if let Some(command) = load_command(&stmt, &tables, &mut log) {
            pack.commands.push(command);
        }
    }

    pack.notices = log.notices;
    pack
}

// ---------------------------------------------------------------------------
// Pack-level tables
// ---------------------------------------------------------------------------

/// The pack-wide availability / identity defaults a command inherits.
#[derive(Debug, Default, Clone)]
struct PackDefaults {
    dialects: Option<DialectSet>,
    required_package: Option<&'static str>,
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

impl PackTables {
    fn add_values(&mut self, stmt: &Stmt, log: &mut Log) {
        let name = stmt.word_text(1).to_owned();
        let Some(body) = stmt.arg(2) else {
            log.say(stmt.line, "`values` needs a name and a block");
            return;
        };
        if self.values.contains_key(&name) {
            log.say(stmt.line, format!("`values {name}` redeclared; last wins"));
        }
        self.values.insert(name, value_rows(&block(body), log));
    }

    fn add_descriptor(&mut self, stmt: &Stmt, log: &mut Log) {
        let key = stmt.word_text(1).to_owned();
        let name = stmt.word_text(2).to_owned();
        let Some(body) = stmt.arg(3) else {
            log.say(stmt.line, "`descriptor` needs a key, a name, and a block");
            return;
        };
        self.descriptors.insert((key, name), block(body));
    }

    fn add_hook(&mut self, stmt: &Stmt, log: &mut Log) {
        let name = stmt.word_text(1).to_owned();
        let (Some(params), Some(body)) = (stmt.arg(2), stmt.arg(3)) else {
            log.say(
                stmt.line,
                "`hook` needs a name, a parameter list, and a body",
            );
            return;
        };
        self.hooks
            .insert(name, (list_words(&params.text), body.text.clone()));
    }

    fn add_default(&mut self, stmt: &Stmt, log: &mut Log) {
        let key = stmt.word_text(1);
        let value = stmt.word_text(2);
        match key {
            "dialects" => self.defaults.dialects = parse_dialects(value, stmt.line, log),
            "required_package" => self.defaults.required_package = Some(leak_str(value)),
            "tcllib_package" => self.defaults.tcllib_package = Some(leak_str(value)),
            "introduced_version" => self.defaults.introduced_version = Some(leak_str(value)),
            "deprecated_version" => self.defaults.deprecated_version = Some(leak_str(value)),
            "retired_version" => self.defaults.retired_version = Some(leak_str(value)),
            "warn_missing_import" => {
                self.defaults.warn_missing_import = Some(parse_flag(stmt.tail()));
            }
            "is_namespace_exported" => {
                self.defaults.is_namespace_exported = Some(parse_flag(stmt.tail()));
            }
            other => log.say(
                stmt.line,
                format!("`default {other}` is not an availability key; dropped"),
            ),
        }
    }

    fn descriptor(&self, key: &str, name: &str) -> Option<&[Stmt]> {
        self.descriptors
            .get(&(key.to_owned(), name.to_owned()))
            .map(Vec::as_slice)
    }
}

/// `value V ?-detail {…}? ?-min-tcl VER? ?-code N?` rows.
fn value_rows(stmts: &[Stmt], log: &mut Log) -> Vec<ArgValue> {
    let mut out = Vec::new();
    for stmt in stmts {
        if stmt.word_text(0) != "value" {
            log.unknown_property(stmt);
            continue;
        }
        let mut row = ArgValue {
            value: leak_str(stmt.word_text(1)),
            detail: "",
            min_tcl: None,
            code: None,
        };
        let mut i = 2;
        let words = &stmt.words;
        while i < words.len() {
            match words[i].text.as_str() {
                "-detail" => {
                    row.detail = leak_str(&next_text(words, &mut i));
                }
                "-min-tcl" => {
                    let raw = next_text(words, &mut i);
                    match parse_version(&raw) {
                        Some(version) => row.min_tcl = Some(version),
                        None => log.say(stmt.line, format!("unknown Tcl version `{raw}` dropped")),
                    }
                }
                "-code" => {
                    let raw = next_text(words, &mut i);
                    match raw.parse::<i64>() {
                        Ok(code) => row.code = Some(code),
                        Err(_) => log.say(stmt.line, format!("`-code {raw}` is not an integer")),
                    }
                }
                other => log.unknown_flag("value", stmt.line, other),
            }
            i += 1;
        }
        out.push(row);
    }
    out
}

// ---------------------------------------------------------------------------
// Small word / value parsers
// ---------------------------------------------------------------------------

/// Advance past a flag and return its value word's text.
fn next_text(words: &[Word], i: &mut usize) -> String {
    *i += 1;
    words.get(*i).map(|w| w.text.clone()).unwrap_or_default()
}

/// The Tcl list elements of a word, braces stripped, tolerating a malformed
/// list rather than refusing the whole declaration.
fn list_words(text: &str) -> Vec<String> {
    tcl_syntax::list::split_list_lenient(text)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// A bare property word means `yes`; an explicit `yes` / `no` overrides it.
fn parse_flag(tail: &[Word]) -> bool {
    !matches!(
        tail.first().map(|w| w.text.as_str()),
        Some("no" | "false" | "0")
    )
}

/// The tri-state form, where the argument is required.
fn parse_tristate(text: &str) -> Option<bool> {
    match text {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_version(name: &str) -> Option<TclVersion> {
    match name {
        "tcl8.4" | "8.4" => Some(TclVersion::V8_4),
        "tcl8.5" | "8.5" => Some(TclVersion::V8_5),
        "tcl8.6" | "8.6" => Some(TclVersion::V8_6),
        "tcl9.0" | "9.0" => Some(TclVersion::V9_0),
        "tcl9.1" | "9.1" => Some(TclVersion::V9_1),
        _ => None,
    }
}

/// The `tclX.Y+` "and later" closure.
fn version_and_later(base: TclVersion) -> DialectSet {
    const LADDER: &[(TclVersion, DialectSet)] = &[
        (TclVersion::V8_4, DialectSet::TCL84),
        (TclVersion::V8_5, DialectSet::TCL85),
        (TclVersion::V8_6, DialectSet::TCL86),
        (TclVersion::V9_0, DialectSet::TCL90),
        (TclVersion::V9_1, DialectSet::TCL91),
    ];
    LADDER
        .iter()
        .filter(|(version, _)| *version >= base)
        .fold(DialectSet::empty(), |set, (_, bit)| set | *bit)
}

/// A dialect set word: members verbatim, plus `tclX.Y+`, `all-tcl`, `tcl8.x`.
fn parse_dialects(text: &str, line: u32, log: &mut Log) -> Option<DialectSet> {
    let mut set = DialectSet::empty();
    let mut saw_any = false;
    for member in list_words(text) {
        let bit = match member.as_str() {
            "all-tcl" => Some(DialectSet::ALL_TCL),
            "tcl8.x" => Some(DialectSet::TCL8X),
            other => match other.strip_suffix('+') {
                Some(base) => parse_version(base).map(version_and_later),
                None => catalogue::dialect_bit(other),
            },
        };
        match bit {
            Some(bit) => {
                set |= bit;
                saw_any = true;
            }
            None => log.say(line, format!("unknown dialect `{member}` dropped")),
        }
    }
    saw_any.then_some(set)
}

fn parse_traits(text: &str, line: u32, log: &mut Log) -> Traits {
    let mut traits = Traits::empty();
    for name in list_words(text) {
        match catalogue::trait_bit(&name) {
            Some(bit) => traits |= bit,
            None => log.say(line, format!("unknown trait `{name}` dropped")),
        }
    }
    traits
}

fn parse_taint(text: &str, line: u32, log: &mut Log) -> tcl_registry::taint::TaintColour {
    let mut colour = tcl_registry::taint::TaintColour::empty();
    for name in list_words(text) {
        match catalogue::taint_bit(&name) {
            Some(bit) => colour |= bit,
            None => log.say(line, format!("unknown taint colour `{name}` dropped")),
        }
    }
    colour
}

/// Resolve a fieldless enum value by its Rust variant spelling.
fn by_name<T: Copy + fmt::Debug>(all: &[T], name: &str) -> Option<T> {
    all.iter()
        .copied()
        .find(|value| catalogue::variant_name(value) == name)
}

/// [`by_name`], reporting an unknown spelling rather than silently dropping.
fn enum_by_name<T: Copy + fmt::Debug>(
    all: &[T],
    name: &str,
    what: &str,
    line: u32,
    log: &mut Log,
) -> Option<T> {
    let found = by_name(all, name);
    if found.is_none() {
        log.say(line, format!("unknown {what} `{name}` dropped"));
    }
    found
}

const TCL_TYPES: &[TclType] = &[
    TclType::String,
    TclType::Int,
    TclType::Double,
    TclType::Boolean,
    TclType::List,
    TclType::Dict,
    TclType::ByteArray,
    TclType::Numeric,
    TclType::Object,
    TclType::Channel,
];

const SIDE_EFFECT_TARGETS: &[SideEffectTarget] = &[
    SideEffectTarget::Variable,
    SideEffectTarget::SessionTable,
    SideEffectTarget::PersistenceTable,
    SideEffectTarget::DataGroup,
    SideEffectTarget::HttpHeader,
    SideEffectTarget::HttpBody,
    SideEffectTarget::HttpStatus,
    SideEffectTarget::HttpUri,
    SideEffectTarget::HttpCookie,
    SideEffectTarget::HttpMethod,
    SideEffectTarget::Http2State,
    SideEffectTarget::ResponseCommit,
    SideEffectTarget::ConnectionControl,
    SideEffectTarget::TcpState,
    SideEffectTarget::SslState,
    SideEffectTarget::UdpState,
    SideEffectTarget::PoolSelection,
    SideEffectTarget::NodeSelection,
    SideEffectTarget::SnatSelection,
    SideEffectTarget::FileIo,
    SideEffectTarget::NetworkIo,
    SideEffectTarget::LogIo,
    SideEffectTarget::StreamProfile,
    SideEffectTarget::DnsState,
    SideEffectTarget::ClassificationState,
    SideEffectTarget::Dosl7State,
    SideEffectTarget::FlowState,
    SideEffectTarget::LsnState,
    SideEffectTarget::FtpState,
    SideEffectTarget::IcapState,
    SideEffectTarget::MessageState,
    SideEffectTarget::IStats,
    SideEffectTarget::ApmState,
    SideEffectTarget::AsmState,
    SideEffectTarget::BigipConfig,
    SideEffectTarget::ProcDefinition,
    SideEffectTarget::NamespaceState,
    SideEffectTarget::InterpState,
    SideEffectTarget::Process,
    SideEffectTarget::ChannelIo,
    SideEffectTarget::EventControl,
    SideEffectTarget::Unknown,
];

const CONNECTION_SIDES: &[ConnectionSide] = &[
    ConnectionSide::None,
    ConnectionSide::Client,
    ConnectionSide::Server,
    ConnectionSide::Both,
    ConnectionSide::Global,
];

const FORM_KINDS: &[FormKind] = &[FormKind::Default, FormKind::Getter, FormKind::Setter];

const BODY_KINDS: &[BodyKind] = &[BodyKind::Plain, BodyKind::Structural];

const BYTE_ARRAY_EFFECTS: &[ByteArrayEffect] = &[
    ByteArrayEffect::None,
    ByteArrayEffect::Transparent,
    ByteArrayEffect::Coerces,
    ByteArrayEffect::CaseFolds,
    ByteArrayEffect::Encodes,
];

const STORAGE_TYPES: &[StorageType] = &[StorageType::Dict, StorageType::List, StorageType::Array];

const FORMAT_TYPES: &[FormatType] = &[
    FormatType::Sprintf,
    FormatType::Clock,
    FormatType::Binary,
    FormatType::Regsub,
];

/// One variant today, and a table anyway: the DSL word is the variant name,
/// so a second shape arrives as a table entry rather than a new parser.
const DEFAULT_FORM_FIRST_WORDS: &[DefaultFormFirstWord] = &[DefaultFormFirstWord::Integer];

const PRESENTATIONS: &[ArgPresentation] =
    &[ArgPresentation::BlockScript, ArgPresentation::InlineScript];

const FRAME_LEVEL_WORDS: &[FrameLevelWord] = &[
    FrameLevelWord::None,
    FrameLevelWord::ArityParity,
    FrameLevelWord::LeadingProbe,
];

const FRAME_LAYOUTS: &[FrameArgLayout] = &[
    FrameArgLayout::AliasPairs,
    FrameArgLayout::ScriptInSelectedFrame,
    FrameArgLayout::ScriptInCurrentFrame,
    FrameArgLayout::OpaqueCallerVars,
];

const LOWERING_HOOKS: &[LoweringHookId] = &[
    LoweringHookId::Expr,
    LoweringHookId::Return,
    LoweringHookId::Set,
    LoweringHookId::Incr,
    LoweringHookId::AppendOrLappend,
    LoweringHookId::Unset,
    LoweringHookId::Global,
    LoweringHookId::Variable,
    LoweringHookId::Upvar,
    LoweringHookId::Proc,
    LoweringHookId::When,
    LoweringHookId::NamespaceEval,
    LoweringHookId::If,
    LoweringHookId::Switch,
    LoweringHookId::For,
    LoweringHookId::While,
    LoweringHookId::Foreach,
    LoweringHookId::Lmap,
    LoweringHookId::ForeachLine,
    LoweringHookId::Catch,
    LoweringHookId::Try,
    LoweringHookId::Dict,
    LoweringHookId::Eval,
    LoweringHookId::Uplevel,
    LoweringHookId::Apply,
    LoweringHookId::ArrayFor,
];

const CODEGEN_HOOKS: &[CodegenHookId] = &[
    CodegenHookId::Lassign,
    CodegenHookId::Llength,
    CodegenHookId::Lrange,
    CodegenHookId::Linsert,
    CodegenHookId::Lset,
    CodegenHookId::Dict,
    CodegenHookId::Array,
    CodegenHookId::Namespace,
    CodegenHookId::Append,
    CodegenHookId::Lappend,
    CodegenHookId::Unset,
    CodegenHookId::Tailcall,
    CodegenHookId::Concat,
    CodegenHookId::Global,
    CodegenHookId::Upvar,
];

const INLINE_CODEGEN_HOOKS: &[InlineCodegenHookId] = &[
    InlineCodegenHookId::Expr,
    InlineCodegenHookId::Incr,
    InlineCodegenHookId::InfoExists,
    InlineCodegenHookId::String,
    InlineCodegenHookId::Lindex,
    InlineCodegenHookId::Lrange,
    InlineCodegenHookId::Lreplace,
    InlineCodegenHookId::Linsert,
    InlineCodegenHookId::Regexp,
    InlineCodegenHookId::List,
    InlineCodegenHookId::Array,
    InlineCodegenHookId::DictGet,
    InlineCodegenHookId::Catch,
    InlineCodegenHookId::Return,
    InlineCodegenHookId::Error,
    InlineCodegenHookId::Break,
    InlineCodegenHookId::Continue,
    InlineCodegenHookId::Try,
];

const ANALYSER_HOOKS: &[AnalyserHookId] = &[
    AnalyserHookId::Set,
    AnalyserHookId::Variable,
    AnalyserHookId::Global,
    AnalyserHookId::Proc,
    AnalyserHookId::OptProc,
    AnalyserHookId::Apply,
    AnalyserHookId::Uplevel,
    AnalyserHookId::NamespaceEval,
    AnalyserHookId::NamespaceEnsemble,
    AnalyserHookId::NamespaceImport,
    AnalyserHookId::NamespaceExport,
    AnalyserHookId::NamespaceForget,
    AnalyserHookId::NamespacePath,
    AnalyserHookId::NamespaceUnknown,
    AnalyserHookId::NamespaceUpvar,
    AnalyserHookId::Foreach,
    AnalyserHookId::For,
    AnalyserHookId::Switch,
    AnalyserHookId::Catch,
    AnalyserHookId::Try,
    AnalyserHookId::Upvar,
    AnalyserHookId::DictFor,
    AnalyserHookId::DictUpdate,
    AnalyserHookId::DictWith,
    AnalyserHookId::InterpAlias,
    AnalyserHookId::InterpEval,
    AnalyserHookId::InterpCreate,
    AnalyserHookId::InterpDelete,
    AnalyserHookId::InterpHide,
    AnalyserHookId::InterpExpose,
    AnalyserHookId::Rename,
    AnalyserHookId::OoDefine,
    AnalyserHookId::OoObjdefine,
    AnalyserHookId::PackageRequire,
    AnalyserHookId::PackageProvide,
    AnalyserHookId::PackageIfneeded,
    AnalyserHookId::PackagePrefer,
    AnalyserHookId::Source,
    AnalyserHookId::Append,
    AnalyserHookId::Lappend,
    AnalyserHookId::RegexPatternCapture,
    AnalyserHookId::Incr,
    AnalyserHookId::Load,
];

const INTRINSICS: &[IntrinsicId] = &[
    IntrinsicId::ListAssign,
    IntrinsicId::ListLength,
    IntrinsicId::ListIndex,
    IntrinsicId::ListRange,
    IntrinsicId::ListReplace,
    IntrinsicId::ListInsert,
    IntrinsicId::ListSet,
    IntrinsicId::ListConstruct,
    IntrinsicId::DictGet,
    IntrinsicId::DictSet,
    IntrinsicId::DictUnset,
    IntrinsicId::DictIncr,
    IntrinsicId::DictAppend,
    IntrinsicId::DictListAppend,
    IntrinsicId::StringIndex,
    IntrinsicId::StringRange,
    IntrinsicId::StringEqual,
    IntrinsicId::StringCompare,
    IntrinsicId::StringReplace,
    IntrinsicId::StringLength,
    IntrinsicId::StringIs,
    IntrinsicId::Regexp,
    IntrinsicId::InfoExists,
    IntrinsicId::ArrayExists,
    IntrinsicId::ArrayNames,
    IntrinsicId::ArraySize,
    IntrinsicId::Concat,
    IntrinsicId::ChannelWrite,
];

/// `arity 3.. -step 2 -also 2` and its five simpler spellings.
fn parse_arity(stmt: &Stmt, log: &mut Log) -> Arity {
    let range = stmt.word_text(1);
    let mut arity = match range.split_once("..") {
        None => range.parse::<u16>().map_or_else(
            |_| {
                log.say(
                    stmt.line,
                    format!("unreadable arity `{range}`; treated as any"),
                );
                Arity::any()
            },
            Arity::exact,
        ),
        Some((low, high)) => {
            let min = if low.is_empty() {
                0
            } else {
                low.parse().unwrap_or(0)
            };
            let max = if high.is_empty() {
                Arity::UNLIMITED
            } else {
                high.parse().unwrap_or(Arity::UNLIMITED)
            };
            Arity::new(min, max)
        }
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-step" => arity.step = next_text(words, &mut i).parse().unwrap_or(0),
            "-also" => arity.also_exact = next_text(words, &mut i).parse().ok(),
            other => log.unknown_flag("arity", stmt.line, other),
        }
        i += 1;
    }
    arity
}

/// `{Range 2 max}`, `Any`, `Port` — `max` / `min` are the integer sentinels.
fn parse_integer_domain(text: &str, line: u32, log: &mut Log) -> Option<IntegerDomain> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "Range" && rest.len() == 2 => {
            let bound = |word: &str, sentinel: i64| -> i64 {
                match word {
                    "max" => i64::MAX,
                    "min" => i64::MIN,
                    other => other.parse().unwrap_or(sentinel),
                }
            };
            Some(IntegerDomain::Range(
                bound(&rest[0], i64::MIN),
                bound(&rest[1], i64::MAX),
            ))
        }
        Some((head, rest)) if head == "Any" && rest.is_empty() => Some(IntegerDomain::Any),
        Some((head, rest)) if head == "Port" && rest.is_empty() => Some(IntegerDomain::Port),
        _ => {
            log.say(line, format!("unreadable integer domain `{text}` dropped"));
            None
        }
    }
}

/// `{Exactly N}` / `{AtLeast N}` / `Unknown`.
fn parse_appended_arity(text: &str, line: u32, log: &mut Log) -> Option<AppendedArity> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "Exactly" && rest.len() == 1 => {
            rest[0].parse().ok().map(AppendedArity::Exactly)
        }
        Some((head, rest)) if head == "AtLeast" && rest.len() == 1 => {
            rest[0].parse().ok().map(AppendedArity::AtLeast)
        }
        Some((head, rest)) if head == "Unknown" && rest.is_empty() => Some(AppendedArity::Unknown),
        _ => {
            log.say(line, format!("unreadable appended arity `{text}` dropped"));
            None
        }
    }
}

/// `ReturnValue` / `Destructured` / `{Fixed T}` / `{ElementsOf N}`.
fn parse_var_write_typing(text: &str, line: u32, log: &mut Log) -> Option<VarWriteTyping> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "ReturnValue" && rest.is_empty() => {
            Some(VarWriteTyping::ReturnValue)
        }
        Some((head, rest)) if head == "Destructured" && rest.is_empty() => {
            Some(VarWriteTyping::Destructured)
        }
        Some((head, rest)) if head == "Fixed" && rest.len() == 1 => {
            by_name(TCL_TYPES, &rest[0]).map(VarWriteTyping::Fixed)
        }
        Some((head, rest)) if head == "ElementsOf" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|container_arg| VarWriteTyping::ElementsOf { container_arg }),
        _ => {
            log.say(
                line,
                format!("unreadable var_write_typing `{text}` dropped"),
            );
            None
        }
    }
}

/// `{ListOfArgs N}` / `{DictOfPairs N}` / `{ElementOf N}` / `{SubListOf N}`.
///
/// The `{VARIANT payload …}` rule `README.md` gives for `var_write_typing`,
/// applied to the three element-structure facts that follow it in the coverage
/// matrix: a variant word, then its fields in declaration order.
fn parse_return_elements(text: &str, line: u32, log: &mut Log) -> Option<ReturnElements> {
    let parts = list_words(text);
    let index = |rest: &[String]| rest[0].parse::<u8>().ok();
    match parts.split_first() {
        Some((head, rest)) if head == "ListOfArgs" && rest.len() == 1 => {
            index(rest).map(|from| ReturnElements::ListOfArgs { from })
        }
        Some((head, rest)) if head == "DictOfPairs" && rest.len() == 1 => {
            index(rest).map(|from| ReturnElements::DictOfPairs { from })
        }
        Some((head, rest)) if head == "ElementOf" && rest.len() == 1 => {
            index(rest).map(|container_arg| ReturnElements::ElementOf { container_arg })
        }
        Some((head, rest)) if head == "SubListOf" && rest.len() == 1 => {
            index(rest).map(|container_arg| ReturnElements::SubListOf { container_arg })
        }
        _ => None,
    }
    .or_else(|| {
        log.say(line, format!("unreadable return_elements `{text}` dropped"));
        None
    })
}

/// `{AppendsListElements N}` / `SetsDictValue` /
/// `{ExtendsDictValuesByName N}` / `ListifiesDictValue`.
fn parse_var_elements_effect(text: &str, line: u32, log: &mut Log) -> Option<VarElementsEffect> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "AppendsListElements" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|values_from| VarElementsEffect::AppendsListElements { values_from }),
        Some((head, rest)) if head == "SetsDictValue" && rest.is_empty() => {
            Some(VarElementsEffect::SetsDictValue)
        }
        Some((head, rest)) if head == "ExtendsDictValuesByName" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|values_from| VarElementsEffect::ExtendsDictValuesByName { values_from }),
        Some((head, rest)) if head == "ListifiesDictValue" && rest.is_empty() => {
            Some(VarElementsEffect::ListifiesDictValue)
        }
        _ => None,
    }
    .or_else(|| {
        log.say(
            line,
            format!("unreadable var_elements_effect `{text}` dropped"),
        );
        None
    })
}

/// `None` / `{CopyOnWriteContainerMutation VAR MIN}`.
fn parse_representation_effect(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<RepresentationEffect> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "None" && rest.is_empty() => Some(RepresentationEffect::None),
        Some((head, rest)) if head == "CopyOnWriteContainerMutation" && rest.len() == 2 => {
            match (rest[0].parse(), rest[1].parse()) {
                (Ok(variable_arg), Ok(minimum_arguments)) => {
                    Some(RepresentationEffect::CopyOnWriteContainerMutation {
                        variable_arg,
                        minimum_arguments,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
    .or_else(|| {
        log.say(
            line,
            format!("unreadable representation_effect `{text}` dropped"),
        );
        None
    })
}

/// The five fieldless byte-array effects by name, plus the one that carries a
/// payload: `{Rebinarifies N}`.
///
/// Split out from the plain [`enum_by_name`] table because `Rebinarifies` is
/// not one word — it names the value operand whose byte-array representation
/// the operation installs in place, which is the whole content of the S110
/// documented fix.
fn parse_byte_array_effect(text: &str, line: u32, log: &mut Log) -> Option<ByteArrayEffect> {
    let parts = list_words(text);
    if let Some((head, rest)) = parts.split_first()
        && head == "Rebinarifies"
        && rest.len() == 1
    {
        let Ok(value_arg) = rest[0].parse() else {
            log.say(
                line,
                format!("unreadable byte_array_effect `{text}` dropped"),
            );
            return None;
        };
        return Some(ByteArrayEffect::Rebinarifies { value_arg });
    }
    enum_by_name(BYTE_ARRAY_EFFECTS, text, "byte array effect", line, log)
}

/// `deprecation_fix -replace WORD -description {…} -safety S`.
///
/// Only the declarative variants: `Custom { resolver }` names a registry
/// callback, which `README.md` marks reference-only, and the two positional
/// variants take the index they replace as `-replace-arg N`.
fn parse_deprecation_fix(stmt: &Stmt, log: &mut Log) -> Option<DeprecationFixHook> {
    let mut replacement: Option<&'static str> = None;
    let mut replace_arg: Option<u8> = None;
    let mut whole_invocation = false;
    let mut description: &'static str = "";
    let mut safety = DeprecationFixSafety::RequiresReview;

    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-replace" => {
                replacement = Some(leak_str(&next_text(words, &mut i)));
            }
            "-replace-arg" => {
                replace_arg = next_text(words, &mut i).parse().ok();
            }
            "-replace-invocation" => whole_invocation = true,
            "-description" => description = leak_str(&next_text(words, &mut i)),
            "-safety" => {
                const SAFETY: &[DeprecationFixSafety] = &[
                    DeprecationFixSafety::SemanticsEquivalent,
                    DeprecationFixSafety::RequiresReview,
                ];
                if let Some(chosen) = enum_by_name(
                    SAFETY,
                    &next_text(words, &mut i),
                    "deprecation fix safety",
                    stmt.line,
                    log,
                ) {
                    safety = chosen;
                }
            }
            other => log.unknown_flag("deprecation_fix", stmt.line, other),
        }
        i += 1;
    }
    let Some(replacement) = replacement else {
        log.say(stmt.line, "`deprecation_fix` needs `-replace WORD`");
        return None;
    };
    Some(match (replace_arg, whole_invocation) {
        (Some(index), _) => DeprecationFixHook::ReplaceArgument {
            index,
            replacement,
            description,
            safety,
        },
        (None, true) => DeprecationFixHook::ReplaceInvocation {
            replacement,
            description,
            safety,
        },
        (None, false) => DeprecationFixHook::ReplaceMatchedWord {
            replacement,
            description,
            safety,
        },
    })
}

/// `setter_constraint N -prefix P -code CODE -message {…}`.
fn parse_setter_constraint(stmt: &Stmt, log: &mut Log) -> Option<SetterConstraint> {
    let Ok(arg_index) = stmt.word_text(1).parse::<u8>() else {
        log.say(
            stmt.line,
            format!(
                "`setter_constraint` needs an argument index, got `{}`",
                stmt.word_text(1)
            ),
        );
        return None;
    };
    let mut required_prefix: &'static str = "";
    let mut code: Option<DiagCode> = None;
    let mut message: &'static str = "";

    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-prefix" => required_prefix = leak_str(&next_text(words, &mut i)),
            "-code" => {
                let text = next_text(words, &mut i);
                match text.parse() {
                    Ok(parsed) => code = Some(parsed),
                    Err(_) => log.say(stmt.line, format!("unknown diagnostic code `{text}`")),
                }
            }
            "-message" => message = leak_str(&next_text(words, &mut i)),
            other => log.unknown_flag("setter_constraint", stmt.line, other),
        }
        i += 1;
    }
    let Some(code) = code else {
        log.say(stmt.line, "`setter_constraint` needs `-code CODE`");
        return None;
    };
    Some(SetterConstraint {
        arg_index,
        required_prefix,
        code,
        message,
    })
}

/// `byte_array_payload -replace-data-index N ?-message-flag-shift?`.
fn parse_byte_array_payload(stmt: &Stmt, log: &mut Log) -> Option<BytePayloadSpec> {
    let mut replace_data_index: Option<u8> = None;
    let mut message_flag_shift = false;

    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-replace-data-index" => replace_data_index = next_text(words, &mut i).parse().ok(),
            "-message-flag-shift" => message_flag_shift = true,
            other => log.unknown_flag("byte_array_payload", stmt.line, other),
        }
        i += 1;
    }
    let Some(replace_data_index) = replace_data_index else {
        log.say(
            stmt.line,
            "`byte_array_payload` needs `-replace-data-index N`",
        );
        return None;
    };
    Some(BytePayloadSpec {
        replace_data_index,
        message_flag_shift,
    })
}

/// `defines_symbol -name-arg N ?-detail-arg N? ?-requires-arg N? -kind KIND`.
fn parse_defines_symbol(stmt: &Stmt, log: &mut Log) -> Option<SymbolDef> {
    const KINDS: &[DefinedSymbolKind] = &[
        DefinedSymbolKind::Test,
        DefinedSymbolKind::Constraint,
        DefinedSymbolKind::Matcher,
        DefinedSymbolKind::Event,
    ];
    let mut name_arg: Option<u8> = None;
    let mut detail_arg: Option<u8> = None;
    let mut requires_arg: Option<u8> = None;
    let mut kind: Option<DefinedSymbolKind> = None;

    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-name-arg" => name_arg = next_text(words, &mut i).parse().ok(),
            "-detail-arg" => detail_arg = next_text(words, &mut i).parse().ok(),
            "-requires-arg" => requires_arg = next_text(words, &mut i).parse().ok(),
            "-kind" => {
                kind = enum_by_name(
                    KINDS,
                    &next_text(words, &mut i),
                    "defined symbol kind",
                    stmt.line,
                    log,
                );
            }
            other => log.unknown_flag("defines_symbol", stmt.line, other),
        }
        i += 1;
    }
    let (Some(name_arg), Some(kind)) = (name_arg, kind) else {
        log.say(
            stmt.line,
            "`defines_symbol` needs `-name-arg N` and `-kind KIND`",
        );
        return None;
    };
    Some(SymbolDef {
        name_arg,
        detail_arg,
        requires_arg,
        kind,
    })
}

/// `Invoke` / `{Intrinsic ID}` / `{StructuredLowering ID}`.
fn parse_semantic_operation(text: &str, line: u32, log: &mut Log) -> Option<SemanticOperationId> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "Invoke" && rest.is_empty() => {
            Some(SemanticOperationId::Invoke)
        }
        Some((head, rest)) if head == "Intrinsic" && rest.len() == 1 => {
            by_name(INTRINSICS, &rest[0]).map(SemanticOperationId::Intrinsic)
        }
        Some((head, rest)) if head == "StructuredLowering" && rest.len() == 1 => {
            by_name(LOWERING_HOOKS, &rest[0]).map(SemanticOperationId::StructuredLowering)
        }
        _ => {
            log.say(
                line,
                format!("unreadable semantic_operation `{text}` dropped"),
            );
            None
        }
    }
}

/// `-binds-handle {-name-from {Word N} -class-from {Word N} ?-keyword {N W}?}`.
fn parse_handle_binding(text: &str, line: u32, log: &mut Log) -> Option<HandleBindingSpec> {
    let parts = list_words(text);
    let mut name_from = None;
    let mut class_from = None;
    let mut keyword = None;
    let mut i = 0;
    while i < parts.len() {
        let flag = parts[i].clone();
        let value = parts.get(i + 1).cloned().unwrap_or_default();
        match flag.as_str() {
            "-name-from" => {
                let inner = list_words(&value);
                name_from = match inner.split_first() {
                    Some((head, rest)) if head == "Word" && rest.len() == 1 => {
                        rest[0].parse().ok().map(HandleName::Word)
                    }
                    Some((head, rest)) if head == "Implicit" && rest.len() == 1 => {
                        Some(HandleName::Implicit(leak_str(&rest[0])))
                    }
                    _ => None,
                };
            }
            "-class-from" => {
                let inner = list_words(&value);
                class_from = match inner.split_first() {
                    Some((head, rest)) if head == "Word" && rest.len() == 1 => {
                        rest[0].parse().ok().map(HandleClassSource::Word)
                    }
                    Some((head, rest)) if head == "ConstructionValue" && rest.len() == 1 => rest[0]
                        .parse()
                        .ok()
                        .map(HandleClassSource::ConstructionValue),
                    _ => None,
                };
            }
            "-keyword" => {
                let inner = list_words(&value);
                if inner.len() == 2 {
                    keyword = inner[0].parse().ok().map(|at| HandleKeyword {
                        at,
                        word: leak_str(&inner[1]),
                    });
                }
            }
            other => log.unknown_flag("binds_handle", line, other),
        }
        i += 2;
    }
    let (Some(name_from), Some(class_from)) = (name_from, class_from) else {
        log.say(line, "binds_handle needs -name-from and -class-from");
        return None;
    };
    Some(HandleBindingSpec {
        name_from,
        class_from,
        keyword,
    })
}

/// `object_class NAME ?-superclass {…}? ?-allow-unknown? ?{ method … }?`.
///
/// The ratified spelling (`docs/design/spec-dsl-examples/README.md`, "Rulings
/// on the census drafts"): `ObjectClassSpec` has four fields, three of which
/// are one word each, so they ride on the statement rather than in the block.
/// The `NAME` word is [`ObjectClassSpec::class_name`] and is **not** always the
/// command name — a factory command may manufacture a differently-named class.
/// The optional trailing block holds `method NAME { … }` rows, which reuse the
/// `subcommand` body grammar unchanged because
/// [`ObjectClassSpec::instance_methods`] really is `&[SubCommand]`.
fn object_class_row(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<tcl_registry::spec::ObjectClassSpec> {
    let class_name = stmt.word_text(1).to_owned();
    if class_name.is_empty() {
        log.say(stmt.line, "`object_class` with no class name dropped");
        return None;
    }
    let mut superclasses: Vec<String> = Vec::new();
    let mut allow_unknown_methods = false;
    let mut body: Option<&Word> = None;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-superclass" => superclasses = list_words(&next_text(words, &mut i)),
            "-allow-unknown" => allow_unknown_methods = true,
            // The one unflagged word left is the member block, and it is
            // always last.
            _ if words[i].braced && i + 1 == words.len() => body = Some(&words[i]),
            other => log.unknown_flag("object_class", stmt.line, other),
        }
        i += 1;
    }

    let outer = log.context.clone();
    let methods = log.scoped(format!("{outer} / object_class {class_name}"), |log| {
        let mut methods: Vec<SubCommand> = Vec::new();
        let Some(body) = body else {
            return methods;
        };
        for stmt in block(body) {
            if stmt.word_text(0) != "method" {
                log.unknown_property(&stmt);
                continue;
            }
            // A member row's own hooks have no owner the pack-hook binder can
            // name today — `HookOwner::Subcommand` resolves against
            // `spec.subcommands`, which an instance method is not in — so the
            // declarations are collected here and reported rather than bound
            // to the wrong table.
            let mut hooks: Vec<HookDecl> = Vec::new();
            let Some(method) = load_subcommand(&stmt, tables, &mut hooks, "method", log) else {
                continue;
            };
            for hook in &hooks {
                if matches!(hook.source, HookSource::Body { .. }) {
                    log.say(
                        stmt.line,
                        format!(
                            "`{}` hook body on `method {}` is not yet bindable; the field \
                             abstains",
                            hook.field, method.name
                        ),
                    );
                }
            }
            methods.push(method);
        }
        methods
    });

    Some(tcl_registry::spec::ObjectClassSpec {
        class_name: leak_str(&class_name),
        instance_methods: leak_slice(methods),
        superclasses: leak_strs(&superclasses),
        allow_unknown_methods,
    })
}

// ---------------------------------------------------------------------------
// Per-argument rows — six schema keys, one statement
// ---------------------------------------------------------------------------

/// The `u8` argument tables cap the index; an index above 255 is dropped with
/// a notice rather than wrapping.
const MAX_ARG_INDEX: usize = 255;

#[derive(Debug, Default)]
struct ArgRows {
    roles: Vec<(u8, ArgRole)>,
    types: Vec<(u8, ArgTypeHint)>,
    values: Vec<(u8, &'static [ArgValue])>,
    closed: Vec<u8>,
    presentation: Vec<(u8, ArgPresentation)>,
    prefixes: Vec<(u8, AppendedArity)>,
}

impl ArgRows {
    // One flag per `arg` row column, and there are eleven of them; splitting
    // the match would put each column further from the others it constrains.
    #[allow(clippy::too_many_lines)]
    fn apply(&mut self, stmt: &Stmt, tables: &PackTables, log: &mut Log) {
        let raw = stmt.word_text(1);
        let Ok(index) = raw.parse::<usize>() else {
            log.say(
                stmt.line,
                format!("unreadable argument index `{raw}` dropped"),
            );
            return;
        };
        if index > MAX_ARG_INDEX {
            log.say(
                stmt.line,
                format!(
                    "argument index {index} is above the {MAX_ARG_INDEX} the tables hold; dropped"
                ),
            );
            return;
        }
        let index = u8::try_from(index).unwrap_or(u8::MAX);

        let mut hint = ArgTypeHint {
            expected: None,
            shimmers: false,
            transparent_from: &[],
        };
        let mut saw_type_flag = false;
        let words = &stmt.words;
        let mut i = 2;
        while i < words.len() {
            match words[i].text.as_str() {
                "-role" => {
                    let name = next_text(words, &mut i);
                    if let Some(role) =
                        enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log)
                    {
                        self.roles.push((index, role));
                    }
                }
                "-type" => {
                    let name = next_text(words, &mut i);
                    hint.expected = enum_by_name(TCL_TYPES, &name, "argument type", stmt.line, log);
                    saw_type_flag = true;
                }
                "-shimmers" => {
                    hint.shimmers = true;
                    saw_type_flag = true;
                }
                "-transparent" => {
                    let text = next_text(words, &mut i);
                    let types: Vec<TclType> = list_words(&text)
                        .iter()
                        .filter_map(|name| {
                            enum_by_name(TCL_TYPES, name, "argument type", stmt.line, log)
                        })
                        .collect();
                    hint.transparent_from = leak_slice(types);
                    saw_type_flag = true;
                }
                "-values" => {
                    let text = next_text(words, &mut i);
                    let values: Vec<ArgValue> = list_words(&text)
                        .iter()
                        .map(|value| ArgValue {
                            value: leak_str(value),
                            detail: "",
                            min_tcl: None,
                            code: None,
                        })
                        .collect();
                    self.values.push((index, leak_slice(values)));
                }
                "-values-from" => {
                    let name = next_text(words, &mut i);
                    match tables.values.get(&name) {
                        Some(values) => self.values.push((index, leak_slice(values.clone()))),
                        None => log.say(
                            stmt.line,
                            format!("no `values {name}` table in this pack; row dropped"),
                        ),
                    }
                }
                "-closed" => self.closed.push(index),
                "-layout" => {
                    let name = next_text(words, &mut i);
                    if let Some(layout) =
                        enum_by_name(PRESENTATIONS, &name, "argument layout", stmt.line, log)
                    {
                        self.presentation.push((index, layout));
                    }
                }
                "-appends" => {
                    let text = next_text(words, &mut i);
                    if let Some(appended) = parse_appended_arity(&text, stmt.line, log) {
                        self.prefixes.push((index, appended));
                        // `-appends` implies the position is a command prefix.
                        if !self.roles.iter().any(|(at, _)| *at == index) {
                            self.roles.push((index, ArgRole::CommandPrefix));
                        }
                    }
                }
                other => log.unknown_flag("arg", stmt.line, other),
            }
            i += 1;
        }
        if saw_type_flag {
            self.types.push((index, hint));
        }
    }
}

// ---------------------------------------------------------------------------
// Option rows
// ---------------------------------------------------------------------------

/// Parse one `option NAME …` row, returning the spec and any `-arity-hook`.
#[allow(clippy::too_many_lines)]
fn option_row(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> (OptionSpec, Option<(HookSource, String)>) {
    let name = leak_str(stmt.word_text(1));
    let mut option = OptionSpec {
        name,
        ..OptionSpec::DEFAULT
    };
    let mut arg: Option<OptionArg> = None;
    let mut hook: Option<(HookSource, String)> = None;

    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-takes" => {
                let hint = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                entry.hint = leak_str(&hint);
            }
            "-detail" => option.detail = leak_str(&next_text(words, &mut i)),
            "-aliases" => {
                let text = next_text(words, &mut i);
                option.aliases = leak_strs(&list_words(&text));
            }
            "-dialects" => {
                let text = next_text(words, &mut i);
                option.dialects = parse_dialects(&text, stmt.line, log);
            }
            "-min-abbrev" => option.min_abbrev = next_text(words, &mut i).parse().ok(),
            "-introduced" => {
                option.lifecycle.introduced = Some(leak_str(&next_text(words, &mut i)));
            }
            "-deprecated" => {
                option.lifecycle.deprecated = Some(leak_str(&next_text(words, &mut i)));
            }
            "-retired" => option.lifecycle.retired = Some(leak_str(&next_text(words, &mut i))),
            "-arity" => {
                let text = next_text(words, &mut i);
                let parts = list_words(&text);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                match parts.split_first() {
                    Some((head, rest)) if head == "Fixed" && rest.len() == 1 => {
                        match rest[0].parse() {
                            Ok(n) => entry.arity = OptionArity::Fixed(n),
                            Err(_) => log.say(stmt.line, format!("unreadable `-arity {text}`")),
                        }
                    }
                    Some((head, rest)) if head == "One" && rest.is_empty() => {
                        entry.arity = OptionArity::One;
                    }
                    _ => log.say(
                        stmt.line,
                        format!("`-arity {text}` is not a static option arity; use -arity-hook"),
                    ),
                }
            }
            "-arity-hook" => {
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                entry.arity = OptionArity::Hook(consume_one_word);
                let first = next_text(words, &mut i);
                if first == "-native" {
                    let id = next_text(words, &mut i);
                    hook = Some((HookSource::Native { id }, option.name.to_owned()));
                } else {
                    let body = next_text(words, &mut i);
                    hook = Some((
                        HookSource::Body {
                            params: list_words(&first),
                            body,
                            // An option row's `-arity-hook` has no `-inputs`
                            // flag of its own: the row's own flag parser owns
                            // this word position, and the family reads the
                            // option keys, which are not part of the shape
                            // key — so it is uncacheable by construction.
                            inputs: HookInputs::unrestricted(),
                        },
                        option.name.to_owned(),
                    ));
                }
            }
            "-role" => {
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(role) =
                    enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log)
                {
                    entry.role = role;
                }
            }
            "-also-role" => {
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                entry.also_role =
                    enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log);
            }
            "-body-kind" => {
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(kind) = enum_by_name(BODY_KINDS, &name, "body kind", stmt.line, log) {
                    entry.body_kind = kind;
                }
            }
            "-values" => {
                let text = next_text(words, &mut i);
                let values: Vec<ArgValue> = list_words(&text)
                    .iter()
                    .map(|value| ArgValue {
                        value: leak_str(value),
                        detail: "",
                        min_tcl: None,
                        code: None,
                    })
                    .collect();
                arg.get_or_insert(OptionArg::DEFAULT).values = leak_slice(values);
            }
            "-values-from" => {
                let name = next_text(words, &mut i);
                match tables.values.get(&name) {
                    Some(values) => {
                        arg.get_or_insert(OptionArg::DEFAULT).values = leak_slice(values.clone());
                    }
                    None => log.say(
                        stmt.line,
                        format!("no `values {name}` table in this pack; -values-from dropped"),
                    ),
                }
            }
            "-closed" => arg.get_or_insert(OptionArg::DEFAULT).closed = true,
            "-integer" => {
                let text = next_text(words, &mut i);
                arg.get_or_insert(OptionArg::DEFAULT).integer =
                    parse_integer_domain(&text, stmt.line, log);
            }
            "-appends" => {
                let text = next_text(words, &mut i);
                if let Some(appended) = parse_appended_arity(&text, stmt.line, log) {
                    arg.get_or_insert(OptionArg::DEFAULT).appended_arity = appended;
                }
            }
            other => log.unknown_flag("option", stmt.line, other),
        }
        i += 1;
    }

    if let Some(arg) = arg {
        option.value = OptionValue::Takes(arg);
    }
    (option, hook)
}

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------

fn hover_block(stmts: &[Stmt], log: &mut Log) -> HoverSnippet {
    let mut summary = String::new();
    let mut synopsis: Vec<String> = Vec::new();
    let mut snippet = String::new();
    let mut source = String::new();
    let mut examples: Vec<String> = Vec::new();
    let mut return_value = String::new();
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "summary" => summary = value,
            "synopsis" => synopsis.push(value),
            // Three words are renamed from their Rust field names; nothing
            // else in the DSL renames a key.
            "description" => snippet = value,
            "source" => source = value,
            "example" => examples.push(value),
            "returns" => return_value = value,
            _ => log.unknown_property(stmt),
        }
    }
    HoverSnippet {
        summary: leak_str(&summary),
        synopsis: leak_strs(&synopsis),
        snippet: leak_str(&snippet),
        source: leak_str(&source),
        // Repeated `example` rows join with a single newline.
        examples: leak_str(&examples.join("\n")),
        return_value: leak_str(&return_value),
    }
}

// ---------------------------------------------------------------------------
// Block-valued descriptors
// ---------------------------------------------------------------------------

fn event_requires_block(stmts: &[Stmt], log: &mut Log) -> EventRequires {
    let mut requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &[],
        also_in: &[],
        init_only: false,
        flow: false,
        capability: None,
    };
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "client_side" => requires.client_side = parse_flag(stmt.tail()),
            "server_side" => requires.server_side = parse_flag(stmt.tail()),
            "init_only" => requires.init_only = parse_flag(stmt.tail()),
            "flow" => requires.flow = parse_flag(stmt.tail()),
            "transport" => requires.transport = Some(leak_str(&value)),
            "capability" => requires.capability = Some(leak_str(&value)),
            "profiles" => requires.profiles = leak_strs(&list_words(&value)),
            "also_in" => requires.also_in = leak_strs(&list_words(&value)),
            _ => log.unknown_property(stmt),
        }
    }
    requires
}

fn case_list_block(stmts: &[Stmt], log: &mut Log) -> CaseListSpec {
    let mut spec = CaseListSpec {
        subject_args: 0,
        regex_option: None,
        exact_option: None,
        glob_option: None,
        nocase_option: None,
        end_options_option: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        keyword_patterns: &[],
        keyword_patterns_require_final: false,
    };
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "subject_args" => spec.subject_args = value.parse().unwrap_or(0),
            "exact_option" => spec.exact_option = Some(leak_str(&value)),
            "glob_option" => spec.glob_option = Some(leak_str(&value)),
            "regex_option" => spec.regex_option = Some(leak_str(&value)),
            "nocase_option" => spec.nocase_option = Some(leak_str(&value)),
            "end_options_option" => spec.end_options_option = Some(leak_str(&value)),
            "fallthrough_body" => spec.fallthrough_body = Some(leak_str(&value)),
            "value_options_require_regex" => {
                spec.value_options_require_regex = leak_strs(&list_words(&value));
            }
            "clause_flags" => spec.clause_flags = leak_strs(&list_words(&value)),
            "clause_regex_flag" => spec.clause_regex_flag = Some(leak_str(&value)),
            "clause_value_flags" => spec.clause_value_flags = leak_strs(&list_words(&value)),
            "keyword_patterns" => {
                spec.keyword_patterns = leak_strs(&list_words(&value));
                spec.keyword_patterns_require_final =
                    stmt.words.iter().any(|w| w.text == "-final-only");
            }
            _ => log.unknown_property(stmt),
        }
    }
    spec
}

/// The clause grammar, kept as declared so a later walk can derive both hook
/// behaviours from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClauseGrammar {
    /// The mandatory leading clause's slots, matched positionally.
    pub head: Vec<String>,
    /// Zero-or-more clauses, each introduced by its literal keyword.
    pub repeated: Vec<(String, Vec<String>)>,
    /// At most one trailing clause; the keyword is optional when written
    /// `?else?`.
    pub tail: Option<(Option<String>, bool, Vec<String>)>,
}

/// The outcome of walking a call against a [`ClauseGrammar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseWalk {
    /// The roles the walk assigns, 0-based after the command name.
    pub roles: Vec<(u8, ArgRole)>,
    /// The first structural defect, or `None` for any shape the grammar
    /// accepts.
    pub error: Option<ClauseShapeError>,
}

impl ClauseGrammar {
    /// Walk `args` against the grammar, deriving **both** hook behaviours at
    /// once — the roles `arg_role_resolver` would assign and the defect
    /// `clause_shape_check` would report.
    ///
    /// **Normative — where keywords match.** A keyword is compared only at a
    /// clause boundary and at a `?noise?` position; every other slot is filled
    /// positionally and consumes whatever word is there, *including one
    /// spelled like a keyword*. That is what makes `if else {a}` a well-formed
    /// `if` whose condition is the bareword `else`, and `if 1 a elseif else b`
    /// a well-formed chain whose second condition is the bareword `else`. The
    /// one-line version: at each step the walk asks "does a clause start
    /// here?", and only that question ever compares a word against a keyword.
    #[must_use]
    pub fn walk(&self, args: &[&str]) -> ClauseWalk {
        let mut roles = Vec::new();
        let n = args.len();
        let mut i = 0usize;

        if let Err(error) = Self::fill(&self.head, args, &mut i, &mut roles) {
            return ClauseWalk { roles, error };
        }

        loop {
            if i >= n {
                return ClauseWalk { roles, error: None };
            }
            if let Some((_, slots)) = self.repeated.iter().find(|(keyword, _)| args[i] == keyword) {
                push_role(&mut roles, i, ArgRole::Keyword);
                i += 1;
                if let Err(error) = Self::fill(slots, args, &mut i, &mut roles) {
                    return ClauseWalk { roles, error };
                }
                continue;
            }
            let Some((keyword, optional, slots)) = &self.tail else {
                return ClauseWalk {
                    roles,
                    error: Some(ClauseShapeError::ExtraWords { first_extra: i }),
                };
            };
            match keyword {
                Some(keyword) if args[i] == *keyword => {
                    push_role(&mut roles, i, ArgRole::Keyword);
                    i += 1;
                }
                // A tail whose keyword is mandatory does not match here, so
                // nothing more is a clause and everything left is extra.
                Some(_) if !optional => {
                    return ClauseWalk {
                        roles,
                        error: Some(ClauseShapeError::ExtraWords { first_extra: i }),
                    };
                }
                // `?else?` — the optional introducing keyword that makes a
                // bare trailing body legal with no keyword at all.
                _ => {}
            }
            if let Err(error) = Self::fill(slots, args, &mut i, &mut roles) {
                return ClauseWalk { roles, error };
            }
            // `tail` is last, which is what makes anything after it an error.
            let error = (i < n).then_some(ClauseShapeError::ExtraWords { first_extra: i });
            return ClauseWalk { roles, error };
        }
    }

    /// Fill one clause's slots positionally from `args[*i..]`.
    fn fill(
        slots: &[String],
        args: &[&str],
        i: &mut usize,
        roles: &mut Vec<(u8, ArgRole)>,
    ) -> Result<(), Option<ClauseShapeError>> {
        for slot in slots {
            if let Some(noise) = slot
                .strip_prefix('?')
                .and_then(|rest| rest.strip_suffix('?'))
            {
                if args.get(*i).is_some_and(|word| *word == noise) {
                    push_role(roles, *i, ArgRole::Keyword);
                    *i += 1;
                }
                continue;
            }
            let role = by_name(ArgRole::ALL, slot).unwrap_or(ArgRole::Value);
            if *i >= args.len() {
                let after = i.checked_sub(1);
                return Err(Some(if role == ArgRole::Expr {
                    ClauseShapeError::MissingExpr { after }
                } else {
                    // `after` is the index of the last present word; a body
                    // slot always has one, because a clause is never entered
                    // with nothing before it.
                    ClauseShapeError::MissingBody {
                        after: after.unwrap_or(0),
                    }
                }));
            }
            push_role(roles, *i, role);
            *i += 1;
        }
        Ok(())
    }
}

/// Record a role, dropping an index the `u8` tables cannot hold.
fn push_role(roles: &mut Vec<(u8, ArgRole)>, index: usize, role: ArgRole) {
    if let Ok(index) = u8::try_from(index) {
        roles.push((index, role));
    }
}

/// The normative `arg_role_resolver from-manufacturers` rule.
///
/// Look `args[0]` up in **this spec's own `manufacturer` rows**; if it names
/// one and that row has a `-definition-body-at N`, emit `role N Body`,
/// provided `N` is a valid index into the call's arguments; otherwise emit
/// nothing. Three details are load-bearing and none is negotiable:
///
/// - **`args[0]` only** — the manufacturer keyword is the first argument
///   word, never searched for elsewhere in the call.
/// - **`Body` only** — `-names-instance-at` and `-constructor-args-from` are
///   read by other consumers and contribute no role here.
/// - **Bounds-checked** — `oo::class create` with no body word emits nothing
///   rather than a role pointing past the end.
#[must_use]
pub fn roles_from_manufacturers(spec: &CommandSpec, args: &[&str]) -> Vec<(u8, ArgRole)> {
    let Some(body) = args
        .first()
        .and_then(|word| {
            spec.manufacturer_methods
                .iter()
                .find(|method| method.keyword == *word)
        })
        .and_then(|method| method.definition_body_at)
    else {
        return Vec::new();
    };
    if usize::from(body) < args.len() {
        vec![(body, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

fn clause_grammar_block(stmts: &[Stmt], log: &mut Log) -> ClauseGrammar {
    let mut grammar = ClauseGrammar::default();
    for stmt in stmts {
        match stmt.word_text(0) {
            "head" => grammar.head = list_words(stmt.word_text(1)),
            "repeated" => grammar
                .repeated
                .push((stmt.word_text(1).to_owned(), list_words(stmt.word_text(2)))),
            "tail" => {
                let (keyword, slots) = if stmt.words.len() >= 3 {
                    (
                        Some(stmt.word_text(1).to_owned()),
                        list_words(stmt.word_text(2)),
                    )
                } else {
                    (None, list_words(stmt.word_text(1)))
                };
                let optional = keyword
                    .as_deref()
                    .is_some_and(|k| k.starts_with('?') && k.ends_with('?') && k.len() > 1);
                let keyword = keyword.map(|k| k.trim_matches('?').to_owned());
                grammar.tail = Some((keyword, optional, slots));
            }
            _ => log.unknown_property(stmt),
        }
    }
    grammar
}

/// `definition_body { … }` — the inline definer grammar.
fn definition_body_block(stmts: &[Stmt], log: &mut Log) -> DefinitionBodyGrammar {
    let mut grammar = DefinitionBodyGrammar {
        family: DefinerFamily::TclOo,
        members: &[],
        implicit_vars: &[],
        member_body_namespace_path: &[],
        builtin_type_methods: &[],
        builtin_object_methods: &[],
        builtin_terminating_methods: &[],
        member_body_commands: &[],
        bare_word_construction: false,
        bare_word_construction_hint: None,
        dynamic_method_dispatch: false,
        manufacturers: &[],
        unknown_dispatch_method: None,
        property_accessor_methods: &[],
    };
    let mut members: Vec<MemberSpec> = Vec::new();
    let mut object_methods: Vec<BuiltinObjectMethod> = Vec::new();
    let mut body_commands: Vec<MemberBodyCommand> = Vec::new();
    let mut manufacturers: Vec<ManufacturerMethod> = Vec::new();

    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "family" => {
                const FAMILIES: &[DefinerFamily] = &[
                    DefinerFamily::TclOo,
                    DefinerFamily::Snit,
                    DefinerFamily::Itcl,
                ];
                if let Some(family) =
                    enum_by_name(FAMILIES, &value, "definer family", stmt.line, log)
                {
                    grammar.family = family;
                }
            }
            "member" => members.push(member_row(stmt, log)),
            "member_option" => log.say(
                stmt.line,
                "`member_option` is not yet loadable; row dropped",
            ),
            "implicit_vars" => grammar.implicit_vars = leak_strs(&list_words(&value)),
            "member_body_namespace_path" => {
                grammar.member_body_namespace_path = leak_strs(&list_words(&value));
            }
            "builtin_type_methods" => {
                grammar.builtin_type_methods = leak_strs(&list_words(&value));
            }
            "builtin_object_method" => object_methods.push(builtin_object_method_row(stmt, log)),
            "builtin_terminating_methods" => {
                grammar.builtin_terminating_methods = leak_strs(&list_words(&value));
            }
            "member_body_command" => body_commands.push(member_body_command_row(stmt, log)),
            "bare_word_construction" => {
                grammar.bare_word_construction = true;
                // The one function pointer in the grammar is, read as data, an
                // exact-word set plus a prefix set. A family whose hint is not
                // that shape keeps `-native`.
                grammar.bare_word_construction_hint = Some(snit_shaped_hint);
            }
            "dynamic_method_dispatch" => grammar.dynamic_method_dispatch = true,
            "manufacturer" => manufacturers.push(manufacturer_row(stmt, log)),
            "unknown_dispatch_method" => {
                grammar.unknown_dispatch_method = Some(leak_str(&value));
            }
            "property_accessor_methods" => {
                grammar.property_accessor_methods = leak_strs(&list_words(&value));
            }
            _ => log.unknown_property(stmt),
        }
    }
    grammar.members = leak_slice(members);
    grammar.builtin_object_methods = leak_slice(object_methods);
    grammar.member_body_commands = leak_slice(body_commands);
    grammar.manufacturers = leak_slice(manufacturers);
    grammar
}

/// Placeholder for a declared `bare_word_construction` hint until the hint's
/// own word / prefix sets are threaded through the grammar type.
fn snit_shaped_hint(word: &str) -> bool {
    word == "%AUTO%" || word.starts_with('.')
}

fn member_row(stmt: &Stmt, log: &mut Log) -> MemberSpec {
    let mut member = MemberSpec {
        keyword: leak_str(stmt.word_text(1)),
        arg_roles: &[],
        optional_argument: None,
        all_args_var: false,
        all_args_ref: None,
        kind: MemberKind::Flat,
        wrapper_block_body: false,
        dialects: None,
        retraction: None,
        slot: None,
        visibility_effect: None,
    };
    let mut slot_op: Option<SlotOp> = None;
    let mut dedup = false;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-roles" => {
                let text = next_text(words, &mut i);
                let parts = list_words(&text);
                let mut roles = Vec::new();
                for pair in parts.chunks(2) {
                    if let [index, role] = pair {
                        match (index.parse::<u8>(), by_name(ArgRole::ALL, role)) {
                            (Ok(index), Some(role)) => roles.push((index, role)),
                            _ => log.say(
                                stmt.line,
                                format!("unreadable member role pair `{index} {role}` dropped"),
                            ),
                        }
                    }
                }
                member.arg_roles = leak_slice(roles);
            }
            "-all-vars" => member.all_args_var = true,
            "-all-refs" => {
                const REFS: &[MemberRefKind] = &[MemberRefKind::Class, MemberRefKind::Method];
                let name = next_text(words, &mut i);
                member.all_args_ref =
                    enum_by_name(REFS, &name, "member reference kind", stmt.line, log);
            }
            "-kind" => {
                const KINDS: &[MemberKind] =
                    &[MemberKind::Flat, MemberKind::Wrapper, MemberKind::FlagKeyed];
                let name = next_text(words, &mut i);
                if let Some(kind) = enum_by_name(KINDS, &name, "member kind", stmt.line, log) {
                    member.kind = kind;
                }
            }
            "-block-body" => member.wrapper_block_body = true,
            "-dialects" => {
                let text = next_text(words, &mut i);
                member.dialects = parse_dialects(&text, stmt.line, log);
            }
            "-retracts" => {
                const RETRACTIONS: &[MemberRetraction] = &[
                    MemberRetraction::EveryArgument,
                    MemberRetraction::FirstArgument,
                ];
                let name = next_text(words, &mut i);
                member.retraction =
                    enum_by_name(RETRACTIONS, &name, "member retraction", stmt.line, log);
            }
            "-slot" => {
                const OPS: &[SlotOp] = &[
                    SlotOp::Set,
                    SlotOp::Append,
                    SlotOp::AppendIfNew,
                    SlotOp::Prepend,
                    SlotOp::Remove,
                    SlotOp::Clear,
                ];
                let name = next_text(words, &mut i);
                slot_op = enum_by_name(OPS, &name, "slot operation", stmt.line, log);
            }
            "-dedup" => dedup = true,
            "-visibility" => {
                const VISIBILITIES: &[MemberVisibility] =
                    &[MemberVisibility::Exported, MemberVisibility::Unexported];
                let name = next_text(words, &mut i);
                member.visibility_effect =
                    enum_by_name(VISIBILITIES, &name, "member visibility", stmt.line, log);
            }
            other => log.unknown_flag("member", stmt.line, other),
        }
        i += 1;
    }
    if let Some(default_op) = slot_op {
        member.slot = Some(SlotSpec { default_op, dedup });
    }
    member
}

fn builtin_object_method_row(stmt: &Stmt, log: &mut Log) -> BuiltinObjectMethod {
    let mut method = BuiltinObjectMethod {
        name: leak_str(stmt.word_text(1)),
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "",
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-unexported" => method.visibility = MemberVisibility::Unexported,
            "-receiver" => {
                const RECEIVERS: &[BuiltinMethodReceiver] = &[
                    BuiltinMethodReceiver::AnyObject,
                    BuiltinMethodReceiver::ClassObject,
                ];
                let name = next_text(words, &mut i);
                if let Some(receiver) =
                    enum_by_name(RECEIVERS, &name, "builtin method receiver", stmt.line, log)
                {
                    method.receiver = receiver;
                }
            }
            "-detail" => method.detail = leak_str(&next_text(words, &mut i)),
            other => log.unknown_flag("builtin_object_method", stmt.line, other),
        }
        i += 1;
    }
    method
}

fn member_body_command_row(stmt: &Stmt, log: &mut Log) -> MemberBodyCommand {
    let mut command = MemberBodyCommand {
        name: leak_str(stmt.word_text(1)),
        detail: "",
        binds_handle: None,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-detail" => command.detail = leak_str(&next_text(words, &mut i)),
            "-binds-handle" => {
                let text = next_text(words, &mut i);
                command.binds_handle = parse_handle_binding(&text, stmt.line, log);
            }
            other => log.unknown_flag("member_body_command", stmt.line, other),
        }
        i += 1;
    }
    command
}

fn manufacturer_row(stmt: &Stmt, log: &mut Log) -> ManufacturerMethod {
    let mut method = ManufacturerMethod {
        keyword: leak_str(stmt.word_text(1)),
        visibility: MemberVisibility::Exported,
        names_instance_at: None,
        definition_body_at: None,
        constructor_args_from: 0,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-unexported" => method.visibility = MemberVisibility::Unexported,
            "-names-instance-at" => {
                method.names_instance_at = next_text(words, &mut i).parse().ok();
            }
            "-definition-body-at" => {
                method.definition_body_at = next_text(words, &mut i).parse().ok();
            }
            "-constructor-args-from" => {
                method.constructor_args_from = next_text(words, &mut i).parse().unwrap_or(0);
            }
            other => log.unknown_flag("manufacturer", stmt.line, other),
        }
        i += 1;
    }
    method
}

/// The shipped definer grammars a pack may name.
fn shipped_definition_body(name: &str) -> Option<&'static DefinitionBodyGrammar> {
    match name {
        "tcloo" => Some(&tcl_registry::definer::TCLOO_GRAMMAR),
        "tcloo-configurable" => Some(&tcl_registry::definer::TCLOO_CONFIGURABLE_GRAMMAR),
        "snit" => Some(&tcl_registry::definer::SNIT_GRAMMAR),
        "snit-widget" => Some(&tcl_registry::definer::SNIT_WIDGET_GRAMMAR),
        "itcl" => Some(&tcl_registry::definer::ITCL_GRAMMAR),
        _ => None,
    }
}

/// The shipped case-list descriptors a pack may name.
fn shipped_case_list(name: &str) -> Option<&'static CaseListSpec> {
    match name {
        "switch" => Some(&CaseListSpec::SWITCH),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Command loading
// ---------------------------------------------------------------------------

/// Everything a command body accumulates before the spec is sealed.
#[derive(Default)]
struct CommandAcc {
    args: ArgRows,
    options: Vec<OptionSpec>,
    forms: Vec<FormSpec>,
    side_effects: Vec<SideEffect>,
    subcommands: Vec<SubCommand>,
    manufacturers: Vec<ManufacturerMethod>,
    repeats: Vec<tcl_registry::repeated::RepeatedArgLayout>,
    option_constraints: Vec<tcl_registry::spec::OptionConstraint>,
    setter_constraints: Vec<tcl_registry::taint::SetterConstraint>,
    oo_context_facts: Vec<(&'static str, tcl_registry::spec::OoContextFact)>,
    hooks: Vec<HookDecl>,
    clause_grammar: Option<ClauseGrammar>,
}

fn load_command(stmt: &Stmt, tables: &PackTables, log: &mut Log) -> Option<PackCommand> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`command` with no name dropped");
        return None;
    }
    let overrides_shipped = stmt.words.iter().any(|w| w.text == "-override");
    let body = stmt.words.iter().skip(2).find(|w| w.braced)?;

    log.scoped(format!("command {name}"), |log| {
        let mut spec = CommandSpec {
            name: leak_str(name),
            dialects: tables.defaults.dialects,
            required_package: tables.defaults.required_package,
            tcllib_package: tables.defaults.tcllib_package,
            ..CommandSpec::DEFAULT
        };
        spec.lifecycle.introduced = tables.defaults.introduced_version;
        spec.lifecycle.deprecated = tables.defaults.deprecated_version;
        spec.lifecycle.retired = tables.defaults.retired_version;
        if let Some(value) = tables.defaults.warn_missing_import {
            spec.warn_missing_import = value;
        }
        if let Some(value) = tables.defaults.is_namespace_exported {
            spec.is_namespace_exported = value;
        }

        let mut acc = CommandAcc::default();
        for stmt in block(body) {
            apply_command_stmt(&mut spec, &mut acc, &stmt, tables, log);
        }

        // A `clause_grammar` derives BOTH hook behaviours; the pack still
        // declares STRUCTURALLY_CHECKED_ARITY and the loader warns if it does
        // not.
        if let Some(grammar) = &acc.clause_grammar {
            spec.arg_role_resolver = Some(abstain_arg_roles);
            spec.clause_shape_check = Some(accept_clause_shape);
            for field in ["arg_role_resolver", "clause_shape_check"] {
                acc.hooks.push(HookDecl {
                    owner: HookOwner::Command,
                    field,
                    family: if field == "arg_role_resolver" {
                        HookFamily::ArgRoleResolver
                    } else {
                        HookFamily::ClauseShapeCheck
                    },
                    source: HookSource::Derived {
                        keyword: "clause_grammar".to_owned(),
                    },
                });
            }
            if !spec.traits.contains(Traits::STRUCTURALLY_CHECKED_ARITY) {
                log.say(
                    stmt.line,
                    "a `clause_grammar` command should also declare the \
                     STRUCTURALLY_CHECKED_ARITY trait",
                );
            }
            if grammar.head.is_empty() {
                log.say(stmt.line, "a `clause_grammar` needs a `head` clause");
            }
        }

        spec.arg_roles = leak_slice(acc.args.roles);
        spec.arg_types = leak_slice(acc.args.types);
        spec.arg_values = leak_slice(acc.args.values);
        spec.closed_value_args = leak_slice(acc.args.closed);
        spec.arg_presentation = leak_slice(acc.args.presentation);
        spec.command_prefixes = leak_slice(acc.args.prefixes);
        spec.options = leak_slice(acc.options);
        spec.forms = leak_slice(acc.forms);
        spec.side_effects = leak_slice(acc.side_effects);
        spec.subcommands = leak_slice(acc.subcommands);
        spec.manufacturer_methods = leak_slice(acc.manufacturers);
        spec.repeated_args = leak_slice(acc.repeats);
        spec.option_constraints = leak_slice(acc.option_constraints);
        spec.setter_constraints = leak_slice(acc.setter_constraints);
        spec.oo_context_facts = leak_slice(acc.oo_context_facts);

        Some(PackCommand {
            spec: leak_one(spec),
            overrides_shipped,
            hooks: acc.hooks,
            clause_grammar: acc.clause_grammar,
        })
    })
}

/// Read a hook property statement: a body, a `-native` id, or a derivation
/// keyword. Returns the source, or `None` when the statement is malformed.
fn hook_source(stmt: &Stmt) -> Option<HookSource> {
    // An optional leading `-inputs {…}` declares what the body reads, and is
    // the only flag a hook statement takes; everything after it is the shape
    // below.
    let (inputs, rest) = if stmt.words.len() >= 2 && stmt.word_text(1) == "-inputs" {
        let declared = list_words(stmt.word_text(2));
        let declared: Vec<&str> = declared.iter().map(String::as_str).collect();
        (HookInputs::parse(&declared), 2)
    } else {
        (HookInputs::unrestricted(), 0)
    };
    match stmt.words.len() - rest {
        2 if rest == 0 => Some(HookSource::Derived {
            keyword: stmt.word_text(1).to_owned(),
        }),
        3 if stmt.word_text(rest + 1) == "-native" => Some(HookSource::Native {
            id: stmt.word_text(rest + 2).to_owned(),
        }),
        3 => Some(HookSource::Body {
            params: list_words(stmt.word_text(rest + 1)),
            body: stmt.word_text(rest + 2).to_owned(),
            inputs,
        }),
        _ => None,
    }
}

#[allow(clippy::too_many_lines)]
fn apply_command_stmt(
    spec: &mut CommandSpec,
    acc: &mut CommandAcc,
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) {
    let key = stmt.word_text(0).to_owned();
    let value = stmt.word_text(1).to_owned();
    match key.as_str() {
        // --- identity and availability -----------------------------------
        "dialects" => spec.dialects = parse_dialects(&value, stmt.line, log),
        "traits" => spec.traits |= parse_traits(&value, stmt.line, log),
        "arity" => spec.arity = parse_arity(stmt, log),
        "required_package" => spec.required_package = Some(leak_str(&value)),
        "tcllib_package" => spec.tcllib_package = Some(leak_str(&value)),
        "implementation_namespace" => spec.implementation_namespace = Some(leak_str(&value)),
        "introduced_version" => spec.lifecycle.introduced = Some(leak_str(&value)),
        "deprecated_version" => spec.lifecycle.deprecated = Some(leak_str(&value)),
        "retired_version" => spec.lifecycle.retired = Some(leak_str(&value)),
        "warn_missing_import" => spec.warn_missing_import = parse_flag(stmt.tail()),
        "is_namespace_exported" => spec.is_namespace_exported = parse_flag(stmt.tail()),
        "unsafe_command" => spec.unsafe_command = parse_flag(stmt.tail()),
        "excluded_events" => spec.excluded_events = leak_strs(&list_words(&value)),
        "safe_on_uninit" => spec.safe_on_uninit = parse_dialects(&value, stmt.line, log),
        "deprecated_replacement" => spec.deprecated_replacement = Some(leak_str(&value)),
        "deprecated_replacement_drop_in" => {
            spec.deprecated_replacement_drop_in = parse_flag(stmt.tail());
        }
        "xc_translatable" => {
            spec.xc_translatable = parse_tristate(&value);
            if spec.xc_translatable.is_none() {
                log.say(stmt.line, "`xc_translatable` requires yes or no");
            }
        }
        "xc_operation" => spec.xc_operation = Some(leak_str(&value)),

        // --- shape --------------------------------------------------------
        "arg" => acc.args.apply(stmt, tables, log),
        "repeat" => {
            if let Some(layout) = repeat_row(stmt, log) {
                acc.repeats.push(layout);
            }
        }
        "reserved_trailing_words" => {
            spec.reserved_trailing_words = value.parse().unwrap_or(0);
        }
        "assigns_variable_at" => spec.assigns_variable_at = value.parse().ok(),
        "creates_instance_at" => spec.creates_instance_at = value.parse().ok(),
        "defines_command_at" => spec.defines_command_at = value.parse().ok(),
        "body_arg_implicit_args" => spec.body_arg_implicit_args = value.parse().unwrap_or(0),
        "body_kind" => {
            if let Some(kind) = enum_by_name(BODY_KINDS, &value, "body kind", stmt.line, log) {
                spec.body_kind = kind;
            }
        }
        "allow_unknown_subcommands" => {
            spec.allow_unknown_subcommands = parse_flag(stmt.tail());
        }
        "prefix_matching" => {
            const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];
            if let Some(mode) = enum_by_name(MATCHING, &value, "prefix matching", stmt.line, log) {
                spec.prefix_matching = mode;
            }
        }
        "self_receiver_words" => spec.self_receiver_words = leak_strs(&list_words(&value)),

        // --- types --------------------------------------------------------
        "return_type" => {
            spec.return_type = enum_by_name(TCL_TYPES, &value, "return type", stmt.line, log);
        }
        "var_write_typing" => {
            if let Some(typing) = parse_var_write_typing(&value, stmt.line, log) {
                spec.var_write_typing = typing;
            }
        }
        "inferred_storage_type" => {
            spec.inferred_storage_type =
                enum_by_name(STORAGE_TYPES, &value, "storage type", stmt.line, log);
        }
        "byte_array_effect" => {
            if let Some(effect) = parse_byte_array_effect(&value, stmt.line, log) {
                spec.byte_array_effect = effect;
            }
        }
        "byte_array_payload" => {
            spec.byte_array_payload = parse_byte_array_payload(stmt, log);
        }
        "pattern_type" => {
            const PATTERNS: &[PatternType] = &[PatternType::Glob, PatternType::Regex];
            spec.pattern_type = enum_by_name(PATTERNS, &value, "pattern type", stmt.line, log);
        }
        "format_string_type" => {
            spec.format_string_type =
                enum_by_name(FORMAT_TYPES, &value, "format string type", stmt.line, log);
        }
        "return_elements" => {
            spec.return_elements = parse_return_elements(&value, stmt.line, log);
        }
        "var_elements_effect" => {
            spec.var_elements_effect = parse_var_elements_effect(&value, stmt.line, log);
        }
        "representation_effect" => {
            spec.representation_effect = parse_representation_effect(&value, stmt.line, log);
        }
        "default_form_first_word" => {
            spec.default_form_first_word = enum_by_name(
                DEFAULT_FORM_FIRST_WORDS,
                &value,
                "default-form first word",
                stmt.line,
                log,
            );
        }
        "defines_symbol" => spec.defines_symbol = parse_defines_symbol(stmt, log),
        "deprecation_fix" => {
            spec.lifecycle.deprecation_fix = parse_deprecation_fix(stmt, log);
        }
        "setter_constraint" => {
            if let Some(constraint) = parse_setter_constraint(stmt, log) {
                acc.setter_constraints.push(constraint);
            }
        }

        // --- documentation ------------------------------------------------
        "hover" => {
            if let Some(word) = stmt.arg(1) {
                spec.hover = Some(hover_block(&block(word), log));
            }
        }
        "form" => acc.forms.push(form_row(stmt, log)),

        // --- effects ------------------------------------------------------
        "side_effect" => {
            if let Some(effect) = side_effect_row(stmt, log) {
                acc.side_effects.push(effect);
            }
        }
        "command_table_effect" => {
            const EFFECTS: &[CommandTableEffect] = &[
                CommandTableEffect::DefinesProcedure,
                CommandTableEffect::RenamesCommands,
                CommandTableEffect::CreatesAliases,
            ];
            spec.command_table_effect =
                enum_by_name(EFFECTS, &value, "command-table effect", stmt.line, log);
        }
        "frame_effect" => spec.frame_effect = frame_effect_row(stmt, log),
        "world_effects" => spec.world_effects = world_effects_value(stmt, tables, log),
        "state_transitions" => {
            spec.state_transitions = state_transitions_value(stmt, tables, log);
        }

        // --- taint --------------------------------------------------------
        "taint_source" => spec.taint_source = Some(parse_taint(&value, stmt.line, log)),
        "taint_transform" => spec.taint_transform = Some(parse_taint(&value, stmt.line, log)),
        "taint_double_encode_colour" => {
            spec.taint_double_encode_colour = Some(parse_taint(&value, stmt.line, log));
        }
        "taint_sink_safe_colour" => {
            spec.taint_sink_safe_colour = Some(parse_taint(&value, stmt.line, log));
        }
        "taint_output_sink" => spec.taint_output_sink = Some(leak_str(&value)),
        "taint_log_sink" => spec.taint_log_sink = Some(leak_str(&value)),
        "taint_output_sink_subcommands" => {
            spec.taint_output_sink_subcommands = leak_strs(&list_words(&value));
        }
        "taint_interp_eval_subcommands" => {
            spec.taint_interp_eval_subcommands = leak_strs(&list_words(&value));
        }
        "taint_network_sink_args" => {
            spec.taint_network_sink_args = Some(leak_slice(index_list(&value)));
        }
        "taint_code_sink_args" => {
            spec.taint_code_sink_args = Some(leak_slice(index_list(&value)));
        }
        "credential_options" => spec.credential_options = leak_strs(&list_words(&value)),
        "sensitive_headers" => spec.sensitive_headers = leak_strs(&list_words(&value)),

        // --- iRules -------------------------------------------------------
        "event_requires" => spec.event_requires = event_requires_value(stmt, tables, log),

        // --- options ------------------------------------------------------
        "option" => {
            let (option, hook) = option_row(stmt, tables, log);
            if let Some((source, option_name)) = hook {
                acc.hooks.push(HookDecl {
                    owner: HookOwner::Option {
                        subcommand: None,
                        option: option_name,
                    },
                    field: "options.arity_hook",
                    family: HookFamily::OptionArity,
                    source,
                });
            }
            acc.options.push(option);
        }
        "option_conflict" => {
            acc.option_constraints.push(option_conflict_row(stmt, log));
        }

        // --- descriptors --------------------------------------------------
        "case_list" => spec.case_list = case_list_value(stmt, tables, log),
        "definition_body" => spec.definition_body = definition_body_value(stmt, tables, log),
        "manufacturer" => acc.manufacturers.push(manufacturer_row(stmt, log)),
        "clause_grammar" => {
            if let Some(word) = stmt.arg(1) {
                acc.clause_grammar = Some(clause_grammar_block(&block(word), log));
            }
        }
        "binds_handle" => {
            spec.binds_handle = parse_handle_binding(&value, stmt.line, log).map(leak_one);
        }
        "object_class" => spec.object_class = object_class_row(stmt, tables, log).map(leak_one),
        "oo_context_fact" => {
            const FACTS: &[tcl_registry::spec::OoContextFact] =
                &[tcl_registry::spec::OoContextFact::DefiningClass];
            if let Some(fact) =
                enum_by_name(FACTS, stmt.word_text(2), "oo context fact", stmt.line, log)
            {
                acc.oo_context_facts.push((leak_str(&value), fact));
            }
        }

        // --- subcommands ---------------------------------------------------
        "subcommand" => {
            if let Some(sub) = load_subcommand(stmt, tables, &mut acc.hooks, "subcommand", log) {
                acc.subcommands.push(sub);
            }
        }

        // --- named engine hooks -------------------------------------------
        "lowering_hook" => {
            spec.lowering_hook = native_id(stmt, LOWERING_HOOKS, "lowering hook", log);
            if spec.lowering_hook.is_some() {
                log.say(
                    stmt.line,
                    "names a lowering hook: this changes how the compiler translates \
                     the command, not just what the editor knows about it",
                );
            }
        }
        "codegen_hook" => {
            spec.codegen_hook = native_id(stmt, CODEGEN_HOOKS, "codegen hook", log);
            if spec.codegen_hook.is_some() {
                log.say(
                    stmt.line,
                    "names a codegen hook: this changes how the compiler translates \
                     the command, not just what the editor knows about it",
                );
            }
        }
        "inline_codegen_hook" => {
            spec.inline_codegen_hook =
                native_id(stmt, INLINE_CODEGEN_HOOKS, "inline codegen hook", log);
        }
        "analyser_hook" => {
            spec.analyser_hook = native_id(stmt, ANALYSER_HOOKS, "analyser hook", log);
        }
        "semantic_operation" => {
            spec.semantic_operation = parse_semantic_operation(&value, stmt.line, log);
        }

        // --- Tcl-body hooks -------------------------------------------------
        "arg_role_resolver"
        | "command_prefix_resolver"
        | "const_fold"
        | "const_fold_versioned"
        | "taint_sink_gate"
        | "context_gate"
        | "literal_argument_validator"
        | "clause_shape_check" => {
            let Some(source) = hook_source(stmt) else {
                log.say(stmt.line, format!("unreadable `{key}` hook dropped"));
                return;
            };
            let (field, family) = match key.as_str() {
                "arg_role_resolver" => {
                    spec.arg_role_resolver = Some(abstain_arg_roles);
                    ("arg_role_resolver", HookFamily::ArgRoleResolver)
                }
                "command_prefix_resolver" => {
                    spec.command_prefix_resolver = Some(abstain_command_prefixes);
                    ("command_prefix_resolver", HookFamily::CommandPrefixResolver)
                }
                "const_fold" => {
                    spec.const_fold = Some(abstain_const_fold);
                    ("const_fold", HookFamily::ConstFold)
                }
                "const_fold_versioned" => {
                    spec.const_fold_versioned = Some(abstain_const_fold_versioned);
                    ("const_fold_versioned", HookFamily::ConstFoldVersioned)
                }
                "taint_sink_gate" => {
                    spec.taint_sink_gate = Some(sink_applies);
                    ("taint_sink_gate", HookFamily::TaintSinkGate)
                }
                "context_gate" => {
                    spec.context_gate = Some(allow_context);
                    ("context_gate", HookFamily::ContextGate)
                }
                "literal_argument_validator" => {
                    spec.literal_argument_validator = Some(literals_valid);
                    (
                        "literal_argument_validator",
                        HookFamily::LiteralArgumentValidator,
                    )
                }
                _ => {
                    spec.clause_shape_check = Some(accept_clause_shape);
                    ("clause_shape_check", HookFamily::ClauseShapeCheck)
                }
            };
            acc.hooks.push(HookDecl {
                owner: HookOwner::Command,
                field,
                family,
                source,
            });
        }

        _ => log.unknown_property(stmt),
    }
}

fn index_list(text: &str) -> Vec<u8> {
    list_words(text)
        .iter()
        .filter_map(|word| word.parse().ok())
        .collect()
}

fn native_id<T: Copy + fmt::Debug>(stmt: &Stmt, all: &[T], what: &str, log: &mut Log) -> Option<T> {
    if stmt.word_text(1) != "-native" {
        log.say(stmt.line, format!("`{what}` takes `-native ID`"));
        return None;
    }
    enum_by_name(all, stmt.word_text(2), what, stmt.line, log)
}

fn repeat_row(stmt: &Stmt, log: &mut Log) -> Option<tcl_registry::repeated::RepeatedArgLayout> {
    let role = enum_by_name(
        ArgRole::ALL,
        stmt.word_text(1),
        "argument role",
        stmt.line,
        log,
    )?;
    let mut layout = tcl_registry::repeated::RepeatedArgLayout {
        role,
        start: 0,
        stride: 1,
        exclude_trailing: 0,
        optional_leading_word: false,
        conditional_binding: false,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-from" => layout.start = next_text(words, &mut i).parse().unwrap_or(0),
            "-stride" => layout.stride = next_text(words, &mut i).parse().unwrap_or(1),
            "-exclude-trailing" => {
                layout.exclude_trailing = next_text(words, &mut i).parse().unwrap_or(0);
            }
            "-optional-leading" => layout.optional_leading_word = true,
            "-conditional" => layout.conditional_binding = true,
            other => log.unknown_flag("repeat", stmt.line, other),
        }
        i += 1;
    }
    Some(layout)
}

fn form_row(stmt: &Stmt, log: &mut Log) -> FormSpec {
    let kind = enum_by_name(FORM_KINDS, stmt.word_text(1), "form kind", stmt.line, log)
        .unwrap_or(FormKind::Default);
    let mut form = FormSpec {
        kind,
        synopsis: leak_str(stmt.word_text(2)),
        dialects: None,
    };
    let words = &stmt.words;
    let mut i = 3;
    while i < words.len() {
        match words[i].text.as_str() {
            "-dialects" => {
                let text = next_text(words, &mut i);
                form.dialects = parse_dialects(&text, stmt.line, log);
            }
            other => log.unknown_flag("form", stmt.line, other),
        }
        i += 1;
    }
    form
}

fn side_effect_row(stmt: &Stmt, log: &mut Log) -> Option<SideEffect> {
    let target = enum_by_name(
        SIDE_EFFECT_TARGETS,
        stmt.word_text(1),
        "side-effect target",
        stmt.line,
        log,
    )?;
    let mut effect = SideEffect {
        target,
        reads: false,
        writes: false,
        connection_side: ConnectionSide::None,
        dialects: None,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-reads" => effect.reads = true,
            "-writes" => effect.writes = true,
            "-side" => {
                let name = next_text(words, &mut i);
                if let Some(side) =
                    enum_by_name(CONNECTION_SIDES, &name, "connection side", stmt.line, log)
                {
                    effect.connection_side = side;
                }
            }
            "-dialects" => {
                let text = next_text(words, &mut i);
                effect.dialects = parse_dialects(&text, stmt.line, log);
            }
            other => log.unknown_flag("side_effect", stmt.line, other),
        }
        i += 1;
    }
    Some(effect)
}

fn option_conflict_row(stmt: &Stmt, log: &mut Log) -> tcl_registry::spec::OptionConstraint {
    let mut constraint = tcl_registry::spec::OptionConstraint {
        options: leak_strs(&list_words(stmt.word_text(1))),
        dialects: None,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-dialects" => {
                let text = next_text(words, &mut i);
                constraint.dialects = parse_dialects(&text, stmt.line, log);
            }
            other => log.unknown_flag("option_conflict", stmt.line, other),
        }
        i += 1;
    }
    constraint
}

fn frame_effect_row(stmt: &Stmt, log: &mut Log) -> Option<FrameEffectSpec> {
    let mut level_word = None;
    let mut layout = None;
    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        match words[i].text.as_str() {
            "-level-word" => {
                let name = next_text(words, &mut i);
                level_word =
                    enum_by_name(FRAME_LEVEL_WORDS, &name, "frame level word", stmt.line, log);
            }
            "-layout" => {
                let name = next_text(words, &mut i);
                layout = enum_by_name(
                    FRAME_LAYOUTS,
                    &name,
                    "frame argument layout",
                    stmt.line,
                    log,
                );
            }
            other => log.unknown_flag("frame_effect", stmt.line, other),
        }
        i += 1;
    }
    let (Some(level_word), Some(layout)) = (level_word, layout) else {
        log.say(stmt.line, "`frame_effect` needs -level-word and -layout");
        return None;
    };
    Some(FrameEffectSpec { level_word, layout })
}

/// The block-or-name resolution every block statement shares.
fn resolve_block<'a>(
    stmt: &'a Stmt,
    key: &str,
    tables: &'a PackTables,
    log: &mut Log,
) -> Option<Vec<Stmt>> {
    let word = stmt.arg(1)?;
    if word.braced {
        return Some(block(word));
    }
    let Some(stmts) = tables.descriptor(key, &word.text) else {
        log.say(
            stmt.line,
            format!("no `descriptor {key} {}` in this pack", word.text),
        );
        return None;
    };
    Some(stmts.to_vec())
}

fn event_requires_value(stmt: &Stmt, tables: &PackTables, log: &mut Log) -> Option<EventRequires> {
    resolve_block(stmt, "event_requires", tables, log)
        .map(|stmts| event_requires_block(&stmts, log))
}

fn case_list_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<&'static CaseListSpec> {
    let word = stmt.arg(1)?;
    if !word.braced
        && let Some(shipped) = shipped_case_list(&word.text)
    {
        return Some(shipped);
    }
    resolve_block(stmt, "case_list", tables, log)
        .map(|stmts| leak_one(case_list_block(&stmts, log)))
}

fn definition_body_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<&'static DefinitionBodyGrammar> {
    let word = stmt.arg(1)?;
    if !word.braced
        && let Some(shipped) = shipped_definition_body(&word.text)
    {
        return Some(shipped);
    }
    resolve_block(stmt, "definition_body", tables, log)
        .map(|stmts| leak_one(definition_body_block(&stmts, log)))
}

/// `world_effects none | NAME | { … }`.
///
/// The block's plain data is read; its `resolver` stays reference-only, which
/// is the boundary the design draws.
fn world_effects_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<WorldEffectDescriptor> {
    let word = stmt.arg(1)?;
    if !word.braced && word.text == "none" {
        return Some(WorldEffectDescriptor::EMPTY);
    }
    let stmts = resolve_block(stmt, "world_effects", tables, log)?;
    let mut descriptor = WorldEffectDescriptor::EMPTY;
    for stmt in &stmts {
        match stmt.word_text(0) {
            "composition" => {
                const COMPOSITIONS: &[tcl_registry::world_effect::WorldEffectComposition] = &[
                    tcl_registry::world_effect::WorldEffectComposition::Replace,
                    tcl_registry::world_effect::WorldEffectComposition::Extend,
                ];
                if let Some(composition) = enum_by_name(
                    COMPOSITIONS,
                    stmt.word_text(1),
                    "world-effect composition",
                    stmt.line,
                    log,
                ) {
                    descriptor.composition = composition;
                }
            }
            // The remaining rows (`access`, `callback`, `resolver`,
            // `dynamic_fallback`) describe typed effect facts the DSL cannot
            // yet construct; the declaration is kept as a notice rather than
            // silently claiming a footprint the pack did not get.
            other => log.say(
                stmt.line,
                format!("`world_effects` row `{other}` is not yet loadable; dropped"),
            ),
        }
    }
    Some(descriptor)
}

/// `state_transitions NAME | { … }`.
fn state_transitions_value(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<tcl_registry::state_transition::StateTransitionDescriptor> {
    let stmts = resolve_block(stmt, "state_transitions", tables, log)?;
    let mut descriptor = tcl_registry::state_transition::StateTransitionDescriptor::EMPTY;
    for stmt in &stmts {
        match stmt.word_text(0) {
            "composition" => {
                const COMPOSITIONS:
                    &[tcl_registry::state_transition::StateTransitionComposition] = &[
                    tcl_registry::state_transition::StateTransitionComposition::Replace,
                    tcl_registry::state_transition::StateTransitionComposition::Extend,
                ];
                if let Some(composition) = enum_by_name(
                    COMPOSITIONS,
                    stmt.word_text(1),
                    "state-transition composition",
                    stmt.line,
                    log,
                ) {
                    descriptor.composition = composition;
                }
            }
            // `argument_shape`, `resolver`, `widen`, `covers`, and `commit`
            // name typed transition facts; the resolver in particular is
            // reference-only by design.
            other => log.say(
                stmt.line,
                format!("`state_transitions` row `{other}` is not yet loadable; dropped"),
            ),
        }
    }
    Some(descriptor)
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

#[derive(Default)]
struct SubAcc {
    args: ArgRows,
    options: Vec<OptionSpec>,
    side_effects: Vec<SideEffect>,
    sub_subcommands: Vec<SubSubCommand>,
    repeats: Vec<tcl_registry::repeated::RepeatedArgLayout>,
    option_constraints: Vec<tcl_registry::spec::OptionConstraint>,
    versioned_arg_values: Vec<tcl_registry::spec::VersionedArgValue>,
}

/// Read a `subcommand NAME { … }` body, or the `method NAME { … }` row of an
/// `object_class` block — one grammar, `kind` naming which statement word
/// spelled it so notices say `method walk` rather than `subcommand walk`.
fn load_subcommand(
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    kind: &str,
    log: &mut Log,
) -> Option<SubCommand> {
    let name = stmt.word_text(1).to_owned();
    let body = stmt.arg(2)?;
    let outer = log.context.clone();
    log.scoped(format!("{outer} / {kind} {name}"), |log| {
        let mut sub = SubCommand {
            name: leak_str(&name),
            ..SubCommand::DEFAULT
        };
        let mut acc = SubAcc::default();
        for stmt in block(body) {
            apply_subcommand_stmt(&mut sub, &mut acc, &stmt, tables, hooks, &name, log);
        }
        sub.arg_roles = leak_slice(acc.args.roles);
        sub.arg_types = leak_slice(acc.args.types);
        sub.arg_values = leak_slice(acc.args.values);
        sub.closed_value_args = leak_slice(acc.args.closed);
        sub.arg_presentation = leak_slice(acc.args.presentation);
        sub.command_prefixes = leak_slice(acc.args.prefixes);
        sub.options = leak_slice(acc.options);
        sub.side_effects = leak_slice(acc.side_effects);
        sub.sub_subcommands = leak_slice(acc.sub_subcommands);
        sub.repeated_args = leak_slice(acc.repeats);
        sub.option_constraints = leak_slice(acc.option_constraints);
        sub.versioned_arg_values = leak_slice(acc.versioned_arg_values);
        Some(sub)
    })
}

#[allow(clippy::too_many_lines)]
fn apply_subcommand_stmt(
    sub: &mut SubCommand,
    acc: &mut SubAcc,
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    owner: &str,
    log: &mut Log,
) {
    let key = stmt.word_text(0).to_owned();
    let value = stmt.word_text(1).to_owned();
    match key.as_str() {
        "traits" => sub.traits |= parse_traits(&value, stmt.line, log),
        "arity" => sub.arity = parse_arity(stmt, log),
        "detail" => sub.detail = leak_str(&value),
        "synopsis" => sub.synopsis = leak_str(&value),
        "hover" => {
            if let Some(word) = stmt.arg(1) {
                sub.hover = Some(hover_block(&block(word), log));
            }
        }
        "arg" => acc.args.apply(stmt, tables, log),
        "repeat" => {
            if let Some(layout) = repeat_row(stmt, log) {
                acc.repeats.push(layout);
            }
        }
        "return_type" => {
            sub.return_type = enum_by_name(TCL_TYPES, &value, "return type", stmt.line, log);
        }
        "var_write_typing" => {
            if let Some(typing) = parse_var_write_typing(&value, stmt.line, log) {
                sub.var_write_typing = typing;
            }
        }
        "pure" => sub.pure = parse_flag(stmt.tail()),
        "mutator" => sub.mutator = parse_flag(stmt.tail()),
        "destructive" => sub.destructive = parse_flag(stmt.tail()),
        "returns_path" => sub.returns_path = parse_flag(stmt.tail()),
        "is_unescape" => sub.is_unescape = parse_flag(stmt.tail()),
        "loop_list_header" => sub.loop_list_header = parse_flag(stmt.tail()),
        "creates_scope_alias" => sub.creates_scope_alias = parse_flag(stmt.tail()),
        "arg_values_accept_prefix" => sub.arg_values_accept_prefix = parse_flag(stmt.tail()),
        "cfg_rewrite_name" => sub.cfg_rewrite_name = Some(leak_str(&value)),
        "min_abbrev" => sub.min_abbrev = value.parse().ok(),
        "max_leading_option_words" => sub.max_leading_option_words = value.parse().ok(),
        "defines_command_at" => sub.defines_command_at = value.parse().ok(),
        "body_arg_implicit_args" => sub.body_arg_implicit_args = value.parse().unwrap_or(0),
        "credential_arg" => sub.credential_arg = value.parse().ok(),
        "sensitive_headers" => sub.sensitive_headers = leak_strs(&list_words(&value)),
        "dialects" => sub.dialects = parse_dialects(&value, stmt.line, log),
        "safe_on_uninit" => sub.safe_on_uninit = parse_dialects(&value, stmt.line, log),
        "introduced_version" => sub.lifecycle.introduced = Some(leak_str(&value)),
        "deprecated_version" => sub.lifecycle.deprecated = Some(leak_str(&value)),
        "retired_version" => sub.lifecycle.retired = Some(leak_str(&value)),
        "xc_operation" => sub.xc_operation = Some(leak_str(&value)),
        "taint_output_sink" => sub.taint_output_sink = Some(leak_str(&value)),
        "taint_transform" => sub.taint_transform = Some(parse_taint(&value, stmt.line, log)),
        "taint_double_encode_colour" => {
            sub.taint_double_encode_colour = Some(parse_taint(&value, stmt.line, log));
        }
        "body_kind" => {
            if let Some(kind) = enum_by_name(BODY_KINDS, &value, "body kind", stmt.line, log) {
                sub.body_kind = kind;
            }
        }
        "byte_array_effect" => {
            if let Some(effect) = parse_byte_array_effect(&value, stmt.line, log) {
                sub.byte_array_effect = effect;
            }
        }
        "format_string_type" => {
            sub.format_string_type =
                enum_by_name(FORMAT_TYPES, &value, "format string type", stmt.line, log);
        }
        "return_elements" => {
            sub.return_elements = parse_return_elements(&value, stmt.line, log);
        }
        "var_elements_effect" => {
            sub.var_elements_effect = parse_var_elements_effect(&value, stmt.line, log);
        }
        "representation_effect" => {
            sub.representation_effect = parse_representation_effect(&value, stmt.line, log);
        }
        "deprecation_fix" => {
            sub.lifecycle.deprecation_fix = parse_deprecation_fix(stmt, log);
        }
        "inferred_storage_type" => {
            sub.inferred_storage_type =
                enum_by_name(STORAGE_TYPES, &value, "storage type", stmt.line, log);
        }
        "prefix_matching" => {
            const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];
            if let Some(mode) = enum_by_name(MATCHING, &value, "prefix matching", stmt.line, log) {
                sub.prefix_matching = mode;
            }
        }
        "pattern_type" => {
            const PATTERNS: &[PatternType] = &[PatternType::Glob, PatternType::Regex];
            sub.pattern_type = enum_by_name(PATTERNS, &value, "pattern type", stmt.line, log);
        }
        "option" => {
            let (option, hook) = option_row(stmt, tables, log);
            if let Some((source, option_name)) = hook {
                hooks.push(HookDecl {
                    owner: HookOwner::Option {
                        subcommand: Some(owner.to_owned()),
                        option: option_name,
                    },
                    field: "options.arity_hook",
                    family: HookFamily::OptionArity,
                    source,
                });
            }
            acc.options.push(option);
        }
        "option_conflict" => acc.option_constraints.push(option_conflict_row(stmt, log)),
        "side_effect" => {
            if let Some(effect) = side_effect_row(stmt, log) {
                acc.side_effects.push(effect);
            }
        }
        "sub_subcommand" => acc.sub_subcommands.push(sub_subcommand_row(stmt, log)),
        "versioned_arg_value" => {
            if let Some(gate) = versioned_arg_value_row(stmt, log) {
                acc.versioned_arg_values.push(gate);
            }
        }
        "command_table_effect" => {
            const EFFECTS: &[CommandTableEffect] = &[
                CommandTableEffect::DefinesProcedure,
                CommandTableEffect::RenamesCommands,
                CommandTableEffect::CreatesAliases,
            ];
            sub.command_table_effect =
                enum_by_name(EFFECTS, &value, "command-table effect", stmt.line, log);
        }
        "lowering_hook" => {
            sub.lowering_hook = native_id(stmt, LOWERING_HOOKS, "lowering hook", log);
        }
        "codegen_hook" => sub.codegen_hook = native_id(stmt, CODEGEN_HOOKS, "codegen hook", log),
        "inline_codegen_hook" => {
            sub.inline_codegen_hook =
                native_id(stmt, INLINE_CODEGEN_HOOKS, "inline codegen hook", log);
        }
        "analyser_hook" => {
            sub.analyser_hook = native_id(stmt, ANALYSER_HOOKS, "analyser hook", log);
        }
        "semantic_operation" => {
            sub.semantic_operation = parse_semantic_operation(&value, stmt.line, log);
        }
        "world_effects" => sub.world_effects = world_effects_value(stmt, tables, log),
        "state_transitions" => sub.state_transitions = state_transitions_value(stmt, tables, log),
        "arg_role_resolver"
        | "command_prefix_resolver"
        | "const_fold"
        | "const_fold_versioned"
        | "literal_argument_validator" => {
            let Some(source) = hook_source(stmt) else {
                log.say(stmt.line, format!("unreadable `{key}` hook dropped"));
                return;
            };
            let (field, family) = match key.as_str() {
                "arg_role_resolver" => {
                    sub.arg_role_resolver = Some(abstain_arg_roles);
                    ("arg_role_resolver", HookFamily::ArgRoleResolver)
                }
                "command_prefix_resolver" => {
                    sub.command_prefix_resolver = Some(abstain_command_prefixes);
                    ("command_prefix_resolver", HookFamily::CommandPrefixResolver)
                }
                "const_fold" => {
                    sub.const_fold = Some(abstain_const_fold);
                    ("const_fold", HookFamily::ConstFold)
                }
                "const_fold_versioned" => {
                    sub.const_fold_versioned = Some(abstain_const_fold_versioned);
                    ("const_fold_versioned", HookFamily::ConstFoldVersioned)
                }
                _ => {
                    sub.literal_argument_validator = Some(literals_valid);
                    (
                        "literal_argument_validator",
                        HookFamily::LiteralArgumentValidator,
                    )
                }
            };
            hooks.push(HookDecl {
                owner: HookOwner::Subcommand(owner.to_owned()),
                field,
                family,
                source,
            });
        }
        _ => log.unknown_property(stmt),
    }
}

fn sub_subcommand_row(stmt: &Stmt, log: &mut Log) -> SubSubCommand {
    let mut row = SubSubCommand {
        name: leak_str(stmt.word_text(1)),
        detail: "",
        synopsis: "",
        dialects: None,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-detail" => row.detail = leak_str(&next_text(words, &mut i)),
            "-synopsis" => row.synopsis = leak_str(&next_text(words, &mut i)),
            "-dialects" => {
                let text = next_text(words, &mut i);
                row.dialects = parse_dialects(&text, stmt.line, log);
            }
            other => log.unknown_flag("sub_subcommand", stmt.line, other),
        }
        i += 1;
    }
    row
}

fn versioned_arg_value_row(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::spec::VersionedArgValue> {
    let index = stmt.word_text(1).parse().ok()?;
    let mut gate = tcl_registry::spec::VersionedArgValue {
        index,
        value: leak_str(stmt.word_text(2)),
        lifecycle: tcl_registry::lifecycle::Lifecycle::UNSPECIFIED,
    };
    let words = &stmt.words;
    let mut i = 3;
    while i < words.len() {
        match words[i].text.as_str() {
            "-introduced" => gate.lifecycle.introduced = Some(leak_str(&next_text(words, &mut i))),
            "-deprecated" => gate.lifecycle.deprecated = Some(leak_str(&next_text(words, &mut i))),
            "-retired" => gate.lifecycle.retired = Some(leak_str(&next_text(words, &mut i))),
            other => log.unknown_flag("versioned_arg_value", stmt.line, other),
        }
        i += 1;
    }
    Some(gate)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The loader resolves a `-native ID` by matching the catalogue's own
    /// spelling, so its value tables have to name every variant the catalogue
    /// does. A registry that grows a hook and a studio catalogue that lists it
    /// would otherwise leave the loader silently dropping the new id.
    #[test]
    fn native_hook_tables_cover_their_catalogues() {
        fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
            all.iter().map(catalogue::variant_name).collect()
        }
        for (what, mine, catalogue) in [
            ("lowering", names(LOWERING_HOOKS), catalogue::LOWERING_HOOKS),
            ("codegen", names(CODEGEN_HOOKS), catalogue::CODEGEN_HOOKS),
            (
                "inline codegen",
                names(INLINE_CODEGEN_HOOKS),
                catalogue::INLINE_CODEGEN_HOOKS,
            ),
            ("analyser", names(ANALYSER_HOOKS), catalogue::ANALYSER_HOOKS),
        ] {
            let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
            let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
            expected.sort_unstable();
            mine.sort_unstable();
            assert_eq!(mine, expected, "the {what}-hook table is out of step");
        }
    }

    /// Same obligation for the vocabularies a row flag resolves.
    #[test]
    fn value_tables_cover_their_catalogues() {
        fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
            all.iter().map(catalogue::variant_name).collect()
        }
        for (what, mine, catalogue) in [
            ("argument role", names(ArgRole::ALL), catalogue::ARG_ROLES),
            ("Tcl type", names(TCL_TYPES), catalogue::TCL_TYPES),
            ("body kind", names(BODY_KINDS), catalogue::BODY_KINDS),
            (
                "argument presentation",
                names(PRESENTATIONS),
                catalogue::ARG_PRESENTATIONS,
            ),
            (
                "side-effect target",
                names(SIDE_EFFECT_TARGETS),
                catalogue::SIDE_EFFECT_TARGETS,
            ),
            (
                "connection side",
                names(CONNECTION_SIDES),
                catalogue::CONNECTION_SIDES,
            ),
            ("form kind", names(FORM_KINDS), catalogue::FORM_KINDS),
            (
                "storage type",
                names(STORAGE_TYPES),
                catalogue::STORAGE_TYPES,
            ),
        ] {
            let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
            let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
            expected.sort_unstable();
            mine.sort_unstable();
            assert_eq!(mine, expected, "the {what} table is out of step");
        }
    }

    #[test]
    fn a_pack_with_no_speclib_wrapper_loads_nothing_and_says_so() {
        let pack = load_pack("command lonely { arity 1 }");
        assert!(pack.commands.is_empty());
        assert_eq!(pack.notices.len(), 1);
        assert!(pack.notices[0].message.contains("no `speclib`"));
    }

    #[test]
    fn statement_separation_is_ordinary_tcl() {
        // `;` separates, `;#` is a trailing comment, and a bare `#` mid-line is
        // NOT one — it becomes flags on the row, all dropped with a notice.
        let pack = load_pack(
            "speclib probe 1.0 {\n\
             command demo { arity 2 # not a comment ; return_type String }\n\
             }",
        );
        let demo = pack.command("demo").expect("demo loads");
        assert_eq!(demo.spec.arity, Arity::exact(2));
        let dropped: Vec<&str> = pack
            .notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect();
        assert_eq!(
            dropped.len(),
            4,
            "each of `#`, `not`, `a`, `comment` drops: {dropped:?}"
        );
    }

    #[test]
    fn dialect_shorthands_close_over_the_right_versions() {
        let mut log = Log::default();
        assert_eq!(
            parse_dialects("tcl8.6+", 1, &mut log),
            Some(DialectSet::TCL86 | DialectSet::TCL90 | DialectSet::TCL91)
        );
        assert_eq!(
            parse_dialects("all-tcl f5-irules", 1, &mut log),
            Some(DialectSet::ALL_TCL | DialectSet::IRULES)
        );
        assert_eq!(
            parse_dialects("tcl8.x", 1, &mut log),
            Some(DialectSet::TCL8X)
        );
        assert!(log.notices.is_empty());
    }

    #[test]
    fn an_argument_index_above_the_table_width_drops() {
        let pack = load_pack(
            "speclib probe 1.0 {\n\
             command demo { arg 300 -role Body\n arg 2 -role Body }\n\
             }",
        );
        let demo = pack.command("demo").expect("demo loads");
        assert_eq!(demo.spec.arg_roles, &[(2, ArgRole::Body)]);
        assert!(pack.notices.iter().any(|n| n.message.contains("above the")));
    }

    /// The frozen spelling, whole: `object_class NAME ?-superclass {…}?
    /// ?-allow-unknown? { method … }`, with `method` rows reusing the
    /// `subcommand` body grammar.
    #[test]
    fn an_object_class_carries_its_name_flags_and_method_table() {
        let pack = load_pack(
            "speclib probe 1.0 {\n\
             command factory {\n\
               arity 1..\n\
               object_class ::probe::Widget -superclass {::probe::Base ::probe::Mixin} \
                 -allow-unknown {\n\
                 method configure {\n\
                   arity 1..\n\
                   detail   {Reconfigure the widget.}\n\
                   synopsis {$w configure ?-opt value ...?}\n\
                   mutator\n\
                   option -text -takes value\n\
                 }\n\
                 method cget { arity 1 ; detail {Read one option.} ; pure }\n\
               }\n\
             }\n\
             }",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let spec = pack.command("factory").expect("factory loads").spec;
        let class = spec.object_class.expect("the object_class is read");
        assert_eq!(class.class_name, "::probe::Widget");
        assert_eq!(class.superclasses, &["::probe::Base", "::probe::Mixin"]);
        assert!(class.allow_unknown_methods);

        let names: Vec<&str> = class.instance_methods.iter().map(|m| m.name).collect();
        assert_eq!(names, ["configure", "cget"]);
        let configure = class.instance_method("configure").expect("by name");
        assert_eq!(configure.detail, "Reconfigure the widget.");
        assert_eq!(configure.synopsis, "$w configure ?-opt value ...?");
        assert!(configure.mutator);
        assert_eq!(configure.options.len(), 1);
        assert_eq!(configure.options[0].name, "-text");
        assert!(class.instance_method("cget").expect("by name").pure);

        // The method table is the class's, not the command's: a factory
        // declaring no `subcommand` rows still has none.
        assert!(spec.subcommands.is_empty());
    }

    /// `object_class NAME` with neither flags nor a block is the documented
    /// short form — a class known by name whose methods are not enumerated.
    #[test]
    fn an_object_class_may_be_a_bare_name() {
        let pack = load_pack(
            "speclib probe 1.0 {\n\
             command factory { object_class ::probe::Opaque }\n\
             }",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let class = pack
            .command("factory")
            .expect("factory loads")
            .spec
            .object_class
            .expect("the object_class is read");
        assert_eq!(class.class_name, "::probe::Opaque");
        assert!(class.instance_methods.is_empty());
        assert!(class.superclasses.is_empty());
        assert!(!class.allow_unknown_methods);
    }

    /// A nameless class, an unknown flag, and a non-`method` member row are
    /// each a degradation with a notice, never a failure.
    #[test]
    fn a_malformed_object_class_degrades_with_a_notice() {
        let pack = load_pack(
            "speclib probe 1.0 {\n\
             command nameless { object_class }\n\
             command odd { object_class ::probe::C -mixin {::X} { subcommand s { arity 1 } } }\n\
             }",
        );
        let said = |needle: &str| {
            pack.notices
                .iter()
                .any(|notice| notice.message.contains(needle))
        };
        assert!(
            pack.command("nameless")
                .expect("still loads")
                .spec
                .object_class
                .is_none()
        );
        assert!(said("`object_class` with no class name dropped"));

        let odd = pack.command("odd").expect("still loads").spec;
        let class = odd.object_class.expect("the named class survives");
        assert_eq!(class.class_name, "::probe::C");
        assert!(class.instance_methods.is_empty());
        assert!(said("unknown flag `-mixin` on `object_class` dropped"));
        assert!(said("unknown property `subcommand` dropped"));
    }
}
