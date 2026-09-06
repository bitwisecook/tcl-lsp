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

//! Intermediate representation (IR) for Tcl analysis.
//!
//! This is a structured IR front-end that keeps source [`Span`]s on all
//! nodes. Later passes lower this to CFG + SSA.
//!
//! **Architectural note:** every IR node carries a [`Span`] (two `u32`
//! byte offsets) rather than inline `(line, character, offset)` pairs.
//! Text and positions are resolved on demand via a
//! [`SourceMap`](tcl_lexer::SourceMap). This mirrors the span-first
//! design established in the lexer crate.

use tcl_lexer::{LexerConfig, SourceMap, Span, Token};

use crate::expr_ast::ExprNode;
use crate::segmenter::SegmentedCommand;

// Command tokens

/// A stable, function-local identity for a semantic IR node.
///
/// A node ID is a structural path, not a source offset.  Rebase operations
/// therefore preserve it, which is essential for compilation-unit caching and
/// for eventual guard/deoptimisation maps.  The ID is intentionally local to a
/// executable-semantic function; a procedure name or other function key
/// supplies the enclosing identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(Vec<u32>);

impl NodeId {
    /// Create an ID from its deterministic structural path.
    ///
    /// This is public for IR transforms that preserve a node's identity. New
    /// source nodes should be allocated by the semantic-IR builder instead of
    /// manufacturing paths ad hoc.
    #[must_use]
    pub fn from_path(path: Vec<u32>) -> Self {
        Self(path)
    }

    /// Return the path segments that make up this function-local ID.
    #[must_use]
    pub fn path(&self) -> &[u32] {
        &self.0
    }
}

/// Why an IR site exists at a source location.
///
/// The source variant is used by the current lowering compatibility layer.
/// Transforming passes can retain the relationship to their source node(s)
/// through [`Self::Derived`] without inventing a misleading source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Provenance {
    /// Directly lowered from user-authored Tcl source.
    #[default]
    Source,
    /// Created by a semantic transform from one or more source nodes.
    Derived {
        /// Category of semantic transform that made this node.
        kind: DerivedKind,
        /// The source nodes that justify this derived node.
        origins: Vec<NodeId>,
    },
    /// A conservative compatibility placeholder for a hand-built or lossy
    /// IR node whose source origin cannot be established.
    Opaque,
}

/// The transform class for [`Provenance::Derived`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DerivedKind {
    /// Desugared from an equivalent source construct.
    Desugared,
    /// Produced by constant folding.
    Folded,
    /// Cloned into a caller by inlining.
    Inlined,
    /// Created by a guarded specialisation.
    Specialised,
    /// Created by another named semantic transform.
    Synthetic,
}

/// A source range together with the reason the IR associates work with it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSite {
    /// Byte span in the original source when known.
    pub span: Span,
    /// Origin relationship for this site.
    pub provenance: Provenance,
}

impl SourceSite {
    /// Build a direct user-source site.
    #[must_use]
    pub const fn source(span: Span) -> Self {
        Self {
            span,
            provenance: Provenance::Source,
        }
    }

    /// Build a transformed site while preserving its source relationship.
    #[must_use]
    pub fn derived(span: Span, kind: DerivedKind, origins: Vec<NodeId>) -> Self {
        Self {
            span,
            provenance: Provenance::Derived { kind, origins },
        }
    }

    /// Build an explicitly opaque compatibility site.
    #[must_use]
    pub const fn opaque(span: Span) -> Self {
        Self {
            span,
            provenance: Provenance::Opaque,
        }
    }
}

