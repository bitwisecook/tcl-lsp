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
//! [`build_document`](tcl_compiler::parsing::syntax::build::build_document) /
//! [`segments_from_document`](tcl_compiler::parsing::syntax::segment::segments_from_document)
//! pair every static scan in the toolchain uses — and the readers turn the
//! declarations into live [`CommandSpec`]s, which the Spec Studio then seeds a
//! draft from through the ordinary `draft::from_command_spec`. Loading a pack
//! and browsing a shipped command therefore produce drafts by the *same* code,
//! which is what makes the per-port equivalence gate in
//! `tcl-spec-studio/tests/spectcl_ports.rs` meaningful.
//!
//! The frozen syntax is `docs/design/spec-dsl-examples/README.md`; the
//! architecture around it is `docs/design/spec-packs.md`.
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

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_core_types::DiagCode;
use tcl_dialect::{DialectSet, TclVersion};
use tcl_lexer::{LeadingBom, LexerConfig, SourceMap, TokenType};
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::arity::{Arity, ArityWindow};
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
use tcl_registry::events::{
    DataCollectionOperation, EventHandlerPriority, EventRequirementForm, EventRequires,
};
use tcl_registry::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevelWord};
use tcl_registry::handle_binding::{
    HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
use tcl_registry::hooks::{
    AnalyserHookId, ArgTypeHint, CodegenHookId, InlineCodegenHookId, LoweringHookId,
};
use tcl_registry::hover::{
    ArgValue, CallbackTaintInput, FormKind, FormSpec, HoverSnippet, IntegerDomain, OptionArg,
    OptionArity, OptionSpec, OptionValue, OptionValueOutcome, ScriptTiming, VariableScope,
};
use tcl_registry::intrinsic::IntrinsicId;
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::literal_validation::LiteralArgumentValidation;
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::patterns::{FormatType, PatternType};
use tcl_registry::presentation::ArgPresentation;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::result_stability::ResultStability;
use tcl_registry::scoped::{ScopedCommand, ScopedCommandEnv};
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::side_effects::{
    ConnectionSide, SideEffect, SideEffectTarget, SideSwitchTarget, StorageType,
};
use tcl_registry::spec::{
    BytePayloadSpec, CaseListSpec, CommandSpec, DefaultFormFirstWord, OptionPlacement, SubCommand,
    SubSubCommand,
};
use tcl_registry::symbol_def::{DefinedSymbolKind, SymbolDef};
use tcl_registry::taint::SetterConstraint;
use tcl_registry::traits::Traits;
use tcl_registry::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use tcl_registry::world_effect::WorldEffectDescriptor;
use tcl_registry::world_effect::WorldStateDomain;
use tcl_registry::{CommandPrefixArguments, InvocationArguments};

use crate::catalogue;

mod available;
mod dialect_block;
mod environment_block;
mod eval;
mod vocabulary_class;

pub use dialect_block::{PackDialect, PackDialectAxis};
pub(crate) use environment_block::reserved_name as reserved_environment_name;
pub use environment_block::{PackCore, PackEnvironment, PackEnvironmentTier};
pub use eval::{
    EvalOptions, EvalSnapshotKey, LOADER_EVAL_VERSION, eval_snapshot_key, evaluate_pack,
    evaluate_pack_in, evaluate_pack_with, provenance_violation,
};
pub use vocabulary_class::VocabularyClass;

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
    /// The §6.1 compatibility class of the word this notice is about.
    ///
    /// [`Presentation`](VocabularyClass::Presentation) for everything the
    /// loader has always warned-and-dropped; the stronger classes are only
    /// ever reached in the forward direction (a pack declaring a vocabulary
    /// this build postdates) and inside the `dialect` / `environment`
    /// blocks, where every word is semantic by construction.
    pub class: VocabularyClass,
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
}

impl Log {
    /// Record a use of `word`, which no vocabulary before `since` has.
    fn since(&mut self, line: u32, word: &str, since: &'static str) {
        self.newer_words
            .push((self.context.clone(), line, word.to_owned(), since));
    }

    fn v11(&mut self, line: u32, word: &str) {
        self.since(line, word, "1.1");
    }

    fn v12(&mut self, line: u32, word: &str) {
        self.since(line, word, "1.2");
    }

    fn v20(&mut self, line: u32, word: &str) {
        self.since(line, word, "2.0");
    }

    fn say(&mut self, line: u32, message: impl Into<String>) {
        self.notices.push(Notice {
            context: self.context.clone(),
            line,
            message: message.into(),
            class: VocabularyClass::Presentation,
        });
    }

    /// Say something in a stronger §6.1 class, and remember that the
    /// current spec was degraded by it.
    fn say_classified(&mut self, line: u32, class: VocabularyClass, message: impl Into<String>) {
        self.notices.push(Notice {
            context: self.context.clone(),
            line,
            message: message.into(),
            class,
        });
        match class {
            VocabularyClass::Presentation => {}
            VocabularyClass::Assistance => self.assistance_unknown = true,
            VocabularyClass::Semantic => self.semantic_unknown = true,
        }
    }

    /// Classify an unknown `word` and report it.
    ///
    /// The escalation past `Presentation` happens **only** when the pack
    /// declares a vocabulary this build postdates — §6.1's forward
    /// direction, "an older loader meeting newer vocabulary". An unknown
    /// word in a pack whose vocabulary this build knows in full is an
    /// author's typo, not a word with a meaning being dropped, and it keeps
    /// today's warn-and-drop treatment exactly.
    fn unknown_word(&mut self, line: u32, word: &str, message: String) {
        let class = if self.forward_vocabulary {
            vocabulary_class::classify(word)
        } else {
            VocabularyClass::Presentation
        };
        match class {
            VocabularyClass::Presentation => self.say(line, message),
            VocabularyClass::Assistance => self.say_classified(
                line,
                class,
                format!(
                    "{message}; it is assistance-class vocabulary this build does not \
                     speak, so the command loads with the affected capability degraded \
                     (design §6.1)"
                ),
            ),
            VocabularyClass::Semantic => self.say_classified(
                line,
                class,
                format!(
                    "{message}; it is semantic-class vocabulary this build does not \
                     speak, so the command is excluded from strong analysis rather than \
                     analysed without it (design §6.1)"
                ),
            ),
        }
    }

    /// Forget the per-spec degradation state before reading a new spec.
    fn begin_spec(&mut self) {
        self.assistance_unknown = false;
        self.semantic_unknown = false;
    }

    /// Run `body` with `context` pushed, restoring the previous label after.
    fn scoped<T>(&mut self, context: String, body: impl FnOnce(&mut Self) -> T) -> T {
        let previous = std::mem::replace(&mut self.context, context);
        let out = body(self);
        self.context = previous;
        out
    }

    fn unknown_property(&mut self, stmt: &Stmt) {
        // `word_text(0)` on a braced word is *every byte between the braces*,
        // so interpolating it raw put whole multi-line bodies into a
        // Problems-panel message (issue #1634). A diagnostic has to stay one
        // readable line, and a block that reached here is an orphan — the
        // useful thing to say is that it has no preceding declaration, not to
        // quote it back.
        let stmt_is_block = stmt.words.first().is_some_and(|w| w.braced);
        if stmt_is_block {
            self.say(
                stmt.line,
                "a `{ … }` block with no preceding declaration was dropped \
                 (an opening brace must be on the same line as the word it \
                 belongs to)",
            );
            return;
        }
        // `quotable` is the existing helper for exactly this — `unknown_flag`
        // has always used it; `unknown_property` simply never did, which is
        // the whole of the defect.
        let raw = stmt.word_text(0);
        let word = quotable(raw);
        let message = format!("unknown property `{word}` dropped");
        self.unknown_word(stmt.line, raw, message);
    }

    fn unknown_flag(&mut self, row: &str, line: u32, flag: &str) {
        let message = format!("unknown flag `{}` on `{row}` dropped", quotable(flag));
        self.unknown_word(line, flag, message);
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
    pub(crate) fn word_text(&self, i: usize) -> &str {
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

/// What a U+FEFF at byte 0 of the buffer being segmented *is*.
///
/// A `.tclspec` is a file, and a byte-order mark at the head of a file is a
/// prologue rather than the first character of the first command word — which
/// is exactly the distinction `tcl_dialect::LexerGrammar::script_skips_leading_bom`
/// draws for Tcl 9's `source` (issue #1218). Nested blocks are *not* files:
/// a U+FEFF at the start of a `hover` summary is a character the author typed,
/// so only [`pack_statements`] and [`speclib_version_span`] may ask to skip it.
///
/// It is a parameter rather than a constant inside [`segment`] because
/// [`block`] re-enters the same function for every braced word, and flipping
/// the flag there would silently edit pack *content*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileBom {
    /// This buffer is a whole pack file: a leading mark is a prologue.
    Skip,
    /// This buffer is a nested block: a leading mark is data.
    Content,
}

impl FileBom {
    /// The lexer's own spelling of the same question.
    fn leading(self) -> LeadingBom {
        match self {
            FileBom::Skip => LeadingBom::Skip,
            FileBom::Content => LeadingBom::Content,
        }
    }

    /// A discriminator for the memo key. Two segmentations of the same bytes
    /// under different dispositions are different answers, so they cannot
    /// share a cache slot.
    pub(crate) fn key(self) -> u8 {
        match self {
            FileBom::Skip => 1,
            FileBom::Content => 0,
        }
    }
}

/// Segment `source` into statements, numbering lines from `base_line`.
///
/// Comments, `;` separators, and line continuations are handled by the lexer,
/// so the loader inherits exactly the Tcl an author already knows — including
/// the trap that `#` only starts a comment where a command word would start.
///
/// Segmentation is a **pure function of `(source, base_line, bom)`**, which is
/// what lets [`crate::cache`] memoise it: every level of a pack — the file, the
/// `speclib` body, each `command` body, each descriptor block — reaches the
/// CST through this one door, so memoising here captures the whole tree
/// without the loader's readers knowing a cache exists. The memo is a no-op
/// unless a caller installed one.
///
/// `bom` joins the memo key as well as the signature, because it changes the
/// answer rather than merely the question. That is a correctness measure, not
/// a fix for a reachable bug — see [`crate::cache`]'s `Memo` for why the two
/// dispositions cannot currently meet in one memo.
fn statements(source: &str, base_line: u32, bom: FileBom) -> Vec<Stmt> {
    if let Some(hit) = crate::cache::memo_get(source, base_line, bom) {
        return hit;
    }
    let parsed = segment(source, base_line, bom);
    crate::cache::memo_put(source, base_line, bom, &parsed);
    parsed
}

/// The uncached segmentation — [`statements`] minus the memo.
fn segment(source: &str, base_line: u32, bom: FileBom) -> Vec<Stmt> {
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(
        source,
        LexerConfig {
            leading_bom: bom.leading(),
            ..LexerConfig::default()
        },
    );
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
///
/// [`FileBom::Content`], always: a block is not a file, so a U+FEFF at the head
/// of a `hover` summary is a character the author typed and must survive into
/// the string the registry gets.
pub(crate) fn block(word: &Word) -> Vec<Stmt> {
    statements(&word.text, word.line, FileBom::Content)
}

/// Segment a whole pack source — the loader's entry into [`statements`], and
/// the one [`crate::cache`] keys its on-disk entry from.
///
/// The one place [`FileBom::Skip`] is right: this is the file entry point, so a
/// leading byte-order mark is a prologue. A `.tclspec` saved by an editor that
/// writes "UTF-8 with BOM" otherwise loses its entire `speclib` declaration —
/// the first word decodes as `\u{feff}speclib`, which matches nothing
/// (issue #1635).
pub(crate) fn pack_statements(source: &str) -> Vec<Stmt> {
    statements(source, 1, FileBom::Skip)
}

/// Locate the top-level `speclib` statement's version word: the byte range
/// of the word's *content* (inside any braces/quotes, so a rewrite keeps
/// the author's delimiters) and its decoded text.
///
/// This is the hook `tcl spec upgrade` rewrites through. It must read words
/// exactly the way the loader does — same lexer, same segmentation — so
/// `speclib demo {1.0} { … }` and `speclib demo "1.0" { … }` decode to
/// `1.0` here just as they do at load.
///
/// Same [`FileBom::Skip`] as [`pack_statements`], and for the same reason: this
/// reads a whole file. If it disagreed with the loader about whether a leading
/// mark is a prologue, `tcl spec upgrade` would compute its byte range against
/// a different tokenisation than the one the loader used, and rewrite the
/// wrong span.
#[must_use]
pub fn speclib_version_span(source: &str) -> Option<(std::ops::Range<usize>, String)> {
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(
        source,
        LexerConfig {
            leading_bom: FileBom::Skip.leading(),
            ..LexerConfig::default()
        },
    );
    for segment in segments_from_document(document, &source_map) {
        if segment.texts.first().map(String::as_str) != Some("speclib") {
            continue;
        }
        let text = segment.texts.get(2)?.clone();
        let token = segment.argv.get(2)?;
        // Word tokens exclude the closing delimiter from `span.end` and
        // carry the opening one via `content_offset`, so
        // `start + content_offset .. end` is exactly the content range for
        // wrapped and bare words alike.
        let start = token.span.start() as usize + usize::from(token.content_offset);
        return Some((start..token.span.end() as usize, text));
    }
    None
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

// ---------------------------------------------------------------------------
// Hook bodies — carried as text, never run
// ---------------------------------------------------------------------------

/// Which of the ten hook families a body belongs to.
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

fn abstain_script_timings(_args: &[&str]) -> Vec<(u8, ScriptTiming)> {
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

/// The `constraints` placeholder a declared-but-unbound hook carries: no
/// report, which is exactly what a pack with no `constraints` hook answers.
fn no_constraint_reports(
    _facts: &tcl_registry::spec::OptionFacts<'_>,
) -> Vec<tcl_registry::spec::ConstraintReport> {
    Vec::new()
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
    /// rather than at the file's first line (issues #1637, #1638). The loader
    /// knows the line; the *file* is added later by the merge, which is the
    /// only layer that knows which file a command came from.
    pub line: u32,
    /// The file this command was declared in.
    ///
    /// Empty as the loader builds it — [`crate::loader`] parses one source at a
    /// time and is never told its path. [`crate::pack::load`] fills it in
    /// during the merge, so every command in a [`crate::MergedPack`] carries
    /// exact `(file, line)` attribution even when the pack spans many files.
    pub file: std::path::PathBuf,
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
    /// `co_provides` relations (`SpecTcl` 2.0, review B11): predicated
    /// "requiring NAME routes to this pack's package" declarations,
    /// carried as data — the loader-alias mechanics that consume them are
    /// later wire-up (P3+), and carrying them is what keeps an older
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
    /// first (issue #1631).
    pub ambient_packages: Vec<AmbientPackage>,
    /// The `environment NAME { … }` blocks the pack declares (`SpecTcl`
    /// 2.0, §6.2), in declaration order, with the rejected ones dropped.
    pub environments: Vec<PackEnvironment>,
    /// The `dialect NAME { … }` blocks the pack declares (`SpecTcl` 2.0,
    /// §6.2 owner directive), in declaration order, with the rejected ones
    /// dropped.
    pub dialects: Vec<PackDialect>,
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
/// row (`SpecTcl` 2.0, review B11): loading this pack's package
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
    /// belongs to a different document (found reviewing #1637).
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
        environments: Vec::new(),
        dialects: Vec::new(),
        dsl_version: String::new(),
        commands: Vec::new(),
        load_error: None,
        target_dependent: false,
        registrations: Vec::new(),
        notices: Vec::new(),
    }
}

/// Apply one pack-scope statement — every arm of the pack vocabulary except
/// `command`, which the caller collects for the second pass. Returns `true`
/// exactly when the statement is a `command` declaration.
///
/// The one reader for a pack-level word: the evaluation loader's replay is
/// its only caller, whether the row was written literally or computed.
fn apply_pack_stmt(pack: &mut Pack, tables: &mut PackTables, stmt: &Stmt, log: &mut Log) -> bool {
    match stmt.word_text(0) {
        "values" => tables.add_values(stmt, log),
        "descriptor" => tables.add_descriptor(stmt, log),
        "hook" => tables.add_hook(stmt, log),
        "default" => tables.add_default(stmt, log),
        "display_name" => {
            let name = stmt.word_text(1);
            if name.is_empty() {
                log.say(stmt.line, "`display_name` needs a name");
            } else {
                if pack.display_name.is_some() {
                    log.say(stmt.line, "`display_name` redeclared; last wins");
                }
                pack.display_name = Some(name.to_owned());
            }
        }
        "file_extension" => {
            if let Some(row) = file_extension_row(stmt, log) {
                if pack
                    .file_extensions
                    .iter()
                    .any(|prior| prior.extension == row.extension)
                {
                    log.say(
                        stmt.line,
                        format!("`file_extension {}` redeclared; first wins", row.extension),
                    );
                } else {
                    pack.file_extensions.push(row);
                }
            }
        }
        "ambient_package" => {
            log.v12(stmt.line, "ambient_package");
            if let Some(row) = ambient_package_row(stmt, log) {
                pack.ambient_packages.push(row);
            }
        }
        "provides" => {
            log.v20(stmt.line, "provides");
            if let Some(row) = provides_row(stmt, log) {
                // §6.2: commands default their provider to the pack's
                // `provides`. The first `provides` name is the fallback; an
                // explicit `default required_package` / `default available`
                // always wins over it (see `command_from_parts`).
                if tables.defaults.provides_package.is_none() {
                    tables.defaults.provides_package = Some(row.name);
                }
                pack.provides.push(row);
            }
        }
        "co_provides" => {
            log.v20(stmt.line, "co_provides");
            if let Some(row) = co_provides_row(stmt, log) {
                pack.co_provides.push(row);
            }
        }
        "include" => {
            // A literal pack-scope `include` is consumed by the capture layer
            // (`stage_include`) before any row reaches this reader, so the
            // only way here is a *computed* include — a templated pack
            // building the row, which the determinism contract does not
            // follow.
            log.v20(stmt.line, "include");
            log.say_classified(
                stmt.line,
                VocabularyClass::Semantic,
                "`include` must be a literal pack-scope row (the determinism contract \
                 forbids a computed include); the row is dropped and its declarations \
                 are not loaded",
            );
        }
        "environment" => {
            log.v20(stmt.line, "environment");
            if let Some(environment) = environment_block::parse(stmt, log) {
                if pack
                    .environments
                    .iter()
                    .any(|prior| prior.id == environment.id && prior.extends == environment.extends)
                {
                    log.say(
                        stmt.line,
                        format!(
                            "`environment {}` redeclared; the first is kept",
                            environment.id
                        ),
                    );
                } else {
                    pack.environments.push(environment);
                }
            }
        }
        "dialect" => {
            log.v20(stmt.line, "dialect");
            if let Some(dialect) = dialect_block::parse(stmt, log) {
                if pack.dialects.iter().any(|prior| prior.name == dialect.name) {
                    log.say(
                        stmt.line,
                        format!("`dialect {}` redeclared; the first is kept", dialect.name),
                    );
                } else {
                    pack.dialects.push(dialect);
                }
            }
        }
        "command" => return true,
        _ => log.unknown_property(stmt),
    }
    false
}

/// Vocabulary consistency: words newer than the pack's declaration.
/// Additions never gate, so THIS loader read them fine — but a loader
/// speaking only the declared vocabulary drops each one silently, and
/// raising the declaration is how a pack says it needs them. One notice
/// per site, so an editor can mark every offending row.
///
/// A pack with no `speclib` line declares nothing to be inconsistent with,
/// and one declaring a vocabulary at or above the word's own is correct;
/// both are skipped.
fn finish_newer_words(pack: &Pack, log: &mut Log) {
    if pack.dsl_version.is_empty() {
        log.newer_words.clear();
        return;
    }
    let declared = pack.dsl_version.clone();
    let name = pack.name.clone();
    for (context, line, word, since) in std::mem::take(&mut log.newer_words) {
        if tcl_registry::version::compare(&declared, since).is_ge() {
            continue;
        }
        log.notices.push(Notice {
            context,
            line,
            class: VocabularyClass::Presentation,
            message: format!(
                "`{word}` is SpecTcl {since} vocabulary, but this pack declares \
                 vocabulary {declared}; a {declared} loader drops the word — declare \
                 `speclib {name} {since}`"
            ),
        });
    }
}

/// Resolve every `environment … { core DIALECT RELEASE }` row against the
/// pack's own `dialect` blocks, once the whole file has been read.
///
/// A `core` row naming neither a compiled family nor a dialect this pack
/// declares is §6.1's semantic class — the row says what language the
/// environment's documents are — so the environment block is **rejected**,
/// not degraded. Run after both passes because a `dialect` block may be
/// written after the `environment` that rides it, and a pack's statement
/// order is the author's business.
fn finish_pack_cores(pack: &mut Pack, log: &mut Log) {
    if pack.environments.iter().all(|e| e.pack_core.is_none()) {
        return;
    }
    let dialects: Vec<(String, Vec<String>)> = pack
        .dialects
        .iter()
        .map(|dialect| {
            (
                dialect.name.clone(),
                dialect
                    .releases
                    .iter()
                    .map(|release| release.release.clone())
                    .collect(),
            )
        })
        .collect();
    let mut rejected: Vec<String> = Vec::new();
    for environment in &pack.environments {
        let Some(core) = &environment.pack_core else {
            continue;
        };
        let found = dialects.iter().find(|(name, _)| *name == core.dialect);
        let message = match found {
            None => Some(format!(
                "`environment {}` names core family `{}`, which is neither a compiled \
                 family (`tcl`, `f5-tcl`, `f5-irules`, `jim`) nor a `dialect` block this \
                 pack declares; the environment block is rejected",
                environment.id, core.dialect
            )),
            Some((_, releases)) if !releases.contains(&core.release) => Some(format!(
                "`environment {}` names core `{} {}`, which is not a `release` row of \
                 `dialect {}`; the environment block is rejected",
                environment.id, core.dialect, core.release, core.dialect
            )),
            Some(_) => None,
        };
        if let Some(message) = message {
            log.say(core.line, message);
            rejected.push(environment.id.clone());
        }
    }
    pack.environments
        .retain(|environment| !rejected.contains(&environment.id));
}

/// The `SpecTcl` **vocabulary** versions this loader reads, newest last.
///
/// Additive words never bump the vocabulary version
/// (`docs/design/spec-packs.md`, "Compatibility policy"), so `1`, `1.0`, `1.1`
/// and `1.2` all name a vocabulary this build understands in full. `2.0` joins
/// them under the same rule: the redesign's §6.1 compatibility contract keeps
/// one parser for every word ever ratified, so a 1.x pack and a 2.0 pack are
/// read by the same code and a 1.x pack loads to an identical surface. A pack
/// naming anything else still loads (unless its *major* is unsupported — see
/// [`check_vocabulary_version`]): the words it uses that this build knows are
/// read, and the rest hit the ordinary unknown-property rule.
pub const KNOWN_VOCABULARY_VERSIONS: &[&str] = &["1", "1.0", "1.1", "1.2", "2", "2.0"];

/// The newest vocabulary this loader speaks, for the notice below.
pub const NEWEST_VOCABULARY_VERSION: &str = "2.0";

/// The newest `speclib` **major** this loader supports.
///
/// §6.1: an unsupported major fails the pack closed. An unknown *minor*
/// within a supported major keeps loading maximally, because a minor is only
/// ever additive — but a major says the meaning of words this loader thinks it
/// knows has changed, and reading them anyway is exactly the "stronger,
/// safer-looking result" the contract forbids.
const NEWEST_SUPPORTED_MAJOR: u32 = 2;

/// `ambient_package NAME VERSION` — one package the pack's dialect provides
/// without a `package require`.
///
/// Both words are required. A row naming no version is dropped rather than
/// defaulted: an ambient package with no version would floor at nothing, which
/// is what the row exists to stop being the case.
fn ambient_package_row(stmt: &Stmt, log: &mut Log) -> Option<AmbientPackage> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`ambient_package` needs a package name");
        return None;
    }
    let version = stmt.word_text(2);
    if version.is_empty() {
        log.say(
            stmt.line,
            format!("`ambient_package {name}` needs the version the runtime provides; dropped"),
        );
        return None;
    }
    for extra in stmt.words.iter().skip(3) {
        log.unknown_flag("ambient_package", stmt.line, &extra.text);
    }
    Some(AmbientPackage {
        name: leak_str(name),
        version: leak_str(version),
        line: stmt.line,
    })
}

/// `provides NAME ?VERSION…?` — one package train this pack describes.
fn provides_row(stmt: &Stmt, log: &mut Log) -> Option<PackProvides> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`provides` needs a package name");
        return None;
    }
    let versions: Vec<&'static str> = stmt
        .words
        .iter()
        .skip(2)
        .map(|word| leak_str(&word.text))
        .collect();
    Some(PackProvides {
        name: leak_str(name),
        versions,
        line: stmt.line,
    })
}

