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

//! The document's **realm command-binding state** (redesign §4.2,
//! centralisation ledger C4; issues #1185, #1275) — the single-realm
//! `command_bindings` map of the model's `RealmState`, produced by one
//! top-level scan and answered as [`tcl_registry::model::BindingKnowledge`]
//! ([`CommandBindingRealm::knowledge_at`]) or as the head-word projection
//! source-text consumers read ([`CommandBindingRealm::resolve`] /
//! [`HeadWords`]). This retires the parallel offset-keyed head-identity
//! table wholesale: the same facts, one vocabulary, spec-keyed.
//!
//! Tcl resolves a command by its interpreter-level *binding*, not by the
//! spelling used to invoke it.  Three statically visible statements move that
//! binding, and this module turns the ones a document states unconditionally
//! at top level into a small, offset-keyed table the source-text consumers
//! consult before they hand a head to the registry:
//!
//! ```tcl
//! namespace import ::tcltest::*   ;# `test`   now *is* `::tcltest::test`
//! interp alias {} myfmt {} format ;# `myfmt`  now *is* `format`
//! rename format origfmt           ;# `origfmt` is `format`, and `format` is gone
//! proc format {args} { … }        ;# `format` is a user proc, not the built-in
//! ```
//!
//! Every fact carries the byte offset of the statement that established it and
//! applies only to heads **at or after** it, so an alias cannot retroactively
//! re-tag an earlier call and a `rename` correctly leaves the calls before it
//! alone.  Verified against tclsh 9.0.4 and 8.6.16 (byte-identical): after
//! `rename format origfmt; proc format {args} {return USER}`, `format x`
//! answers `USER` and `origfmt %d 7` answers `7`.
//!
//! # What this deliberately does not do
//!
//! The table is **sound by abstention** — every shape it cannot prove leaves
//! the head unchanged, and every shape that provably *breaks* a registry
//! binding marks the head [`RealmBinding::Rebound`] so no registry grammar is
//! applied to it.  Specifically:
//!
//! * **Dynamic heads and dynamic bindings** — `rename $old new`,
//!   `interp alias {} $n {} eval`, `interp alias {} n {} $t` — record nothing
//!   ([`tcl_syntax::naming::is_dynamic_word`] rejects them), so a call through
//!   the name keeps its literal identity rather than gaining a wrong one.
//! * **Pre-bound alias arguments** — `interp alias {} pad {} format %08x`
//!   shifts every argument index, so the layout cannot be reused; the alias
//!   name is marked `Rebound` instead of aliased.
//! * **Another interpreter** — a non-empty `srcPath` (`interp alias slave …`)
//!   binds a name in a *child* interpreter and changes nothing here, so it
//!   records nothing at all; a non-empty `targetPath` points at a command this
//!   document cannot see, so the name is marked `Rebound`.  Hidden commands in
//!   a safe interpreter are likewise invisible: this document's own command
//!   table is what is being described.
//! * **Conditional or nested bindings** — only *top-level* statements are
//!   scanned.  A `rename` inside an `if` body, a proc, an `eval`, or a
//!   `namespace eval` block is not an unconditional document-wide fact, and a
//!   `proc` inside `namespace eval ::n` defines `::n::format`, not `::format`
//!   (tclsh 9.0.4: `::n::format q` → `ns:q`, global `format` untouched).
//! * **`unknown` fallback and traces** — nothing is inferred from them.
//!
//! # Positioned and unpositioned readers
//!
//! [`CommandBindingRealm::resolve`] answers for a head at a known byte offset.
//! Several consumers re-lex a body out of its own decoded text (the formatter
//! reformats `arg.text`, the minifier re-minifies a body slice, the call-graph
//! scan segments a body string at offset 0), so no absolute offset exists at
//! the point of the query.  Those read
//! [`CommandBindingRealm::resolve_unpositioned`], which considers *every* fact
//! about the spelling at once and abstains ([`RealmBinding::Rebound`]) unless
//! they all agree — a document that binds a name twice cannot be read without
//! a position, and guessing one of the two is exactly the fallback-to-spelling
//! this module exists to remove.

use std::borrow::Cow;

use crate::alias::{command_table_transitions, is_current_interpreter};
use crate::segmenter::segment_commands_with_offset_and_config;
use rustc_hash::FxHashMap;
use tcl_registry::model::{BindingKnowledge, BindingTarget, ResolvedContext, SpecKey};
use tcl_registry::{CommandBindingTransition, CommandRegistry, CommandSpec, TransitionSubject};
use tcl_syntax::naming::is_dynamic_word;

/// What a command head resolves to at one point in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealmBinding<'a> {
    /// The head names this registry command — the head text itself when no
    /// fact applies, or the effective target of a proven import / alias /
    /// rename.
    Command(&'a str),
    /// The head's binding was provably taken over by something the registry
    /// does not model: a `rename` moved the built-in away, a user `proc`
    /// redefined it, or an alias bound it to an unresolvable target.  No
    /// registry grammar applies.
    Rebound,
}

/// A binding fact without borrowing the head spelling supplied by the caller.
///
/// This is the cross-crate form of [`CommandBindingRealm`] lookup: a consumer can
/// preserve its own written text when no fact applies, while a resolved target
/// borrows from the map. That separation avoids making a syntax/registry
/// caller manufacture compiler-owned copies merely to compare a command head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealmBindingFact<'a> {
    /// No applicable fact; the written spelling remains the identity.
    Unchanged,
    /// A proven alias/import/rename target.
    Command(&'a str),
    /// The spelling was provably rebound away from registry semantics.
    Rebound,
}

impl<'a> RealmBinding<'a> {
    /// The name to resolve registry grammar against.
    ///
    /// [`Self::Rebound`] answers with the empty string, which
    /// [`CommandRegistry::get`] never resolves — so every registry query a
    /// consumer already makes (`arg_indices_for_role`, `format_string_args`,
    /// `handle_binding`, …) answers "not a known command" without that
    /// consumer testing the variant.
    #[must_use]
    pub fn spec_name(self) -> &'a str {
        match self {
            Self::Command(name) => name,
            Self::Rebound => "",
        }
    }

    /// Whether the head's registry binding was provably taken over.
    #[must_use]
    pub fn is_rebound(self) -> bool {
        matches!(self, Self::Rebound)
    }
}

