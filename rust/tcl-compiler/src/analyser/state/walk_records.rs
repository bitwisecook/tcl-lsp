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

//! The records a walk queues for later settlement: dispatch sites, pending instance and ensemble sites, definition-abort candidates and interpreter frames.

use tcl_lexer::Span;

/// One captured instance-creation site on the per-item path: the raw
/// `(command, args, creation namespace, site offset)` handed back to
/// [`Analyser::record_instance_creation`] by the post-graft replay.
pub(in super::super) type PendingInstanceCreation = (String, Vec<String>, String, u32);

/// One `$cmd`-head dispatch site, pending settlement against the
/// compiler's flow-sensitive value model once the CFG/SSA
/// `CompilationUnit` is built: the value and its writable provenance come
/// from the SSA use-version's reaching constant definitions, never from a
/// lexical last-write-wins map.
#[derive(Debug, Clone, PartialEq)]
pub(in super::super) struct ConstDispatchSite {
    /// The dispatching variable's name (no leading `$`).
    pub var_name: String,
    /// Span of the `$cmd` head token (the reference anchor).
    pub span: Span,
    /// Command-resolution namespace at the dispatch site.
    pub ns: String,
    /// The head word carries `{*}` expansion (`{*}$cmd args…`): the
    /// value is a **command prefix** — its first list element names the
    /// command, and the writable provenance narrows to that element's
    /// sub-span within the defining literal.  Without expansion the whole
    /// value is
    /// the command name.
    pub head_expanded: bool,
}

/// One `$class`-headed `TclOO` instance-creation site, pending settlement
/// against the compiler's flow-sensitive value
/// model once the CFG/SSA `CompilationUnit` is built — the same
/// settle-late discipline [`ConstDispatchSite`] uses, since `class_var`'s
/// value can't be proven constant until SSA reaching-definitions exist.
/// Recorded by [`super::super::commands::Analyser::record_instance_creation`]
/// when the constructor call's class head is a plain `$var` reference
/// instead of the literal bareword
/// [`super::super::commands::Analyser::class_from_constructor_subst`] resolves
/// directly (`set class ::Derived; set obj [$class create NAME]`,
/// tcllib's `httpd/httpd.tcl`).
#[derive(Debug, Clone, PartialEq)]
pub(in super::super) struct PendingInstanceClassSite {
    /// The class-naming variable's name (no leading `$`).
    pub class_var: String,
    /// The word dispatched on the class command.  It is retained until the
    /// class variable resolves so that class's registry grammar can decide
    /// whether the word is a manufacturer.
    pub manufacturer_word: String,
    /// Span of the `$class` head, for the SSA use-position lookup.
    pub span: Span,
    /// The `instance_classes` key to bind once `class_var` resolves to a
    /// single known user class — the assigned variable (`set obj [...]`)
    /// or created instance-command name.
    pub target_name: String,
}

/// One `<ensemble> <subcommand> …` call site the shell pass could not
/// resolve yet, held until every deferred proc/method
/// body has been walked.
///
/// The whole-file DFS walks a proc body at its *definition* point, so a
/// `namespace ensemble create -map` written inside `proc ::app::widget::Setup`
/// is on the books long before a top-level `::app::widget show` further down
/// the file. The per-item shell pass defers that body, so the identical call
/// site is walked while `ensemble_subcommand_targets` is still empty and the
/// subcommand reference is silently never recorded — go-to-definition
/// (an on-demand lookup against the *finished* analysis) answered, but
/// find-references / rename / code-lens / call-hierarchy (which enumerate
/// recorded invocations) could not. Deferring the miss and replaying it in
/// [`Analyser::flush_pending_ensemble_subcommand_invocations`] restores the
/// whole-file result exactly.
#[derive(Debug, Clone, PartialEq)]
pub(in super::super) struct PendingEnsembleSubcommand {
    /// The ensemble's own resolved command name — the
    /// [`super::super::types::AnalysisResult::ensemble_subcommand_targets`] key the
    /// walk would have looked up, computed exactly as the immediate path
    /// computes it so the replay cannot resolve differently.
    pub ensemble: String,
    /// The static subcommand word.
    pub sub: String,
    /// The subcommand word's span — the reference this records.
    pub span: Span,
    /// The ensemble invocation's head offset, where dispatch looks up the
    /// target; `None` for a callback prefix that dispatches later.
    pub head: Option<u32>,
    /// Arguments *after* the consumed subcommand word (`None` when any is
    /// `{*}`-expanded), the same convention the immediate path uses.
    pub argc: Option<usize>,
}