/// A lexical word expression retained by the semantic-IR compatibility layer.
///
/// This preserves ordering and source ranges for substitutions that the lexer
/// identifies.  It deliberately does **not** claim to fully model Tcl's
/// backslash substitution or all parser recovery forms: those remain
/// [`Self::Opaque`] until a later executable lowering can give them an exact
/// evaluation model.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WordExpr {
    /// A single bare word with no substitutions or backslash processing.
    Literal {
        /// Compatibility argv spelling.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A single braced word, whose content undergoes no substitution.
    BracedLiteral {
        /// De-braced word content.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A single variable substitution word.
    Variable {
        /// Compatibility argv spelling, including a Tcl variable wrapper.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A single command-substitution word.
    CommandSubstitution {
        /// Compatibility argv spelling, including `[` and `]` where present.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A compound word whose parts are evaluated left-to-right.
    Template {
        /// Ordered lexical parts.
        parts: Vec<WordPart>,
        /// Full source range of the word.
        source: SourceSite,
    },
    /// Tcl 8.5+ argument expansion around another word expression.
    Expand {
        /// Source range from the `{*}` marker through the inner word.
        source: SourceSite,
        /// Expanded word.
        word: Box<WordExpr>,
    },
    /// A word whose exact substitution behaviour is not represented by the
    /// legacy token snapshot.
    Opaque {
        /// Compatibility argv text.
        text: String,
        /// Best available source site.
        source: SourceSite,
        /// Why the semantic layer must not infer stronger meaning yet.
        reason: WordOpacity,
    },
}

impl WordExpr {
    /// Return this word's source site.
    #[must_use]
    pub const fn source(&self) -> &SourceSite {
        match self {
            Self::Literal { source, .. }
            | Self::BracedLiteral { source, .. }
            | Self::Variable { source, .. }
            | Self::CommandSubstitution { source, .. }
            | Self::Template { source, .. }
            | Self::Expand { source, .. }
            | Self::Opaque { source, .. } => source,
        }
    }

    /// Recover the compatibility argv text used by existing consumers.
    #[must_use]
    pub fn legacy_text(&self) -> String {
        match self {
            Self::Literal { text, .. }
            | Self::BracedLiteral { text, .. }
            | Self::Opaque { text, .. } => text.clone(),
            Self::Variable { spelling, .. } | Self::CommandSubstitution { spelling, .. } => {
                spelling.clone()
            }
            Self::Template { parts, .. } => parts.iter().map(WordPart::legacy_text).collect(),
            Self::Expand { word, .. } => word.legacy_text(),
        }
    }

    fn from_lossy_snapshot(
        text: &str,
        span: Span,
        kind: tcl_lexer::TokenType,
        single_token: bool,
        expanded: bool,
    ) -> Self {
        let source = SourceSite::opaque(span);
        let word = if single_token {
            match kind {
                tcl_lexer::TokenType::Str => Self::BracedLiteral {
                    text: text.to_owned(),
                    source,
                },
                tcl_lexer::TokenType::Var => Self::Variable {
                    spelling: text.to_owned(),
                    source,
                },
                tcl_lexer::TokenType::Cmd => Self::CommandSubstitution {
                    spelling: text.to_owned(),
                    source,
                },
                _ => Self::Opaque {
                    text: text.to_owned(),
                    source,
                    reason: WordOpacity::LossySnapshot,
                },
            }
        } else {
            Self::Opaque {
                text: text.to_owned(),
                source,
                reason: WordOpacity::LossySnapshot,
            }
        };
        if expanded {
            Self::Expand {
                source: SourceSite::opaque(span),
                word: Box::new(word),
            }
        } else {
            word
        }
    }
}

/// One ordered lexical component of a [`WordExpr::Template`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WordPart {
    /// A plain or escaped text fragment. Backslash semantics remain explicit
    /// but unevaluated at this compatibility boundary.
    Text {
        /// Compatibility argv spelling.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A variable substitution.
    Variable {
        /// Compatibility argv spelling.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A command substitution.
    CommandSubstitution {
        /// Compatibility argv spelling.
        spelling: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
    /// A recovery or otherwise unmodelled token fragment.
    Opaque {
        /// Compatibility argv spelling.
        text: String,
        /// Exact lexical source site.
        source: SourceSite,
    },
}

impl WordPart {
    fn legacy_text(&self) -> String {
        match self {
            Self::Text { text, .. } | Self::Opaque { text, .. } => text.clone(),
            Self::Variable { spelling, .. } | Self::CommandSubstitution { spelling, .. } => {
                spelling.clone()
            }
        }
    }
}

/// Why a [`WordExpr::Opaque`] node cannot make stronger semantic claims yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordOpacity {
    /// The compatibility snapshot lacked ordered lexical fragments.
    MissingFragments,
    /// The snapshot was synthetically constructed or retained only its legacy
    /// representative-token fields.
    LossySnapshot,
    /// C Tcl's parser rejects the word; the payload is its exact message
    /// (`missing close-bracket`, `missing )`, …) for diagnostics.
    ParseError(&'static str),
    /// The word carries a dialect substitution the word-component owner does
    /// not model — today only `JimTcl`'s `$(expr)` (`TokenType::ExprSugar`).
    ///
    /// `word_parts::decompose` applies Tcl's `$` rules, under which `$(` is
    /// ordinary text, so such a word would otherwise decompose to a single
    /// `Text` part and be promoted to [`WordExpr::Literal`] — and lowering
    /// would emit the *spelling* `$(1+2)` instead of evaluating it. The word
    /// is opaque instead, which is what the pre-#1785 fragment walk produced
    /// (a `Template` whose only part was opaque).
    DialectSubstitution,
}

/// A statement the CFG builder **synthesised**: it models an analysis effect
/// or a codegen placeholder, and is never a Tcl command to run.
///
/// A [`Statement::Barrier`] normally *is* a command whose side effects defeat
/// static analysis (`eval`, a dynamic-body `catch`, `return -code …`), so
/// codegen dispatches it. The CFG builder also inserts statements that carry
/// only a `defs` list or a widening effect, and codegen must never emit an
/// invoke for one of *those*: before issue #1602's fix the caller-frame
/// widening reused the callee's own name, so
/// `proc p {} { upvar 1 {a b} v ; puts "u=$v" }; p` invoked `p` twice and
/// printed its body's output twice where tclsh 8.6.16 / 9.0.4 print it once.
///
/// The identity has to be **typed**, which is why this enum exists rather than
/// a list of reserved command spellings. A Tcl command name is an arbitrary
/// string, so every spelling the marker set used is a name a script may
/// legally define and call, and a name-based check silently dropped such a
/// call. Verified identical on tclsh 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1:
///
/// ```text
/// proc <cond> {} { puts "hit-cond" }
/// proc <caller-frame-opaque> {} { puts "hit-cfo" }
/// proc <global-frame-script> {} { puts "hit-gfs" }
/// proc <upvar-invalidate> {} { puts "hit-uv" }
/// proc <empty_clause> {} { puts "hit-ec" }
/// <cond> ; <caller-frame-opaque> ; <global-frame-script>
/// <upvar-invalidate> ; <empty_clause>
/// ->  hit-cond / hit-cfo / hit-gfs / hit-uv / hit-ec   (all five run)
/// ```
///
/// Not even the empty name is reservable — `proc {} {} { puts EMPTY-NAME-RAN
/// }; {}` prints `EMPTY-NAME-RAN` on all five releases — so no string sentinel
/// can carry this. A marker is recognised only through
/// [`CommandTokens::synthetic`], which both [`CommandTokens`] constructors
/// always leave `None`; nothing a script can write reaches
/// [`CommandTokens::marker`]. The statement keeps a readable `command`
/// spelling for the disassembly and the explorer — it is a label, not the
/// identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntheticMarker {
    /// The `if` / `while` condition kill-set: a [`Statement::Call`] whose
    /// `defs` are the variables the condition's own substitutions write, so a
    /// value-motion pass cannot carry a stale value across the branch.
    Condition,
    /// An empty `for` init / next clause. Codegen emits the three `nop`s
    /// tclsh's bytecode has there, so the clause keeps its place in the
    /// instruction stream.
    EmptyClause,
    /// Caller-side `defs` for an `[upvar_proc …]` embedded in a word, carried
    /// on a statement of its own because the host statement cannot hold them.
    UpvarInvalidate,
    /// A callee that runs an unreadable script at the global frame
    /// (`uplevel #0 $body`, issue #1198): it can write any global or namespace
    /// name, so the site widens instead of enumerating defs.
    GlobalFrameScript,
    /// A callee whose caller-frame `upvar` alias cannot be placed
    /// (`upvar 1 $computed x`): it can write any caller variable, so the call
    /// site widens. It sits *beside* the call it widens for, which is why
    /// naming the callee on it made codegen run the callee twice (issue #1602).
    CallerFrameOpaque,
}

/// Original parsed tokens for a command invocation.
///
/// Carried on [`Statement::Call`] and [`Statement::Barrier`] so
/// downstream passes (optimiser, compiler checks) can inspect tokens
/// without re-lexing.
///
/// Token references use [`Span`]s into the
/// source buffer. The `argv_texts` and `single_token_word` fields
/// preserve the per-word metadata needed by analysis passes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTokens {
    /// Per-word representative token spans.
    pub argv: Vec<Span>,
    /// Per-word text values.
    pub argv_texts: Vec<String>,
    /// Structured per-word syntax derived once during segmentation/lowering.
    ///
    /// [`Self::argv_texts`] and the representative-token arrays remain the
    /// compatibility view consumed by existing analyser and codegen passes.
    /// The semantic IR uses this ordered representation so it never needs to
    /// re-lex a Tcl word or infer substitution order from a flattened string.
    pub word_exprs: Vec<WordExpr>,
    /// Per-word representative token kind. Preserves the [`tcl_lexer::TokenType`]
    /// of each argv entry so analysis passes can distinguish a
    /// brace-string literal (`Str`) from a bareword (`Esc`), a
    /// variable reference (`Var`), or a command substitution
    /// (`Cmd`).
    pub argv_kinds: Vec<tcl_lexer::TokenType>,
    /// Whether each word consists of a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Span>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
    /// The [`SyntheticMarker`] this statement *is*, when the CFG builder
    /// synthesised it to carry an effect rather than a command to run.
    ///
    /// `None` for everything that came from a Tcl word — both constructors
    /// hard-code it, so the only way to set it is [`Self::marker`], which no
    /// lowering path calls. That is the whole point: command *names* are
    /// forgeable (see [`SyntheticMarker`]), a private constructor is not.
    pub synthetic: Option<SyntheticMarker>,
}

impl CommandTokens {
    /// Build a token snapshot from the segmenter without losing word shape.
    ///
    /// `sm` is the buffer the segment's spans index into — the document, or a
    /// sub-lexed slice carrying its document base
    /// ([`SourceMap::with_base`]) — and `config` is that document's grammar,
    /// resolved once at the ingress and threaded here so every word is
    /// decomposed under the release it is written for.
    #[must_use]
    pub fn from_segmented(sm: &SourceMap<'_>, config: LexerConfig, seg: &SegmentedCommand) -> Self {
        let expand = seg.expand_word.as_deref();
        // One reused buffer: `WordExpr::from_word` takes the word's lexer
        // tokens, and the segmenter keeps them behind its own fragment type.
        let mut fragments: Vec<Token> = Vec::new();
        let mut word_exprs = Vec::with_capacity(seg.argv.len());
        for (idx, token) in seg.argv.iter().enumerate() {
            fragments.clear();
            fragments.extend(
                seg.word_fragments
                    .get(idx)
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(|fragment| fragment.token),
            );
            let text = seg.texts.get(idx).map_or("", String::as_str);
            let expanded = expand
                .and_then(|flags| flags.get(idx))
                .copied()
                .unwrap_or(false);
            let expansion_span = expanded
                .then(|| {
                    seg.all_tokens
                        .iter()
                        .rev()
                        .find(|candidate| {
                            candidate.kind == tcl_lexer::TokenType::Expand
                                && candidate.span.end() <= token.span.start()
                        })
                        .map(|candidate| candidate.span)
                })
                .flatten();
            word_exprs.push(WordExpr::from_word(
                sm,
                config,
                &fragments,
                text,
                expanded,
                expansion_span,
            ));
        }
        Self {
            argv: seg.argv.iter().map(|token| token.span).collect(),
            argv_texts: seg.texts.clone(),
            word_exprs,
            argv_kinds: seg.argv.iter().map(|token| token.kind).collect(),
            single_token_word: seg.single_token_word.clone(),
            all_tokens: seg.all_tokens.iter().map(|token| token.span).collect(),
            expand_word: seg.expand_word.clone(),
            // Came from a Tcl word, so it is a command however it is spelled.
            synthetic: None,
        }
    }

    /// A token snapshot for a statement the CFG builder synthesised — see
    /// [`SyntheticMarker`] for why the marker has to live here rather than in
    /// the statement's `command` spelling.
    ///
    /// The word vectors are empty: a marker has no words, and
    /// [`Self::words_align_with_argv_text`] holds trivially. Consumers that
    /// need real word data (the WASM source-compatibility view) already
    /// decline a snapshot they cannot line up, which is the conservative
    /// direction.
    #[must_use]
    pub fn marker(kind: SyntheticMarker) -> Self {
        Self {
            argv: Vec::new(),
            argv_texts: Vec::new(),
            word_exprs: Vec::new(),
            argv_kinds: Vec::new(),
            single_token_word: Vec::new(),
            all_tokens: Vec::new(),
            expand_word: None,
            synthetic: Some(kind),
        }
    }

    /// Build compatibility token data when only the historical parallel
    /// arrays are available.
    ///
    /// New source lowering should use [`Self::from_segmented`]. This adapter
    /// intentionally produces opaque word nodes for data that cannot preserve
    /// ordered fragments; it never guesses Tcl substitution semantics.
    #[must_use]
    pub fn from_lossy_parts(
        argv: Vec<Span>,
        argv_texts: Vec<String>,
        argv_kinds: Vec<tcl_lexer::TokenType>,
        single_token_word: Vec<bool>,
        all_tokens: Vec<Span>,
        expand_word: Option<Vec<bool>>,
    ) -> Self {
        let word_exprs = argv_texts
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                let span = argv.get(idx).copied().unwrap_or_else(|| Span::new(0, 0));
                let kind = argv_kinds
                    .get(idx)
                    .copied()
                    .unwrap_or(tcl_lexer::TokenType::Esc);
                let single = single_token_word.get(idx).copied().unwrap_or(false);
                let expanded = expand_word
                    .as_ref()
                    .and_then(|flags| flags.get(idx))
                    .copied()
                    .unwrap_or(false);
                WordExpr::from_lossy_snapshot(text, span, kind, single, expanded)
            })
            .collect();
        Self {
            argv,
            argv_texts,
            word_exprs,
            argv_kinds,
            single_token_word,
            all_tokens,
            expand_word,
            // A lossy snapshot of real words is still a command.
            synthetic: None,
        }
    }

    /// Structured word expressions in argv order.
    #[must_use]
    pub fn words(&self) -> &[WordExpr] {
        &self.word_exprs
    }

    /// Whether the structured compatibility view still aligns with argv text.
    ///
    /// Synthetic codegen snapshots may intentionally omit representative spans
    /// or argv text, so this checks the one invariant the semantic sidecar
    /// requires rather than imposing a stronger invariant on historical
    /// consumers.
    #[must_use]
    pub fn words_align_with_argv_text(&self) -> bool {
        self.word_exprs.len() == self.argv_texts.len()
    }

    /// Whether **argument** `idx` (0-based, so `argv[idx + 1]`) is a
    /// brace-quoted literal word — `{…}`, the one word form Tcl leaves
    /// completely unsubstituted.
    ///
    /// The IR stores argument *content*, which cannot tell `{$a}` (the
    /// literal three-character name `$a`) from `$a` (a substitution); the
    /// per-word token kind can.  A `$var` lexes to `Var`, a `[cmd]` to `Cmd`,
    /// a `"…"` (which *does* substitute) to `Esc`, and only a brace-quoted
    /// word to `Str`.
    ///
    /// The single-token conjunct is belt-and-braces: a word that opens with
    /// `{` must be entirely brace-quoted (`set x {a}{b}` is a tclsh parse
    /// error, "extra characters after close-brace"), so a `Str`
    /// representative token already implies a single-token word.
    ///
    /// The one definition of this question in the compiler — the dynamic-name
    /// barrier, the place bridge, the inliner's literal-argument binding and
    /// the def/use naming layer all ask it here rather than re-deriving it
    /// (issue #1078).
    #[must_use]
    pub fn arg_is_braced_literal(&self, idx: usize) -> bool {
        self.argv_kinds.get(idx + 1) == Some(&tcl_lexer::TokenType::Str)
            && self.single_token_word.get(idx + 1).copied().unwrap_or(true)
    }
}

// IR node types
//
// IR node kinds are modelled as a flat enum with struct variants.
// Each variant carries a `Span` for position tracking, plus
// variant-specific fields.

/// Command-resolution context for an executable script root or nested body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionNamespace {
    /// The selected frame's defining namespace is statically known.
    Exact(String),
    /// A receiver- or caller-selected namespace is known only at runtime.
    RuntimeSelected,
}