/// A command head in its two forms — the spelling the source wrote, and the
/// registry name that spelling effectively resolves to.
///
/// The two are the same for almost every head, and differ once the document
/// rebinds a name.  Which of the two a test must read is a real distinction,
/// not a convenience:
///
/// * a **global command** lookup reads [`Self::resolved`], so `::snit::type`,
///   a proven alias of it, and the bare spelling all answer alike (and a
///   rebound spelling answers nothing);
/// * a **lexical** test reads [`Self::written`] — a class-body member
///   sub-keyword (`method`, `constructor`) or a `$var` head is not a command
///   binding at all, so a top-level `rename method …` says nothing about the
///   word inside an `oo::define`.
#[derive(Debug, Clone, Copy)]
pub struct HeadWords<'a> {
    /// The head exactly as the source spells it.
    pub written: &'a str,
    /// The registry name the spelling resolves to — empty when the head was
    /// provably rebound, which every registry query then answers "unknown" for.
    pub resolved: &'a str,
}

impl<'a> HeadWords<'a> {
    /// A head with nothing proven about it: written and resolved alike.
    #[must_use]
    pub fn plain(written: &'a str) -> Self {
        Self {
            written,
            resolved: written,
        }
    }
}

/// What one realm fact binds a head spelling to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactBinding {
    /// A proven registry identity: the head *is* this spec here (a proven
    /// import / alias / rename chain).
    Spec(SpecKey),
    /// The head's binding was provably taken over by something the
    /// registry does not model — a user `proc` shadow, an alias with
    /// pre-bound arguments or an unmodelled target. The command *exists*;
    /// no registry grammar applies.
    TakenOver,
    /// The head was provably deleted (`rename NAME {}`, an alias
    /// deletion, or a `rename OLD NEW` moving `OLD` away): nothing is
    /// bound here from this point on.
    Deleted,
}

/// One binding fact about one head spelling.
#[derive(Debug, Clone, Copy)]
struct RealmFact {
    /// Byte offset of the statement that established it; the fact applies to
    /// heads at or after this offset only.
    from: u32,
    /// What the head is bound to from that offset.
    binding: FactBinding,
}

/// Every statically proven command-identity fact in one document, keyed by the
/// head spelling as written.
///
/// Both the bare and the explicitly global spelling of a bound name are
/// recorded, because C Tcl resolves them to the same command
/// (`namespace which -command ::myfmt` → `::myfmt`) and a consumer must not
/// have to strip qualifiers itself.
#[derive(Debug, Default)]
pub struct CommandBindingRealm {
    facts: FxHashMap<String, Vec<RealmFact>>,
}

/// The shared empty map, for a consumer that has no document to scan (an
/// IR-only caller, a unit-test harness).  Nothing is bound, so every head
/// keeps its own spelling.
static EMPTY_REALM: std::sync::LazyLock<CommandBindingRealm> =
    std::sync::LazyLock::new(CommandBindingRealm::default);