/// One W315 candidate from an `oo::objdefine` walk, held
/// until the whole document is walked so
/// [`Analyser::flush_objdefine_abort_diagnostics`] can consult
/// document-wide facts before deciding it is real.
#[derive(Debug, Clone, PartialEq)]
pub(in super::super) struct ObjdefineAbortCandidate {
    /// The definition-aborting word, as the seeded walk recorded it.
    pub abort: super::super::types::DefinitionAbort,
    /// The receiver key the block was recorded under (variable simple name
    /// or resolved object name).
    pub receiver: String,
    /// The binding's accumulated table held conditional evidence when this
    /// block was walked — absence/presence judgements against it are not
    /// order-provable, so the emission abstains.
    pub prior_state_conditional: bool,
}

/// Child source declaration and scoped visibility advice, independently of
/// actual entered interpreter state or native hidden allocations.
#[derive(Debug, Clone, Default, PartialEq)]
pub(in super::super) struct InterpState {
    /// Created with `-safe`.
    pub safe: bool,
    /// Original scoped source visibility, independent of current callability.
    pub visibility: Option<super::super::SourceInterpreterVisibilitySnapshot>,
    /// An original visibility operation could not retain its source operands.
    /// Its evaluation bodies cannot reuse the withdrawn source snapshot.
    pub tainted: bool,
}

/// Source-only child visibility snapshots retain complete original inputs.
pub(in super::super) type SafeInterpCtx = super::super::SourceInterpreterVisibilitySnapshot;

/// One `interp eval` body on the walk stack — the interpreter-domain
/// identity of the script currently being analysed.
///
/// Pushed by `isolate_interp_eval_body` (`super::super::handlers`) for both
/// `interp eval PATH {…}` and the handle form `NAME eval {…}`, so every piece
/// of analyser state that models *per-interpreter runtime state* can key
/// itself by the domain the code actually executes in rather than merging
/// every interpreter's state into one flat file-wide bucket.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in super::super) struct InterpFrame {
    /// The interpreter path key relative-qualified against the enclosing
    /// frames (`s`, `s t`) — what `interp` operations *inside* the body
    /// qualify their own path operands against.
    pub key: String,
    /// The synthetic `@interp@<key>[#<epoch>]` domain identity minted for
    /// this body's scope (see `interp_domain_name` in `super::super::handlers`).
    /// Two evals into the same live interpreter share it; a
    /// deleted-and-recreated path does not.
    pub domain: String,
    /// `false` once this frame — or any frame enclosing it — targeted an
    /// interpreter whose path could not be resolved statically
    /// (`interp eval $unknown {…}`). The body then really runs in *some*
    /// interpreter we cannot name, so per-domain state must widen rather
    /// than treat the domain as distinct from every other.
    pub resolved: bool,
}

/// How a [`VarCommandSite`]'s command head names the object it dispatches
/// on — the axis that decides *where the receiver's class comes from* and
/// *which of its methods the call can reach*.
///
/// Every dispatch site the walker records is one of these three; adding a
/// fourth spelling means adding a variant here and a resolution arm beside
/// the others, never a name test in the walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchReceiver {
    /// `$obj method` — the head is a variable whose *value* is an object
    /// handle.  A `$var` can hold anything at run time, so its class
    /// evidence comes from the SSA type lattice / constructor harvest only.
    Variable,
    /// `objcmd method` — a named receiver retained by the genuine original
    /// source-instance issuer. The carrier supplies conditional class metadata;
    /// actual Native dispatch and object lifetime require their own proofs.
    InstanceCommand,
    /// `my method` — a bareword head the registry declares a **self-dispatch
    /// keyword** (`CommandRegistry::method_dispatch_keyword` answering
    /// `SelfDispatch`).  The receiver is the object whose
    /// method body encloses the call, so the class comes from
    /// `Analyser::enclosing_class_at_offset` with no name resolution at all
    /// — and, uniquely among the three, the dispatch bypasses export
    /// filtering, so it can reach unexported members.
    SelfDispatch,
}

impl DispatchReceiver {
    /// How this head reaches the receiver, for the registry's built-in
    /// object-method visibility rule.
    ///
    /// Only [`Self::SelfDispatch`] bypasses export filtering.  A bareword
    /// instance command is still the object's *own command*, so it sees
    /// exactly what `$obj` sees.
    pub(crate) fn method_reach(self) -> tcl_registry::definer::MethodReach {
        match self {
            Self::SelfDispatch => tcl_registry::definer::MethodReach::SelfDispatch,
            Self::Variable | Self::InstanceCommand => {
                tcl_registry::definer::MethodReach::ObjectCommand
            }
        }
    }
}