impl ExecutionNamespace {
    /// Construct an exact command-resolution namespace.
    #[must_use]
    pub fn exact(namespace: impl Into<String>) -> Self {
        Self::Exact(namespace.into())
    }

    /// Resolve one command head. Absolute names remain exact even in a
    /// runtime-selected frame; relative names do not.
    #[must_use]
    pub fn for_head(&self, head: &str) -> Option<&str> {
        if head.starts_with("::") {
            Some("::")
        } else {
            match self {
                Self::Exact(namespace) => Some(namespace),
                Self::RuntimeSelected => None,
            }
        }
    }

    pub(crate) fn selected_frame(&self, absolute: bool, frame_shift: i32) -> Self {
        match (absolute, frame_shift) {
            (true, 0) => Self::exact("::"),
            (false, 0) => self.clone(),
            _ => Self::RuntimeSelected,
        }
    }
}

/// A script: a sequence of IR statements.
///
/// Using `Vec` rather than a tuple because scripts are built incrementally
/// during lowering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Script {
    /// Statements in execution order.
    pub statements: Vec<Statement>,
    /// Registry-resolved command heads consumed by typed lowering.
    ///
    /// Keeping these sites beside the script makes the dependency explicit in
    /// IR and lets CFG/codegen preserve it without re-parsing source fragments.
    pub command_binding_sites: CommandBindingSites,
    /// Exact user-procedure definitions whose bodies were copied into this
    /// script by an executable inlining transform.
    pub procedure_binding_requirements: ProcedureBindingRequirements,
}

/// Compact storage for the uncommon typed-lowering dependency sidecar.
///
/// Most scripts contain no command whose head disappears during structured
/// lowering. A boxed slice omits `Vec`'s unused capacity word and avoids
/// inflating every recursively nested [`Script`] (and the [`Statement`]
/// variants which contain several of them).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CommandBindingSites(Box<[CommandBindingSite]>);

impl CommandBindingSites {
    /// Iterate over the retained typed-lowering dependencies.
    pub fn iter(&self) -> impl Iterator<Item = &CommandBindingSite> {
        self.0.iter()
    }

    /// Mutably iterate over the retained typed-lowering dependencies.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CommandBindingSite> {
        self.0.iter_mut()
    }
}

/// Compact storage for user-procedure dependencies introduced by inlining.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ProcedureBindingRequirements(Box<[tcl_runtime_api::ProcedureBindingIdentity]>);

impl ProcedureBindingRequirements {
    /// Iterate over the exact user-procedure dependencies.
    pub fn iter(&self) -> impl Iterator<Item = &tcl_runtime_api::ProcedureBindingIdentity> {
        self.0.iter()
    }
}

impl From<Vec<tcl_runtime_api::ProcedureBindingIdentity>> for ProcedureBindingRequirements {
    fn from(requirements: Vec<tcl_runtime_api::ProcedureBindingIdentity>) -> Self {
        Self(requirements.into_boxed_slice())
    }
}

impl From<Vec<CommandBindingSite>> for CommandBindingSites {
    fn from(sites: Vec<CommandBindingSite>) -> Self {
        Self(sites.into_boxed_slice())
    }
}

/// One command dependency retained after typed lowering consumes its head.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandBindingSite {
    /// Exact source span of the complete owning Tcl command.
    pub span: Span,
    /// Source spelling and registry-resolved implementation identity.
    pub binding: tcl_runtime_api::CommandBindingIdentity,
}

impl Script {
    /// Create an empty script.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
            command_binding_sites: CommandBindingSites::default(),
            procedure_binding_requirements: ProcedureBindingRequirements::default(),
        }
    }

    /// Create a script from a list of statements.
    #[must_use]
    pub fn from_statements(statements: Vec<Statement>) -> Self {
        Self {
            statements,
            command_binding_sites: CommandBindingSites::default(),
            procedure_binding_requirements: ProcedureBindingRequirements::default(),
        }
    }

    /// Create a lowered script with explicit typed-lowering dependencies.
    #[must_use]
    pub fn from_lowered_parts(
        statements: Vec<Statement>,
        command_binding_sites: Vec<CommandBindingSite>,
    ) -> Self {
        Self {
            statements,
            command_binding_sites: command_binding_sites.into(),
            procedure_binding_requirements: ProcedureBindingRequirements::default(),
        }
    }

    /// Create transformed executable IR with both registry and user-procedure
    /// binding dependencies retained explicitly.
    #[must_use]
    pub fn from_transformed_parts(
        statements: Vec<Statement>,
        command_binding_sites: Vec<CommandBindingSite>,
        procedure_binding_requirements: Vec<tcl_runtime_api::ProcedureBindingIdentity>,
    ) -> Self {
        Self {
            statements,
            command_binding_sites: command_binding_sites.into(),
            procedure_binding_requirements: procedure_binding_requirements.into(),
        }
    }
}

/// Depth cap for [`for_each_statement`]'s recursion over nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`uplevel` bodies —
/// issue #996. Transitively bounded today via `MAX_LOWER_NEST_DEPTH` (every
/// `Script` this crate hands to `for_each_statement` was built by
/// [`crate::lowering`], which already caps its own construction at 256 and
/// emits a `Statement::Barrier` past that point), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker in
/// this crate (`optimiser::MAX_OPTIMISER_WALK_DEPTH`,
/// `codegen::structured::MAX_STRUCTURED_DEPTH`), since this is a `pub`
/// utility any future caller could hand a `Script` built some other way.
const MAX_FOR_EACH_STATEMENT_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Recursively visit every statement in `script`, including nested bodies
/// (`if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`), in source order.
/// Each statement is visited before its own nested bodies (pre-order): a
/// compound statement (`If`, `For`, …) is passed to `visit` once for itself,
/// then its nested scripts are walked in turn.
///
/// Shared traversal for whole-module / whole-body scans that need "every
/// statement anywhere in this script" without each re-deriving the same
/// nested-body match arms (e.g. [`crate::var_observability::scan_module_global_names`]).
///
/// Nesting past [`MAX_FOR_EACH_STATEMENT_DEPTH`] (256) is silently not
/// descended into — `visit` still runs for every statement up to and
/// including that depth, matching this function's existing "best-effort
/// scan" contract (callers already tolerate this visitor skipping statements
/// it cannot reach, e.g. inside opaque `Statement::Barrier`s).
pub fn for_each_statement(script: &Script, visit: &mut impl FnMut(&Statement)) {
    for_each_statement_inner(script, visit, 0);
}