impl CommandBindingRealm {
    /// The empty map — no document, so no binding fact.
    #[must_use]
    pub fn none() -> &'static Self {
        &EMPTY_REALM
    }

    /// Whether the document stated any binding fact at all — lets a caller
    /// skip per-head lookups entirely for the overwhelmingly common document
    /// that imports, aliases, and renames nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// The effective identity of `head` invoked at byte offset `at`.
    ///
    /// The **latest** fact at or before `at` wins, so a document that renames a
    /// name and then rebinds it again reads correctly at every point between.
    /// With no applicable fact the head keeps its own spelling.
    #[must_use]
    pub fn resolve<'a>(&'a self, head: &'a str, at: u32) -> RealmBinding<'a> {
        match self.binding_at(head, at) {
            RealmBindingFact::Unchanged => RealmBinding::Command(head),
            RealmBindingFact::Command(name) => RealmBinding::Command(name),
            RealmBindingFact::Rebound => RealmBinding::Rebound,
        }
    }

    /// The command-table fact applicable to `head` at `at`, if any.
    ///
    /// Unlike [`Self::resolve`], this never borrows the caller's spelling, so
    /// registry-owned recursive walkers can use it through
    /// [`tcl_registry::events::CommandHeadResolver`] without a dependency from
    /// the registry back into the compiler.
    #[must_use]
    pub fn binding_at(&self, head: &str, at: u32) -> RealmBindingFact<'_> {
        match self.fact_at(head, at) {
            None => RealmBindingFact::Unchanged,
            Some(FactBinding::Spec(key)) => RealmBindingFact::Command(key.name()),
            Some(FactBinding::TakenOver | FactBinding::Deleted) => RealmBindingFact::Rebound,
        }
    }

    /// The latest applicable fact's binding for `head` at `at`, if any.
    fn fact_at(&self, head: &str, at: u32) -> Option<FactBinding> {
        self.facts
            .get(head)?
            .iter()
            .filter(|f| f.from <= at)
            .max_by_key(|f| f.from)
            .map(|f| f.binding)
    }

    /// The realm's [`BindingKnowledge`] for `head` at byte offset `at` —
    /// the document's facts composed over the environment (the one
    /// `exists` oracle, centralisation R-c, for head-identity consumers):
    ///
    /// - a proven import / alias / rename chain answers
    ///   [`BindingKnowledge::Must`] with its [`BindingTarget::Spec`];
    /// - a proven takeover (user `proc` shadow, unmodelled alias) answers
    ///   `Must` with a [`BindingTarget::Document`] — the command exists,
    ///   no catalogue semantics apply, so no hook ever specialises (I4);
    /// - a proven deletion answers [`BindingKnowledge::Absent`];
    /// - with no document fact the environment answers: `Must(Spec)` when
    ///   `context` provides the name, [`BindingKnowledge::Absent`] under a
    ///   **closed world** (the guarantee iRules derives rather than
    ///   assumes — B12 is policy over this oracle, not a second oracle),
    ///   and [`BindingKnowledge::Unknown`] otherwise (an open world can
    ///   gain commands at load time).
    #[must_use]
    pub fn knowledge_at(
        &self,
        context: &ResolvedContext,
        commands: &CommandRegistry,
        head: &str,
        at: u32,
    ) -> BindingKnowledge {
        match self.fact_at(head, at) {
            Some(FactBinding::Spec(key)) => BindingKnowledge::Must(BindingTarget::Spec(key)),
            Some(FactBinding::TakenOver) => BindingKnowledge::Must(BindingTarget::document(head)),
            Some(FactBinding::Deleted) => BindingKnowledge::Absent,
            None => match context.resolve_spec(commands, head) {
                Some(spec) => BindingKnowledge::Must(BindingTarget::Spec(SpecKey::new(spec))),
                None => match context.environment.policy_defaults.closed_world {
                    tcl_dialect::model::WorldPolicy::Closed => BindingKnowledge::Absent,
                    _ => BindingKnowledge::Unknown,
                },
            },
        }
    }

    /// The effective identity of `head` when the call's byte offset is not
    /// available — a body the consumer re-lexed out of its own decoded text.
    ///
    /// Every fact about the spelling is considered at once.  With none, the
    /// head keeps its own spelling; with facts that all name the same target,
    /// that target; otherwise [`RealmBinding::Rebound`], because the reader
    /// cannot tell which of two bindings is in force and the written spelling
    /// is precisely the answer that must not be assumed.
    #[must_use]
    pub fn resolve_unpositioned<'a>(&'a self, head: &'a str) -> RealmBinding<'a> {
        let Some(facts) = self.facts.get(head) else {
            return RealmBinding::Command(head);
        };
        let Some(first) = facts.first() else {
            return RealmBinding::Command(head);
        };
        // Facts naming different specs cannot be read without a position;
        // a takeover beside a deletion reads Rebound either way, so only
        // spec disagreement matters here.
        match first.binding {
            FactBinding::Spec(key)
                if facts
                    .iter()
                    .all(|f| matches!(f.binding, FactBinding::Spec(other) if other == key)) =>
            {
                RealmBinding::Command(key.name())
            }
            _ => RealmBinding::Rebound,
        }
    }

    /// `head` in both its forms, resolved at byte offset `at`.
    #[must_use]
    pub fn head_words<'a>(&'a self, head: &'a str, at: u32) -> HeadWords<'a> {
        if self.facts.is_empty() {
            return HeadWords::plain(head);
        }
        HeadWords {
            written: head,
            resolved: self.resolve(head, at).spec_name(),
        }
    }

    /// `head` in both its forms, resolved without a position — see
    /// [`Self::resolve_unpositioned`].
    #[must_use]
    pub fn head_words_unpositioned<'a>(&'a self, head: &'a str) -> HeadWords<'a> {
        if self.facts.is_empty() {
            return HeadWords::plain(head);
        }
        HeadWords {
            written: head,
            resolved: self.resolve_unpositioned(head).spec_name(),
        }
    }

    /// Whether an *earlier* fact already gives `head` a registry identity at
    /// offset `at` — used to notice that a `proc` takes back a name a previous
    /// `rename` / alias had bound to a built-in.
    fn resolves_to_a_command(&self, head: &str, at: u32) -> bool {
        matches!(self.resolve(head, at), RealmBinding::Command(name) if name != head)
    }

    /// Record `head` → `binding` from byte offset `from`.
    fn record(&mut self, head: &str, binding: FactBinding, from: u32) {
        if head.is_empty() {
            return;
        }
        self.facts
            .entry(head.to_owned())
            .or_default()
            .push(RealmFact { from, binding });
    }

    /// Record a fact under both the written spelling and its explicitly global
    /// twin, so `myfmt` and `::myfmt` classify alike.
    fn record_both_spellings(&mut self, name: &str, binding: FactBinding, from: u32) {
        let bare = name.strip_prefix("::").unwrap_or(name);
        self.record(bare, binding, from);
        self.record(&format!("::{bare}"), binding, from);
    }
}

impl tcl_registry::events::CommandHeadResolver for CommandBindingRealm {
    fn resolve<'a>(&'a self, written: &str, offset: u32) -> Cow<'a, str> {
        match self.binding_at(written, offset) {
            RealmBindingFact::Unchanged => Cow::Owned(written.to_owned()),
            RealmBindingFact::Command(name) => Cow::Borrowed(name),
            RealmBindingFact::Rebound => Cow::Borrowed(""),
        }
    }
}

/// Whether `word` is a name this scan may treat as a static command spelling.
fn is_static_name(word: &str) -> bool {
    !word.is_empty() && !is_dynamic_word(word) && !word.contains(char::is_whitespace)
}

/// Scan `source` for the top-level statements that move a command binding and
/// build the document's [`CommandBindingRealm`].
///
/// Which commands mutate the command table is registry data
/// ([`CommandTableEffect`]) and the argument shapes come from the compiler's
/// own detectors ([`detect_interp_alias`] / [`detect_rename`]) — the same ones
/// the IR-lowering pipeline uses — so no command name is spelled here.
#[must_use]
pub fn document_realm_bindings(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> CommandBindingRealm {
    document_realm_bindings_with_config(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        registry,
    )
}

/// [`document_realm_bindings`] with an explicit lexer configuration, for a
/// consumer that already holds one (the formatter and the param-trait scan
/// carry a [`tcl_lexer::LexerConfig`] rather than a dialect string).
#[must_use]
pub fn document_realm_bindings_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
) -> CommandBindingRealm {
    // One top-level segmentation feeds both halves — the `namespace import`
    // scan and the command-table mutators.
    let segments = segment_commands_with_offset_and_config(source, 0, config);
    let mut map = CommandBindingRealm::default();
    for (name, (key, offset)) in imported_command_aliases(&segments, registry) {
        map.record(&name, FactBinding::Spec(key), offset);
    }
    for seg in &segments {
        let Some(head) = seg.texts.first() else {
            continue;
        };
        let args = &seg.texts[1..];
        let transitions = command_table_transitions(registry, head, args);
        if transitions.command_bindings().next().is_none() {
            continue;
        }
        let at = seg.argv[0].span.start();
        // A `proc` declaration is the one binding fact whose *validity* is
        // dialect-gated: iRules restricts `proc` to its shared declaration
        // surface, and a malformed body is not executable.
        let declares_valid_procedure = valid_irules_procedure_declaration(source, seg, registry);
        for transition in transitions.command_bindings() {
            record_binding_transition(&mut map, transition, at, registry, declares_valid_procedure);
        }
    }
    map
}