/// `co_provides NAME ?-requires-exact PACKAGE? ?-when PREDICATE?`.
fn co_provides_row(stmt: &Stmt, log: &mut Log) -> Option<CoProvides> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`co_provides` needs the co-provided name");
        return None;
    }
    let mut row = CoProvides {
        name: leak_str(name),
        requires_exact: None,
        when: None,
        line: stmt.line,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-requires-exact" => {
                let package = next_text(words, &mut i);
                if package.is_empty() {
                    log.say(stmt.line, "`-requires-exact` needs a package name");
                } else {
                    row.requires_exact = Some(leak_str(&package));
                }
            }
            "-when" => {
                let predicate = next_text(words, &mut i);
                if predicate.is_empty() {
                    log.say(stmt.line, "`-when` needs a predicate");
                } else {
                    row.when = Some(leak_str(&predicate));
                }
            }
            other => log.unknown_flag("co_provides", stmt.line, other),
        }
        i += 1;
    }
    Some(row)
}

/// `file_extension EXT ?-name {…}? ?-dialect DIALECT?` — one extension the
/// pack's language is written under. The extension is normalised to
/// lower-case without a leading dot; `-dialect` must name a canonical
/// profile (the routing consumer needs an interned name), and a typo drops
/// only the routing, not the row.
fn file_extension_row(stmt: &Stmt, log: &mut Log) -> Option<FileExtension> {
    let raw = stmt.word_text(1);
    if raw.is_empty() {
        log.say(stmt.line, "`file_extension` needs an extension");
        return None;
    }
    let extension = raw.trim_start_matches('.').to_ascii_lowercase();
    if extension.is_empty() || extension.contains('.') || extension.contains(char::is_whitespace) {
        log.say(
            stmt.line,
            format!("`file_extension {raw}` is not a single extension"),
        );
        return None;
    }
    let mut row = FileExtension {
        extension,
        display_name: None,
        dialect: None,
        line: stmt.line,
        // Filled in by the merge, the only layer that knows the path.
        file: std::path::PathBuf::new(),
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-name" => {
                let name = next_text(words, &mut i);
                if name.is_empty() {
                    log.say(stmt.line, "`-name` needs a value");
                } else {
                    row.display_name = Some(name);
                }
            }
            "-dialect" => {
                let dialect = next_text(words, &mut i);
                match crate::environment::catalogue_profile_for_dialect(&dialect) {
                    Some(profile) => row.dialect = Some(profile.name),
                    None => log.say(
                        stmt.line,
                        format!(
                            "`-dialect {dialect}` names no dialect profile; \
                             the extension is kept without routing"
                        ),
                    ),
                }
            }
            other => log.unknown_flag("file_extension", stmt.line, other),
        }
        i += 1;
    }
    Some(row)
}

// ---------------------------------------------------------------------------
// `include` — pack-file inclusion under the determinism contract (2.0, Q6)
// ---------------------------------------------------------------------------

/// Resolves `include NAME` rows to source text.
///
/// The resolver is the whole IO surface: the loader itself never opens a
/// file, so "no IO beyond the pack search path" is enforced by handing the
/// loader a resolver that reaches nothing else — [`IncludeContext::for_file`]
/// reads only sibling files of the including pack, which is inside the
/// search path the discovery layer already walks.
pub struct IncludeContext {
    resolver: IncludeResolver,
}

/// The resolver an [`IncludeContext`] wraps: include name in, source text
/// (or a one-line reason) out.
type IncludeResolver = Box<dyn Fn(&str) -> Result<String, String>>;

impl std::fmt::Debug for IncludeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncludeContext").finish_non_exhaustive()
    }
}

impl IncludeContext {
    /// A context over an arbitrary resolver — the seam tests and the
    /// studio use.
    pub fn new(resolver: impl Fn(&str) -> Result<String, String> + 'static) -> Self {
        Self {
            resolver: Box::new(resolver),
        }
    }

    /// The file-system context for a pack at `path`: an include name
    /// resolves against the pack's **own directory** only. Name
    /// validation (no separators, no `..`) happens in the loader before
    /// the resolver is asked, so this cannot be steered outside the
    /// directory even by a hostile name.
    #[must_use]
    pub fn for_file(path: &std::path::Path) -> Self {
        let dir = path.parent().map(std::path::Path::to_path_buf);
        Self::new(move |name| {
            let Some(dir) = &dir else {
                return Err("the including pack has no parent directory".to_owned());
            };
            std::fs::read_to_string(dir.join(name)).map_err(|e| e.to_string())
        })
    }

    fn resolve(&self, name: &str) -> Result<String, String> {
        (self.resolver)(name)
    }
}

/// The most deeply nested chain of `include`s the loader follows. The
/// cap is a determinism bound, not a feature: a chain this deep is a
/// mistake, and an unbounded walk over a resolver is not a loader.
pub(crate) const INCLUDE_DEPTH_LIMIT: usize = 8;

/// Whether `source`'s first pack body carries a pack-scope `include` row —
/// the trigger [`crate::pack`] uses to load through a file-system
/// [`IncludeContext`] instead of the compiled cache, whose key cannot see
/// an included file's bytes. The substring test keeps the common
/// include-free pack on the cached path with no second parse.
#[must_use]
pub(crate) fn uses_include(source: &str) -> bool {
    if !source.contains("include") {
        return false;
    }
    let top = pack_statements(source);
    let Some(speclib) = top.iter().find(|s| s.word_text(0) == "speclib") else {
        return false;
    };
    let Some(body) = speclib.arg(3) else {
        return false;
    };
    block(body)
        .iter()
        .any(|stmt| stmt.word_text(0) == "include")
}

/// Why one `include NAME` word is unusable, or `Ok` with the name.
///
/// The one rule for what a valid include row is, whichever route captured
/// it: exactly two words, a non-empty literal name, and no path structure —
/// the resolver decides what a name means, but a name is never a path.
pub(crate) fn include_name(words: &[&str], line: u32) -> Result<String, Notice> {
    let reject = |message: String| Notice {
        context: "pack".to_owned(),
        line,
        message,
        class: VocabularyClass::Semantic,
    };
    if words.len() != 1 || words[0].is_empty() {
        return Err(reject(
            "`include` takes exactly one file name; the row is dropped and its \
             declarations are not loaded"
                .to_owned(),
        ));
    }
    let name = words[0];
    if name.contains(['/', '\\']) || name.contains("..") {
        return Err(reject(format!(
            "`include {name}` is not a plain file name (no path separators, no `..`); \
             the row is dropped and its declarations are not loaded"
        )));
    }
    Ok(name.to_owned())
}

/// The major component of a `speclib` version word, when it has one.
fn declared_major(declared: &str) -> Option<u32> {
    declared
        .split('.')
        .next()
        .and_then(|major| major.parse().ok())
}

/// Warn when `speclib`'s version word is not a vocabulary this build knows,
/// and report whether the pack must fail closed.
///
/// `true` means the declaration named a **major** past this loader
/// (`speclib probe 3.0`). §6.1: that is a load error, not a notice — every
/// word in the file may have been redefined, so loading the ones this build
/// recognises would publish confident answers derived from a vocabulary it
/// does not speak. An unknown *minor* is the additive case and keeps loading.
fn check_vocabulary_version(declared: &str, line: u32, log: &mut Log) -> bool {
    if declared.is_empty() || KNOWN_VOCABULARY_VERSIONS.contains(&declared) {
        return false;
    }
    if declared_major(declared).is_some_and(|major| major > NEWEST_SUPPORTED_MAJOR) {
        log.say(
            line,
            format!(
                "pack declares SpecTcl vocabulary {declared}; this loader supports major \
                 {NEWEST_SUPPORTED_MAJOR} at most, and a new major may redefine words this \
                 loader thinks it knows — nothing is loaded (design §6.1)"
            ),
        );
        return true;
    }
    // "Newer words may be dropped" is only true of a vocabulary this loader
    // postdates. Anything else in the slot — most often a pack that wrote its
    // *library's* version there (`speclib tclinterp 0.15`) — never named a
    // vocabulary at all, and the notice says so instead of implying the
    // loader is behind.
    let msg = if tcl_registry::version::compare(declared, NEWEST_VOCABULARY_VERSION).is_gt() {
        format!(
            "pack declares SpecTcl vocabulary {declared}; this loader knows \
             {NEWEST_VOCABULARY_VERSION} — newer words may be dropped"
        )
    } else {
        format!(
            "`{declared}` is not a SpecTcl vocabulary version (this loader \
             knows {NEWEST_VOCABULARY_VERSION}); if it is the library's own \
             version, it belongs in `introduced_version`, not the `speclib` \
             slot"
        )
    };
    log.say(line, msg);
    false
}

// ---------------------------------------------------------------------------
// Pack-level tables
// ---------------------------------------------------------------------------

/// The pack-wide availability / identity defaults a command inherits.
#[derive(Debug, Default, Clone)]
struct PackDefaults {
    dialects: Option<DialectSet>,
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
            "available" => {
                log.v20(stmt.line, "available");
                let availability = available::from_statement(stmt, 2, log);
                if availability.dialects.is_some() {
                    self.defaults.dialects = availability.dialects;
                }
                if let Some(package) = availability.required_package {
                    self.defaults.required_package = Some(package);
                }
            }
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

/// `value V ?-detail {…}? ?-min-tcl VER? ?-code N?` rows, plus the three
/// lifecycle flags.
///
/// The two version axes are independent: `-min-tcl` is the Tcl-core rung the
/// value needs, `-introduced` / `-deprecated` / `-retired` are the owning
/// package's releases.
fn value_rows(stmts: &[Stmt], log: &mut Log) -> Vec<ArgValue> {
    let mut out = Vec::new();
    for stmt in stmts {
        if stmt.word_text(0) != "value" {
            log.unknown_property(stmt);
            continue;
        }
        let mut row = ArgValue {
            value: leak_str(stmt.word_text(1)),
            ..ArgValue::DEFAULT
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
                other => {
                    if lifecycle_flag(&mut row.lifecycle, other, words, &mut i) {
                        log.v11(stmt.line, other);
                    } else {
                        log.unknown_flag("value", stmt.line, other);
                    }
                }
            }
            i += 1;
        }
        row.lifecycle = checked_lifecycle(
            row.lifecycle,
            &format!("value `{}`", row.value),
            stmt.line,
            log,
        );
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

/// The three lifecycle flags every gateable row shares — `-introduced`,
/// `-deprecated`, `-retired`, each taking one version word.
///
/// Returns whether `flag` was one of them, so a row's own flag reader keeps
/// its `other => unknown_flag(…)` arm for everything else. The releases are on
/// the entity's own package axis; `-min-tcl` stays the Tcl-core axis and the
/// two are independent (`ArgValue`'s two axes, `hover.rs`).
fn lifecycle_flag(lifecycle: &mut Lifecycle, flag: &str, words: &[Word], i: &mut usize) -> bool {
    match flag {
        "-introduced" => lifecycle.introduced = Some(leak_str(&next_text(words, i))),
        "-deprecated" => lifecycle.deprecated = Some(leak_str(&next_text(words, i))),
        "-retired" => lifecycle.retired = Some(leak_str(&next_text(words, i))),
        _ => return false,
    }
    true
}

/// The arity windows a body declared, with overlapping ones dropped.
///
/// Two windows covering the same release make the selected signature depend on
/// declaration order, which is not something a pack can have meant. The pack
/// keeps the first — the ordinary degradation — and gets a notice naming the
/// one dropped, so an editor can mark the row.
///
/// `registry_sweep` rejects the same shape outright for shipped specs. A pack
/// only ever gets a notice: it is authored outside this repository and must
/// still load.
fn checked_arity_windows(
    windows: Vec<ArityWindow>,
    what: &str,
    line: u32,
    log: &mut Log,
) -> &'static [ArityWindow] {
    let mut kept: Vec<ArityWindow> = Vec::with_capacity(windows.len());
    for window in windows {
        if let Some(clash) = kept.iter().find(|other| other.overlaps(window)) {
            log.say(
                line,
                format!(
                    "{what} arity window {:?} overlaps {:?}, which would make the \
                     signature depend on declaration order; the later one is dropped",
                    window.arity, clash.arity
                ),
            );
            continue;
        }
        kept.push(window);
    }
    leak_slice(kept)
}

/// A parsed lifecycle, or nothing when its releases are impossibly ordered.
///
/// An entity whose lifecycle is rejected still loads: the declaration is a
/// degradation like any other, so the pack keeps the option / form / value and
/// loses only the gate it could not have meant.
fn checked_lifecycle(lifecycle: Lifecycle, what: &str, line: u32, log: &mut Log) -> Lifecycle {
    match lifecycle.validate() {
        Ok(()) => lifecycle,
        Err(error) => {
            log.say(line, format!("{what}: {error}; lifecycle dropped"));
            Lifecycle::UNSPECIFIED
        }
    }
}

/// Report a child lifecycle that reaches outside its parent's window.
///
/// Notice-only, and the declaration is kept: the hard gate is the shipped-spec
/// sweep (`tcl-registry/tests/registry_sweep.rs`), which asserts the same
/// containment. Only releases the child declares **for itself** are compared —
/// one it leaves open is the inherited one — and the narrowed window must
/// still be a legal ordering, since a child introduced after its parent
/// retired is unreachable.
fn check_contained(what: &str, child: Lifecycle, parent: Lifecycle, line: u32, log: &mut Log) {
    if child.is_unspecified() || parent.is_unspecified() {
        return;
    }
    let merged = child.intersect(parent);
    for (release, own, narrowed) in [
        ("introduced", child.introduced, merged.introduced),
        ("deprecated", child.deprecated, merged.deprecated),
        ("retired", child.retired, merged.retired),
    ] {
        if let Some(own) = own
            && Some(own) != narrowed
        {
            log.say(
                line,
                format!(
                    "{what}: {release} {own} reaches outside the enclosing declaration's \
                     window; kept as declared"
                ),
            );
        }
    }
    if merged.validate().is_err() {
        log.say(
            line,
            format!("{what}: narrowed by the enclosing declaration it is never available"),
        );
    }
}

/// The Tcl list elements of a word, braces stripped, tolerating a malformed
/// list rather than refusing the whole declaration.
pub(crate) fn list_words(text: &str) -> Vec<String> {
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

/// Apply an [`available::Availability`] at a scope that has no
/// `required_package` field of its own.
///
/// Only `command` and `default` carry one, so a `{package NAME}` row
/// anywhere else has nowhere to land: it is reported rather than dropped in
/// silence, because a reader of the pack would otherwise believe the
/// requirement was in force.
fn apply_availability(
    target: &mut Option<DialectSet>,
    availability: available::Availability,
    scope: &str,
    line: u32,
    log: &mut Log,
) {
    if let Some(package) = availability.required_package {
        log.say(
            line,
            format!(
                "`available {{package {package}}}` on a {scope} has no SpecTcl 1.x field \
                 (only `command` and `default` carry `required_package`); the requirement \
                 is dropped"
            ),
        );
    }
    if availability.dialects.is_some() {
        *target = availability.dialects;
    }
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
const SCRIPT_TIMINGS: &[ScriptTiming] = &[
    ScriptTiming::SameInvocation,
    ScriptTiming::Deferred,
    ScriptTiming::ReferenceOnly,
];
const VARIABLE_SCOPES: &[VariableScope] = &[VariableScope::CurrentFrame, VariableScope::Global];

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

/// `arity 3.. -step 2 -also 2` and its five simpler spellings, optionally
/// gated by the three lifecycle flags (`SpecTcl` 1.2, issue #1627).
///
/// Returns the shape, and the lifecycle when the row carried one. An ungated
/// row is the command's plain arity exactly as before 1.2; a gated one is one
/// *window* of a signature that changed across the owning package's releases,
/// and several may be declared for the same command.
fn parse_arity(stmt: &Stmt, log: &mut Log) -> (Arity, Option<Lifecycle>) {
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
    let mut lifecycle = Lifecycle::UNSPECIFIED;
    let mut gated = false;
    let mut i = 2;
    while i < words.len() {
        let flag = words[i].text.clone();
        if lifecycle_flag(&mut lifecycle, &flag, words, &mut i) {
            log.v12(stmt.line, &flag);
            gated = true;
            i += 1;
            continue;
        }
        match flag.as_str() {
            "-step" => arity.step = next_text(words, &mut i).parse().unwrap_or(0),
            "-also" => arity.also_exact = next_text(words, &mut i).parse().ok(),
            other => log.unknown_flag("arity", stmt.line, other),
        }
        i += 1;
    }
    if !gated {
        return (arity, None);
    }
    let lifecycle = checked_lifecycle(lifecycle, "arity window", stmt.line, log);
    // A lifecycle rejected as impossibly ordered comes back UNSPECIFIED, which
    // as a window would silently cover every release and shadow the ones that
    // are well formed. The row keeps its shape and loses only the gate, so it
    // becomes the plain arity — the same degradation every other rejected
    // lifecycle takes.
    if lifecycle == Lifecycle::UNSPECIFIED {
        return (arity, None);
    }
    (arity, Some(lifecycle))
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
    deprecation_fix_from(&stmt.words, 1, stmt.line, log)
}

/// The flag reader both spellings share: the command / subcommand statement
/// (`deprecation_fix …`, whose own name word is skipped) and the option row's
/// `-deprecation-fix {…}` block, whose words are the block's own.
fn deprecation_fix_from(
    words: &[Word],
    start: usize,
    line: u32,
    log: &mut Log,
) -> Option<DeprecationFixHook> {
    let mut replacement: Option<&'static str> = None;
    let mut replace_arg: Option<u8> = None;
    let mut whole_invocation = false;
    let mut description: &'static str = "";
    let mut safety = DeprecationFixSafety::RequiresReview;

    let mut i = start;
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
                    line,
                    log,
                ) {
                    safety = chosen;
                }
            }
            other => log.unknown_flag("deprecation_fix", line, other),
        }
        i += 1;
    }
    let Some(replacement) = replacement else {
        log.say(line, "`deprecation_fix` needs `-replace WORD`");
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

/// `object_class NAME ?-superclass {…}? ?-allow-unknown?`
/// `?-method-prefix-matching Enabled|Strict? ?{ method … }?`.
///
/// The ratified spelling (`docs/design/spec-dsl-examples/README.md`, "Rulings
/// on the census drafts"): the class's scalar fields ride on the statement
/// rather than in the block.
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
    const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];

    let class_name = stmt.word_text(1).to_owned();
    if class_name.is_empty() {
        log.say(stmt.line, "`object_class` with no class name dropped");
        return None;
    }
    let mut superclasses: Vec<String> = Vec::new();
    let mut allow_unknown_methods = false;
    let mut method_prefix_matching = PrefixMatching::Strict;
    let mut body: Option<&Word> = None;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        match words[i].text.as_str() {
            "-superclass" => superclasses = list_words(&next_text(words, &mut i)),
            "-allow-unknown" => allow_unknown_methods = true,
            // The §6.2 `dynamic_surface` / `unknown_members` fact in its
            // object-class spelling: the instance-method surface is open.
            "-dynamic-surface" | "-unknown-members" => {
                log.v20(stmt.line, &format!("object_class {}", words[i].text));
                allow_unknown_methods = true;
            }
            "-method-prefix-matching" => {
                let value = next_text(words, &mut i);
                if let Some(mode) = enum_by_name(
                    MATCHING,
                    &value,
                    "object method prefix matching",
                    stmt.line,
                    log,
                ) {
                    method_prefix_matching = mode;
                    log.v12(stmt.line, "object_class -method-prefix-matching");
                }
            }
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
        method_prefix_matching,
    })
}