fn for_each_statement_inner(script: &Script, visit: &mut impl FnMut(&Statement), depth: u32) {
    if MAX_FOR_EACH_STATEMENT_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        visit(stmt);
        match stmt {
            Statement::Block { body, .. }
            | Statement::UpFrame { body, .. }
            | Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => {
                for_each_statement_inner(body, visit, depth + 1);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    for_each_statement_inner(&c.body, visit, depth + 1);
                }
                if let Some(b) = else_body {
                    for_each_statement_inner(b, visit, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                for_each_statement_inner(init, visit, depth + 1);
                for_each_statement_inner(next, visit, depth + 1);
                for_each_statement_inner(body, visit, depth + 1);
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                for_each_statement_inner(body, visit, depth + 1);
                for h in handlers {
                    for_each_statement_inner(&h.body, visit, depth + 1);
                }
                if let Some(fb) = finally_body {
                    for_each_statement_inner(fb, visit, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        for_each_statement_inner(b, visit, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    for_each_statement_inner(b, visit, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// An `if` clause: condition + body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfClause {
    /// The parsed condition expression.
    pub condition: ExprNode,
    /// Source span of the condition text.
    pub condition_span: Span,
    /// Absolute source offset of the condition *text*'s first byte, when
    /// the text handed to `parse_expr` is a verbatim source slice (single
    /// braced / bare word).  Expression-AST leaf offsets are relative to
    /// that text, so `condition_base + leaf.start` recovers an absolute
    /// operand span.  `None` when the text was reconstructed (quoted /
    /// multi-token words) — consumers fall back to `condition_span`.
    pub condition_base: Option<u32>,
    /// Body script to execute when the condition is true.
    pub body: Script,
    /// Source span of the body text.
    pub body_span: Span,
}

/// A `try` handler clause (`on`/`trap`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TryHandler {
    /// Handler kind: `"on"` or `"trap"`.
    pub kind: String,
    /// Return code or error class pattern to match.
    pub match_arg: String,
    /// Parsed error-code prefix for a statically literal `trap` selector.
    ///
    /// `None` means either an `on` handler, a selector containing Tcl word
    /// substitutions, or a malformed Tcl list. Literal `$`, `[`, and
    /// backslashes remain ordinary element data when protected by braces or
    /// escapes; lowering decides that from source token shape before the
    /// decoded word loses the distinction.
    pub trap_pattern: Option<Vec<String>>,
    /// Variable bound to the result, if any.
    pub var_name: Option<String>,
    /// Variable bound to the options dict, if any.
    pub options_var: Option<String>,
    /// Handler body.
    pub body: Script,
    /// Source span of the handler body.
    pub body_span: Span,
    /// `true` when the handler body is the literal `-` fallthrough
    /// marker: the handler shares the next non-fallthrough handler's
    /// body (Tcl `try` semantics, mirroring `switch`). `body` is an
    /// empty script in that case — the `-` is *not* a command call.
    pub fallthrough: bool,
}

/// A `switch` arm: pattern + body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwitchArm {
    /// Pattern text — the word's *value*.
    pub pattern: String,
    /// Whether this arm's pattern word was braced, so its value is literal.
    ///
    /// Per-arm, because the two arm forms differ per *word*: a single braced
    /// `{pat body …}` block holds literal list elements, while the multi-word
    /// form (`switch $s $pat {body}`) is a word each, and `{${x}}` there is
    /// literal while a bare `$pat` substitutes. The whole-switch
    /// `Statement::Switch::patterns_braced` cannot say that, and using it
    /// substituted a braced pattern.
    pub pattern_braced: bool,
    /// Source span of the pattern.
    pub pattern_span: Span,
    /// Body script (`None` for fall-through arms).
    pub body: Option<Script>,
    /// Source span of the body (`None` for fall-through arms).
    pub body_span: Option<Span>,
    /// Whether this arm falls through to the next.
    pub fallthrough: bool,
}

/// An iterator group in a `foreach`/`lmap`/`dict for` loop.
///
/// Each group pairs a list of loop variable names with the list
/// argument text they iterate over.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForeachIterator {
    /// Variable names assigned each iteration.
    pub vars: Vec<String>,
    /// Source text of the list argument.
    pub list_arg: String,
    /// Whether the *list* word was a brace-string literal
    /// (`foreach n {a $b c} …`). Braces suppress substitution, so a
    /// `$b` inside such a word is a literal three-character list
    /// element, not a read of `b` — tclsh 8.6.14: `foreach n {a $b c}
    /// {puts $n}` prints `a`, `$b`, `c` with `b` undefined throughout.
    ///
    /// [`list_arg`](Self::list_arg) stores the word's *content* (braces
    /// already stripped), which cannot tell `{$b}` from `$b`; this flag
    /// is the same distinction [`Statement::AssignConst::braced`],
    /// [`Statement::Return::braced`] and
    /// [`CommandTokens::arg_is_braced_literal`] carry for their own
    /// words (issue #1260).
    pub list_braced: bool,
}

/// An IR statement — the building block of the compiler pipeline.
///
/// Each variant
/// carries a [`Span`] for position tracking. The span is resolved
/// to text or `(line, character)` via a `SourceMap` on demand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    /// Constant assignment: `set name value` where value is a literal.
    AssignConst {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal
        /// (`set {a($x)} v`). Braces suppress substitution, so an
        /// array-element target's key (`a($x)`) must be pushed
        /// LITERALLY (`$x` stays `$x`) rather than substituted — and,
        /// conversely, a *non*-braced name word still has its backslash
        /// escapes to decode before it is a variable name (issue #1616).
        /// Mirrors the `braced` precedent on [`Self::Return`].
        /// Defaults to `false` for every construction site except
        /// the `set` lowering hook (only it knows the source token
        /// kind); resolved by codegen's `store_target`, which
        /// `push_var_ref` / `emit_incr` then consume.
        name_braced: bool,
        /// Constant value text.
        value: String,
        /// Content span of the value *word* when `value` is written
        /// verbatim in the source at this statement (the writable
        /// provenance a rename may splice a new command name over).
        /// `None` when the constant has no exact source
        /// representation — a synthesised constant (folding,
        /// desugaring, an optimiser rewrite) or a word whose content
        /// was reconstructed — in which case value-provenance
        /// consumers must abstain rather than guess a span.
        value_span: Option<Span>,
    },

    /// Expression assignment: `set name [expr ...]`.
    AssignExpr {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal — see
        /// [`Self::AssignConst::name_braced`].
        name_braced: bool,
        /// Parsed expression AST.
        expr: ExprNode,
        /// Live command binding whose registry identity justified fusing the
        /// nested command. This is generic binding metadata (source spelling
        /// plus canonical implementation), not an `expr`-name flag: aliases
        /// therefore carry their own binding identity into bytecode.
        command_binding: Option<tcl_runtime_api::CommandBindingIdentity>,
        /// Original value word (`[expr ...]`) before this statement was
        /// fused. Codegen replays this through the ordinary `set` value path
        /// when the whole-module command-binding summary says `expr` no
        /// longer denotes its registry builtin. Keeping the exact word here
        /// preserves quoting, substitutions, and an overridden command's
        /// argv instead of trying to reconstruct Tcl source from the AST.
        fallback_value: String,
        /// Absolute source offset of the expression text's first byte —
        /// see [`IfClause::condition_base`].  `None` when the expression
        /// text is not a verbatim source slice.
        expr_base: Option<u32>,
    },

    /// General value assignment: `set name value` where value may
    /// contain substitutions.
    AssignValue {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal — see
        /// [`Self::AssignConst::name_braced`].
        name_braced: bool,
        /// Value text (may contain backslash substitutions).
        value: String,
        /// Whether the value contains backslash substitutions.
        value_needs_backsubst: bool,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Increment: `incr name ?amount?`.
    Incr {
        /// Source span.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal — see
        /// [`Self::AssignConst::name_braced`].
        name_braced: bool,
        /// Increment amount (None = 1).
        amount: Option<String>,
        /// Whether it is safe if the variable is uninitialised.
        safe_on_uninit: bool,
    },

    /// Standalone expression evaluation: `expr ...`.
    ExprEval {
        /// Source span.
        span: Span,
        /// Source binding whose registry implementation this specialised
        /// expression evaluation assumes.
        command_binding: tcl_runtime_api::CommandBindingIdentity,
        /// Parsed expression AST.
        expr: ExprNode,
        /// Absolute source offset of the expression text's first byte —
        /// see [`IfClause::condition_base`].  `None` when the expression
        /// text is not a verbatim source slice.
        expr_base: Option<u32>,
    },

    /// A generic command invocation.
    ///
    /// Optionally annotated with variables it defines/reads.
    Call {
        /// Source span.
        span: Span,
        /// Command name as it appeared in source — preserves the
        /// original spelling for diagnostics ("unknown command
        /// `Foo`" reads better than "unknown command `::Foo`").
        command: String,
        /// Canonical command name resolved against the
        /// registry.  `None` when the lowerer hasn't (yet)
        /// populated it or when the command is unknown to the
        /// registry; downstream dispatch sites use
        /// [`Statement::canonical_command_or_source`] so a `None`
        /// canonical falls back to `command`.  The lowerer populates
        /// it when an alias resolves (`interp alias {} foo {} ::ns::bar; foo
        /// 1 2` lowers to a `Call` with `command="foo"` and
        /// `canonical_command=Some("::ns::bar")`).
        canonical_command: Option<String>,
        /// Argument texts.
        args: Vec<String>,
        /// Variables defined by this command.
        defs: Vec<String>,
        /// Variables read by this command (beyond `$`-references in args).
        reads: Vec<String>,
        /// Whether defined variables are also read (read-before-write).
        reads_own_defs: bool,
        /// Whether it is safe if defined variables are uninitialised.
        safe_on_uninit: bool,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
        /// Per-iterator-group sizes for synthetic `foreach` /
        /// `lmap` / `dict for` / `dict map` header calls produced
        /// by the CFG builder.  ``Some([n0, n1, …])`` encodes that
        /// `defs` is a flattened concatenation of `n0` + `n1` + …
        /// vars per iterator group, so the codegen can reconstruct
        /// the original `var-list` ↔ `list-arg` pairing.  ``None``
        /// for every other call shape.
        foreach_groups: Option<Vec<usize>>,
    },

    /// Return statement: `return ?value?`.
    Return {
        /// Source span.
        span: Span,
        /// Return value text, if any.
        value: Option<String>,
        /// Return expression, if any (for `return [expr ...]`).
        expr: Option<ExprNode>,
        /// Live command binding whose registry identity justified compiling a
        /// nested command substitution directly into [`expr`](Self::Return::expr).
        ///
        /// This is separate from the outer `return` command's binding: the
        /// latter remains owned by the statement source boundary, while this
        /// dependency belongs to the consumed nested command (`expr`, or a
        /// registry-proved alias for it).
        command_binding: Option<tcl_runtime_api::CommandBindingIdentity>,
        /// Whether the value was braced.
        braced: bool,
    },

    /// A command whose side effects defeat static analysis.
    ///
    /// Commands like `eval`, `uplevel`, and `upvar` can modify
    /// arbitrary variables at runtime, so no constant propagation or
    /// dead-store reasoning can cross a barrier.
    Barrier {
        /// Source span.
        span: Span,
        /// Human-readable reason for the barrier.
        reason: String,
        /// Original command name (source-surface spelling).
        command: String,
        /// Canonical command name resolved against the
        /// registry.  See [`Statement::Call::canonical_command`].
        canonical_command: Option<String>,
        /// Original argument texts.
        args: Vec<String>,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Inline group of statements to splice into the enclosing
    /// script *without* introducing a separate scope. Produced by
    /// [`crate::inline_uplevel`] when a passthrough callsite is
    /// rewritten — the callee's body runs in the caller's frame, so
    /// it can be flattened directly into the parent block stream.
    /// Also produced by const-propagation when an `eval` /
    /// `uplevel` body resolves to a brace-literal.
    Block {
        /// Source span of the original call that produced this block.
        span: Span,
        /// The pre-lowered body, evaluated in the enclosing scope.
        body: Script,
        /// Fully-qualified namespace the body was lowered in.
        namespace: String,
        /// Original command-tokens snapshot for downstream analysis
        /// (lets diagnostics still report the source surface form).
        tokens: Option<CommandTokens>,
        /// Registry-owned Tcl error context contributed by this inlined body.
        /// `None` for transparent passthrough and synthetic blocks.
        error_context: Option<tcl_registry::InlineBodyErrorContext>,
    },

    /// Static-body uplevel — `uplevel ?level? {body}` where the body
    /// is a brace-string literal so the body's IR can be lowered
    /// inline. Models the "shift the active frame, evaluate body,
    /// restore frame" semantics that `uplevel` provides without the
    /// runtime [`Self::Barrier`] dispatch.
    ///
    /// Codegen emits `frame_depth_stash`
    /// / `frame_depth_restore` around the body.
    UpFrame {
        /// Source span of the original `uplevel` call.
        span: Span,
        /// The level argument's magnitude — `1` for `uplevel 1
        /// {body}` (the canonical form) and for a bare `uplevel
        /// {body}`, `0` for `uplevel #0 {body}` or `uplevel 0
        /// {body}`. Read it together with [`Self::UpFrame::absolute`]:
        /// the same magnitude means two different frames depending on
        /// that flag.
        frame_shift: i32,
        /// `true` when the level was written in the `#N` *absolute*
        /// form, counting down from the global frame (`uplevel #0`
        /// evaluates the body in the global frame). `false` for the
        /// *relative* form, counting up from the current frame
        /// (`uplevel 0` evaluates the body in the current frame,
        /// `uplevel 1` in the caller's).
        ///
        /// The distinction is load-bearing, not cosmetic: `uplevel #0`
        /// and `uplevel 0` are different frames whenever the current
        /// frame is not the global one, so bare command words in the
        /// body resolve against different namespaces
        /// (tclsh8.6/9.0-confirmed).
        absolute: bool,
        /// The pre-lowered body, evaluated at the shifted frame.
        body: Script,
        /// Original command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Conditional: `if cond body ?elseif cond body ...? ?else body?`.
    If {
        /// Source span.
        span: Span,
        /// `if`/`elseif` clauses in order.
        clauses: Vec<IfClause>,
        /// Optional `else` body.
        else_body: Option<Script>,
        /// Source span of the `else` body.
        else_span: Option<Span>,
    },

    /// `for` loop: `for init cond next body`.
    For {
        /// Source span.
        span: Span,
        /// Initialisation script.
        init: Script,
        /// Source span of the init script.
        init_span: Span,
        /// Loop condition expression.
        condition: ExprNode,
        /// Source span of the condition.
        condition_span: Span,
        /// Absolute source offset of the condition text — see
        /// [`IfClause::condition_base`].
        condition_base: Option<u32>,
        /// Next-iteration script.
        next: Script,
        /// Source span of the next script.
        next_span: Span,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Per-word source token metadata for the generic ("frozen") runtime
        /// fallback, when the loop cannot be inlined (a command-substitution
        /// condition). Carries the original word kinds so the fallback pushes
        /// braced `{cond}` / `{body}` words verbatim and substituted words
        /// (`$body`) interpolated — exactly as written. `None` for
        /// synthetically-constructed loops that never take the fallback.
        raw_tokens: Option<CommandTokens>,
    },

    /// `while` loop: `while cond body`.
    While {
        /// Source span.
        span: Span,
        /// Loop condition expression.
        condition: ExprNode,
        /// Source span of the condition.
        condition_span: Span,
        /// Absolute source offset of the condition text — see
        /// [`IfClause::condition_base`].
        condition_base: Option<u32>,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Per-word source token metadata for the generic ("frozen") runtime
        /// fallback — see [`Statement::For::raw_tokens`].
        raw_tokens: Option<CommandTokens>,
    },

    /// `foreach`/`lmap`/`dict for`/`dict map` loop.
    Foreach {
        /// Source span.
        span: Span,
        /// Iterator groups: `(var_list, list_arg)` pairs.
        iterators: Vec<ForeachIterator>,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Whether this is an `lmap` (returns a list).
        is_lmap: bool,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Whether this is `dict for`/`dict map`.
        is_dict_iteration: bool,
        /// Whether this is `array for {k v} arr body` (Tcl 9.0). Unlike a plain
        /// `foreach`, the body runs in the caller's frame over an *array* (not a
        /// list value), so analysis inlines it into the caller unit (binding the
        /// loop vars) while codegen barriers it to the `::tcl::array::for`
        /// ensemble invoke — see `lower_foreach_dispatch`.
        is_array_iteration: bool,
        /// Per-word source token metadata for the generic runtime fallback
        /// (the `dict for`/`map` barrier and the runtime `foreach`/`lmap`
        /// call), so braced var-lists / bodies are pushed verbatim and
        /// `$list` arguments substituted — see [`Statement::For::raw_tokens`].
        raw_tokens: Option<CommandTokens>,
    },

    /// `catch script ?resultVar? ?optionsVar?`.
    Catch {
        /// Source span.
        span: Span,
        /// Body script to evaluate.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Variable for the result, if any.
        result_var: Option<String>,
        /// Variable for the options dict, if any.
        options_var: Option<String>,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Original parsed tokens, including braced/quoted flags.
        /// Threaded through so the CFG's `Catch → Call` lowering
        /// (`emit_opaque_catch`) can preserve the `{…}` braces
        /// around the body when the codegen falls back to
        /// `tcl_eval`.  Without this, ``catch {$undef} msg`` would
        /// be reconstructed as ``catch $undef msg`` and the
        /// unset-var read would fire before catch could intercept
        /// it.
        tokens: Option<CommandTokens>,
    },

    /// `try body ?on/trap ...? ?finally body?`.
    Try {
        /// Source span.
        span: Span,
        /// Body script.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Handler clauses.
        handlers: Vec<TryHandler>,
        /// Optional `finally` body.
        finally_body: Option<Script>,
        /// Source span of the `finally` body.
        finally_span: Option<Span>,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
    },

    /// `switch` statement.
    Switch {
        /// Source span.
        span: Span,
        /// Subject text being matched — the word's *value*, with any
        /// delimiters already removed.
        subject: String,
        /// `true` when the subject came from a braced word, so its value is
        /// literal and suppresses substitution.
        ///
        /// The sibling of [`Self::Switch::patterns_braced`], and asked for the
        /// same reason: codegen hands the subject to the expression emitter as
        /// a word, and a word that lost its braces on the way in is
        /// indistinguishable from a bare one. Without this,
        /// `switch -- {a[nosuchcmd]}` *ran* `nosuchcmd` where both oracles
        /// match the literal and take the default arm. Defaults to `false` —
        /// an unbraced subject substitutes, which is the common form.
        subject_braced: bool,
        /// Source span of the subject.
        subject_span: Span,
        /// Pattern/body arms.
        arms: Vec<SwitchArm>,
        /// Optional default body.
        default_body: Option<Script>,
        /// Source span of the default body.
        default_span: Option<Span>,
        /// Match mode: `"exact"`, `"glob"`, or `"regexp"`.
        mode: SwitchMode,
        /// Whether matching is case-insensitive.
        nocase: bool,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Per-[`Self::Switch::raw_args`] "this word was braced" flags, for the
        /// generic-invoke fallback the opaque modes take.
        ///
        /// `raw_args` are *values*, and a value cannot say how it was written,
        /// so the emitter that pushes them has no way to tell a braced word's
        /// literal content from a bare word that still substitutes. It used to
        /// fabricate the flags — marking only a single braced arm list — and
        /// everything else went out substituting: `switch -glob -- {a[bc]d}`
        /// ran `bc`, and the pattern `{a\[bc\]d}` was decoded to `a[bc]d` and
        /// matched as a character class, so it matched `abd` and not the
        /// literal both oracles match.
        ///
        /// Empty when the flags are unknown (a hand-built statement), which the
        /// emitter reads as "none braced" — the prior behaviour.
        raw_arg_braced: Vec<bool>,
        /// `true` when the arms came from a single braced
        /// `{pat body …}` block — patterns are literal list elements
        /// with no substitution. `false` when supplied as separate
        /// words (`switch $s $pat {body}`), where each pattern
        /// undergoes normal `$var` / `[cmd]` runtime substitution
        /// before matching. Defaults to `true`, the canonical braced form.
        patterns_braced: bool,
    },
}

impl Statement {
    /// Return the source span of this statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::AssignConst { span, .. }
            | Self::AssignExpr { span, .. }
            | Self::AssignValue { span, .. }
            | Self::Incr { span, .. }
            | Self::ExprEval { span, .. }
            | Self::Call { span, .. }
            | Self::Return { span, .. }
            | Self::Barrier { span, .. }
            | Self::Block { span, .. }
            | Self::UpFrame { span, .. }
            | Self::If { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::Foreach { span, .. }
            | Self::Catch { span, .. }
            | Self::Try { span, .. }
            | Self::Switch { span, .. } => *span,
        }
    }

    /// Dispatch helper that returns the canonical command name
    /// when populated, falling back to the source-surface `command`
    /// otherwise.  Use from downstream dispatch sites (codegen hook
    /// lookup, side-effect classification, GVN purity) so a single
    /// canonical key drives every consumer; use the bare `command`
    /// field directly when rendering source-surface text in
    /// diagnostics.
    ///
    /// Returns `""` for non-Call / non-Barrier statements.
    #[must_use]
    pub fn canonical_command_or_source(&self) -> &str {
        match self {
            Self::Call {
                command,
                canonical_command,
                ..
            }
            | Self::Barrier {
                command,
                canonical_command,
                ..
            } => canonical_command.as_deref().unwrap_or(command.as_str()),
            _ => "",
        }
    }
}

/// Switch matching mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SwitchMode {
    /// Exact string match (default).
    #[default]
    Exact,
    /// Glob pattern match.
    Glob,
    /// Regular expression match.
    Regexp,
}

impl SwitchMode {
    /// Parse a switch mode from its Tcl string form.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "glob" => Self::Glob,
            "regexp" => Self::Regexp,
            _ => Self::Exact,
        }
    }

    /// Return the Tcl string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Glob => "glob",
            Self::Regexp => "regexp",
        }
    }
}