/// Record one registry-stated command-binding transition as a realm fact.
///
/// The argument layout, the dynamic-operand rule, and the alias shape check
/// all live in the registry's stock resolvers now (ledger C8): this reads
/// facts, it does not decode words.
fn record_binding_transition(
    map: &mut CommandBindingRealm,
    transition: &CommandBindingTransition,
    at: u32,
    registry: &CommandRegistry,
    declares_valid_procedure: bool,
) {
    match transition {
        CommandBindingTransition::Define { name, .. } => {
            if declares_valid_procedure {
                record_proc(map, name, at, registry);
            }
        }
        CommandBindingTransition::Move { from, to } => record_move(map, from, to, at, registry),
        CommandBindingTransition::Delete { interpreter, name } => {
            // `rename OLD {}` carries no interpreter; `interp alias {} NAME
            // {}` carries the source path, and only a deletion in *this*
            // interpreter changes what the document's later names mean.
            if interpreter.as_ref().is_none_or(is_current_interpreter)
                && let Some(name) = static_subject(name)
            {
                map.record_both_spellings(name, FactBinding::Deleted, at);
            }
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
        } => record_alias(
            map,
            AliasFact {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
            },
            at,
            registry,
        ),
        CommandBindingTransition::Unknown { operands } => {
            // A `rename` whose *source* is dynamic states nothing at all —
            // neither half of the move can be named. One whose source is
            // known still vacates that name.
            if let Some(from) = operands.first().and_then(static_subject) {
                map.record_both_spellings(from, FactBinding::Deleted, at);
            }
        }
    }
}

/// The literal value of `subject`, when it is a name this scan may treat as
/// a static command spelling.
fn static_subject(subject: &TransitionSubject) -> Option<&str> {
    subject.literal().filter(|name| is_static_name(name))
}

/// One `interp alias` fact's operands, bundled so [`record_alias`] stays at
/// or under the argument limit.
struct AliasFact<'a> {
    source_interpreter: &'a TransitionSubject,
    alias: &'a TransitionSubject,
    target_interpreter: &'a TransitionSubject,
    target: &'a TransitionSubject,
    arguments: &'a [TransitionSubject],
}

/// Whether this top-level segmented `proc` can actually create an iRules
/// procedure declaration.  iRules keeps Tcl's `proc` spelling but restricts
/// it to the shared declaration surface; a malformed or unterminated body is
/// not executable and therefore must not poison command-head identity for the
/// rest of the document.  Other profiles retain ordinary Tcl's historical
/// name-only identity semantics.
fn valid_irules_procedure_declaration(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> bool {
    if !registry
        .profile()
        .is_some_and(tcl_dialect::DialectProfile::is_irules)
    {
        return true;
    }
    let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
    let Some(closed) = tcl_registry::events::closed_braced_argument_words(
        source,
        seg.arg_tokens(),
        seg.arg_single_token(),
    ) else {
        return false;
    };
    let Some(arguments) = tcl_registry::events::IrulesDeclarationArguments::new(
        &args,
        seg.arg_tokens(),
        seg.arg_single_token(),
        &closed,
    ) else {
        return false;
    };
    matches!(
        registry.irules_top_level_declaration_shape(seg.name(), arguments),
        Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. })
    )
}

/// The spec this profile's command table actually exposes for `name`, or
/// `None` when the profile disables it.
///
/// `CommandRegistry::get` is deliberately dialect-agnostic for diagnostics
/// such as W002. Realm facts instead model an executed statement, so an
/// unavailable Tcl command (notably iRules' disabled `interp`, `rename`, and
/// `namespace`) must produce no fact.
fn available_spec(registry: &CommandRegistry, name: &str) -> Option<&'static CommandSpec> {
    registry.profile().map_or_else(
        || registry.get(name),
        |profile| registry.get_for_dialect(name, profile.availability_mask),
    )
}

/// [`available_spec`] as a boolean, for the callers that only gate.
fn command_is_available(registry: &CommandRegistry, name: &str) -> bool {
    available_spec(registry, name).is_some()
}

/// `rename OLD NEW` — `NEW` inherits `OLD`'s identity and `OLD` stops
/// existing.  (`rename OLD {}` reaches the realm as a `Delete` fact
/// instead, so this arm only ever sees a genuine move.)
fn record_move(
    map: &mut CommandBindingRealm,
    from: &TransitionSubject,
    to: &TransitionSubject,
    at: u32,
    registry: &CommandRegistry,
) {
    // A dynamic source means neither half of the move can be stated.
    let Some(old) = static_subject(from) else {
        return;
    };
    if let Some(new) = static_subject(to) {
        // Only a source the registry models — directly or through an earlier
        // fact — is worth aliasing; renaming an ordinary user proc leaves `NEW`
        // an ordinary unknown name, which is already what an absent fact
        // produces.  A *provably rebound* source is stated, though: it moves
        // something the registry does not model onto `NEW`, and `NEW` must not
        // then be read under the built-in's grammar.
        match inherited_spec(map, old, at, registry) {
            Some(key) => map.record_both_spellings(new, FactBinding::Spec(key), at),
            None if map.resolve(old, at).is_rebound() => {
                map.record_both_spellings(new, FactBinding::TakenOver, at);
            }
            None => {}
        }
    }
    // Either way the old name is gone from this point on.
    map.record_both_spellings(old, FactBinding::Deleted, at);
}