// ---------------------------------------------------------------------------
// Per-argument rows — six schema keys, one statement
// ---------------------------------------------------------------------------

/// The `u8` argument tables cap the index; an index above 255 is dropped with
/// a notice rather than wrapping.
const MAX_ARG_INDEX: usize = 255;

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
}

/// The six parallel per-argument slices a command or subcommand stores.
#[derive(Debug, Default)]
struct ArgSlices {
    roles: Vec<(u8, ArgRole)>,
    types: Vec<(u8, ArgTypeHint)>,
    values: Vec<(u8, &'static [ArgValue])>,
    closed: Vec<u8>,
    presentation: Vec<(u8, ArgPresentation)>,
    prefixes: Vec<(u8, tcl_registry::arg_role::AppendedArity)>,
}

/// The `arg` rows a command or subcommand body declared, in source order.
#[derive(Debug, Default)]
struct ArgRows {
    rows: Vec<ArgRow>,
}

impl ArgRows {
    /// Project the authored rows into the parallel slices the registry stores.
    ///
    /// The **one** place a per-argument row becomes the parallel form the rest
    /// of the registry reads: a column added to [`ArgRow`] and not added here
    /// silently vanishes, which `projection_carries_every_row_column` pins.
    fn seal(self) -> ArgSlices {
        let mut out = ArgSlices::default();
        for row in self.rows {
            if let Some(role) = row.role {
                out.roles.push((row.index, role));
            }
            if let Some(hint) = row.type_hint {
                out.types.push((row.index, hint));
            }
            if !row.values.is_empty() {
                out.values.push((row.index, row.values));
            }
            if row.closed {
                out.closed.push(row.index);
            }
            if let Some(presentation) = row.presentation {
                out.presentation.push((row.index, presentation));
            }
            if let Some(appends) = row.appends {
                out.prefixes.push((row.index, appends));
            }
        }
        out
    }

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

        let mut row = ArgRow {
            index,
            ..ArgRow::default()
        };
        let mut hint = ArgTypeHint {
            expected: None,
            shimmers: false,
            transparent_from: &[],
        };
        let mut saw_type_flag = false;
        let words = &stmt.words;
        let mut i = 2;
        while i < words.len() {
            let flag = words[i].text.clone();
            match flag.as_str() {
                "-role" => {
                    let name = next_text(words, &mut i);
                    if let Some(role) =
                        enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log)
                    {
                        // First declaration wins, matching `arg_role_at`'s
                        // `find` over the parallel slice: a row that named a
                        // role twice always resolved to the first one, and a
                        // `-appends`-implied `CommandPrefix` counts as one.
                        row.role.get_or_insert(role);
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
                            ..ArgValue::DEFAULT
                        })
                        .collect();
                    row.values = leak_slice(values);
                }
                "-values-from" => {
                    let name = next_text(words, &mut i);
                    match tables.values.get(&name) {
                        Some(values) => row.values = leak_slice(values.clone()),
                        None => log.say(
                            stmt.line,
                            format!("no `values {name}` table in this pack; row dropped"),
                        ),
                    }
                }
                "-closed" => row.closed = true,
                "-layout" => {
                    let name = next_text(words, &mut i);
                    if let Some(layout) =
                        enum_by_name(PRESENTATIONS, &name, "argument layout", stmt.line, log)
                    {
                        row.presentation = Some(layout);
                    }
                }
                "-appends" => {
                    let text = next_text(words, &mut i);
                    if let Some(appended) = parse_appended_arity(&text, stmt.line, log) {
                        row.appends = Some(appended);
                        // `-appends` implies the position is a command prefix,
                        // unless this index already has a role — from this row
                        // or an earlier one, since the parallel slice this
                        // projects into is index-keyed across rows.
                        if row.role.is_none()
                            && !self
                                .rows
                                .iter()
                                .any(|other| other.index == index && other.role.is_some())
                        {
                            row.role = Some(ArgRole::CommandPrefix);
                        }
                    }
                }
                other => log.unknown_flag("arg", stmt.line, other),
            }
            i += 1;
        }
        if saw_type_flag {
            row.type_hint = Some(hint);
        }
        self.rows.push(row);
    }
}

// ---------------------------------------------------------------------------
// Option rows
// ---------------------------------------------------------------------------

const fn is_variable_role(role: ArgRole) -> bool {
    matches!(role, ArgRole::VarRead | ArgRole::VarWrite)
}

/// Parse the finite Tk callback-marker vocabulary. A pack cannot invent a
/// marker: accepting arbitrary `%x` text here would silently turn framework
/// metadata into a security source.
fn parse_callback_taint_inputs(
    text: &str,
    line: u32,
    log: &mut Log,
) -> &'static [CallbackTaintInput] {
    let mut out = Vec::new();
    for marker in list_words(text) {
        let input = match marker.as_str() {
            "%P" => CallbackTaintInput::TK_PROPOSED_VALUE,
            "%s" => CallbackTaintInput::TK_CURRENT_VALUE,
            "%S" => CallbackTaintInput::TK_EDIT_TEXT,
            "%A" => CallbackTaintInput::TK_EVENT_CHAR,
            "%K" => CallbackTaintInput::TK_EVENT_KEYSYM,
            _ => {
                log.say(
                    line,
                    format!(
                        "`{marker}` is not a user-controlled Tk callback substitution; \
                         framework metadata must not be declared tainted"
                    ),
                );
                continue;
            }
        };
        if !out.contains(&input) {
            out.push(input);
        }
    }
    leak_slice(out)
}