// Procedure and module types

/// A procedure definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Procedure {
    /// Short procedure name.
    pub name: String,
    /// Fully qualified name (e.g. `::ns::proc`).
    pub qualified_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Source span of the definition.
    pub span: Span,
    /// Procedure body.
    pub body: Script,
    /// Raw parameter list text.
    pub params_raw: String,
    /// Source text of the body (`None` for synthetic procs like `when`).
    pub body_source: Option<String>,
    /// Offset of [`Self::body_source`]'s first byte in the unit source.
    ///
    /// A statement of the body carries an absolute span; subtracting this
    /// offset turns it into the body-relative position Tcl counts `errorLine`
    /// and `(procedure "p" line N)` from. `0` for a procedure with no body
    /// text of its own (a synthetic or lambda body unit).
    pub body_offset: u32,
    /// Whether defined inside `namespace eval`.
    pub namespace_scoped: bool,
    /// BIG-IP handler priority (`0..=1000`, default 500).
    pub base_priority: u32,
}

/// A method definition within a class body.
///
/// Compiles like [`Procedure`] but carries class context for
/// interprocedural analysis and devirtualisation.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    /// Enclosing class name.
    pub class_name: String,
    /// Method name.
    pub method_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Method body.
    pub body: Script,
    /// Current namespace used to resolve command heads in the method body.
    /// `TclOO` selects the receiver's object namespace at runtime; other
    /// registry definer families can name their definition target exactly.
    pub execution_namespace: ExecutionNamespace,
    /// Method kind.
    pub kind: MethodKind,
    /// Source span (may be absent for synthetic methods).
    pub span: Option<Span>,
    /// Instance-variable names in scope for this method (class-level
    /// `variable` declarations + the method's own `variable` decls).
    /// A method that *writes* any of these mutates object state and is
    /// therefore impure for O126 purposes, even though the write looks
    /// like a plain local `set`. Used by interprocedural method-purity.
    pub instance_vars: std::collections::HashSet<String>,
}