/// The registry name `source` names at offset `at`, folding in any earlier
/// fact so a **chain** of bindings composes.
///
/// Reading `source` through the map rather than straight off the registry is
/// what makes `interp alias {} a {} format; rename a b` leave `b` naming
/// `format` — the behaviour C Tcl has (tclsh 8.6.16 and 9.0.4, byte-identical:
/// `b %08x 42` answers `0000002a` while `info commands a` answers empty).  The
/// same read is what stops a chain inheriting a *broken* binding:
/// `proc format {…} {…}; rename format myfmt` moves the **user proc**, so
/// `myfmt` must not pick up the built-in's grammar.
///
/// `None` means "names nothing the registry models" — either a provably
/// taken-over spelling or an ordinary unknown name.  The two callers differ in
/// what they do with that, so the distinction stays at the call site.
fn inherited_spec(
    map: &CommandBindingRealm,
    source: &str,
    at: u32,
    registry: &CommandRegistry,
) -> Option<SpecKey> {
    match map.fact_at(source, at) {
        Some(FactBinding::Spec(key)) => Some(key),
        Some(FactBinding::TakenOver | FactBinding::Deleted) => None,
        None => available_spec(registry, source).map(SpecKey::new),
    }
}

/// `interp alias {} NEW {} TARGET ?arg…?`.  (The deletion form reaches the
/// realm as a `Delete` fact instead.)
fn record_alias(
    map: &mut CommandBindingRealm,
    fact: AliasFact<'_>,
    at: u32,
    registry: &CommandRegistry,
) {
    if !is_current_interpreter(fact.source_interpreter) {
        // A foreign `srcPath` binds a name in a child interpreter; nothing
        // changes here.  A dynamic path cannot be stated either.
        return;
    }
    let Some(alias) = static_subject(fact.alias) else {
        return;
    };
    if !is_current_interpreter(fact.target_interpreter) {
        // A foreign *target* path is the one shape worth marking rebound —
        // the alias exists here but runs elsewhere.
        map.record_both_spellings(alias, FactBinding::TakenOver, at);
        return;
    }
    // A dynamic target (`interp alias {} myEval {} $target`) names nothing
    // statically: recording the alias would map `myEval` onto a
    // never-registered name, so abstain rather than state half a fact.
    let Some(target) = fact.target.literal() else {
        return;
    };
    // Pre-bound arguments shift every index, so the target's layout cannot be
    // reused; a target that names nothing the registry models has no layout to
    // reuse either.  Both cases still *take over* the name — C Tcl lets an
    // alias shadow an existing command outright (tclsh 8.6.16 / 9.0.4:
    // `proc myproc …; interp alias {} lindex {} myproc` makes `lindex {a b c}
    // 1` answer `MINE`) — so the name is marked rebound rather than left alone.
    // The target is read through the map, so a chain of bindings composes.
    let effective = fact
        .arguments
        .is_empty()
        .then(|| inherited_spec(map, target, at, registry))
        .flatten();
    map.record_both_spellings(
        alias,
        effective.map_or(FactBinding::TakenOver, FactBinding::Spec),
        at,
    );
}

/// `proc NAME …` at top level — a user proc that shadows a registry built-in
/// takes the name over (tclsh 9.0.4: after `proc format {args} {return USER}`,
/// `format x` is `USER`).
///
/// Only a name that *would otherwise* resolve to a registry command is
/// recorded — either directly, or through an earlier fact (a `proc origfmt`
/// after `rename format origfmt` takes the moved built-in's name back).  Every
/// other `proc` leaves the head an ordinary unknown name, which is already what
/// an absent fact produces.
fn record_proc(
    map: &mut CommandBindingRealm,
    name: &TransitionSubject,
    at: u32,
    registry: &CommandRegistry,
) {
    let Some(name) = static_subject(name) else {
        return;
    };
    if name.contains("::") {
        // A qualified `proc ::ns::format` defines a *different* command; only
        // the global-namespace shadow is stated here.
        return;
    }
    if command_is_available(registry, name) || map.resolves_to_a_command(name, at) {
        map.record_both_spellings(name, FactBinding::TakenOver, at);
    }
}