/// One entry in [`Analyser::var_command_sites`] —
/// `(var_name, method_name?, cmd_token_span, in_method)`.
///
/// Used by the W307 (variable-as-command misuse) post-pass.
#[derive(Debug, Clone, PartialEq)]
pub struct VarCommandSite {
    /// Variable name used as a command head (no leading ``$``).
    pub var_name: String,
    /// Optional method name when the call shape is
    /// ``$obj method args…``.
    pub method_name: Option<String>,
    /// Content span of the method-name word (delimiters trimmed), when
    /// [`Self::method_name`] is present — the tight anchor for W308.
    pub method_span: Option<Span>,
    /// Span of the command-head token.
    pub cmd_span: Span,
    /// True when the call site is inside a class method body.
    pub in_method: bool,
    /// Number of positional arguments passed at the dispatch site
    /// (`$cmd a b c` → 3, i.e. including the method-name word for a
    /// `$obj method a b` shape).  Used by the dispatch-protocol W214
    /// suppression to require an arity-compatible dispatcher.
    pub argc: usize,
    /// True when any word at the dispatch site (method name or
    /// arguments) is `{*}`-expanded, making the runtime argument count
    /// unknowable statically.  The W308 method-arity check abstains
    /// entirely when this is set, matching every other arity check's
    /// `{*}`-expansion convention.
    pub has_expand: bool,
    /// Which spelling named the receiver — see [`DispatchReceiver`].
    ///
    /// Kept as an explicit recorded fact rather than re-derived from the
    /// site's shape at diagnosis time, so the diagnostic and the LSP's
    /// shared resolver can never disagree about which sites a given
    /// receiver lookup applies to.
    pub receiver: DispatchReceiver,
}

impl VarCommandSite {
    /// Shift every span this site carries by `delta`.
    ///
    /// The **one** place a `VarCommandSite`'s spans are relocated from a
    /// per-item body fragment's local offsets into the whole document's, so
    /// a span field added later cannot be silently left un-rebased.  It once
    /// could: `method_span` was omitted from the hand-written rebase loop,
    /// and every W308 raised inside a proc or method body on the incremental
    /// path was reported — and its "did you mean" quick-fix anchored — at
    /// the *fragment's* offsets, landing on unrelated text elsewhere in the
    /// file.
    pub(crate) fn rebase(&mut self, delta: u32) {
        self.cmd_span = shift_span(self.cmd_span, delta);
        self.method_span = self.method_span.map(|sp| shift_span(sp, delta));
    }
}

/// One entry in [`Analyser::cmd_command_sites`] —
/// `([cmd] text, method_name?, cmd_token_span, in_method)`.
///
/// Same shape as [`VarCommandSite`] except the head is a
/// command-substitution rather than a variable.
#[derive(Debug, Clone, PartialEq)]
pub struct CmdCommandSite {
    /// Text of the bracketed command substitution (no brackets).
    pub cmd_text: String,
    /// Original nested invocation carriers stamped by the source owner.
    pub commands: Vec<crate::ir::CommandTokens>,
    /// Optional method name.
    pub method_name: Option<String>,
    /// Content span of the method-name word (delimiters trimmed), when
    /// [`Self::method_name`] is present — the tight anchor for W308.
    pub method_span: Option<Span>,
    /// Span of the whole `[…]` command-substitution head word, closing
    /// bracket included.
    ///
    /// Widened past the head token's own span with
    /// [`tcl_lexer::word_span`]: a `Cmd` token's raw span stops at the end
    /// of its *content*, so anchoring a diagnostic on it underlines
    /// `[Dog new` rather than `[Dog new]`, and on a head written across a
    /// line continuation it underlines a region that ends mid-word on the
    /// next line.  Only the end moves — every consumer that
    /// keys off `cmd_span.start()` (enclosing class, child-interp
    /// containment, head return type) is unaffected.
    pub cmd_span: Span,
    /// True when inside a class method body.
    pub in_method: bool,
}

impl CmdCommandSite {
    /// Shift every span this site carries by `delta` — see
    /// [`VarCommandSite::rebase`] for why this is a method rather than a
    /// per-field loop at the call site.
    pub(crate) fn rebase(&mut self, delta: u32) {
        for tokens in &mut self.commands {
            crate::lattice_rebase::rebase_command_tokens(tokens, i64::from(delta));
        }
        self.cmd_span = shift_span(self.cmd_span, delta);
        self.method_span = self.method_span.map(|sp| shift_span(sp, delta));
    }
}

/// Shift `span` right by `delta` — the primitive
/// [`VarCommandSite::rebase`] / [`CmdCommandSite::rebase`] are built from.
fn shift_span(span: Span, delta: u32) -> Span {
    Span::new(span.start() + delta, span.end() + delta)
}