/// The kind of a class method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MethodKind {
    /// Regular instance method.
    #[default]
    Method,
    /// Class-level method.
    ClassMethod,
    /// Constructor.
    Constructor,
    /// Destructor.
    Destructor,
}

impl MethodKind {
    /// Parse from the string representation.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "classmethod" => Self::ClassMethod,
            "constructor" => Self::Constructor,
            "destructor" => Self::Destructor,
            _ => Self::Method,
        }
    }

    /// Return the string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::ClassMethod => "classmethod",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
        }
    }
}

/// Whole-module evidence about OO definitions the lowering could not
/// statically model, grouped so consumers widen on exactly the right
/// axis (issues #1164 / #1166).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OoDefinitionEvidence {
    /// An `oo::class create` / `oo::define` targeted a class whose name
    /// is dynamic (`oo::define $cls { … }`): *any* class's methods may
    /// have been touched, so whole-module OO evidence is incomplete.
    pub dynamic_target: bool,
    /// A class-reference member carried a dynamic word (`superclass $b`):
    /// that class's ancestry is unknown, so hierarchy-scoped analysis
    /// must fall back to whole-module widening.
    pub dynamic_class_relations: bool,
    /// At least one executable definition/member body could not be published
    /// through [`Module::executable_script_roots`]. This includes dynamic or
    /// otherwise unsupported definer bodies, accessor/configuration scripts,
    /// and definitions nested in a method frame that the source-root post-pass
    /// does not revisit. Whole-module command-effect consumers must fail closed
    /// when this is set: absence from `methods` is not evidence that the runtime
    /// body cannot mutate the command table.
    pub unretained_executable_roots: bool,
}

/// A top-level module: procedures + top-level script.
///
/// This is the only mutable IR type — it accumulates procedures and
/// methods during lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// The original source text this module was lowered from. Spans on the
    /// lowered statements index into it, so codegen can recover each command's
    /// surface text for `errorInfo` (`while executing "…"`). Empty when not
    /// supplied (older construction paths / hand-built test modules).
    pub source: String,
    /// Rooted constructed namespace in which [`Self::top_level`] resolves
    /// command names. This is `"::"` for an ordinary source module and the
    /// procedure's defining namespace for a runtime procedure-body compile.
    pub top_level_namespace: String,
    /// The dialect this module was lowered for (`"tcl8.6"`, `"f5-irules"`, …),
    /// `None` for an unnamed/permissive compile.
    ///
    /// The dialect is a top-level property of the compile, threaded from the
    /// entry point down to codegen so the backend emits code *for* a release
    /// rather than deferring release-dependent decisions to run time — a
    /// numeric literal is a case in point: `0755` is 493 under 8.6 and 755
    /// under 9.0, and which one it is settled here.
    pub dialect: Option<String>,
    /// Whether lowering deliberately preserved ordinary command dispatch for
    /// every invocation (trace/invalidation recovery compilation).
    pub plain_command_dispatch: bool,
    /// Top-level script (code outside any procedure).
    pub top_level: Script,
    /// Named procedures.
    pub procedures: std::collections::HashMap<String, Procedure>,
    /// Named methods (keyed by `class::method`).
    pub methods: std::collections::HashMap<String, MethodDef>,
    /// Synthetic *body units* — the bodies of commands that run their script
    /// argument in a fresh frame but are **not** real named procedures:
    /// `apply` lambdas and `namespace eval` bodies (keyed by a synthetic
    /// qualified name like `::apply#0`, or `::NS::namespace-eval#0` — `NS`
    /// *prefixes* the marker, matching every other qname's "everything
    /// before the last `::` is the enclosing namespace" convention, so a
    /// bare call inside the body resolves against the namespace it actually
    /// targets rather than always the global namespace).
    ///
    /// These are lowered into [`Procedure`]s purely so the static-analysis
    /// pipeline (CFG → SSA → SCCP → taint) reaches *inside* the body — the
    /// same coverage a real `proc` gets. They are held in a **separate** map
    /// from [`Self::procedures`] so codegen (which emits `procedures`) never
    /// materialises them as callable procs, and from [`Self::methods`] so the
    /// TclOO-specific consumers (interproc method purity, the O126 optimiser
    /// gate) never mistake them for methods. The body still executes at
    /// runtime via its command's own `Statement::Barrier`, so bytecode is
    /// byte-identical whether or not a body unit is recorded.
    pub body_units: std::collections::HashMap<String, Procedure>,
    /// The subset of [`Self::body_units`] that are **lambda** frames — an
    /// `apply` literal's body, whose parameter list binds real locals exactly
    /// as a `proc`'s does.
    ///
    /// A `namespace eval` body unit is *not* one: its variables are the
    /// namespace's, shared with every other body that opens the same
    /// namespace, so a name it reads without writing is routinely defined
    /// elsewhere.  The read-before-set family (`W210`) therefore runs over
    /// this subset only — a lambda body is a closed frame and an unwritten
    /// read in it is a genuine error (issue #1070).
    pub lambda_body_units: std::collections::BTreeSet<String>,
    /// Procedure names that were defined more than once.
    pub redefined_procedures: std::collections::HashSet<String>,
    /// `TclOO` methods defined more than once (a later `oo::define` /
    /// in-body redefinition replaces the body at runtime), keyed by
    /// method qname. [`Self::methods`] keeps the *first* body
    /// (last-definition-wins stays with the navigation layer); each
    /// **replacement** body is retained here in definition order (issue
    /// #1166), so analysis consumers — the SCCP method-dispatch barrier,
    /// O126 method purity — can scan every body a dispatch may run
    /// instead of abstaining on the mere fact of redefinition. The union
    /// of bodies over-approximates whichever is live at dispatch time,
    /// so scanning all of them is sound where scanning one is not.
    pub redefined_methods: std::collections::HashMap<String, Vec<MethodDef>>,
    /// Class qnames with a member definition the lowering could not
    /// statically model — a dynamic member name (`method $n …`) or a
    /// dynamic member body (`method m {} $body`) inside an `oo::class` /
    /// `oo::define` block. Any method of such a class may have been
    /// (re)defined with an unknown body, so per-class analysis
    /// (purity, the method-dispatch propagation barrier) must abstain
    /// for the whole class.
    pub oo_unanalysed_classes: std::collections::HashSet<String>,
    /// Whole-module OO-definition evidence flags — see
    /// [`OoDefinitionEvidence`].
    pub oo_evidence: OoDefinitionEvidence,
    /// Class-hierarchy relation declarations captured from `oo::class` /
    /// `oo::define` bodies: `(declaring class qname, written related-class
    /// word)` for each argument of a member whose registry grammar marks
    /// every argument as a class reference
    /// ([`tcl_registry::definer::MemberRefKind::Class`] — `superclass`,
    /// `mixin`). The written word is kept as written; consumers resolve it
    /// against the module's class set conservatively (issue #1164: the
    /// method-dispatch propagation barrier scopes itself to
    /// hierarchy-connected classes instead of the whole module).
    pub class_relations: Vec<(String, String)>,
    /// `namespace import` directives captured at lowering time —
    /// `(context_namespace, absolute_pattern)` pairs. Future codegen
    /// passes pattern-match each against the final
    /// `Self::procedures` table to resolve unqualified calls
    /// directly instead of falling back to the runtime
    /// interpreter. Only absolute patterns (`::foo::*` /
    /// `::foo::bar`) are recorded; relative patterns require
    /// runtime namespace-path walking which compile-time
    /// resolution does not model.
    pub namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives captured at lowering time —
    /// `(context_namespace, pattern)` pairs. Codegen consults this
    /// list to gate the import shortcut on actual exportedness so
    /// `namespace import ::foo::*` only resolves names that
    /// `::foo` actually exports.
    pub namespace_exports: Vec<(String, String)>,
    /// Literal command names that have an execution trace
    /// registered (`trace add execution NAME enter|leave HANDLER`).
    /// GVN consults this set to gate purity (a traced call is
    /// never pure because the trace handler composes side effects
    /// in).
    pub traced_commands: std::collections::BTreeSet<String>,
    /// `true` when a `trace add execution` was seen with a
    /// non-literal command target (`trace add execution $cmd ...`).
    /// Forces GVN / partial-redundancy / loop-invariant passes to
    /// treat *every* call as potentially traced.
    pub has_dynamic_trace: bool,
    /// Literal variable names targeted by an active *variable* trace
    /// (`trace add variable NAME ops HANDLER`, or the deprecated
    /// `trace variable`/`vdelete` spellings) anywhere in the module —
    /// top level or any procedure body, regardless of whether the
    /// installing call is reachable before or after a given use.  A
    /// read of a traced variable can run arbitrary handler code, and a
    /// write handler can rewrite the value being stored, so the
    /// propagation optimiser (`O102` load-forwarding) and dead-store
    /// elimination must never treat a use of one of these names as
    /// equivalent to its last literal assignment. Whole-module and
    /// position-independent by design — mirrors [`Self::traced_commands`]
    /// but for `ArgRole::VarWrite`-role variable-trace targets
    /// (`Traits::ESTABLISHES_VARIABLE_TRACE`) rather than execution-trace
    /// command names.
    pub traced_variables: std::collections::BTreeSet<String>,
    /// `true` when a variable-trace install/remove call was seen with a
    /// non-literal variable-name target (`trace add variable $name ...`).
    /// Forces the propagation optimiser to treat *every* variable as
    /// potentially traced.
    pub has_dynamic_variable_trace: bool,
}