/// Scan `source` for `namespace import` declarations and map each bare command
/// name they bring into the global scope to its qualified registry spec name
/// (`test` → `tcltest::test`, issue #776).
///
/// Recognises the two literal forms — `namespace import EXPORTING::*`
/// (import-all) and `namespace import EXPORTING::name` (single) — matched
/// against the registry's `is_namespace_exported` commands in the exporting
/// namespace.  A bare name that already resolves to a global command is left
/// alone (Tcl's own `namespace import` refuses to shadow an existing command
/// without `-force`, and we must not mis-resolve a genuine builtin).  This is a
/// highlighting-only convenience: it never changes which commands exist, only
/// lets the registry-driven argument overrides see the real spec for an
/// unqualified imported command.  Returns an empty map when nothing is imported.
fn imported_command_aliases(
    segments: &[crate::segmenter::SegmentedCommand],
    registry: &CommandRegistry,
) -> FxHashMap<String, (SpecKey, u32)> {
    let mut aliases: FxHashMap<String, (SpecKey, u32)> = FxHashMap::default();
    if !command_is_available(registry, "namespace") {
        return aliases;
    }
    // Wholesale imports (`ns::*`) and single-name imports (`ns::name`), each
    // tagged with the byte offset of its `namespace import` statement so an
    // alias only applies to heads at or after it (source order).  Only
    // top-level imports are seen — a `namespace import` nested inside a
    // `namespace eval` body is not a top-level segment, so it never leaks a
    // global bare alias.
    let mut import_all: Vec<(String, u32)> = Vec::new();
    let mut import_one: Vec<(String, String, u32)> = Vec::new();
    for seg in segments {
        if seg.texts.len() < 3 || seg.texts[0] != "namespace" || seg.texts[1] != "import" {
            continue;
        }
        let import_off = seg.argv[0].span.start();
        for pat in &seg.texts[2..] {
            // Skip option flags (`-force`); computed patterns are left alone.
            if pat.starts_with('-') {
                continue;
            }
            if let Some(ns) = pat.strip_suffix("::*") {
                import_all.push((ns.trim_start_matches(':').to_string(), import_off));
            } else if let Some((ns, name)) = pat.rsplit_once("::") {
                import_one.push((
                    ns.trim_start_matches(':').to_string(),
                    name.to_string(),
                    import_off,
                ));
            }
        }
    }
    if import_all.is_empty() && import_one.is_empty() {
        return aliases;
    }
    // Record `name → (spec, offset)`, keeping the *earliest* enabling
    // import when several would produce the same alias.
    let mut record = |name: String, key: SpecKey, off: u32| {
        aliases
            .entry(name)
            .and_modify(|(_, o)| *o = (*o).min(off))
            .or_insert((key, off));
    };
    // Import-all: every exported command in an imported namespace whose bare
    // tail does not already name a global command.
    if !import_all.is_empty() {
        for name in registry.command_names() {
            let Some((ns, tail)) = name.rsplit_once("::") else {
                continue;
            };
            if tail.is_empty() || command_is_available(registry, tail) {
                continue;
            }
            let ns = ns.trim_start_matches(':');
            let Some(off) = import_all
                .iter()
                .filter(|(n, _)| n == ns)
                .map(|(_, o)| *o)
                .min()
            else {
                continue;
            };
            if command_is_available(registry, name)
                && let Some(spec) = registry.get(name).filter(|s| s.is_namespace_exported)
            {
                record(tail.to_string(), SpecKey::new(spec), off);
            }
        }
    }
    // Single-name imports.
    for (ns, name, off) in &import_one {
        if command_is_available(registry, name) {
            continue;
        }
        let qualified = format!("{ns}::{name}");
        if let Some(spec) = registry
            .get(&qualified)
            .filter(|_| command_is_available(registry, &qualified))
            .filter(|s| s.is_namespace_exported)
        {
            record(name.clone(), SpecKey::new(spec), *off);
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_for(src: &str) -> CommandBindingRealm {
        let registry = tcl_registry::model::ingress::static_context_for("tcl").commands();
        document_realm_bindings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            registry,
        )
    }

    fn irules_map_for(src: &str) -> CommandBindingRealm {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        document_realm_bindings(src, tcl_dialect::DialectProfile::irules(), registry)
    }

    /// Offset just past the first line of `src`, i.e. "after statement 1".
    fn after_first_line(src: &str) -> u32 {
        u32::try_from(src.find('\n').map_or(src.len(), |i| i + 1)).unwrap_or(0)
    }

    /// The [`BindingKnowledge`] view (the R-c oracle for head-identity
    /// consumers): document facts answer `Must(Spec)` / `Must(Document)` /
    /// `Absent`; with no fact the environment answers, and the world
    /// policy (B12) decides `Absent` vs `Unknown` for an unprovided name.
    #[test]
    fn knowledge_composes_facts_over_the_environment() {
        let generation = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let context = generation.context();
        let registry = generation.commands();
        let src = "rename format origfmt\nproc lindex {args} { return MINE }\nrename lsort {}\n";
        let map = document_realm_bindings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            registry,
        );
        let end = u32::try_from(src.len()).unwrap_or(0);
        // A proven rename chain is a spec-keyed Must — the I4 licence.
        let origfmt = map.knowledge_at(context, registry, "origfmt", end);
        assert_eq!(
            origfmt.proved_spec().map(SpecKey::name),
            Some("format"),
            "a proven chain carries the registry spec"
        );
        // A user-proc takeover exists but licenses no hook.
        let lindex = map.knowledge_at(context, registry, "lindex", end);
        assert!(lindex.is_proved());
        assert_eq!(lindex.proved_spec(), None, "no catalogue semantics (I4)");
        // A deletion is honest absence.
        assert_eq!(
            map.knowledge_at(context, registry, "lsort", end),
            BindingKnowledge::Absent
        );
        // No fact: the environment resolves…
        let set = map.knowledge_at(context, registry, "set", end);
        assert_eq!(set.proved_spec().map(SpecKey::name), Some("set"));
        // …and an unprovided name under tcl9.0's open world stays Unknown
        // (a `package require` can introduce it at load time).
        assert_eq!(
            map.knowledge_at(context, registry, "no-such-cmd", end),
            BindingKnowledge::Unknown
        );
    }

    /// Under a closed world (`f5-irules`) an unprovided name is proved
    /// `Absent` — the static decidability iRules derives rather than
    /// assumes (B12 as policy over the oracle).
    #[test]
    fn a_closed_world_proves_absence() {
        let generation = tcl_registry::model::ingress::static_context_for("f5-irules");
        let context = generation.context();
        let registry = generation.commands();
        let map = CommandBindingRealm::default();
        assert_eq!(
            map.knowledge_at(context, registry, "no-such-cmd", 0),
            BindingKnowledge::Absent
        );
        // The disabled mutators are likewise absent from the surface…
        assert_eq!(
            map.knowledge_at(context, registry, "interp", 0),
            BindingKnowledge::Absent
        );
        // …while a provided command proves Must.
        assert!(
            map.knowledge_at(context, registry, "when", 0)
                .proved_spec()
                .is_some()
        );
    }

    #[test]
    fn static_rename_moves_the_identity_and_clears_the_old_name() {
        let src = "rename format origfmt\norigfmt %d 7\nformat x\n";
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(map.resolve("origfmt", at), RealmBinding::Command("format"));
        // The `::`-qualified spelling of the new name resolves alike.
        assert_eq!(
            map.resolve("::origfmt", at),
            RealmBinding::Command("format")
        );
        // The old name is gone from the rename onwards …
        assert_eq!(map.resolve("format", at), RealmBinding::Rebound);
        // … but calls *before* it still see the built-in.
        let src = "format {%08x} 42\nrename format origfmt\n";
        let map = map_for(src);
        assert_eq!(map.resolve("format", 0), RealmBinding::Command("format"));
        assert_eq!(
            map.resolve("format", after_first_line(src)),
            RealmBinding::Rebound
        );
    }

    #[test]
    fn interp_alias_aliases_only_the_argument_preserving_form() {
        let src = "interp alias {} myfmt {} format\n";
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(map.resolve("myfmt", at), RealmBinding::Command("format"));
        assert_eq!(map.resolve("::myfmt", at), RealmBinding::Command("format"));

        // Pre-bound arguments shift every index — the layout cannot be reused.
        let src = "interp alias {} pad {} format %08x\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("pad", after_first_line(src)),
            RealmBinding::Rebound
        );
    }

    #[test]
    fn dynamic_bindings_abstain() {
        for src in [
            "rename $old myfmt\n",
            "interp alias {} $n {} format\n",
            "interp alias {} myfmt {} $t\n",
        ] {
            let map = map_for(src);
            let at = after_first_line(src);
            assert_eq!(
                map.resolve("myfmt", at),
                RealmBinding::Command("myfmt"),
                "dynamic binding must not state a fact: {src}"
            );
            // A dynamic `rename` must not claim the *old* name is gone either.
            assert_eq!(map.resolve("format", at), RealmBinding::Command("format"));
        }
    }

    #[test]
    fn a_child_interpreter_alias_changes_nothing_here() {
        let src = "interp alias slave myfmt {} format\n";
        let map = map_for(src);
        assert!(map.is_empty(), "a foreign srcPath states no fact here");
    }

    /// FP guard — a **safe / sub-interpreter** command table is not this
    /// document's own, so hiding or exposing a command there states nothing
    /// about a bare call here.
    ///
    /// tclsh-proof (9.0.4 / 8.6.16): `interp create -safe s; interp hide s
    /// format` leaves the parent's `format %d 7` answering `7`, while
    /// `s eval {format %d 7}` fails `invalid command name "format"`.
    #[test]
    fn a_safe_interpreters_hidden_command_states_nothing_here() {
        for src in [
            "interp create -safe s\n",
            "interp hide s format\n",
            "interp expose s format\n",
            "interp invokehidden s format %d 7\n",
        ] {
            let map = map_for(src);
            assert!(
                map.is_empty(),
                "a child interpreter's command table must state nothing here: {src}"
            );
        }
    }

    /// TN — a malformed call states nothing rather than half a fact.
    #[test]
    fn malformed_binding_calls_abstain() {
        for src in [
            // Arity too short for a rename / an alias creation.
            "rename\n",
            "rename format\n",
            "interp alias\n",
            "interp alias {}\n",
            // Not the alias subcommand at all (`aliases` merely queries).
            "interp aliases {}\n",
            // `proc` with no name.
            "proc\n",
        ] {
            let map = map_for(src);
            assert_eq!(
                map.resolve("format", u32::MAX),
                RealmBinding::Command("format"),
                "a malformed binding must not disturb `format`: {src}"
            );
        }
    }

    #[test]
    fn a_foreign_target_path_rebinds_without_aliasing() {
        let src = "interp alias {} myfmt slave format\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("myfmt", after_first_line(src)),
            RealmBinding::Rebound
        );
    }

    #[test]
    fn a_top_level_proc_shadows_the_builtin_it_names() {
        let src = "proc format {args} { return USER }\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("format", after_first_line(src)),
            RealmBinding::Rebound
        );
        // A proc that shadows nothing states nothing.
        let src = "proc mything {args} { return 1 }\n";
        assert!(map_for(src).is_empty());
        // A qualified proc defines a different command entirely.
        let src = "proc ::ns::format {args} { return 1 }\n";
        assert!(map_for(src).is_empty());
    }

    #[test]
    fn a_nested_binding_is_not_a_document_wide_fact() {
        // Neither the conditional rename nor the namespaced proc is an
        // unconditional top-level statement.
        for src in [
            "if {$x} { rename format origfmt }\n",
            "namespace eval ::n { proc format {a} { return 1 } }\n",
            "proc p {} { rename format origfmt }\n",
            "eval { rename format origfmt }\n",
            "uplevel #0 { rename format origfmt }\n",
            "interp eval $child { rename format origfmt }\n",
        ] {
            assert!(map_for(src).is_empty(), "nested binding leaked: {src}");
        }
    }

    #[test]
    fn a_qualified_mutator_head_states_the_same_fact() {
        // C Tcl resolves `::rename` to `::rename` — the mutator's own spelling
        // must not be a false negative either (issue #1185).
        let src = "::rename format origfmt\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("origfmt", after_first_line(src)),
            RealmBinding::Command("format")
        );
    }

    #[test]
    fn the_latest_applicable_fact_wins() {
        let src = "rename format origfmt\norigfmt %d 7\nproc origfmt {args} { return 1 }\n";
        let map = map_for(src);
        let after_all = u32::try_from(src.len()).unwrap_or(0);
        // Between the `rename` and the `proc`, `origfmt` *is* the built-in …
        assert_eq!(
            map.resolve("origfmt", after_first_line(src)),
            RealmBinding::Command("format")
        );
        // … and after the `proc` takes the name back it is a user command,
        // even though `origfmt` is not itself a registry name.
        assert_eq!(map.resolve("origfmt", after_all), RealmBinding::Rebound);
    }

    /// Issue #1275's "chained bindings do not compose" residual.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical):
    ///
    /// ```tcl
    /// interp alias {} a {} format ; rename a b ; b %08x 42   ;# 0000002a
    /// info commands a                                        ;# (empty)
    /// rename lindex li1 ; rename li1 li2 ; li2 {x y z} 2      ;# z
    /// rename lsort mysort ; interp alias {} sorter {} mysort
    /// sorter {c a b}                                          ;# a b c
    /// ```
    #[test]
    fn chained_bindings_compose() {
        // alias → rename
        let src = "interp alias {} a {} format\nrename a b\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("b", end), RealmBinding::Command("format"));
        assert_eq!(map.resolve("::b", end), RealmBinding::Command("format"));
        // The intermediate name is gone once the rename moves it.
        assert_eq!(map.resolve("a", end), RealmBinding::Rebound);

        // rename → rename
        let src = "rename lindex li1\nrename li1 li2\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("li2", end), RealmBinding::Command("lindex"));

        // rename → alias
        let src = "rename lsort mysort\ninterp alias {} sorter {} mysort\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("sorter", end), RealmBinding::Command("lsort"));
    }

    /// A chain must not inherit a *broken* binding: the rename moves the user
    /// proc, not the built-in it shadowed, so the new name gets no grammar.
    #[test]
    fn a_chain_through_a_shadowed_builtin_stays_rebound() {
        let src = "proc format {args} { return USER }\nrename format myfmt\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("myfmt", end), RealmBinding::Rebound);
        assert_eq!(map.resolve("::myfmt", end), RealmBinding::Rebound);
    }

    /// An alias whose target is an ordinary user proc still *takes over* the
    /// name it binds — C Tcl lets an alias shadow an existing command
    /// (tclsh 8.6.16 / 9.0.4: `proc myproc …; interp alias {} lindex {} myproc`
    /// makes `lindex {a b c} 1` answer `MINE`).
    #[test]
    fn an_alias_over_a_builtin_rebinds_it() {
        let src = "proc myproc {args} { return MINE }\ninterp alias {} lindex {} myproc\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("lindex", end), RealmBinding::Rebound);
    }

    #[test]
    fn an_unpositioned_read_abstains_when_the_facts_disagree() {
        // One fact — the unpositioned read matches the positioned one.
        let src = "rename format origfmt\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve_unpositioned("origfmt"),
            RealmBinding::Command("format")
        );
        assert_eq!(map.resolve_unpositioned("format"), RealmBinding::Rebound);
        // A head nothing binds keeps its own spelling.
        assert_eq!(
            map.resolve_unpositioned("lindex"),
            RealmBinding::Command("lindex")
        );

        // Two facts that disagree — without a position, neither can be chosen.
        let src = "rename format origfmt\nproc origfmt {args} { return 1 }\n";
        let map = map_for(src);
        assert_eq!(map.resolve_unpositioned("origfmt"), RealmBinding::Rebound);
    }

    #[test]
    fn the_shared_empty_map_binds_nothing() {
        let map = CommandBindingRealm::none();
        assert!(map.is_empty());
        assert_eq!(map.resolve("format", 0), RealmBinding::Command("format"));
        assert_eq!(
            map.resolve_unpositioned("format"),
            RealmBinding::Command("format")
        );
    }

    #[test]
    fn a_rebound_head_resolves_to_no_registry_spec() {
        // The `Rebound` sentinel must be unresolvable, since that is what makes
        // every registry query answer "unknown" without a variant check.
        let registry = tcl_registry::model::ingress::static_context_for("tcl").commands();
        assert!(registry.get(RealmBinding::Rebound.spec_name()).is_none());
    }

    #[test]
    fn irules_proc_identity_requires_a_closed_declaration_body() {
        // In iRules, unlike generic Tcl, only a real file-level declaration
        // takes over a command identity.  These malformed spellings must not
        // hide the following event handler.
        for src in [
            "proc when {} bare\nwhen HTTP_REQUEST {}\n",
            "proc when {} \"quoted\"\nwhen HTTP_REQUEST {}\n",
            "proc when {} {closed} trailing\nwhen HTTP_REQUEST {}\n",
            // The unterminated body consumes the tail as Tcl recovery does;
            // it still must not leave a persistent `when` rebinding behind.
            "proc when {} {unterminated",
        ] {
            let map = irules_map_for(src);
            assert_eq!(
                map.resolve("when", u32::MAX),
                RealmBinding::Command("when"),
                "malformed iRules proc poisoned `when`: {src:?}"
            );
        }

        let valid = irules_map_for("proc when {args} {}\nwhen HTTP_REQUEST {}\n");
        assert_eq!(valid.resolve("when", u32::MAX), RealmBinding::Rebound);

        // Generic Tcl deliberately preserves its existing `proc` binding
        // behaviour; the iRules declaration gate is profile-specific.
        let generic = map_for("proc format {} bare\nformat %d 1\n");
        assert_eq!(generic.resolve("format", u32::MAX), RealmBinding::Rebound);
    }

    #[test]
    fn unavailable_irules_mutators_do_not_change_event_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let profile = registry.profile().expect("dialect registry has a profile");
        for command in ["interp", "rename", "namespace"] {
            assert!(
                registry
                    .get_for_dialect(command, profile.availability_mask)
                    .is_none(),
                "F5 K36322151 disables {command} in iRules"
            );
        }
        let events = tcl_registry::events::EventRegistry::build();
        let profiles = tcl_registry::profiles::ProfileRegistry::build();
        // F5 K36322151: iRules disables `interp`, `rename`, and `namespace`.
        // Their Tcl shapes are data/error commands here, never identity facts.
        let source = "interp alias {} event {} when\nrename when event\nnamespace import ::x::*\nwhen HTTP_REQUEST {}\n";
        let identities =
            document_realm_bindings(source, tcl_dialect::DialectProfile::irules(), registry);
        assert!(
            identities.is_empty(),
            "disabled commands must not produce command-identity facts"
        );
        let inferred =
            tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
                source,
                &events,
                &profiles,
                registry,
                &identities,
            );
        assert!(inferred.contains(&"HTTP".to_owned()));

        // `proc` is available, so a genuine, executable rebinding still
        // removes the registry event-handler grammar.
        let rebound = "proc when {args} {}\nwhen HTTP_REQUEST {}\n";
        let identities =
            document_realm_bindings(rebound, tcl_dialect::DialectProfile::irules(), registry);
        let inferred =
            tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
                rebound,
                &events,
                &profiles,
                registry,
                &identities,
            );
        assert!(!inferred.contains(&"HTTP".to_owned()));
    }
}