/// Parse `callback_taint_inputs {{ARG {%P %S}} …}` for positional callback
/// bodies. Each row names the 0-based callback argument and its finite set of
/// externally controlled substitutions.
fn parse_callback_taint_input_table(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Vec<(u8, &'static [CallbackTaintInput])> {
    let mut out = Vec::new();
    for row in list_words(text) {
        let fields = list_words(&row);
        let Some(index) = fields.first() else {
            continue;
        };
        if fields.len() != 2 {
            log.say(
                line,
                "`callback_taint_inputs` rows need `{argument-index {%P ...}}`",
            );
            continue;
        }
        let Ok(index) = index.parse::<u8>() else {
            log.say(
                line,
                "`callback_taint_inputs` needs an integer argument index",
            );
            continue;
        };
        let inputs = parse_callback_taint_inputs(&fields[1], line, log);
        if inputs.is_empty() {
            continue;
        }
        out.push((index, inputs));
    }
    out
}

/// Reject positional taint metadata that can never describe a deferred
/// executable argument. Dynamic role/timing resolvers count as an exact fact:
/// the registry rechecks their concrete answer for each invocation before it
/// exposes the inputs, so a resolver's abstaining shapes remain harmless.
fn validated_callback_taint_input_table(
    entries: Vec<(u8, &'static [CallbackTaintInput])>,
    roles: &[(u8, ArgRole)],
    has_role_resolver: bool,
    can_defer: bool,
    owner: &str,
    line: u32,
    log: &mut Log,
) -> Vec<(u8, &'static [CallbackTaintInput])> {
    entries
        .into_iter()
        .filter(|(index, _)| {
            let executable = roles
                .iter()
                .any(|(candidate, role)| candidate == index && role.has_script_timing())
                || has_role_resolver;
            if !executable || !can_defer {
                log.say(
                    line,
                    format!(
                        "{owner} declares callback taint inputs for argument {index}, which is not a deferred executable position; dropped"
                    ),
                );
                false
            } else {
                true
            }
        })
        .collect()
}

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
    // Defaults are intentionally valid for every value role.  Remember which
    // semantic flags the author actually wrote so validation can reject only
    // contradictory declarations, not an untouched default.
    let mut wrote_script_timing = false;
    let mut wrote_callback_taint_inputs = false;
    let mut wrote_variable_scope = false;
    let mut wrote_taints_var_write = false;

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
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(&mut option.dialects, availability, "option", stmt.line, log);
            }
            "-min-abbrev" => option.min_abbrev = next_text(words, &mut i).parse().ok(),
            // The data form only: `{-replace WORD ?-replace-arg N? …}`, read by
            // the same flag reader the command-level `deprecation_fix`
            // statement uses. The contextual-callback variant stays
            // reference-only.
            "-deprecation-fix" => {
                log.v11(stmt.line, "-deprecation-fix");
                i += 1;
                match words.get(i) {
                    Some(value) => {
                        let block: Vec<Word> = block(value)
                            .into_iter()
                            .flat_map(|stmt| stmt.words)
                            .collect();
                        option.lifecycle.deprecation_fix =
                            deprecation_fix_from(&block, 0, stmt.line, log);
                    }
                    None => log.say(stmt.line, "`-deprecation-fix` needs a `{…}` block"),
                }
            }
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
            "-script-timing" => {
                log.v12(stmt.line, "-script-timing");
                wrote_script_timing = true;
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(timing) =
                    enum_by_name(SCRIPT_TIMINGS, &name, "script timing", stmt.line, log)
                {
                    entry.script_timing = timing;
                }
            }
            "-callback-taint-inputs" => {
                log.v12(stmt.line, "-callback-taint-inputs");
                wrote_callback_taint_inputs = true;
                let text = next_text(words, &mut i);
                arg.get_or_insert(OptionArg::DEFAULT).callback_taint_inputs =
                    parse_callback_taint_inputs(&text, stmt.line, log);
            }
            "-variable-scope" => {
                log.v12(stmt.line, "-variable-scope");
                wrote_variable_scope = true;
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(scope) =
                    enum_by_name(VARIABLE_SCOPES, &name, "variable scope", stmt.line, log)
                {
                    entry.variable_scope = scope;
                }
            }
            "-values" => {
                let text = next_text(words, &mut i);
                let values: Vec<ArgValue> = list_words(&text)
                    .iter()
                    .map(|value| ArgValue {
                        value: leak_str(value),
                        ..ArgValue::DEFAULT
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
            "-taints-var-write" => {
                log.v12(stmt.line, "-taints-var-write");
                wrote_taints_var_write = true;
                arg.get_or_insert(OptionArg::DEFAULT).taints_var_write = true;
            }
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
            other => {
                if !lifecycle_flag(&mut option.lifecycle, other, words, &mut i) {
                    log.unknown_flag("option", stmt.line, other);
                }
            }
        }
        i += 1;
    }

    if let Some(mut arg) = arg {
        let roles = [Some(arg.role), arg.also_role];
        let executable = roles.into_iter().flatten().any(ArgRole::has_script_timing);
        let variable = roles.into_iter().flatten().any(is_variable_role);
        let variable_write = roles
            .into_iter()
            .flatten()
            .any(|role| role == ArgRole::VarWrite);

        if wrote_script_timing && !executable {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-script-timing` on a non-executable value; dropped",
                    option.name
                ),
            );
            arg.script_timing = ScriptTiming::SameInvocation;
        }
        if wrote_callback_taint_inputs
            && (!executable || arg.script_timing != ScriptTiming::Deferred)
        {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares callback taint inputs for a value that is not a deferred executable; dropped",
                    option.name
                ),
            );
            arg.callback_taint_inputs = &[];
        }
        if wrote_variable_scope && !variable {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-variable-scope` on a non-variable value; dropped",
                    option.name
                ),
            );
            arg.variable_scope = VariableScope::CurrentFrame;
        }
        if wrote_taints_var_write && !variable_write {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-taints-var-write` without a VarWrite role; dropped",
                    option.name
                ),
            );
            arg.taints_var_write = false;
        }
        option.value = OptionValue::Takes(arg);
    }
    option.lifecycle = checked_lifecycle(
        option.lifecycle,
        &format!("option `{}`", option.name),
        stmt.line,
        log,
    );
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
        flow: false,
    };
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "client_side" => requires.client_side = parse_flag(stmt.tail()),
            "server_side" => requires.server_side = parse_flag(stmt.tail()),
            "flow" => requires.flow = parse_flag(stmt.tail()),
            "transport" => requires.transport = Some(leak_str(&value)),
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
        two_arg_optionless_dialects: None,
        regex_option: None,
        exact_option: None,
        glob_option: None,
        nocase_option: None,
        end_options_option: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: None,
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: false,
        keyword_patterns: &[],
        keyword_patterns_require_final: false,
        optional_subject_separator: None,
        warn_unbraced_bodies: false,
    };
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "subject_args" => spec.subject_args = value.parse().unwrap_or(0),
            "two_arg_optionless_dialects" => {
                spec.two_arg_optionless_dialects = parse_dialects(&value, stmt.line, log);
            }
            "exact_option" => spec.exact_option = Some(leak_str(&value)),
            "glob_option" => spec.glob_option = Some(leak_str(&value)),
            "regex_option" => spec.regex_option = Some(leak_str(&value)),
            "nocase_option" => spec.nocase_option = Some(leak_str(&value)),
            "end_options_option" => spec.end_options_option = Some(leak_str(&value)),
            "fallthrough_body" => spec.fallthrough_body = Some(leak_str(&value)),
            "value_options_require_regex" => {
                spec.value_options_require_regex = leak_strs(&list_words(&value));
            }
            "special_match_options" => {
                spec.special_match_options = leak_strs(&list_words(&value));
            }
            "clause_flags" => spec.clause_flags = leak_strs(&list_words(&value)),
            "clause_regex_flag" => spec.clause_regex_flag = Some(leak_str(&value)),
            "clause_value_flags" => spec.clause_value_flags = leak_strs(&list_words(&value)),
            "clause_end_options_flag" => spec.clause_end_options_flag = Some(leak_str(&value)),
            "clause_force_inline_flag" => spec.clause_force_inline_flag = Some(leak_str(&value)),
            "clause_force_list_flag" => spec.clause_force_list_flag = Some(leak_str(&value)),
            "clause_force_list_shape" => {
                spec.clause_force_list_shape = match value.as_str() {
                    "first_arg_only_remainder" => {
                        Some(tcl_registry::CaseForceListShape::FirstArgOnlyRemainder)
                    }
                    _ => None,
                };
            }
            "allow_omitted_final_body" => spec.allow_omitted_final_body = parse_flag(stmt.tail()),
            "optional_subject_separator" => {
                spec.optional_subject_separator = Some(leak_str(&value));
            }
            "warn_unbraced_bodies" => spec.warn_unbraced_bodies = parse_flag(stmt.tail()),
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

/// The `(index, role)` pairs of a member row's `-roles` list.
///
/// Split out of [`member_row`] because the pairing rule — flat list, two
/// words per pair, an unreadable pair dropped with a notice rather than
/// shifting every pair after it — is a self-contained reading of one word.
fn member_arg_roles(text: &str, line: u32, log: &mut Log) -> Vec<(u8, ArgRole)> {
    let mut roles = Vec::new();
    for pair in list_words(text).chunks(2) {
        let [index, role] = pair else { continue };
        match (index.parse::<u8>(), by_name(ArgRole::ALL, role)) {
            (Ok(index), Some(role)) => roles.push((index, role)),
            _ => log.say(
                line,
                format!("unreadable member role pair `{index} {role}` dropped"),
            ),
        }
    }
    roles
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
                member.arg_roles = leak_slice(member_arg_roles(&text, stmt.line, log));
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
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut member.dialects,
                    availability,
                    "object-class method",
                    stmt.line,
                    log,
                );
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

/// The shipped scoped-body environments a pack may name.
fn shipped_body_scope(name: &str) -> Option<&'static ScopedCommandEnv> {
    match name {
        "report-defstyle" => Some(&tcl_registry::scoped::REPORT_DEFSTYLE_ENV),
        "tclpkg-manifest" => Some(&tcl_registry::scoped::TCLPKG_MANIFEST_ENV),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The ratified words (design §6.2)
// ---------------------------------------------------------------------------
//
// Seven words the DSL memo's coverage matrix has always documented and the
// loader had no reader for — the `DraftOpaque`-masks-`LoaderGap` blind spot
// §6.3 names. They are not *new* vocabulary, so they draw no per-site
// version notice: a 1.x pack that spelled one was always meant to load it.

/// Every result-stability spelling with no payload.
const RESULT_STABILITIES: &[ResultStability] = &[
    ResultStability::Unknown,
    ResultStability::ReferentiallyTransparent,
    ResultStability::Volatile,
];

/// The versioned world-state domains a `ReadsVersionedWorld` may list.
const WORLD_STATE_DOMAINS: &[WorldStateDomain] = &[
    WorldStateDomain::InterpreterTopology,
    WorldStateDomain::CommandBindings,
    WorldStateDomain::NamespaceLookup,
    WorldStateDomain::NamespaceUnknown,
    WorldStateDomain::ExecutionTraces,
    WorldStateDomain::VariableTraces,
    WorldStateDomain::CommandTraces,
    WorldStateDomain::OoDispatch,
    WorldStateDomain::InterpreterPolicy,
    WorldStateDomain::PackageState,
    WorldStateDomain::HostCapabilities,
];

/// The sides a `side_switch_target` may select.
const SIDE_SWITCH_TARGETS: &[SideSwitchTarget] = &[
    SideSwitchTarget::Client,
    SideSwitchTarget::Server,
    SideSwitchTarget::Peer,
];

/// `result_stability Unknown|ReferentiallyTransparent|Volatile|{ReadsVersionedWorld {D …}}`.
///
/// The payload-carrying variant is one braced word holding the variant name
/// and its domain list, which is how the memo spells every payload variant.
fn result_stability_row(stmt: &Stmt, log: &mut Log) -> Option<ResultStability> {
    let value = stmt.word_text(1);
    let words = list_words(value);
    if words.first().map(String::as_str) == Some("ReadsVersionedWorld") {
        let listed = words
            .get(1)
            .map(|text| list_words(text))
            .unwrap_or_default();
        if listed.is_empty() {
            log.say(
                stmt.line,
                "`result_stability {ReadsVersionedWorld {D …}}` needs at least one \
                 world-state domain; the row is dropped",
            );
            return None;
        }
        let domains: Vec<WorldStateDomain> = listed
            .iter()
            .filter_map(|name| {
                enum_by_name(
                    WORLD_STATE_DOMAINS,
                    name,
                    "world-state domain",
                    stmt.line,
                    log,
                )
            })
            .collect();
        if domains.len() != listed.len() {
            // An unreadable domain would silently *narrow* the dependency
            // set, which is the direction that makes reuse unsound.
            return None;
        }
        return Some(ResultStability::ReadsVersionedWorld(leak_slice(domains)));
    }
    enum_by_name(
        RESULT_STABILITIES,
        value,
        "result stability",
        stmt.line,
        log,
    )
}

/// `event_handler_priority -default N ?-warn-implicit? …`.
///
/// `-default` is the one required flag; the rest of `EventHandlerPriority`
/// takes the open defaults (`priority`, the whole `u16` range, lower first)
/// so a pack states only what its dialect actually constrains.
fn event_handler_priority_row(stmt: &Stmt, log: &mut Log) -> Option<EventHandlerPriority> {
    let mut policy = EventHandlerPriority {
        keyword: "priority",
        default_priority: 0,
        min_priority: 0,
        max_priority: u16::MAX,
        lower_runs_first: true,
        warn_when_implicit: false,
    };
    let mut stated_default = false;
    let words = &stmt.words;
    let mut index = 1;
    while index < words.len() {
        match words[index].text.as_str() {
            "-keyword" => policy.keyword = leak_str(&next_text(words, &mut index)),
            "-default" => {
                let text = next_text(words, &mut index);
                let Ok(value) = text.parse() else {
                    log.say(
                        stmt.line,
                        format!("`event_handler_priority -default {text}` is not a priority"),
                    );
                    return None;
                };
                policy.default_priority = value;
                stated_default = true;
            }
            "-min" => policy.min_priority = next_text(words, &mut index).parse().unwrap_or(0),
            "-max" => {
                policy.max_priority = next_text(words, &mut index).parse().unwrap_or(u16::MAX);
            }
            "-higher-runs-first" => policy.lower_runs_first = false,
            "-warn-implicit" => policy.warn_when_implicit = true,
            other => log.unknown_flag("event_handler_priority", stmt.line, other),
        }
        index += 1;
    }
    if !stated_default {
        log.say(
            stmt.line,
            "`event_handler_priority` needs `-default N` — the priority the runtime \
             uses when the keyword is omitted; the row is dropped",
        );
        return None;
    }
    if policy.min_priority > policy.max_priority || !policy.accepts(policy.default_priority) {
        log.say(
            stmt.line,
            format!(
                "`event_handler_priority -default {}` is outside `{}..={}`; the row is dropped",
                policy.default_priority, policy.min_priority, policy.max_priority
            ),
        );
        return None;
    }
    Some(policy)
}

/// The shared data-collection descriptors, by the id `-native` names.
///
/// Reference-only by design: the descriptor is paired with protocol
/// machinery outside the registry, so a pack names one rather than
/// spelling it.
fn data_collection_by_id(id: &str) -> Option<DataCollectionOperation> {
    use tcl_registry::events as ev;
    let found = match id {
        "HTTP_COLLECT" => ev::HTTP_COLLECT,
        "HTTP_RELEASE" => ev::HTTP_RELEASE,
        "HTTP_PAYLOAD" => ev::HTTP_PAYLOAD,
        "TCP_COLLECT" => ev::TCP_COLLECT,
        "TCP_RELEASE" => ev::TCP_RELEASE,
        "TCP_PAYLOAD" => ev::TCP_PAYLOAD,
        "SSL_COLLECT" => ev::SSL_COLLECT,
        "SSL_RELEASE" => ev::SSL_RELEASE,
        "SSL_PAYLOAD" => ev::SSL_PAYLOAD,
        "UDP_PAYLOAD" => ev::UDP_PAYLOAD,
        "ASM_PAYLOAD" => ev::ASM_PAYLOAD,
        "MQTT_COLLECT" => ev::MQTT_COLLECT,
        "MQTT_RELEASE" => ev::MQTT_RELEASE,
        "MQTT_PAYLOAD" => ev::MQTT_PAYLOAD,
        "MR_COLLECT" => ev::MR_COLLECT,
        "MR_RELEASE" => ev::MR_RELEASE,
        "MR_PAYLOAD" => ev::MR_PAYLOAD,
        "RTSP_COLLECT" => ev::RTSP_COLLECT,
        "RTSP_RELEASE" => ev::RTSP_RELEASE,
        "RTSP_PAYLOAD" => ev::RTSP_PAYLOAD,
        "SCTP_COLLECT" => ev::SCTP_COLLECT,
        "SCTP_RELEASE" => ev::SCTP_RELEASE,
        "SCTP_PAYLOAD" => ev::SCTP_PAYLOAD,
        "WS_COLLECT" => ev::WS_COLLECT,
        "WS_RELEASE" => ev::WS_RELEASE,
        "WS_PAYLOAD" => ev::WS_PAYLOAD,
        "CACHE_PAYLOAD" => ev::CACHE_PAYLOAD,
        "DIAMETER_PAYLOAD" => ev::DIAMETER_PAYLOAD,
        "GTP_PAYLOAD" => ev::GTP_PAYLOAD,
        "REWRITE_PAYLOAD" => ev::REWRITE_PAYLOAD,
        "SIP_PAYLOAD" => ev::SIP_PAYLOAD,
        "XML_PAYLOAD" => ev::XML_PAYLOAD,
        _ => return None,
    };
    Some(found)
}

/// `data_collection -native ID`.
fn data_collection_row(stmt: &Stmt, log: &mut Log) -> Option<DataCollectionOperation> {
    if stmt.word_text(1) != "-native" {
        log.say(stmt.line, "`data_collection` takes `-native ID`");
        return None;
    }
    let id = stmt.word_text(2);
    let found = data_collection_by_id(id);
    if found.is_none() {
        log.say(
            stmt.line,
            format!("unknown data-collection descriptor `{id}` dropped"),
        );
    }
    found
}

/// `event_requirement_form {word …} ?-only-in {E …}? ?{ … }?`.
///
/// The literal selector is the row's own first word; the trailing block, when
/// there is one, is a nested `event_requires` read by the same reader the
/// standalone row uses.
fn event_requirement_form_row(stmt: &Stmt, log: &mut Log) -> EventRequirementForm {
    let prefix = list_words(stmt.word_text(1));
    let mut only_in: Vec<String> = Vec::new();
    let mut requires = None;
    let words = &stmt.words;
    let mut index = 2;
    while index < words.len() {
        match words[index].text.as_str() {
            "-only-in" => only_in = list_words(&next_text(words, &mut index)),
            _ if words[index].braced => {
                requires = Some(event_requires_block(&block(&words[index]), log));
            }
            other => log.unknown_flag("event_requirement_form", stmt.line, other),
        }
        index += 1;
    }
    EventRequirementForm {
        argument_prefix: leak_strs(&prefix),
        requires,
        only_in: leak_strs(&only_in),
    }
}

/// `body_scope NAME | { … }` — a shipped environment by name, a pack
/// `descriptor body_scope NAME { … }`, or an inline block.
fn body_scope_value(
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    log: &mut Log,
) -> Option<&'static ScopedCommandEnv> {
    let word = stmt.arg(1)?;
    if !word.braced
        && let Some(shipped) = shipped_body_scope(&word.text)
    {
        return Some(shipped);
    }
    resolve_block(stmt, "body_scope", tables, log)
        .map(|stmts| leak_one(body_scope_block(&stmts, tables, hooks, log)))
}

/// The rows of a `body_scope { … }` block: `ScopedCommandEnv`'s own fields.
fn body_scope_block(
    stmts: &[Stmt],
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    log: &mut Log,
) -> ScopedCommandEnv {
    let mut env = ScopedCommandEnv {
        name: "",
        commands: &[],
        include_sibling_definitions: false,
        allow_unknown_commands: false,
    };
    let mut commands: Vec<ScopedCommand> = Vec::new();
    for stmt in stmts {
        match stmt.word_text(0) {
            "name" => env.name = leak_str(stmt.word_text(1)),
            "include_sibling_definitions" => {
                env.include_sibling_definitions = parse_flag(stmt.tail());
            }
            "allow_unknown_commands" => env.allow_unknown_commands = parse_flag(stmt.tail()),
            "command" => {
                if let Some(command) = scoped_command_row(stmt, tables, hooks, log) {
                    commands.push(command);
                }
            }
            _ => log.unknown_property(stmt),
        }
    }
    env.commands = leak_slice(commands);
    env
}

/// One `command NAME { … }` of a `body_scope` block.
fn scoped_command_row(
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    log: &mut Log,
) -> Option<ScopedCommand> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "a `body_scope` `command` needs a name");
        return None;
    }
    let Some(body) = stmt.arg(2) else {
        log.say(
            stmt.line,
            format!("`command {name}` in a `body_scope` has no `{{ … }}` block"),
        );
        return None;
    };
    let mut command = ScopedCommand {
        name: leak_str(name),
        arity: Arity::any(),
        subcommands: &[],
        allow_unknown_subcommands: false,
        detail: "",
        hover: None,
    };
    let mut subcommands: Vec<SubCommand> = Vec::new();
    log.scoped(format!("body_scope command {name}"), |log| {
        for row in block(body) {
            match row.word_text(0) {
                "arity" => command.arity = parse_arity(&row, log).0,
                "detail" => command.detail = leak_str(row.word_text(1)),
                "allow_unknown_subcommands" => {
                    command.allow_unknown_subcommands = parse_flag(row.tail());
                }
                "hover" => {
                    if let Some(word) = row.arg(1) {
                        command.hover = Some(hover_block(&block(word), log));
                    }
                }
                "subcommand" => {
                    if let Some(sub) =
                        load_subcommand(&row, tables, hooks, "body_scope subcommand", log)
                    {
                        subcommands.push(sub);
                    }
                }
                _ => log.unknown_property(&row),
            }
        }
    });
    command.subcommands = leak_slice(subcommands);
    Some(command)
}

// ---------------------------------------------------------------------------
// Command loading
// ---------------------------------------------------------------------------

/// Everything a command body accumulates before the spec is sealed.
#[derive(Default)]
struct CommandAcc {
    args: ArgRows,
    arity_windows: Vec<ArityWindow>,
    options: Vec<OptionSpec>,
    forms: Vec<FormSpec>,
    side_effects: Vec<SideEffect>,
    subcommands: Vec<SubCommand>,
    manufacturers: Vec<ManufacturerMethod>,
    repeats: Vec<tcl_registry::repeated::RepeatedArgLayout>,
    option_relations: Vec<tcl_registry::spec::OptionRelation>,
    versioned_arg_values: Vec<tcl_registry::spec::VersionedArgValue>,
    setter_constraints: Vec<tcl_registry::taint::SetterConstraint>,
    oo_context_facts: Vec<(&'static str, tcl_registry::spec::OoContextFact)>,
    callback_taint_inputs: Vec<(u8, &'static [CallbackTaintInput])>,
    event_requirement_forms: Vec<EventRequirementForm>,
    hooks: Vec<HookDecl>,
    clause_grammar: Option<ClauseGrammar>,
}

/// Build one command: defaults, the body (delivered by `fill` from the
/// evaluation loader's staged nodes), sealing, containment, and §6.1
/// degradation.
///
/// Reads no statement of its own, so a command registered by a `foreach` and
/// one written out literally are built by exactly the same code from exactly
/// the same parts.
// Loading is a single tolerant transaction: defaults, hooks, projected rows,
// lifecycle containment, and degradation notices must seal together.
#[allow(clippy::too_many_lines)]
fn command_from_parts(
    name: &str,
    overrides_shipped: bool,
    line: u32,
    tables: &PackTables,
    log: &mut Log,
    fill: impl FnOnce(&mut CommandSpec, &mut CommandAcc, &mut Log),
) -> Option<PackCommand> {
    log.begin_spec();
    log.scoped(format!("command {name}"), |log| {
        let mut spec = CommandSpec {
            name: leak_str(name),
            dialects: tables.defaults.dialects,
            required_package: tables
                .defaults
                .required_package
                .or(tables.defaults.provides_package),
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
        fill(&mut spec, &mut acc, log);

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
                    line,
                    "a `clause_grammar` command should also declare the \
                     STRUCTURALLY_CHECKED_ARITY trait",
                );
            }
            if grammar.head.is_empty() {
                log.say(line, "a `clause_grammar` needs a `head` clause");
            }
        }

        let args = acc.args.seal();
        let callback_taint_inputs = validated_callback_taint_input_table(
            acc.callback_taint_inputs,
            &args.roles,
            spec.arg_role_resolver.is_some() || spec.command_prefix_resolver.is_some(),
            spec.script_timing_resolver.is_some() || spec.traits.contains(Traits::DEFERS_BODY),
            &format!("command `{}`", spec.name),
            line,
            log,
        );
        spec.arg_roles = leak_slice(args.roles);
        spec.arg_types = leak_slice(args.types);
        spec.arg_values = leak_slice(args.values);
        spec.closed_value_args = leak_slice(args.closed);
        spec.arg_presentation = leak_slice(args.presentation);
        spec.command_prefixes = leak_slice(args.prefixes);
        spec.callback_taint_inputs = leak_slice(callback_taint_inputs);
        spec.arity_windows = checked_arity_windows(acc.arity_windows, "command", line, log);
        spec.options = leak_slice(acc.options);
        spec.forms = leak_slice(acc.forms);
        spec.side_effects = leak_slice(acc.side_effects);
        spec.subcommands = leak_slice(acc.subcommands);
        spec.manufacturer_methods = leak_slice(acc.manufacturers);
        spec.repeated_args = leak_slice(acc.repeats);
        spec.option_relations = leak_slice(acc.option_relations);
        spec.versioned_arg_values = leak_slice(acc.versioned_arg_values);
        spec.setter_constraints = leak_slice(acc.setter_constraints);
        spec.oo_context_facts = leak_slice(acc.oo_context_facts);
        spec.event_requirement_forms = leak_slice(acc.event_requirement_forms);

        spec.lifecycle = checked_lifecycle(
            spec.lifecycle,
            &format!("command `{}`", spec.name),
            line,
            log,
        );
        check_command_containment(&spec, line, log);

        // §6.1's fail-closed rule. A semantic-class word this build cannot
        // read means the command's security / control-flow / binding facts
        // are incomplete, and analysing it anyway would publish confident
        // answers *because* the field was ignored — so the whole spec is
        // excluded. The weaker class keeps the command and marks it, so the
        // affected capability answers `Unknown` instead.
        if log.semantic_unknown {
            log.say_classified(
                line,
                VocabularyClass::Semantic,
                format!(
                    "`command {name}` is excluded from strong analysis: it uses \
                     semantic-class vocabulary this build does not speak"
                ),
            );
            return None;
        }

        Some(PackCommand {
            spec: leak_one(spec),
            overrides_shipped,
            hooks: acc.hooks,
            clause_grammar: acc.clause_grammar,
            degraded: log.assistance_unknown,
            line,
            file: std::path::PathBuf::new(),
        })
    })
}

/// Report every lifecycle in a finished command that reaches outside the
/// window of the declaration enclosing it.
///
/// Run once the whole body is read, because a `command` may state its own
/// releases *after* the rows they gate — which is also why every notice
/// carries the `command` statement's line rather than the row's.
fn check_command_containment(spec: &CommandSpec, line: u32, log: &mut Log) {
    let parent = spec.lifecycle;
    for window in spec.arity_windows {
        check_contained(
            &format!("arity window {:?}", window.arity),
            window.lifecycle,
            parent,
            line,
            log,
        );
    }
    for option in spec.options {
        check_contained(
            &format!("option `{}`", option.name),
            option.lifecycle,
            parent,
            line,
            log,
        );
    }
    for constraint in spec.option_relations {
        check_contained(
            &constraint.describe(),
            constraint.lifecycle,
            parent,
            line,
            log,
        );
    }
    for form in spec.forms {
        check_contained(
            &format!("form `{}`", form.synopsis),
            form.lifecycle,
            parent,
            line,
            log,
        );
    }
    for effect in spec.side_effects {
        check_contained(
            &format!("side_effect `{:?}`", effect.target),
            effect.lifecycle,
            parent,
            line,
            log,
        );
    }
    for (index, values) in spec.arg_values {
        for value in *values {
            check_contained(
                &format!("value `{}` of arg {index}", value.value),
                value.lifecycle,
                parent,
                line,
                log,
            );
        }
    }
    for gate in spec.versioned_arg_values {
        check_contained(
            &format!("versioned_arg_value `{}`", gate.value),
            gate.lifecycle,
            parent,
            line,
            log,
        );
    }
    for sub in spec.subcommands {
        check_contained(
            &format!("subcommand `{}`", sub.name),
            sub.lifecycle,
            parent,
            line,
            log,
        );
        check_subcommand_containment(sub, line, log);
    }
}

/// The same pass for one subcommand's own children.
fn check_subcommand_containment(sub: &SubCommand, line: u32, log: &mut Log) {
    let parent = sub.lifecycle;
    let path = sub.name;
    for window in sub.arity_windows {
        check_contained(
            &format!("subcommand `{path}` arity window {:?}", window.arity),
            window.lifecycle,
            parent,
            line,
            log,
        );
    }
    for option in sub.options {
        check_contained(
            &format!("subcommand `{path}` option `{}`", option.name),
            option.lifecycle,
            parent,
            line,
            log,
        );
    }
    for constraint in sub.option_relations {
        check_contained(
            &format!("subcommand `{path}` `{}`", constraint.describe()),
            constraint.lifecycle,
            parent,
            line,
            log,
        );
    }
    for effect in sub.side_effects {
        check_contained(
            &format!("subcommand `{path}` side_effect `{:?}`", effect.target),
            effect.lifecycle,
            parent,
            line,
            log,
        );
    }
    for row in sub.sub_subcommands {
        check_contained(
            &format!("sub_subcommand `{path} {}`", row.name),
            row.lifecycle,
            parent,
            line,
            log,
        );
    }
    for (index, values) in sub.arg_values {
        for value in *values {
            check_contained(
                &format!("subcommand `{path}` value `{}` of arg {index}", value.value),
                value.lifecycle,
                parent,
                line,
                log,
            );
        }
    }
    for gate in sub.versioned_arg_values {
        check_contained(
            &format!("subcommand `{path}` versioned_arg_value `{}`", gate.value),
            gate.lifecycle,
            parent,
            line,
            log,
        );
    }
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
        "available" => {
            log.v20(stmt.line, "available");
            let availability = available::from_statement(stmt, 1, log);
            if availability.dialects.is_some() {
                spec.dialects = availability.dialects;
            }
            match (availability.required_package, spec.required_package) {
                (Some(named), None) => spec.required_package = Some(named),
                (Some(named), Some(existing)) if named != existing => log.say(
                    stmt.line,
                    format!(
                        "`available {{package {named}}}` disagrees with `required_package \
                         {existing}`; the declared `required_package` is kept"
                    ),
                ),
                _ => {}
            }
        }
        "traits" => spec.traits |= parse_traits(&value, stmt.line, log),
        "arity" => match parse_arity(stmt, log) {
            (arity, None) => spec.arity = arity,
            (arity, Some(lifecycle)) => acc.arity_windows.push(ArityWindow { lifecycle, arity }),
        },
        "required_package" => spec.required_package = Some(leak_str(&value)),
        "tk_geometry" => {
            log.v12(stmt.line, "tk_geometry");
            let policy = match value.as_str() {
                "Exclusive" => tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive,
                "Independent" => tcl_registry::tk_geometry::TkGeometryContainerPolicy::Independent,
                other => {
                    log.say(
                        stmt.line,
                        format!("unknown Tk geometry container policy `{other}`"),
                    );
                    return;
                }
            };
            let mut container_option = None;
            let mut direct_form = false;
            let mut placement_subcommand = None;
            let mut release_subcommands: &'static [&'static str] = &[];
            let mut index = 2;
            while index < stmt.words.len() {
                match stmt.word_text(index) {
                    "-container-option" => {
                        index += 1;
                        if index < stmt.words.len() {
                            container_option = Some(leak_str(stmt.word_text(index)));
                        } else {
                            log.say(stmt.line, "`-container-option` needs an option name");
                        }
                    }
                    "-direct-form" => direct_form = true,
                    "-placement-subcommand" => {
                        index += 1;
                        if index < stmt.words.len() {
                            placement_subcommand = Some(leak_str(stmt.word_text(index)));
                        } else {
                            log.say(stmt.line, "`-placement-subcommand` needs a subcommand name");
                        }
                    }
                    "-release-subcommands" => {
                        index += 1;
                        if index < stmt.words.len() {
                            release_subcommands = leak_strs(&list_words(stmt.word_text(index)));
                        } else {
                            log.say(stmt.line, "`-release-subcommands` needs a list");
                        }
                    }
                    other => log.unknown_flag("tk_geometry", stmt.line, other),
                }
                index += 1;
            }
            spec.tk_geometry = Some(tcl_registry::tk_geometry::TkGeometryManagerSpec {
                container_policy: policy,
                container_option,
                direct_form,
                placement_subcommand,
                release_subcommands,
            });
        }
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
        // §6.2's honesty escape hatch (review B6): a provider whose member
        // set is runtime-extensible declares so instead of pretending
        // closure. Two ratified spellings, one fact — on a command the
        // fact is the existing open-subcommand-table flag; the
        // object-class form is the `-dynamic-surface` flag on
        // `object_class`.
        "dynamic_surface" | "unknown_members" => {
            log.v20(stmt.line, stmt.word_text(0));
            spec.allow_unknown_subcommands = parse_flag(stmt.tail());
        }
        "prefix_matching" => {
            const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];
            if let Some(mode) = enum_by_name(MATCHING, &value, "prefix matching", stmt.line, log) {
                spec.prefix_matching = mode;
            }
        }
        // E-R14: where the relation checker looks for this command's options.
        "option_placement" => {
            const PLACEMENTS: &[OptionPlacement] =
                &[OptionPlacement::Leading, OptionPlacement::Anywhere];
            log.v20(stmt.line, "option_placement");
            if let Some(placement) =
                enum_by_name(PLACEMENTS, &value, "option placement", stmt.line, log)
            {
                spec.option_placement = placement;
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
        // --- the ratified words (design §6.2, §6.3's blind spot) -----------
        "result_stability" => {
            if let Some(stability) = result_stability_row(stmt, log) {
                spec.result_stability = Some(stability);
            }
        }
        "side_switch_target" => {
            spec.side_switch_target = enum_by_name(
                SIDE_SWITCH_TARGETS,
                &value,
                "side-switch target",
                stmt.line,
                log,
            );
        }
        "event_handler_priority" => {
            if let Some(policy) = event_handler_priority_row(stmt, log) {
                spec.event_handler_priority = Some(policy);
            }
        }
        "data_collection" => {
            if let Some(operation) = data_collection_row(stmt, log) {
                spec.data_collection = Some(operation);
            }
        }
        "event_requirement_form" => {
            acc.event_requirement_forms
                .push(event_requirement_form_row(stmt, log));
        }
        "body_scope" => {
            if let Some(env) = body_scope_value(stmt, tables, &mut acc.hooks, log) {
                spec.body_scope = Some(env);
            }
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
        "callback_taint_inputs" => {
            log.v12(stmt.line, "callback_taint_inputs");
            acc.callback_taint_inputs = parse_callback_taint_input_table(&value, stmt.line, log);
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
        // The four E-R14 option-relation statements, one shared row parser.
        // `option_conflict` is the 1.x spelling and keeps its exact shape; the
        // other three are 2.0 vocabulary.
        "option_conflict" => {
            acc.option_relations.push(option_relation_row(
                stmt,
                tcl_registry::RelationKind::MutuallyExclusive,
                log,
            ));
        }
        "option_requires" | "option_requires_one_of" | "option_forbids" => {
            log.v20(stmt.line, &key);
            let kind = match key.as_str() {
                "option_requires" => tcl_registry::RelationKind::Requires,
                "option_requires_one_of" => tcl_registry::RelationKind::RequiresOneOf,
                _ => tcl_registry::RelationKind::Forbids,
            };
            acc.option_relations
                .push(option_relation_row(stmt, kind, log));
        }
        // The command-level mirror of the subcommand row, sharing its parser:
        // a literal value of one argument gated on the package's own axis.
        // The statement is 1.1 vocabulary at THIS scope (it was
        // subcommand-only in 1.0).
        "versioned_arg_value" => {
            log.v11(stmt.line, "versioned_arg_value");
            if let Some(gate) = versioned_arg_value_row(stmt, log) {
                acc.versioned_arg_values.push(gate);
            }
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
        | "script_timing_resolver"
        | "const_fold"
        | "const_fold_versioned"
        | "taint_sink_gate"
        | "context_gate"
        | "literal_argument_validator"
        | "constraints"
        | "clause_shape_check" => {
            if key == "script_timing_resolver" {
                log.v12(stmt.line, "script_timing_resolver");
            }
            if key == "constraints" {
                log.v20(stmt.line, "constraints");
            }
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
                "script_timing_resolver" => {
                    spec.script_timing_resolver = Some(abstain_script_timings);
                    ("script_timing_resolver", HookFamily::ScriptTimingResolver)
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
                // E-R14's escape hatch. The pre-binding placeholder reports
                // nothing, so a pack whose host never installs answers exactly
                // as a pack with no hook: silence.
                "constraints" => {
                    spec.constraints = Some(no_constraint_reports);
                    ("constraints", HookFamily::Constraints)
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
        ..FormSpec::DEFAULT
    };
    let words = &stmt.words;
    let mut i = 3;
    while i < words.len() {
        match words[i].text.as_str() {
            "-dialects" => {
                let text = next_text(words, &mut i);
                form.dialects = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut form.dialects,
                    availability,
                    "command form",
                    stmt.line,
                    log,
                );
            }
            other => {
                if lifecycle_flag(&mut form.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag("form", stmt.line, other);
                }
            }
        }
        i += 1;
    }
    form.lifecycle = checked_lifecycle(
        form.lifecycle,
        &format!("form `{}`", form.synopsis),
        stmt.line,
        log,
    );
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
        ..SideEffect::DEFAULT
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
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut effect.dialects,
                    availability,
                    "side effect",
                    stmt.line,
                    log,
                );
            }
            other => {
                if lifecycle_flag(&mut effect.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag("side_effect", stmt.line, other);
                }
            }
        }
        i += 1;
    }
    effect.lifecycle = checked_lifecycle(
        effect.lifecycle,
        &format!("side_effect `{}`", stmt.word_text(1)),
        stmt.line,
        log,
    );
    Some(effect)
}

/// One [`tcl_registry::OptionTerm`] as an author spells it.
///
/// A bare `-name` is the option; `{-name value}` pins its value; `{arg N}` is
/// the positional at `N` and `{arg N value}` pins its value. The four shapes
/// are the four things a real library's option table talks about, and each is
/// an ordinary Tcl list word — no mini-language (principle P-E).
fn relation_term(
    spelling: &str,
    statement: &str,
    line: u32,
    log: &mut Log,
) -> Option<tcl_registry::OptionTerm> {
    let words = list_words(spelling);
    match words.as_slice() {
        [name] if name.starts_with('-') => Some(tcl_registry::OptionTerm::Option(leak_str(name))),
        [name, value] if name.starts_with('-') => Some(tcl_registry::OptionTerm::OptionValue(
            leak_str(name),
            leak_str(value),
        )),
        [keyword, index] if keyword == "arg" => {
            index.parse().ok().map(tcl_registry::OptionTerm::Argument)
        }
        [keyword, index, value] if keyword == "arg" => index
            .parse()
            .ok()
            .map(|index| tcl_registry::OptionTerm::ArgumentValue(index, leak_str(value))),
        _ => None,
    }
    .or_else(|| {
        log.say(
            line,
            format!("`{statement}`: unreadable relation term `{spelling}` dropped"),
        );
        None
    })
}

/// Every term in a braced list, dropping the unreadable ones with a notice.
fn relation_terms(
    text: &str,
    statement: &str,
    line: u32,
    log: &mut Log,
) -> &'static [tcl_registry::OptionTerm] {
    let terms: Vec<tcl_registry::OptionTerm> = list_words(text)
        .iter()
        .filter_map(|spelling| relation_term(spelling, statement, line, log))
        .collect();
    leak_slice(terms)
}

/// The shared row parser behind all four option-relation statements.
///
/// `option_conflict {-a -b}` keeps its 1.x spelling exactly — the terms are
/// word 1 and there is no subject. The three E-R14 statements take the subject
/// as word 1 and the terms as word 2, with an empty subject (`{}`) making the
/// relation unconditional. Every flag (`-dialects`, `-available`, the
/// lifecycle trio) is shared, so an author learns one row and four verbs.
fn option_relation_row(
    stmt: &Stmt,
    kind: tcl_registry::RelationKind,
    log: &mut Log,
) -> tcl_registry::spec::OptionRelation {
    let statement = kind.statement_word();
    let (subject, terms_word, first_flag) = if kind == tcl_registry::RelationKind::MutuallyExclusive
    {
        (None, stmt.word_text(1).to_owned(), 2)
    } else {
        let spelling = stmt.word_text(1).to_owned();
        let subject = if spelling.trim().is_empty() {
            None
        } else {
            relation_term(&spelling, statement, stmt.line, log)
        };
        (subject, stmt.word_text(2).to_owned(), 3)
    };
    let mut relation = tcl_registry::spec::OptionRelation {
        kind,
        subject,
        terms: relation_terms(&terms_word, statement, stmt.line, log),
        ..tcl_registry::spec::OptionRelation::DEFAULT
    };
    let words = &stmt.words;
    let mut i = first_flag;
    while i < words.len() {
        match words[i].text.as_str() {
            "-dialects" => {
                let text = next_text(words, &mut i);
                relation.dialects = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut relation.dialects,
                    availability,
                    "option relation",
                    stmt.line,
                    log,
                );
            }
            // The library's own error text, quoted instead of generated.
            "-message" => {
                let text = next_text(words, &mut i);
                relation.message = Some(leak_str(&text));
            }
            other => {
                if lifecycle_flag(&mut relation.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag(statement, stmt.line, other);
                }
            }
        }
        i += 1;
    }
    relation.lifecycle =
        checked_lifecycle(relation.lifecycle, &relation.describe(), stmt.line, log);
    relation
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
    arity_windows: Vec<ArityWindow>,
    options: Vec<OptionSpec>,
    side_effects: Vec<SideEffect>,
    sub_subcommands: Vec<SubSubCommand>,
    repeats: Vec<tcl_registry::repeated::RepeatedArgLayout>,
    option_relations: Vec<tcl_registry::spec::OptionRelation>,
    versioned_arg_values: Vec<tcl_registry::spec::VersionedArgValue>,
    callback_taint_inputs: Vec<(u8, &'static [CallbackTaintInput])>,
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
    subcommand_from_parts(&name, kind, stmt.line, log, |sub, acc, log| {
        for stmt in block(body) {
            apply_subcommand_stmt(sub, acc, &stmt, tables, hooks, &name, log);
        }
    })
}

/// Everything [`load_subcommand`] does past reading the statement's own
/// words, shared with the evaluation loader exactly as
/// [`command_from_parts`] is.
fn subcommand_from_parts(
    name: &str,
    kind: &str,
    line: u32,
    log: &mut Log,
    fill: impl FnOnce(&mut SubCommand, &mut SubAcc, &mut Log),
) -> Option<SubCommand> {
    let outer = log.context.clone();
    log.scoped(format!("{outer} / {kind} {name}"), |log| {
        let mut sub = SubCommand {
            name: leak_str(name),
            ..SubCommand::DEFAULT
        };
        let mut acc = SubAcc::default();
        fill(&mut sub, &mut acc, log);
        let args = acc.args.seal();
        let callback_taint_inputs = validated_callback_taint_input_table(
            acc.callback_taint_inputs,
            &args.roles,
            sub.arg_role_resolver.is_some() || sub.command_prefix_resolver.is_some(),
            sub.script_timing_resolver.is_some() || sub.traits.contains(Traits::DEFERS_BODY),
            &format!("{kind} `{name}`"),
            line,
            log,
        );
        sub.arg_roles = leak_slice(args.roles);
        sub.arg_types = leak_slice(args.types);
        sub.arg_values = leak_slice(args.values);
        sub.closed_value_args = leak_slice(args.closed);
        sub.arg_presentation = leak_slice(args.presentation);
        sub.command_prefixes = leak_slice(args.prefixes);
        sub.callback_taint_inputs = leak_slice(callback_taint_inputs);
        sub.arity_windows = checked_arity_windows(acc.arity_windows, kind, line, log);
        sub.options = leak_slice(acc.options);
        sub.side_effects = leak_slice(acc.side_effects);
        sub.sub_subcommands = leak_slice(acc.sub_subcommands);
        sub.repeated_args = leak_slice(acc.repeats);
        sub.option_relations = leak_slice(acc.option_relations);
        sub.versioned_arg_values = leak_slice(acc.versioned_arg_values);
        sub.lifecycle = checked_lifecycle(sub.lifecycle, &format!("{kind} `{name}`"), line, log);
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
        "arity" => match parse_arity(stmt, log) {
            (arity, None) => sub.arity = arity,
            (arity, Some(lifecycle)) => acc.arity_windows.push(ArityWindow { lifecycle, arity }),
        },
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
        "available" => {
            log.v20(stmt.line, "available");
            let availability = available::from_statement(stmt, 1, log);
            apply_availability(
                &mut sub.dialects,
                availability,
                "subcommand",
                stmt.line,
                log,
            );
        }
        "safe_on_uninit" => sub.safe_on_uninit = parse_dialects(&value, stmt.line, log),
        "introduced_version" => sub.lifecycle.introduced = Some(leak_str(&value)),
        "deprecated_version" => sub.lifecycle.deprecated = Some(leak_str(&value)),
        "retired_version" => sub.lifecycle.retired = Some(leak_str(&value)),
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
        "option_placement" => {
            const PLACEMENTS: &[OptionPlacement] =
                &[OptionPlacement::Leading, OptionPlacement::Anywhere];
            log.v20(stmt.line, "option_placement");
            if let Some(placement) =
                enum_by_name(PLACEMENTS, &value, "option placement", stmt.line, log)
            {
                sub.option_placement = placement;
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
        "option_conflict" => acc.option_relations.push(option_relation_row(
            stmt,
            tcl_registry::RelationKind::MutuallyExclusive,
            log,
        )),
        "option_requires" | "option_requires_one_of" | "option_forbids" => {
            log.v20(stmt.line, &key);
            let kind = match key.as_str() {
                "option_requires" => tcl_registry::RelationKind::Requires,
                "option_requires_one_of" => tcl_registry::RelationKind::RequiresOneOf,
                _ => tcl_registry::RelationKind::Forbids,
            };
            acc.option_relations
                .push(option_relation_row(stmt, kind, log));
        }
        // The ratified word `SubCommand` carries in its own right: an
        // ensemble arm's result contract is not its command's.
        "result_stability" => {
            if let Some(stability) = result_stability_row(stmt, log) {
                sub.result_stability = Some(stability);
            }
        }
        "side_effect" => {
            if let Some(effect) = side_effect_row(stmt, log) {
                acc.side_effects.push(effect);
            }
        }
        "sub_subcommand" => acc
            .sub_subcommands
            .push(sub_subcommand_row(stmt, tables, log)),
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
        "callback_taint_inputs" => {
            log.v12(stmt.line, "callback_taint_inputs");
            acc.callback_taint_inputs = parse_callback_taint_input_table(&value, stmt.line, log);
        }
        "world_effects" => sub.world_effects = world_effects_value(stmt, tables, log),
        "state_transitions" => sub.state_transitions = state_transitions_value(stmt, tables, log),
        "arg_role_resolver"
        | "command_prefix_resolver"
        | "script_timing_resolver"
        | "const_fold"
        | "const_fold_versioned"
        | "constraints"
        | "literal_argument_validator" => {
            if key == "script_timing_resolver" {
                log.v12(stmt.line, "script_timing_resolver");
            }
            if key == "constraints" {
                log.v20(stmt.line, "constraints");
            }
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
                "script_timing_resolver" => {
                    sub.script_timing_resolver = Some(abstain_script_timings);
                    ("script_timing_resolver", HookFamily::ScriptTimingResolver)
                }
                "const_fold" => {
                    sub.const_fold = Some(abstain_const_fold);
                    ("const_fold", HookFamily::ConstFold)
                }
                "const_fold_versioned" => {
                    sub.const_fold_versioned = Some(abstain_const_fold_versioned);
                    ("const_fold_versioned", HookFamily::ConstFoldVersioned)
                }
                "constraints" => {
                    sub.constraints = Some(no_constraint_reports);
                    ("constraints", HookFamily::Constraints)
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

/// `sub_subcommand NAME ?flags? ?{ option … }?`.
///
/// The trailing block is optional and holds `option` rows: a second-level
/// operation whose option table genuinely differs from its siblings'
/// (`namespace ensemble create` has `-command`, `configure` has `-namespace`)
/// declares its own, and a consumer that can see the dispatch word prefers it
/// over the owning subcommand's (issue #1610).
///
/// **No block and an empty block mean different things.** No block leaves
/// `options` at `None` — this operation says nothing, so the owning
/// subcommand's table applies. An empty block `{}` sets `Some(&[])`: this
/// operation takes no options at all, and the parent's table must not leak
/// into it (`namespace ensemble exists`, whose C arm takes `cmdname` and
/// nothing else).
///
/// The block is recognised by being a **braced** word in a position where a
/// flag name was expected, which no flag value can be: every flag here is
/// `-name value`, so a value is only ever read through `next_text`.
fn sub_subcommand_row(stmt: &Stmt, tables: &PackTables, log: &mut Log) -> SubSubCommand {
    let mut row = SubSubCommand {
        name: leak_str(stmt.word_text(1)),
        ..SubSubCommand::DEFAULT
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
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut row.dialects,
                    availability,
                    "sub_subcommand",
                    stmt.line,
                    log,
                );
            }
            other if !other.starts_with('-') && words[i].braced => {
                log.v12(stmt.line, "an option block on `sub_subcommand`");
                // `Some`, even when the block is empty — that is the whole
                // point of writing one.
                row.options = Some(leak_slice(sub_subcommand_options(
                    &words[i], row.name, tables, log,
                )));
            }
            other => {
                if lifecycle_flag(&mut row.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag("sub_subcommand", stmt.line, other);
                }
            }
        }
        i += 1;
    }
    row.lifecycle = checked_lifecycle(
        row.lifecycle,
        &format!("sub_subcommand `{}`", row.name),
        stmt.line,
        log,
    );
    row
}

/// The `option` rows inside a `sub_subcommand NAME { … }` body.
///
/// An `-arity-hook` here is reported rather than dropped in silence: the hook
/// binder walks the command and its subcommands only, so a hook declared at
/// the second level would never be bound, and a pack that wrote one would get
/// a silently arity-hookless option. The option itself survives; only the hook
/// is refused.
fn sub_subcommand_options(
    body: &Word,
    owner: &str,
    tables: &PackTables,
    log: &mut Log,
) -> Vec<OptionSpec> {
    let mut options = Vec::new();
    for stmt in block(body) {
        if stmt.word_text(0) != "option" {
            log.say(
                stmt.line,
                format!(
                    "a `sub_subcommand` body holds `option` rows only; `{}` dropped",
                    stmt.word_text(0)
                ),
            );
            continue;
        }
        let (option, hook) = option_row(&stmt, tables, log);
        if hook.is_some() {
            log.say(
                stmt.line,
                format!(
                    "`-arity-hook` on `{}`'s option `{}` is not bindable at the \
                     second subcommand level; the option is kept, the hook is not",
                    owner, option.name
                ),
            );
        }
        options.push(option);
    }
    options
}

fn versioned_arg_value_row(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::spec::VersionedArgValue> {
    let index = stmt.word_text(1).parse().ok()?;
    let mut gate = tcl_registry::spec::VersionedArgValue {
        index,
        value: leak_str(stmt.word_text(2)),
        lifecycle: Lifecycle::UNSPECIFIED,
    };
    let words = &stmt.words;
    let mut i = 3;
    while i < words.len() {
        let flag = words[i].text.clone();
        if !lifecycle_flag(&mut gate.lifecycle, &flag, words, &mut i) {
            log.unknown_flag("versioned_arg_value", stmt.line, &flag);
        }
        i += 1;
    }
    gate.lifecycle = checked_lifecycle(
        gate.lifecycle,
        &format!("versioned_arg_value `{}`", gate.value),
        stmt.line,
        log,
    );
    Some(gate)
}

#[cfg(test)]
mod tests {
    use tcl_dialect::BracedVarStyle;
    use tcl_dialect::model::{BuildProfileId, Provenance, Release, WorldPolicy};

    use super::*;

    /// A pack can declare `DEFERS_BODY` for its own definer, and — the half
    /// that matters — a pack that says nothing leaves the bit **unset**.
    ///
    /// The flag tells a static walk that an unreadable body word costs it
    /// nothing about the call's completion (issue #1571), so the silent
    /// default has to be the abstaining one: unset means "this body may run
    /// here". Traits resolve by name against the registry's own flag list, so
    /// there is no loader table to keep in step — but there is also nothing
    /// stopping a future edit from inventing one, which is what this pins.
    #[test]
    fn a_pack_declares_defers_body_and_omitting_it_stays_abstaining() {
        let declared = evaluate_pack(
            "speclib probe 1.1 { command mydefiner { \
             arity 3; traits {DEFERS_BODY} } }",
        );
        assert!(
            declared
                .command("mydefiner")
                .unwrap()
                .spec
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY),
            "a pack must be able to declare its own definer dormant: {:?}",
            declared.notices
        );
        assert!(declared.notices.is_empty(), "{:?}", declared.notices);

        let silent = evaluate_pack("speclib probe 1.1 { command mydefiner { arity 3 } }");
        assert!(
            !silent
                .command("mydefiner")
                .unwrap()
                .spec
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY),
            "an undeclared body must stay material, never dormant-by-default"
        );
    }

    #[test]
    fn case_list_loader_preserves_every_clause_shape_field() {
        let pack = evaluate_pack(
            "speclib probe 1.1 { command demo { case_list { \
             two_arg_optionless_dialects tcl8.5+; \
             clause_end_options_flag --; clause_force_inline_flag -nobrace; \
             clause_force_list_flag -brace; clause_force_list_shape first_arg_only_remainder; \
             allow_omitted_final_body 1; \
             warn_unbraced_bodies 1 } } }",
        );
        let case = pack.command("demo").unwrap().spec.case_list.unwrap();
        assert_eq!(
            case.two_arg_optionless_dialects,
            Some(tcl_dialect::DialectSet::TCL85_PLUS)
        );
        assert_eq!(case.clause_end_options_flag, Some("--"));
        assert_eq!(case.clause_force_inline_flag, Some("-nobrace"));
        assert_eq!(case.clause_force_list_flag, Some("-brace"));
        assert_eq!(
            case.clause_force_list_shape,
            Some(tcl_registry::CaseForceListShape::FirstArgOnlyRemainder)
        );
        assert!(case.allow_omitted_final_body);
        assert!(case.warn_unbraced_bodies);
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);

        let disabled = evaluate_pack(
            "speclib probe 1.1 { command demo { case_list { \
             allow_omitted_final_body no; warn_unbraced_bodies false } } }",
        );
        let case = disabled.command("demo").unwrap().spec.case_list.unwrap();
        assert!(!case.allow_omitted_final_body);
        assert!(!case.warn_unbraced_bodies);
    }

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
                "variable scope",
                names(VARIABLE_SCOPES),
                catalogue::VARIABLE_SCOPES,
            ),
            (
                "argument presentation",
                names(PRESENTATIONS),
                catalogue::ARG_PRESENTATIONS,
            ),
            (
                "side-effect target",
                names(SIDE_EFFECT_TARGETS),
                catalogue::SIDE_EFFECT_TARGETS.as_slice(),
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
        let pack = evaluate_pack("command lonely { arity 1 }");
        assert!(pack.commands.is_empty());
        assert_eq!(pack.notices.len(), 1);
        assert!(pack.notices[0].message.contains("no `speclib`"));
    }

    #[test]
    fn statement_separation_is_ordinary_tcl() {
        // `;` separates, `;#` is a trailing comment, and a bare `#` mid-line is
        // NOT one — it becomes flags on the row, all dropped with a notice.
        let pack = evaluate_pack(
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
        let pack = evaluate_pack(
            "speclib probe 1.0 {\n\
             command demo { arg 300 -role Body\n arg 2 -role Body }\n\
             }",
        );
        let demo = pack.command("demo").expect("demo loads");
        assert_eq!(demo.spec.arg_roles, &[(2, ArgRole::Body)]);
        assert!(pack.notices.iter().any(|n| n.message.contains("above the")));
    }

    /// The frozen spelling, whole: `object_class NAME ?-superclass {…}?
    /// ?-allow-unknown? ?-method-prefix-matching Enabled|Strict?
    /// { method … }`, with `method` rows reusing the
    /// `subcommand` body grammar.
    #[test]
    fn an_object_class_carries_its_name_flags_and_method_table() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n\
             command factory {\n\
               arity 1..\n\
               object_class ::probe::Widget -superclass {::probe::Base ::probe::Mixin} \
                 -allow-unknown -method-prefix-matching Enabled {\n\
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
        assert_eq!(class.method_prefix_matching, PrefixMatching::Enabled);

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
        let pack = evaluate_pack(
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
        assert_eq!(class.method_prefix_matching, PrefixMatching::Strict);
    }

    /// A nameless class, an unknown flag, and a non-`method` member row are
    /// each a degradation with a notice, never a failure.
    #[test]
    fn a_malformed_object_class_degrades_with_a_notice() {
        let pack = evaluate_pack(
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

    /// Every level the registry can gate reads the same three flags, so one
    /// pack exercises the whole 1.1 vocabulary at once.
    #[test]
    fn every_gateable_row_reads_the_three_lifecycle_flags() {
        let pack = evaluate_pack(
            "speclib probe 1.1 {\n\
             values probe-codes {\n\
               value fast -detail {Quick path.} -min-tcl tcl8.6 -introduced 1.2 \
                 -deprecated 1.6 -retired 2.0\n\
             }\n\
             command demo {\n\
               introduced_version 1.0\n\
               form Default {demo ?-x? word} -introduced 1.1 -retired 2.0\n\
               side_effect FileIo -reads -introduced 1.1\n\
               option_conflict {-a -b} -introduced 1.1 -retired 2.0\n\
               arg 0 -values-from probe-codes\n\
               versioned_arg_value 0 fast -introduced 1.2 -retired 2.0\n\
               subcommand run {\n\
                 arity 1\n\
                 sub_subcommand now -detail {Right away.} -introduced 1.3 -deprecated 1.9\n\
               }\n\
             }\n\
             }",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let spec = pack.command("demo").expect("demo loads").spec;

        let form = spec.forms.first().expect("the form row loads");
        assert_eq!(form.lifecycle.introduced, Some("1.1"));
        assert_eq!(form.lifecycle.retired, Some("2.0"));

        let effect = spec.side_effects.first().expect("the side_effect loads");
        assert_eq!(effect.lifecycle.introduced, Some("1.1"));

        let constraint = spec
            .option_relations
            .first()
            .expect("the option_conflict loads");
        assert_eq!(constraint.lifecycle.introduced, Some("1.1"));
        assert_eq!(constraint.lifecycle.retired, Some("2.0"));

        let value = spec.arg_values[0].1[0];
        assert_eq!(value.min_tcl, Some(TclVersion::V8_6));
        assert_eq!(value.lifecycle.introduced, Some("1.2"));
        assert_eq!(value.lifecycle.deprecated, Some("1.6"));
        assert_eq!(value.lifecycle.retired, Some("2.0"));

        let gate = spec
            .versioned_arg_values
            .first()
            .expect("a command-level versioned_arg_value loads");
        assert_eq!((gate.index, gate.value), (0, "fast"));
        assert_eq!(gate.lifecycle.introduced, Some("1.2"));

        let row = spec.subcommands[0].sub_subcommands[0];
        assert_eq!(row.lifecycle.introduced, Some("1.3"));
        assert_eq!(row.lifecycle.deprecated, Some("1.9"));
    }

    /// The data form of the option's quick-fix hook, read by the same flag
    /// reader as the command-level `deprecation_fix` statement.
    #[test]
    fn an_option_row_reads_a_deprecation_fix_block() {
        let pack = evaluate_pack(
            "speclib probe 1.1 {\n\
             command demo {\n\
               option -old -deprecated 1.4 -deprecation-fix {-replace -new \
                 -description {Renamed in 1.4.} -safety SemanticsEquivalent}\n\
             }\n\
             }",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let option = &pack.command("demo").expect("demo loads").spec.options[0];
        assert_eq!(option.lifecycle.deprecated, Some("1.4"));
        match option.lifecycle.deprecation_fix {
            Some(DeprecationFixHook::ReplaceMatchedWord {
                replacement,
                description,
                safety,
            }) => {
                assert_eq!(replacement, "-new");
                assert_eq!(description, "Renamed in 1.4.");
                assert_eq!(safety, DeprecationFixSafety::SemanticsEquivalent);
            }
            other => panic!("expected a matched-word replacement, got {other:?}"),
        }
    }

    /// An impossible ordering drops the lifecycle and keeps the entity: a pack
    /// is never refused for a fact it got wrong.
    #[test]
    fn an_impossible_lifecycle_is_dropped_with_a_notice() {
        let pack = evaluate_pack(
            "speclib probe 1.1 {\n\
             command demo {\n\
               option -x -introduced 2.0 -retired 1.0\n\
               form Default {demo} -deprecated 2.0 -retired 1.0\n\
             }\n\
             }",
        );
        let spec = pack.command("demo").expect("demo still loads").spec;
        assert_eq!(spec.options[0].name, "-x");
        assert!(spec.options[0].lifecycle.is_unspecified());
        assert!(spec.forms[0].lifecycle.is_unspecified());
        let said = |needle: &str| {
            pack.notices
                .iter()
                .any(|notice| notice.message.contains(needle))
        };
        assert!(
            said(
                "option `-x`: retired release predates the introducing release; lifecycle dropped"
            ),
            "{:?}",
            pack.notices
        );
        assert!(
            said("retired release predates the deprecating release"),
            "{:?}",
            pack.notices
        );
    }

    /// Containment is notice-only: the declaration stays, and the author is
    /// told it claims availability its parent does not have.
    #[test]
    fn a_child_reaching_outside_its_parent_is_a_notice_not_a_drop() {
        let pack = evaluate_pack(
            "speclib probe 1.1 {\n\
             command demo {\n\
               introduced_version 1.5\n\
               retired_version 3.0\n\
               option -early -introduced 1.0\n\
               subcommand run { arity 1 ; retired_version 4.0 }\n\
             }\n\
             }",
        );
        let spec = pack.command("demo").expect("demo loads").spec;
        assert_eq!(spec.options[0].lifecycle.introduced, Some("1.0"));
        assert_eq!(spec.subcommands[0].lifecycle.retired, Some("4.0"));
        let said = |needle: &str| {
            pack.notices
                .iter()
                .any(|notice| notice.message.contains(needle))
        };
        assert!(
            said("option `-early`: introduced 1.0 reaches outside"),
            "{:?}",
            pack.notices
        );
        assert!(
            said("subcommand `run`: retired 4.0 reaches outside"),
            "{:?}",
            pack.notices
        );
    }

    /// `1`, `1.0`, `1.1`, `1.2` and `2.0` are all this vocabulary; an
    /// unknown *minor* loads with a notice rather than being refused.
    #[test]
    fn the_speclib_version_word_names_a_vocabulary_this_loader_knows() {
        for known in KNOWN_VOCABULARY_VERSIONS {
            let pack = evaluate_pack(&format!(
                "speclib probe {known} {{\n command demo {{ arity 1 }}\n}}"
            ));
            assert!(pack.notices.is_empty(), "{known}: {:?}", pack.notices);
            assert_eq!(&pack.dsl_version, known);
        }
        // An unknown minor within a supported major keeps loading maximally
        // (§6.1): a minor is only ever additive.
        let pack = evaluate_pack("speclib probe 2.9 {\n command demo { arity 1 }\n}");
        assert!(pack.command("demo").is_some(), "the pack still loads");
        assert_eq!(pack.dsl_version, "2.9");
        assert_eq!(pack.load_error, None);
        assert_eq!(
            pack.notices
                .iter()
                .map(|notice| notice.message.as_str())
                .collect::<Vec<_>>(),
            vec![
                "pack declares SpecTcl vocabulary 2.9; this loader knows 2.0 — \
                 newer words may be dropped"
            ]
        );
        // A number *below* every vocabulary this loader knows never named a
        // vocabulary at all — the classic slip is writing the library's own
        // release in the slot — so the notice points at `introduced_version`
        // instead of claiming the loader is behind.
        let pack = evaluate_pack("speclib probe 0.15 {\n command demo { arity 1 }\n}");
        assert!(pack.command("demo").is_some(), "the pack still loads");
        assert_eq!(
            pack.notices
                .iter()
                .map(|notice| notice.message.as_str())
                .collect::<Vec<_>>(),
            vec![
                "`0.15` is not a SpecTcl vocabulary version (this loader knows \
                 2.0); if it is the library's own version, it belongs in \
                 `introduced_version`, not the `speclib` slot"
            ]
        );
    }

    /// A 1.1-only word under a 1.0 declaration draws one notice per site:
    /// this loader reads the word fine (additions never gate), but a
    /// genuinely-1.0 loader drops it silently, so the declaration must say
    /// 1.1. Declaring 1.1 clears every notice; the option row's lifecycle
    /// flags stay 1.0 vocabulary and never trip it.
    #[test]
    fn v11_words_under_a_10_declaration_draw_a_per_site_notice() {
        let body = "\n command demo {\n                        arity 1\n                        option -x -introduced 1.2\n                        form Default {demo word} -introduced 1.2\n                        side_effect FileIo -writes -deprecated 2.0\n                        versioned_arg_value 0 utf-8 -introduced 1.2\n                    }\n}";

        let pack = evaluate_pack(&format!("speclib probe 1.0 {{{body}"));
        let v11_notices: Vec<&str> = pack
            .notices
            .iter()
            .filter(|n| n.message.contains("SpecTcl 1.1 vocabulary"))
            .map(|n| n.message.as_str())
            .collect();
        // form -introduced, side_effect -deprecated, and the command-scope
        // versioned_arg_value statement; the option row's -introduced is 1.0
        // vocabulary and stays silent.
        assert_eq!(v11_notices.len(), 3, "{:?}", pack.notices);
        assert!(
            v11_notices
                .iter()
                .all(|m| m.contains("declare `speclib probe 1.1`")),
            "{v11_notices:?}"
        );
        assert!(
            !v11_notices.iter().any(|m| m.contains("`-x`")),
            "option-row lifecycle is 1.0 vocabulary: {v11_notices:?}"
        );

        let pack = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
        assert!(
            pack.notices.is_empty(),
            "declaring 1.1 clears it: {:?}",
            pack.notices
        );
    }

    /// The all-versions contract: the same pack body loads to the identical
    /// command surface under every vocabulary this loader knows — a pack is
    /// never refused, and no known version reads fewer words than another.
    #[test]
    fn every_known_vocabulary_loads_the_same_command_surface() {
        let body = "{\n command demo {\n                        arity 1..\n                        option -mode -takes mode -values {a b} -closed\n                        form Default {demo ?-mode mode? word}\n                        hover { summary {Demo.} synopsis {demo word} }\n                    }\n command other { arity 0 }\n}";
        let baseline = evaluate_pack(&format!("speclib probe 1.2 {body}"));
        let baseline_names: Vec<&str> = baseline.commands.iter().map(|c| c.spec.name).collect();
        for known in ["1", "1.0", "1.1"] {
            let pack = evaluate_pack(&format!("speclib probe {known} {body}"));
            let names: Vec<&str> = pack.commands.iter().map(|c| c.spec.name).collect();
            assert_eq!(names, baseline_names, "{known}");
            assert!(pack.notices.is_empty(), "{known}: {:?}", pack.notices);
            let demo = pack.command("demo").expect("demo loads");
            assert_eq!(demo.spec.options.len(), 1, "{known}");
        }
    }

    #[test]
    fn option_external_input_link_round_trips_as_taint_metadata() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command widget {\n \
             arity 1..\n \
             option -textvariable -takes variable -role VarWrite \
                 -also-role VarRead -taints-var-write -variable-scope Global\n \
             }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let spec = &pack.command("widget").expect("widget loads").spec;
        let option = spec
            .find_option("-textvariable", None, None)
            .expect("linked option loads");
        assert!(option.taints_var_write());
        assert_eq!(option.value_role(), Some(ArgRole::VarWrite));
        assert_eq!(option.value_also_role(), Some(ArgRole::VarRead));
        assert_eq!(option.value_variable_scope(), Some(VariableScope::Global));
    }

    #[test]
    fn option_script_timing_is_loaded_independently_of_body_scope() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command widget {\n \
             option -command -takes script -role Body -body-kind Structural \
                 -script-timing Deferred\n \
             option -body -takes script -role Body -body-kind Structural \
                 -script-timing SameInvocation\n \
             option -remove -takes command-prefix -role CommandPrefix \
                 -script-timing ReferenceOnly\n \
             }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let spec = &pack.command("widget").expect("widget loads").spec;
        let callback = spec
            .find_option("-command", None, None)
            .expect("callback option loads");
        let immediate = spec
            .find_option("-body", None, None)
            .expect("immediate body option loads");
        let reference = spec
            .find_option("-remove", None, None)
            .expect("reference-only option loads");
        assert_eq!(callback.value_script_timing(), Some(ScriptTiming::Deferred));
        assert_eq!(
            immediate.value_script_timing(),
            Some(ScriptTiming::SameInvocation)
        );
        assert_eq!(
            reference.value_script_timing(),
            Some(ScriptTiming::ReferenceOnly)
        );
        let OptionValue::Takes(callback_arg) = callback.value else {
            panic!("callback takes a value")
        };
        let OptionValue::Takes(immediate_arg) = immediate.value else {
            panic!("body takes a value")
        };
        let OptionValue::Takes(reference_arg) = reference.value else {
            panic!("removal reference takes a value")
        };
        assert_eq!(callback_arg.body_kind, BodyKind::Structural);
        assert_eq!(immediate_arg.body_kind, BodyKind::Structural);
        assert_eq!(reference_arg.role, ArgRole::CommandPrefix);
    }

    #[test]
    fn callback_taint_inputs_are_authorable_but_reject_bookkeeping_markers() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command widget {\n \
             traits DEFERS_BODY\n \
             arity 2\n \
             arg 1 -role Body\n \
             callback_taint_inputs {{1 {%A %K %W}}}\n \
             option -validatecommand -takes script -role Body -body-kind Structural \
                 -script-timing Deferred -callback-taint-inputs {%P %s %S %V}\n \
             }\n}",
        );
        let spec = &pack.command("widget").expect("widget loads").spec;
        assert_eq!(spec.callback_taint_inputs[0].0, 1);
        assert_eq!(
            spec.callback_taint_inputs[0].1,
            &[
                CallbackTaintInput::TK_EVENT_CHAR,
                CallbackTaintInput::TK_EVENT_KEYSYM,
            ]
        );
        let option = spec
            .find_option("-validatecommand", None, None)
            .expect("callback option loads");
        assert_eq!(
            option.value_callback_taint_inputs(),
            &[
                CallbackTaintInput::TK_PROPOSED_VALUE,
                CallbackTaintInput::TK_CURRENT_VALUE,
                CallbackTaintInput::TK_EDIT_TEXT,
            ]
        );
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains("%W"))
                && pack
                    .notices
                    .iter()
                    .any(|notice| notice.message.contains("%V")),
            "metadata substitutions must be rejected: {:?}",
            pack.notices
        );
    }

    #[test]
    fn contradictory_option_and_callback_properties_are_not_installed() {
        let pack = evaluate_pack(
            r"speclib probe 1.2 {
 command bad {
   arg 0 -role Value
   callback_taint_inputs {{0 {%A}}}
   option -timing -takes value -role Value -script-timing Deferred
   option -input -takes script -role Body -script-timing SameInvocation \
       -callback-taint-inputs {%P}
   option -scope -takes value -role Value -variable-scope Global
   option -taint -takes variable -role VarRead -taints-var-write
   subcommand child {
     arg 0 -role Body
     callback_taint_inputs {{0 {%S}}}
   }
 }
}",
        );
        let spec = &pack.command("bad").expect("command survives").spec;
        assert!(spec.callback_taint_inputs.is_empty());
        let OptionValue::Takes(timing) = spec.options[0].value else {
            panic!("option takes a value")
        };
        assert_eq!(timing.script_timing, ScriptTiming::SameInvocation);
        let OptionValue::Takes(input) = spec.options[1].value else {
            panic!("option takes a value")
        };
        assert!(input.callback_taint_inputs.is_empty());
        let OptionValue::Takes(scope) = spec.options[2].value else {
            panic!("option takes a value")
        };
        assert_eq!(scope.variable_scope, VariableScope::CurrentFrame);
        let OptionValue::Takes(taint) = spec.options[3].value else {
            panic!("option takes a value")
        };
        assert!(!taint.taints_var_write);
        assert!(spec.subcommands[0].callback_taint_inputs.is_empty());

        for fragment in [
            "non-executable value",
            "not a deferred executable",
            "non-variable value",
            "without a VarWrite role",
            "argument 0, which is not a deferred executable position",
        ] {
            assert!(
                pack.notices
                    .iter()
                    .any(|notice| notice.message.contains(fragment)),
                "missing `{fragment}` notice: {:?}",
                pack.notices
            );
        }
    }

    #[test]
    fn tk_geometry_form_and_release_metadata_loads() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command layout {\n \
             tk_geometry Exclusive -container-option -inside -direct-form \
                 -placement-subcommand arrange \
                 -release-subcommands {release unmanage}\n \
             }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let geometry = pack
            .command("layout")
            .expect("layout loads")
            .spec
            .tk_geometry
            .expect("geometry loads");
        assert_eq!(
            geometry.container_policy,
            tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
        );
        assert_eq!(geometry.container_option, Some("-inside"));
        assert!(geometry.direct_form);
        assert_eq!(geometry.placement_subcommand, Some("arrange"));
        assert_eq!(geometry.release_subcommands, ["release", "unmanage"]);
    }

    /// The 1.2 words go through the *same* per-site mechanism the 1.1 words
    /// do, which is the point of generalising it: a 1.2 word is a defect
    /// under a 1.1 declaration exactly as a 1.1 word is under 1.0, and the
    /// notice names the version that actually has the word rather than a
    /// hard-coded one.
    #[test]
    fn v12_words_under_an_older_declaration_draw_a_per_site_notice() {
        // Two 1.2 sites: the arity window's lifecycle flag and `tk_geometry`.
        // (The per-argument lifecycle that used to be the second site is gone
        // with the `arg_rows` machinery — redesign §11.1 O2.)
        let body = "\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0\n \
             arg 0 -role Body\n \
             tk_geometry Exclusive\n \
             option -x -introduced 1.2\n \
             }\n}";

        // Under 1.1, only the 1.2 words are noticed — the option row's
        // lifecycle flags have been 1.0 vocabulary all along.
        let pack = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
        let messages: Vec<&str> = pack.notices.iter().map(|n| n.message.as_str()).collect();
        assert_eq!(
            messages.len(),
            2,
            "one per 1.2 site, and nothing else: {messages:?}"
        );
        assert!(
            messages
                .iter()
                .all(|m| m.contains("is SpecTcl 1.2 vocabulary")
                    && m.contains("declare `speclib probe 1.2`")),
            "{messages:?}"
        );

        // Under 1.0 the same two sites are still the only 1.2 ones, and the
        // older mechanism keeps working beside it.
        let older = evaluate_pack(&format!("speclib probe 1.0 {{{body}"));
        assert_eq!(
            older
                .notices
                .iter()
                .filter(|n| n.message.contains("SpecTcl 1.2 vocabulary"))
                .count(),
            2,
            "{:?}",
            older.notices
        );

        // Declaring 1.2 clears them.
        let declared = evaluate_pack(&format!("speclib probe 1.2 {{{body}"));
        assert!(
            declared.notices.is_empty(),
            "declaring 1.2 clears it: {:?}",
            declared.notices
        );
    }

    #[test]
    fn callback_and_timing_vocabulary_reports_every_pre_12_site() {
        let body = r"
 command demo {
   traits DEFERS_BODY
   arity 2
   arg 1 -role Body
   callback_taint_inputs {{1 {%A %K}}}
   script_timing_resolver {words ctx} { timing 1 Deferred }
   option -validatecommand -takes script -role Body -script-timing Deferred \
       -callback-taint-inputs {%P %s %S}
   subcommand child {
     traits DEFERS_BODY
     arity 2
     arg 1 -role Body
     callback_taint_inputs {{1 {%A}}}
     script_timing_resolver {words ctx} { timing 1 Deferred }
   }
 }
}";

        let older = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
        let vocabulary: Vec<&str> = older
            .notices
            .iter()
            .filter(|notice| notice.message.contains("SpecTcl 1.2 vocabulary"))
            .map(|notice| notice.message.as_str())
            .collect();
        for (word, expected) in [
            ("`callback_taint_inputs`", 2),
            ("`script_timing_resolver`", 2),
            ("`-script-timing`", 1),
            ("`-callback-taint-inputs`", 1),
        ] {
            assert_eq!(
                vocabulary
                    .iter()
                    .filter(|message| message.contains(word))
                    .count(),
                expected,
                "missing per-site notice for {word}: {:?}",
                older.notices
            );
        }
        assert_eq!(
            vocabulary.len(),
            6,
            "only the six 1.2 sites should be reported: {:?}",
            older.notices
        );

        let declared = evaluate_pack(&format!("speclib probe 1.2 {{{body}"));
        assert!(
            declared
                .notices
                .iter()
                .all(|notice| !notice.message.contains("SpecTcl 1.2 vocabulary")),
            "declaring 1.2 clears every vocabulary notice: {:?}",
            declared.notices
        );
    }

    /// An `arity` row without lifecycle flags is the command's plain arity,
    /// exactly as before 1.2; one *with* them is a window, and the two
    /// coexist — the plain row stays the fallback for a floor no window
    /// covers.
    ///
    /// Consecutive windows are written **closed**: a window with no
    /// `-retired` never ends, so "two arguments from 3.0, three from 5.0" is
    /// spelled with the first retiring where the second begins. Leaving it
    /// open would overlap, which is what
    /// `overlapping_arity_windows_draw_a_notice_and_the_later_one_is_dropped`
    /// covers.
    #[test]
    fn a_gated_arity_row_becomes_a_window_and_leaves_the_plain_arity_alone() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0 -retired 5.0\n \
             arity 3 -introduced 5.0\n \
             }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let spec = pack.command("demo").expect("demo loads").spec;
        assert_eq!(spec.arity, Arity::exact(1), "the ungated row is the arity");
        assert_eq!(spec.arity_windows.len(), 2, "{:?}", spec.arity_windows);
        assert_eq!(spec.arity_windows[0].arity, Arity::exact(2));
        assert_eq!(spec.arity_windows[0].lifecycle.introduced, Some("3.0"));
        assert_eq!(spec.arity_windows[1].arity, Arity::exact(3));

        // The selection contract the windows exist for.
        assert_eq!(
            ArityWindow::select(spec.arity_windows, Some("3.0")).map(|w| w.arity),
            Some(Arity::exact(2))
        );
        assert_eq!(
            ArityWindow::select(spec.arity_windows, Some("5.0")).map(|w| w.arity),
            Some(Arity::exact(3))
        );
        assert_eq!(
            ArityWindow::select(spec.arity_windows, Some("1.0")),
            None,
            "below every window, the plain arity is the fallback"
        );
    }

    /// Two windows covering one release would make the selected signature
    /// depend on declaration order. The pack keeps the first and is told.
    #[test]
    fn overlapping_arity_windows_draw_a_notice_and_the_later_one_is_dropped() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0\n \
             arity 9 -introduced 4.0\n \
             }\n}",
        );
        let spec = pack.command("demo").expect("demo loads").spec;
        assert_eq!(spec.arity_windows.len(), 1, "{:?}", spec.arity_windows);
        assert_eq!(spec.arity_windows[0].arity, Arity::exact(2), "the first");
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("overlaps") && n.message.contains("dropped")),
            "{:?}",
            pack.notices
        );
    }

    /// Windows and gated argument rows join the containment pass every other
    /// lifecycle already goes through: a row that outlives the command
    /// declaring it is a pack defect the author can only find if told.
    #[test]
    fn a_window_reaching_outside_the_command_lifecycle_is_noticed() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command demo {\n \
             introduced_version 2.0\n \
             retired_version 4.0\n \
             arity 1\n \
             arity 2 -introduced 5.0\n \
             }\n}",
        );
        let said = |needle: &str| pack.notices.iter().any(|n| n.message.contains(needle));
        assert!(said("arity window"), "{:?}", pack.notices);
    }

    /// An impossibly-ordered window lifecycle comes back UNSPECIFIED, which
    /// as a window would cover every release and shadow the well-formed ones.
    /// The row keeps its shape and loses only the gate — the same degradation
    /// every other rejected lifecycle takes.
    #[test]
    fn an_arity_window_with_an_impossible_lifecycle_falls_back_to_the_plain_arity() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command demo {\n \
             arity 7 -introduced 5.0 -retired 2.0\n \
             }\n}",
        );
        let spec = pack.command("demo").expect("demo loads").spec;
        assert!(
            spec.arity_windows.is_empty(),
            "a rejected gate must not become an always-on window: {:?}",
            spec.arity_windows
        );
        assert_eq!(spec.arity, Arity::exact(7), "the shape survives");
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("arity window")),
            "{:?}",
            pack.notices
        );
    }

    /// `ambient_package NAME VERSION` names a package the pack's dialect
    /// provides without a `package require`. Both words are required: a row
    /// with no version would floor at nothing, which is the situation the row
    /// exists to end.
    #[test]
    fn ambient_package_rows_load_and_an_incomplete_one_is_dropped() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n \
             ambient_package Tk 8.6\n \
             ambient_package Itcl 4.0\n \
             command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let named: Vec<(&str, &str)> = pack
            .ambient_packages
            .iter()
            .map(|row| (row.name, row.version))
            .collect();
        assert_eq!(named, vec![("Tk", "8.6"), ("Itcl", "4.0")]);

        let missing = evaluate_pack(
            "speclib probe 1.2 {\n \
             ambient_package Tk\n \
             command demo { arity 1 }\n}",
        );
        assert!(
            missing.ambient_packages.is_empty(),
            "a version-less row is dropped: {:?}",
            missing.ambient_packages
        );
        assert!(
            missing
                .notices
                .iter()
                .any(|n| n.message.contains("needs the version the runtime provides")),
            "{:?}",
            missing.notices
        );

        // 1.2 vocabulary: the same per-site notice every other new word gets.
        let older = evaluate_pack(
            "speclib probe 1.1 {\n \
             ambient_package Tk 8.6\n \
             command demo { arity 1 }\n}",
        );
        assert_eq!(
            older.ambient_packages.len(),
            1,
            "the word still loads — additions never gate"
        );
        assert!(
            older.notices.iter().any(|n| n
                .message
                .contains("`ambient_package` is SpecTcl 1.2 vocabulary")),
            "{:?}",
            older.notices
        );
    }

    /// `display_name` and `file_extension` are pack-level metadata: names
    /// for humans, extensions for editors, and `-dialect` routing that must
    /// resolve to a canonical profile — a typo keeps the row and drops only
    /// the routing, with a notice.
    #[test]
    fn display_name_and_file_extension_rows_load() {
        let pack = evaluate_pack(
            "speclib upfdemo 1.1 {\n             display_name {IEEE 1801 UPF}\n             file_extension .UPF -name {Unified Power Format} -dialect synopsys-eda-tcl\n             file_extension pwr -dialect no-such-dialect\n             file_extension upf -name {duplicate}\n             command demo { arity 1 }\n}",
        );
        assert_eq!(pack.display_name.as_deref(), Some("IEEE 1801 UPF"));
        assert_eq!(pack.file_extensions.len(), 2, "{:?}", pack.file_extensions);
        let upf = &pack.file_extensions[0];
        assert_eq!(upf.extension, "upf", "lower-cased, dot stripped");
        assert_eq!(upf.display_name.as_deref(), Some("Unified Power Format"));
        assert_eq!(upf.dialect, Some("synopsys-eda-tcl"));
        let pwr = &pack.file_extensions[1];
        assert_eq!(pwr.extension, "pwr");
        assert_eq!(pwr.dialect, None, "typo'd dialect drops only the routing");
        let messages: Vec<&str> = pack.notices.iter().map(|n| n.message.as_str()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("`-dialect no-such-dialect` names no dialect profile")),
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("`file_extension upf` redeclared")),
            "{messages:?}"
        );
    }
    /// The 2.0 round trip: the same body spelled `dialects` and spelled
    /// `available` loads to byte-equal specs.
    ///
    /// This is the whole of §6.1's "a new word plus a translation of the
    /// legacy word": if the two spellings could ever diverge, `dialects`
    /// would be a second representation rather than a translation, and the
    /// upgrade tool's U9 byte-identical registry snapshot could not hold.
    #[test]
    fn available_and_dialects_load_byte_equal_specs() {
        // (legacy dialect word, 2.0 available row) — the whole U2 table
        // this build can express on the 1.x fields.
        let pairs = [
            ("tcl8.5", "{tcl 8.5}"),
            ("tcl8.6+", "{tcl 8.6-}"),
            ("tcl8.4+", "{tcl 8.4-}"),
            ("all-tcl", "{tcl 8.4-}"),
            ("tcl8.x", "{tcl 8.4-9.0}"),
            ("f5-irules", "{f5-irules}"),
            ("tk", "{package Tk}"),
            ("{tcl8.6 tcl9.0}", "{tcl 8.6} {tcl 9.0}"),
            ("{tcl8.6+ f5-irules}", "{tcl 8.6-} {f5-irules}"),
        ];
        for (legacy, modern) in pairs {
            let old = evaluate_pack(&format!(
                "speclib probe 1.2 {{\n command demo {{\n arity 1\n dialects {legacy}\n \
                 subcommand sub {{\n arity 0\n dialects {legacy}\n }}\n \
                 option -x -dialects {legacy}\n }}\n}}"
            ));
            let new = evaluate_pack(&format!(
                "speclib probe 2.0 {{\n command demo {{\n arity 1\n available {modern}\n \
                 subcommand sub {{\n arity 0\n available {modern}\n }}\n \
                 option -x -available {{{modern}}}\n }}\n}}"
            ));
            assert!(old.notices.is_empty(), "{legacy}: {:?}", old.notices);
            assert!(new.notices.is_empty(), "{modern}: {:?}", new.notices);
            let old_spec = old.command("demo").expect("legacy demo loads").spec;
            let new_spec = new.command("demo").expect("2.0 demo loads").spec;
            assert_eq!(old_spec.dialects, new_spec.dialects, "{legacy} vs {modern}");
            assert_eq!(
                old_spec.subcommands[0].dialects, new_spec.subcommands[0].dialects,
                "{legacy} vs {modern}"
            );
            assert_eq!(
                old_spec.options[0].dialects, new_spec.options[0].dialects,
                "{legacy} vs {modern}"
            );
            assert_eq!(
                format!("{old_spec:?}"),
                format!("{new_spec:?}"),
                "{legacy} vs {modern}"
            );
        }
    }

    /// `available` is accepted at every scope the legacy word is, and the
    /// projection is identical at each.
    #[test]
    fn available_lands_at_every_dialects_scope() {
        let legacy = evaluate_pack(
            "speclib probe 1.2 {\n default dialects tcl8.6+\n \
             command demo {\n arity 1..\n \
             option -x -dialects tcl8.6+\n \
             form Default {demo word} -dialects tcl8.6+\n \
             side_effect FileIo -writes -dialects tcl8.6+\n \
             option_conflict {-x -y} -dialects tcl8.6+\n \
             subcommand sub {\n arity 0\n dialects tcl8.6+\n \
             sub_subcommand deep -dialects tcl8.6+\n }\n }\n}",
        );
        let modern = evaluate_pack(
            "speclib probe 2.0 {\n default available {tcl 8.6-}\n \
             command demo {\n arity 1..\n \
             option -x -available {tcl 8.6-}\n \
             form Default {demo word} -available {tcl 8.6-}\n \
             side_effect FileIo -writes -available {tcl 8.6-}\n \
             option_conflict {-x -y} -available {tcl 8.6-}\n \
             subcommand sub {\n arity 0\n available {tcl 8.6-}\n \
             sub_subcommand deep -available {tcl 8.6-}\n }\n }\n}",
        );
        assert!(legacy.notices.is_empty(), "{:?}", legacy.notices);
        assert!(modern.notices.is_empty(), "{:?}", modern.notices);
        let legacy_spec = legacy.command("demo").expect("legacy demo loads").spec;
        let modern_spec = modern.command("demo").expect("2.0 demo loads").spec;
        assert_eq!(
            format!("{legacy_spec:?}"),
            format!("{modern_spec:?}"),
            "every scope projects alike"
        );
        assert_eq!(modern_spec.dialects, Some(DialectSet::TCL86_PLUS));
    }

    /// `available {package NAME}` fills `required_package` at command
    /// scope, and must agree with a `required_package` the spec declares.
    #[test]
    fn available_carries_a_package_requirement_at_command_scope() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {package Tcllib}\n }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert_eq!(
            pack.command("demo")
                .expect("demo loads")
                .spec
                .required_package,
            Some("Tcllib")
        );

        let pack = evaluate_pack(
            "speclib probe 2.0 {\n command demo {\n arity 1\n \
             required_package Snit\n available {package Tcllib}\n }\n}",
        );
        assert_eq!(
            pack.command("demo")
                .expect("demo loads")
                .spec
                .required_package,
            Some("Snit"),
            "the declared requirement wins"
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("disagrees with `required_package Snit`")),
            "{:?}",
            pack.notices
        );
    }

    /// Q3: `f5-bigip` leaves the Tcl axis, so it is never an `available`
    /// provider — and an unknown provider is reported, not guessed at.
    #[test]
    fn available_refuses_f5_bigip_and_unknown_providers() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {f5-bigip}\n }\n}",
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("off the Tcl axis")),
            "{:?}",
            pack.notices
        );
        assert_eq!(
            pack.command("demo")
                .expect("demo still loads")
                .spec
                .dialects,
            None,
            "a dropped row narrows nothing"
        );

        let pack = evaluate_pack(
            "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {klingon 1.0}\n }\n}",
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("unknown `available` provider `klingon`")),
            "{:?}",
            pack.notices
        );
    }

    /// Jim has no 1.x dialect bit, so a Jim-only window gates the command
    /// off rather than reading as "available everywhere".
    #[test]
    fn a_jim_only_window_narrows_to_nothing_rather_than_widening() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {jim 0.78-}\n }\n}",
        );
        assert_eq!(
            pack.command("demo").expect("demo loads").spec.dialects,
            Some(DialectSet::empty())
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("no SpecTcl 1.x dialect bit")),
            "{:?}",
            pack.notices
        );
    }

    /// A 2.0 word under a 1.x declaration draws the same per-site notice
    /// every newer word does.
    #[test]
    fn v20_words_under_an_older_declaration_draw_a_per_site_notice() {
        let pack = evaluate_pack(
            "speclib probe 1.2 {\n command demo {\n arity 1\n \
             available {tcl 8.6-}\n }\n}",
        );
        assert!(
            pack.notices.iter().any(|n| {
                n.message.contains("`available` is SpecTcl 2.0 vocabulary")
                    && n.message.contains("declare `speclib probe 2.0`")
            }),
            "{:?}",
            pack.notices
        );
    }

    /// The seven ratified words the loader had no reader for (§6.2's list,
    /// §6.3's `DraftOpaque`-masks-`LoaderGap` blind spot): each lands on the
    /// `CommandSpec` field the design memo's coverage matrix names for it.
    #[test]
    fn the_ratified_words_reach_their_model_fields() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n\
             command probe::collect {\n\
             \x20  arity 0\n\
             \x20  result_stability Volatile\n\
             \x20  data_collection -native HTTP_COLLECT\n\
             \x20  side_switch_target Server\n\
             \x20  event_handler_priority -default 500 -min 0 -max 1000 -warn-implicit\n\
             \x20  event_requirement_form {append} -only-in {HTTP_REQUEST} {\n\
             \x20      client_side yes\n\
             \x20  }\n\
             }\n\
             command probe::read {\n\
             \x20  arity 1\n\
             \x20  result_stability {ReadsVersionedWorld {CommandBindings NamespaceLookup}}\n\
             \x20  body_scope {\n\
             \x20      name {probe body}\n\
             \x20      include_sibling_definitions\n\
             \x20      allow_unknown_commands no\n\
             \x20      command top {\n\
             \x20          arity 1..2\n\
             \x20          detail {the top row}\n\
             \x20          subcommand set { arity 1 }\n\
             \x20      }\n\
             \x20  }\n\
             \x20  subcommand line {\n\
             \x20      arity 0\n\
             \x20      result_stability ReferentiallyTransparent\n\
             \x20  }\n\
             }\n\
             }\n",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let collect = pack
            .commands
            .iter()
            .find(|command| command.spec.name == "probe::collect")
            .expect("the collect command loads");
        assert_eq!(
            collect.spec.result_stability,
            Some(ResultStability::Volatile)
        );
        assert_eq!(
            collect.spec.data_collection.map(|op| op.action),
            Some(tcl_registry::events::DataCollectionAction::Collect)
        );
        assert_eq!(
            collect.spec.side_switch_target,
            Some(SideSwitchTarget::Server)
        );
        let priority = collect
            .spec
            .event_handler_priority
            .expect("the priority policy loads");
        assert_eq!(priority.default_priority, 500);
        assert_eq!(priority.max_priority, 1000);
        assert!(priority.warn_when_implicit);
        assert_eq!(collect.spec.event_requirement_forms.len(), 1);
        let form = &collect.spec.event_requirement_forms[0];
        assert_eq!(form.argument_prefix, ["append"]);
        assert_eq!(form.only_in, ["HTTP_REQUEST"]);
        assert!(
            form.requires
                .as_ref()
                .is_some_and(|requires| requires.client_side)
        );

        let read = pack
            .commands
            .iter()
            .find(|command| command.spec.name == "probe::read")
            .expect("the read command loads");
        assert_eq!(
            read.spec.result_stability,
            Some(ResultStability::ReadsVersionedWorld(&[
                WorldStateDomain::CommandBindings,
                WorldStateDomain::NamespaceLookup,
            ]))
        );
        let scope = read.spec.body_scope.expect("the body scope loads");
        assert_eq!(scope.name, "probe body");
        assert!(scope.include_sibling_definitions);
        assert!(!scope.allow_unknown_commands);
        let top = scope.command("top").expect("the scoped command loads");
        assert!(top.arity.accepts(2));
        assert_eq!(top.detail, "the top row");
        assert!(top.subcommand("set").is_some());
        assert_eq!(
            read.spec
                .subcommands
                .iter()
                .find(|sub| sub.name == "line")
                .and_then(|sub| sub.result_stability),
            Some(ResultStability::ReferentiallyTransparent)
        );
    }

    /// A `body_scope` may also name a shipped environment or a pack
    /// `descriptor`, exactly as `definition_body` and `case_list` do.
    #[test]
    fn a_body_scope_resolves_a_shipped_name_and_a_descriptor() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n\
             descriptor body_scope probe-style {\n\
             \x20  name {probe style}\n\
             \x20  command row { arity 1 }\n\
             }\n\
             command probe::defstyle { arity 2; body_scope report-defstyle }\n\
             command probe::style { arity 2; body_scope probe-style }\n\
             }\n",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let named = |name: &str| {
            pack.commands
                .iter()
                .find(|command| command.spec.name == name)
                .and_then(|command| command.spec.body_scope)
                .expect("a body scope")
        };
        assert_eq!(
            named("probe::defstyle").name,
            tcl_registry::scoped::REPORT_DEFSTYLE_ENV.name
        );
        assert!(named("probe::defstyle").is_command("top"));
        assert_eq!(named("probe::style").name, "probe style");
        assert!(named("probe::style").is_command("row"));
    }

    /// A malformed ratified row is dropped with a notice rather than
    /// guessed at: an unreadable world-state domain would *narrow* a
    /// dependency set, and a priority outside its own range is not a range.
    #[test]
    fn a_malformed_ratified_row_is_dropped_with_a_notice() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n\
             command probe::bad {\n\
             \x20  arity 0\n\
             \x20  result_stability {ReadsVersionedWorld {NotADomain}}\n\
             \x20  event_handler_priority -default 2000 -max 1000\n\
             \x20  data_collection -native NOT_A_DESCRIPTOR\n\
             }\n\
             }\n",
        );
        let command = pack
            .commands
            .iter()
            .find(|command| command.spec.name == "probe::bad")
            .expect("the command still loads");
        assert_eq!(command.spec.result_stability, None);
        assert_eq!(command.spec.event_handler_priority, None);
        assert_eq!(command.spec.data_collection, None);
        assert_eq!(pack.notices.len(), 3, "{:?}", pack.notices);
    }

    /// An `environment` block parses, validates, and converts.
    #[test]
    fn an_environment_block_loads_and_converts_to_a_definition() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n environment vivado-tcl {\n \
             display_name {Xilinx Vivado}\n core tcl 8.6\n \
             ambient Vivado keyed ToolVersion\n hosted Tk 8.5-\n \
             alias vivado\n editor_identity tcl\n \
             file_extension .XDC -name {Xilinx Design Constraints}\n \
             filename vivado.jou\n signature {create_project}\n \
             policy ambient-plus-require\n }\n command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert!(pack.command("demo").is_some(), "the pack's commands load");
        let environment = &pack.environments[0];
        assert_eq!(environment.id, "vivado-tcl");
        assert_eq!(environment.aliases, vec!["vivado".to_owned()]);
        assert_eq!(environment.world_policy, WorldPolicy::AmbientPlusRequire);
        assert_eq!(environment.file_extensions[0].extension.as_ref(), "xdc");

        let definition = environment.to_definition(PackEnvironmentTier::Workspace);
        assert_eq!(definition.id.as_str(), "vivado-tcl");
        assert_eq!(definition.display_name.as_ref(), "Xilinx Vivado");
        assert_eq!(definition.provenance, Provenance::WorkspaceTrusted);
        assert_eq!(
            definition.core.expect("a core selector").default_release,
            Release::TCL_8_6
        );
        assert_eq!(definition.expected_packages.len(), 2);
        assert!(definition.expected_packages[0].ambient);
        assert!(!definition.expected_packages[1].ambient);
        assert_eq!(definition.server_detection.filenames.len(), 1);
        assert_eq!(definition.server_detection.content_signatures.len(), 1);
        assert_eq!(
            definition
                .editor_identity
                .expect("a contributed identity")
                .as_str(),
            "tcl"
        );
        assert_eq!(
            environment
                .to_definition(PackEnvironmentTier::Bundled)
                .provenance,
            Provenance::BundledPack
        );
    }

    /// §3.3: a compiled canonical name (or alias) is reserved, and a block
    /// claiming one is rejected rather than merged.
    #[test]
    fn an_environment_block_claiming_a_compiled_name_is_rejected() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n environment tcl8.6 {\n core tcl 8.6\n }\n \
             command demo { arity 1 }\n}",
        );
        assert!(pack.environments.is_empty(), "{:?}", pack.environments);
        assert!(pack.command("demo").is_some(), "the rest still loads");
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("a compiled environment name")),
            "{:?}",
            pack.notices
        );
    }

    /// An unknown row in an `environment` block is semantic-class: the
    /// block is rejected, and the pack's other content still loads.
    #[test]
    fn an_unknown_environment_row_rejects_the_block() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             no_such_row yes\n }\n command demo { arity 1 }\n}",
        );
        assert!(pack.environments.is_empty());
        assert!(pack.command("demo").is_some(), "the rest still loads");
        assert!(
            pack.notices.iter().any(|n| {
                n.class == VocabularyClass::Presentation
                    && n.message.contains("semantic-class vocabulary")
            }),
            "{:?}",
            pack.notices
        );
    }

    /// An `environment NAME -extend { … }` block parses additively: it may
    /// name a compiled environment, carries detection rows and placements,
    /// and refuses identity rows.
    #[test]
    fn an_environment_extend_block_is_additive() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n environment synopsys-eda-tcl -extend {\n \
             file_extension upf -name {Unified Power Format}\n \
             ambient upf_extras 1.0\n }\n command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let environment = &pack.environments[0];
        assert!(environment.extends);
        assert_eq!(environment.id, "synopsys-eda-tcl");
        assert_eq!(environment.file_extensions[0].extension.as_ref(), "upf");
        assert_eq!(environment.placements.len(), 1);
        let extension = environment.to_extension(PackEnvironmentTier::Bundled);
        assert_eq!(extension.base, "synopsys-eda-tcl");
        assert_eq!(extension.provenance, Provenance::BundledPack);
        assert_eq!(extension.file_extensions.len(), 1);
        assert_eq!(extension.placements.len(), 1);

        // An identity row inside an extend block rejects the block.
        let rejected = evaluate_pack(
            "speclib probe 2.0 {\n environment synopsys-eda-tcl -extend {\n \
             core tcl 8.6\n }\n command demo { arity 1 }\n}",
        );
        assert!(rejected.environments.is_empty());
        assert!(
            rejected
                .notices
                .iter()
                .any(|n| n.message.contains("identity row")),
            "{:?}",
            rejected.notices
        );
    }

    /// `provides` declares the pack's package trains and defaults command
    /// providers; an explicit `default required_package` still wins.
    #[test]
    fn provides_defaults_the_command_provider() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n provides upf 1.0 2.1\n \
             command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert_eq!(pack.provides.len(), 1);
        assert_eq!(pack.provides[0].name, "upf");
        assert_eq!(pack.provides[0].versions, vec!["1.0", "2.1"]);
        assert_eq!(
            pack.command("demo").expect("demo").spec.required_package,
            Some("upf")
        );

        let explicit = evaluate_pack(
            "speclib probe 2.0 {\n provides upf\n \
             default required_package sdc\n command demo { arity 1 }\n}",
        );
        assert_eq!(
            explicit
                .command("demo")
                .expect("demo")
                .spec
                .required_package,
            Some("sdc"),
            "an explicit default beats the provides fallback"
        );
    }

    /// `co_provides` parses its predicated relation and carries it as data.
    #[test]
    fn co_provides_carries_the_predicated_relation() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n provides tk\n \
             co_provides Tk -requires-exact tk -when {without TK_NO_DEPRECATED}\n \
             command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let relation = &pack.co_provides[0];
        assert_eq!(relation.name, "Tk");
        assert_eq!(relation.requires_exact, Some("tk"));
        assert_eq!(relation.when, Some("without TK_NO_DEPRECATED"));
    }

    /// `dynamic_surface` / `unknown_members` set the open-surface fact on a
    /// command, and the object-class flags set it on a class.
    #[test]
    fn dynamic_surface_opens_a_commands_member_set() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n command demo {\n arity 1\n \
             dynamic_surface\n }\n command duo {\n arity 1\n \
             unknown_members\n }\n command trio { arity 1 }\n \
             command quad {\n arity 1\n \
             object_class ::probe::tree -dynamic-surface {\n \
             method walk { arity 0 }\n }\n }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert!(
            pack.command("demo")
                .expect("demo")
                .spec
                .allow_unknown_subcommands
        );
        assert!(
            pack.command("duo")
                .expect("duo")
                .spec
                .allow_unknown_subcommands
        );
        assert!(
            !pack
                .command("trio")
                .expect("trio")
                .spec
                .allow_unknown_subcommands,
            "saying nothing keeps the closed default"
        );
    }

    /// `include` splices a fragment's declarations in place — provenance
    /// inherited, registrations carrying the included statements — and the
    /// determinism guards hold: no context, a cycle, and a path-shaped name
    /// each drop the row with a semantic notice.
    #[test]
    fn include_splices_a_fragment_under_the_determinism_contract() {
        let fragment = "command extra {\n arity 2\n}\n";
        let context = IncludeContext::new(move |name| match name {
            "extra.tclspec-frag" => Ok(fragment.to_owned()),
            "self.tclspec-frag" => Ok("include self.tclspec-frag\n".to_owned()),
            other => Err(format!("no such fragment `{other}`")),
        });
        let context = std::rc::Rc::new(context);
        let including = |source: &str| {
            evaluate_pack_in(
                source,
                &EvalOptions::default(),
                Some(std::rc::Rc::clone(&context)),
            )
        };
        let pack = including(
            "speclib probe 2.0 {\n include extra.tclspec-frag\n \
             command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert!(
            pack.command("extra").is_some(),
            "the fragment's command loads"
        );
        assert!(pack.command("demo").is_some());
        assert!(
            pack.registrations.iter().any(|reg| reg.arg(1) == "extra"),
            "the record carries the included statements"
        );
        assert!(
            !pack.registrations.iter().any(|reg| reg.word() == "include"),
            "…and not the include row itself"
        );

        // No context: the row drops with a semantic notice.
        let dropped = evaluate_pack("speclib probe 2.0 {\n include extra.tclspec-frag\n}");
        assert!(
            dropped.notices.iter().any(|n| {
                n.class == VocabularyClass::Semantic
                    && n.message.contains("needs a pack search path")
            }),
            "{:?}",
            dropped.notices
        );

        // A cycle is rejected by content hash.
        let cyclic = including("speclib probe 2.0 {\n include self.tclspec-frag\n}");
        assert!(
            cyclic
                .notices
                .iter()
                .any(|n| n.message.contains("include cycle")),
            "{:?}",
            cyclic.notices
        );

        // A path-shaped name never reaches the resolver.
        let hostile = including("speclib probe 2.0 {\n include ../outside\n}");
        assert!(
            hostile
                .notices
                .iter()
                .any(|n| n.message.contains("not a plain file name")),
            "{:?}",
            hostile.notices
        );

        // An unresolvable name reports the resolver's reason.
        let missing = including("speclib probe 2.0 {\n include nowhere.tclspec-frag\n}");
        assert!(
            missing
                .notices
                .iter()
                .any(|n| n.message.contains("did not resolve")),
            "{:?}",
            missing.notices
        );
    }

    /// An unknown editor identity keeps the row and drops only the routing
    /// (review B7: an environment selects from the contributed set).
    #[test]
    fn an_unknown_editor_identity_drops_only_the_routing() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             editor_identity klingon\n }\n}",
        );
        let environment = &pack.environments[0];
        assert_eq!(environment.editor_identity, None);
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("not a contributed editor language id")),
            "{:?}",
            pack.notices
        );
    }

    /// A `dialect` block parses its ladder and its axes against the closed
    /// vocabulary.
    #[test]
    fn a_dialect_block_loads_its_ladder_and_axes() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n dialect picol2 {\n release 2.0\n \
             release 2.1 -build Unknown\n axis expand_syntax off\n \
             axis braced_var first-close\n axis numbers tcl90\n \
             axis escapes tcl90\n axis expr_comments hash\n \
             axis bom_skip on\n }\n command demo { arity 1 }\n}",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let dialect = &pack.dialects[0];
        assert_eq!(dialect.name, "picol2");
        assert_eq!(dialect.releases.len(), 2);
        assert_eq!(dialect.releases[1].build, BuildProfileId::Unknown);
        assert_eq!(dialect.axis("expand_syntax"), Some("off"));
        assert_eq!(dialect.axis("nonesuch"), None);
        let grammar = dialect.to_grammar().expect("every value has a backing");
        assert!(!grammar.expand_syntax);
        assert_eq!(grammar.braced_var, BracedVarStyle::FirstClose);
    }

    /// An unknown axis, or an unknown value on a known axis, is §6.1's
    /// semantic class: the whole block is rejected, the notice names the
    /// axis, and the pack's other content still loads.
    #[test]
    fn an_unknown_axis_or_value_rejects_the_dialect_block() {
        for (body, needle) in [
            (
                "axis no_such_axis on",
                "is not in the closed axis vocabulary",
            ),
            ("axis numbers tcl99", "is not a value of `numbers`"),
        ] {
            let pack = evaluate_pack(&format!(
                "speclib probe 2.0 {{\n dialect probe-dialect {{\n {body}\n }}\n \
                 command demo {{ arity 1 }}\n}}"
            ));
            assert!(pack.dialects.is_empty(), "{body}: {:?}", pack.dialects);
            assert!(pack.command("demo").is_some(), "{body}: the rest loads");
            assert!(
                pack.notices.iter().any(|n| n.message.contains(needle)),
                "{body}: {:?}",
                pack.notices
            );
        }
    }

    /// §2's classification gate: a `dialect` block whose axes reproduce a
    /// compiled release is not a dialect at all, and the notice names the
    /// environment it should have been.
    #[test]
    fn a_dialect_block_duplicating_a_compiled_release_is_rejected() {
        let pack = evaluate_pack(
            "speclib probe 2.0 {\n dialect my-tcl {\n release 1.0\n \
             axis expand_syntax on\n axis braced_var first-close\n \
             axis numbers tcl85\n axis escapes tcl86\n \
             axis expr_comments none\n axis bom_skip off\n \
             axis irules_brace_separator off\n }\n command demo { arity 1 }\n}",
        );
        assert!(pack.dialects.is_empty(), "{:?}", pack.dialects);
        assert!(
            pack.notices.iter().any(|n| {
                n.message.contains("declares the grammar of tcl 8.6")
                    && n.message.contains("environment my-tcl { core tcl 8.6")
            }),
            "{:?}",
            pack.notices
        );
    }

    /// §6.1's three classes, in the forward direction that is the only one
    /// they apply in.
    ///
    /// The downgrade fixture pattern: each body's word is absent from this
    /// build's vocabulary, and the assertion is that its absence yields
    /// *abstention* — a dropped label, a degraded capability, or an excluded
    /// command — never a stronger claim.
    #[test]
    fn unknown_words_classify_by_compatibility_effect() {
        // Presentation: prose. Warn and drop, as ever.
        let pack = evaluate_pack(
            "speclib probe 2.9 {\n command demo {\n arity 1\n \
             marketing_blurb {Buy now.}\n }\n}",
        );
        let demo = pack.command("demo").expect("the command loads");
        assert!(!demo.degraded, "prose costs the command nothing");
        assert_eq!(pack.notices[0].class, VocabularyClass::Presentation);

        // Assistance: a shape word. The command stays known, marked.
        let pack = evaluate_pack(
            "speclib probe 2.9 {\n command demo {\n arity 1\n \
             arg_cardinality 3\n }\n}",
        );
        let demo = pack.command("demo").expect("the command still loads");
        assert!(demo.degraded, "the affected capability must answer Unknown");
        assert!(
            pack.notices
                .iter()
                .any(|n| n.class == VocabularyClass::Assistance),
            "{:?}",
            pack.notices
        );

        // Semantic: a security word. The command is excluded outright.
        let pack = evaluate_pack(
            "speclib probe 2.9 {\n command demo {\n arity 1\n \
             taint_launders yes\n }\n command safe_neighbour { arity 0 }\n}",
        );
        assert!(
            pack.command("demo").is_none(),
            "a command whose security facts cannot be read is excluded"
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.class == VocabularyClass::Semantic
                    && n.message.contains("excluded from strong analysis")),
            "{:?}",
            pack.notices
        );
    }

    /// The classes apply **only** in §6.1's forward direction. An unknown
    /// word in a pack whose vocabulary this build knows in full is an
    /// author's typo, and keeps today's warn-and-drop treatment exactly —
    /// which is what keeps every 1.x pack in the corpus loading unchanged.
    #[test]
    fn unknown_words_in_a_known_vocabulary_stay_presentation_class() {
        for declared in ["1.2", "2.0"] {
            let pack = evaluate_pack(&format!(
                "speclib probe {declared} {{\n command demo {{\n arity 1\n \
                 taint_launders yes\n }}\n}}"
            ));
            let demo = pack.command("demo").expect("the command loads");
            assert!(!demo.degraded, "{declared}");
            assert!(
                pack.notices
                    .iter()
                    .all(|n| n.class == VocabularyClass::Presentation),
                "{declared}: {:?}",
                pack.notices
            );
        }
    }
}