fn execution_namespace_for_qname(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}

impl Module {
    /// Executable roots whose invocation is independent of an enclosing
    /// statement: the module load script, retained procedures and methods,
    /// and retained replacement methods.
    ///
    /// Synthetic [`Self::body_units`] are deliberately absent. An `apply` or
    /// `namespace eval` body executes only through its owning invocation, in
    /// that invocation's program-order position; treating the same body as an
    /// independently callable fixpoint root both invents an execution path and
    /// repeatedly replays nested body suffixes. Whole-module static analyses
    /// that need every body in isolation should use
    /// [`Self::executable_script_roots`] instead.
    #[must_use]
    pub(crate) fn independent_executable_script_roots(&self) -> Vec<(&Script, ExecutionNamespace)> {
        let mut roots = vec![(
            &self.top_level,
            ExecutionNamespace::exact(if self.top_level_namespace.is_empty() {
                "::".to_owned()
            } else {
                self.top_level_namespace.clone()
            }),
        )];

        let mut procedures: Vec<_> = self.procedures.iter().collect();
        procedures.sort_by(|a, b| a.0.cmp(b.0));
        roots.extend(procedures.into_iter().map(|(qname, procedure)| {
            (
                &procedure.body,
                ExecutionNamespace::exact(execution_namespace_for_qname(qname)),
            )
        }));

        let mut methods: Vec<_> = self.methods.iter().collect();
        methods.sort_by(|a, b| a.0.cmp(b.0));
        roots.extend(
            methods
                .into_iter()
                .map(|(_, method)| (&method.body, method.execution_namespace.clone())),
        );

        let mut redefined_methods: Vec<_> = self.redefined_methods.iter().collect();
        redefined_methods.sort_by(|a, b| a.0.cmp(b.0));
        for (_, replacements) in redefined_methods {
            roots.extend(
                replacements
                    .iter()
                    .map(|method| (&method.body, method.execution_namespace.clone())),
            );
        }

        roots
    }

    /// Every retained executable script root and the namespace in which its
    /// command heads resolve.
    ///
    /// Whole-module semantic scans use this inventory instead of each keeping
    /// a partial copy of the procedure/method/body-unit map traversal. The
    /// ordering is deterministic so monotone state replays and memo inputs do
    /// not depend on `HashMap` seeds. Earlier bodies of a redefined procedure
    /// are not retained; consumers that need them must widen when
    /// [`Self::redefined_procedures`] is non-empty.
    #[must_use]
    pub fn executable_script_roots(&self) -> Vec<(&Script, ExecutionNamespace)> {
        let mut roots = self.independent_executable_script_roots();

        let mut body_units: Vec<_> = self.body_units.iter().collect();
        body_units.sort_by(|a, b| a.0.cmp(b.0));
        roots.extend(body_units.into_iter().map(|(qname, unit)| {
            (
                &unit.body,
                ExecutionNamespace::exact(execution_namespace_for_qname(qname)),
            )
        }));

        roots
    }
}

/// Extract the event name from a `::when::` qualified name.
///
/// Handles both `::when::HTTP_REQUEST` and indexed forms like
/// `::when::HTTP_REQUEST#1`.
#[must_use]
pub fn when_event_name(qualified_name: &str) -> &str {
    let bare = qualified_name
        .strip_prefix("::when::")
        .unwrap_or(qualified_name);
    match bare.find('#') {
        Some(idx) => &bare[..idx],
        None => bare,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_script() {
        let script = Script::new();
        assert!(script.statements.is_empty());
    }

    #[test]
    fn script_from_statements() {
        let stmts = vec![Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }];
        let script = Script::from_statements(stmts);
        assert_eq!(script.statements.len(), 1);
    }

    #[test]
    fn for_each_statement_visits_nested_bodies_in_order() {
        // `if {c} { inner1 } else { inner2 }` — the visitor must see the
        // `If` itself plus both nested-body calls, in source order.
        let inner1 = Statement::Call {
            span: Span::new(0, 0),
            command: "inner1".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let inner2 = Statement::Call {
            span: Span::new(0, 0),
            command: "inner2".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let if_stmt = Statement::If {
            span: Span::new(0, 0),
            clauses: vec![IfClause {
                condition: ExprNode::Raw { text: "c".into() },
                condition_span: Span::new(0, 0),
                body: Script::from_statements(vec![inner1]),
                body_span: Span::new(0, 0),
                condition_base: None,
            }],
            else_body: Some(Script::from_statements(vec![inner2])),
            else_span: None,
        };
        let script = Script::from_statements(vec![if_stmt]);

        let mut seen: Vec<String> = Vec::new();
        for_each_statement(&script, &mut |stmt| {
            let name = match stmt {
                Statement::If { .. } => "if".to_owned(),
                Statement::Call { command, .. } => command.clone(),
                _ => "other".to_owned(),
            };
            seen.push(name);
        });
        assert_eq!(seen, vec!["if", "inner1", "inner2"]);
    }

    /// Regression coverage for issue #996: `for_each_statement` recurses
    /// once per nested `If`/`For`/`While`/`Foreach`/`Catch`/`Try`/`Switch`/
    /// `UpFrame` body, with no depth cap of its own before this fix.
    /// Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering
    /// pass today (every `Script` this crate hands to `for_each_statement`
    /// is built by `crate::lowering`), so this is defence-in-depth /
    /// consistency with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. This test builds the nested `Script`
    /// directly (bypassing lowering) so it genuinely exercises
    /// `for_each_statement`'s own new cap rather than lowering's; 2000
    /// levels is comfortably past the new 256-level cap. The assertion is
    /// that the call returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_for_each_statement() {
        const DEPTH: usize = 2000;
        let leaf = Statement::Call {
            span: Span::new(0, 0),
            command: "leaf".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut script = Script::from_statements(vec![leaf]);
        for _ in 0..DEPTH {
            script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Raw { text: "1".into() },
                    condition_span: Span::new(0, 0),
                    body: script,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }

        let mut count = 0usize;
        for_each_statement(&script, &mut |_| count += 1);
        assert!(count > 0, "must still visit statements within the cap");
        assert!(count <= DEPTH + 1, "must not exceed the number of nodes");
    }

    #[test]
    fn for_each_statement_descends_into_upframe_body() {
        // FN guard (P1, code review): `UpFrame` (a static-body `uplevel
        // ?level? {...}`) has a nested `body: Script` just like `Block` /
        // `While` / `Catch` / `Foreach`, but was missing from the grouped
        // match arm — the visitor stopped at the `UpFrame` statement itself
        // and never walked its body, so a whole-module scan built on this
        // visitor (e.g. `var_observability::scan_module_global_names`)
        // silently missed anything declared inside a static `uplevel` body.
        let inner = Statement::Call {
            span: Span::new(0, 0),
            command: "inner".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let upframe = Statement::UpFrame {
            span: Span::new(0, 0),
            frame_shift: 0,
            absolute: true,
            body: Script::from_statements(vec![inner]),
            tokens: None,
        };
        let script = Script::from_statements(vec![upframe]);

        let mut seen: Vec<String> = Vec::new();
        for_each_statement(&script, &mut |stmt| {
            let name = match stmt {
                Statement::UpFrame { .. } => "upframe".to_owned(),
                Statement::Call { command, .. } => command.clone(),
                _ => "other".to_owned(),
            };
            seen.push(name);
        });
        assert_eq!(seen, vec!["upframe", "inner"]);
    }

    #[test]
    fn statement_span_accessor() {
        let stmt = Statement::Call {
            span: Span::new(5, 20),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["hello".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.span(), Span::new(5, 20));
    }

    #[test]
    fn assign_const_roundtrip() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "42".into(),
            value_span: None,
        };
        if let Statement::AssignConst { name, value, .. } = &stmt {
            assert_eq!(name, "x");
            assert_eq!(value, "42");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn barrier_with_tokens() {
        let tokens = CommandTokens::from_lossy_parts(
            vec![Span::new(0, 4), Span::new(5, 10)],
            vec!["eval".into(), "body".into()],
            vec![tcl_lexer::TokenType::Esc, tcl_lexer::TokenType::Str],
            vec![true, true],
            vec![Span::new(0, 4), Span::new(4, 5), Span::new(5, 10)],
            None,
        );
        let stmt = Statement::Barrier {
            span: Span::new(0, 10),
            reason: "eval".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["body".into()],
            tokens: Some(tokens.clone()),
        };
        if let Statement::Barrier {
            tokens: Some(t), ..
        } = &stmt
        {
            assert_eq!(t.argv_texts, vec!["eval", "body"]);
        } else {
            panic!("expected tokens");
        }
    }

    #[test]
    fn if_statement_structure() {
        let stmt = Statement::If {
            span: Span::new(0, 50),
            clauses: vec![IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 3,
                    end: 4,
                },
                condition_span: Span::new(3, 4),
                body: Script::from_statements(vec![Statement::Call {
                    span: Span::new(6, 15),
                    command: "puts".into(),
                    canonical_command: None,
                    args: vec!["yes".into()],
                    defs: Vec::new(),
                    reads: Vec::new(),
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                }]),
                body_span: Span::new(5, 16),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        };
        assert_eq!(stmt.span(), Span::new(0, 50));
        if let Statement::If { clauses, .. } = &stmt {
            assert_eq!(clauses.len(), 1);
        }
    }

    #[test]
    fn for_loop_structure() {
        let stmt = Statement::For {
            span: Span::new(0, 40),
            init: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(5, 12),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            }]),
            init_span: Span::new(4, 13),
            condition: ExprNode::Binary {
                op: crate::expr_ast::BinOp::Lt,
                left: Box::new(ExprNode::Var {
                    text: "$i".into(),
                    name: "i".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "10".into(),
                    start: 5,
                    end: 7,
                }),
            },
            condition_span: Span::new(14, 22),
            next: Script::from_statements(vec![Statement::Incr {
                span: Span::new(24, 30),
                name: "i".into(),
                name_braced: false,
                amount: None,
                safe_on_uninit: false,
            }]),
            next_span: Span::new(23, 31),
            body: Script::new(),
            body_span: Span::new(32, 34),
            raw_args: Vec::new(),
            raw_tokens: None,
            condition_base: None,
        };
        assert_eq!(stmt.span(), Span::new(0, 40));
    }

    #[test]
    fn switch_mode_parse() {
        assert_eq!(SwitchMode::from_str_lossy("exact"), SwitchMode::Exact);
        assert_eq!(SwitchMode::from_str_lossy("glob"), SwitchMode::Glob);
        assert_eq!(SwitchMode::from_str_lossy("regexp"), SwitchMode::Regexp);
        assert_eq!(SwitchMode::from_str_lossy("unknown"), SwitchMode::Exact);
    }

    #[test]
    fn switch_mode_str() {
        assert_eq!(SwitchMode::Exact.as_str(), "exact");
        assert_eq!(SwitchMode::Glob.as_str(), "glob");
        assert_eq!(SwitchMode::Regexp.as_str(), "regexp");
    }

    #[test]
    fn method_kind_roundtrip() {
        for kind in [
            MethodKind::Method,
            MethodKind::ClassMethod,
            MethodKind::Constructor,
            MethodKind::Destructor,
        ] {
            assert_eq!(MethodKind::from_str_lossy(kind.as_str()), kind);
        }
    }

    #[test]
    fn when_event_name_simple() {
        assert_eq!(when_event_name("::when::HTTP_REQUEST"), "HTTP_REQUEST");
    }

    #[test]
    fn when_event_name_indexed() {
        assert_eq!(when_event_name("::when::HTTP_REQUEST#1"), "HTTP_REQUEST");
    }

    #[test]
    fn when_event_name_no_prefix() {
        assert_eq!(when_event_name("HTTP_REQUEST"), "HTTP_REQUEST");
    }

    #[test]
    fn procedure_construction() {
        let proc = Procedure {
            name: "greet".into(),
            qualified_name: "::greet".into(),
            params: vec!["name".into()],
            span: Span::new(0, 50),
            body: Script::from_statements(vec![Statement::Call {
                span: Span::new(20, 40),
                command: "puts".into(),
                canonical_command: None,
                args: vec!["Hello $name".into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }]),
            params_raw: "name".into(),
            body_source: Some("puts \"Hello $name\"".into()),
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        };
        assert_eq!(proc.name, "greet");
        assert_eq!(proc.params, vec!["name"]);
    }

    #[test]
    fn module_default() {
        let module = Module::default();
        assert!(module.top_level.statements.is_empty());
        assert!(module.procedures.is_empty());
        assert!(module.methods.is_empty());
        assert!(module.redefined_procedures.is_empty());
        assert!(module.redefined_methods.is_empty());
    }

    #[test]
    fn foreach_with_iterators() {
        let stmt = Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["k".into(), "v".into()],
                list_arg: "$dict".into(),
                list_braced: false,
            }],
            body: Script::new(),
            body_span: Span::new(25, 28),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: true,
            is_array_iteration: false,
            raw_tokens: None,
        };
        if let Statement::Foreach {
            iterators,
            is_dict_iteration,
            ..
        } = &stmt
        {
            assert_eq!(iterators.len(), 1);
            assert_eq!(iterators[0].vars, vec!["k", "v"]);
            assert!(is_dict_iteration);
        }
    }

    #[test]
    fn catch_statement() {
        let stmt = Statement::Catch {
            span: Span::new(0, 30),
            body: Script::new(),
            body_span: Span::new(6, 12),
            result_var: Some("result".into()),
            options_var: Some("opts".into()),
            raw_args: Vec::new(),
            tokens: None,
        };
        if let Statement::Catch {
            result_var,
            options_var,
            ..
        } = &stmt
        {
            assert_eq!(result_var.as_deref(), Some("result"));
            assert_eq!(options_var.as_deref(), Some("opts"));
        }
    }

    #[test]
    fn try_with_handlers() {
        let stmt = Statement::Try {
            span: Span::new(0, 60),
            body: Script::new(),
            body_span: Span::new(4, 10),
            handlers: vec![TryHandler {
                kind: "on".into(),
                match_arg: "error".into(),
                trap_pattern: None,
                var_name: Some("e".into()),
                options_var: None,
                body: Script::new(),
                body_span: Span::new(30, 40),
                fallthrough: false,
            }],
            finally_body: Some(Script::new()),
            finally_span: Some(Span::new(50, 58)),
            raw_args: Vec::new(),
        };
        if let Statement::Try {
            handlers,
            finally_body,
            ..
        } = &stmt
        {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].kind, "on");
            assert!(finally_body.is_some());
        }
    }

    #[test]
    fn switch_statement() {
        let stmt = Statement::Switch {
            subject_braced: false,
            raw_arg_braced: Vec::new(),
            span: Span::new(0, 80),
            subject: "$cmd".into(),
            subject_span: Span::new(7, 11),
            arms: vec![
                SwitchArm {
                    pattern: "start".into(),
                    pattern_braced: true,
                    pattern_span: Span::new(13, 18),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(19, 22)),
                    fallthrough: false,
                },
                SwitchArm {
                    pattern: "stop".into(),
                    pattern_braced: true,
                    pattern_span: Span::new(23, 27),
                    body: None,
                    body_span: None,
                    fallthrough: true,
                },
            ],
            default_body: Some(Script::new()),
            default_span: Some(Span::new(50, 60)),
            mode: SwitchMode::Exact,
            nocase: false,
            raw_args: Vec::new(),
            patterns_braced: true,
        };
        if let Statement::Switch { arms, mode, .. } = &stmt {
            assert_eq!(arms.len(), 2);
            assert!(arms[1].fallthrough);
            assert_eq!(*mode, SwitchMode::Exact);
        }
    }

    #[test]
    fn clone_preserves_equality() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let cloned = stmt.clone();
        assert_eq!(stmt, cloned);
    }

    #[test]
    fn return_with_expr() {
        let stmt = Statement::Return {
            span: Span::new(0, 20),
            value: None,
            expr: Some(ExprNode::Binary {
                op: crate::expr_ast::BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "1".into(),
                    start: 5,
                    end: 6,
                }),
            }),
            command_binding: None,
            braced: false,
        };
        if let Statement::Return { expr: Some(e), .. } = &stmt {
            let vars = e.vars_with_grammar(tcl_dialect::LexerGrammar::default());
            assert!(vars.contains("a"));
        }
    }

    // canonical_command_or_source helper

    #[test]
    fn canonical_command_or_source_falls_back_to_source_when_none() {
        let stmt = Statement::Call {
            span: Span::new(0, 3),
            command: "foo".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "foo");
    }

    #[test]
    fn canonical_command_or_source_returns_canonical_when_some() {
        let stmt = Statement::Call {
            span: Span::new(0, 3),
            command: "foo".into(),
            canonical_command: Some("::ns::bar".into()),
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "::ns::bar");
    }

    #[test]
    fn canonical_command_or_source_works_on_barrier() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 6),
            reason: "eval".into(),
            command: "eval".into(),
            canonical_command: Some("eval".into()),
            args: vec!["body".into()],
            tokens: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "eval");
    }

    #[test]
    fn canonical_command_or_source_returns_empty_for_non_call_non_barrier() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 5),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "");
    }
}
