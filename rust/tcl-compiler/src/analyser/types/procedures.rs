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

//! Procedure records: `ProcDef` and its parameter traits, `DefinedSymbol`, the pending arity checks and the `unknown` handler's dispatch shape.

use super::original_constructor_call::OriginalConstructorArityCall;
use super::original_member_context::OriginalLexicalNextCall;
use crate::signature_scan::types::ParamDef;
use std::collections::HashMap;
use tcl_lexer::Span;

/// One statically recorded `namespace ensemble` subcommand: the command it
/// dispatches to, plus **how** the ensemble bound the two together.
///
/// The value half of
/// [`AnalysisResult::ensemble_subcommand_targets`]'s inner map. Navigation
/// (`definition` / `hover` / `references`) only wants [`Self::target`]; a
/// source-rewriting consumer (rename) has to read [`Self::provenance`] as
/// well, because the subcommand word is the target's own tail under
/// `-subcommands` and an arbitrary key under `-map`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnsembleSubcommandTarget {
    /// The resolved, qualified command name the subcommand dispatches to
    /// (`::app::widget::Show`). For a `-map` target that is a command
    /// *prefix* (`-map {go {string length}}`) this is the prefix's head —
    /// the command actually invoked.
    pub target: String,
    /// Which option declared the mapping.
    pub provenance: crate::signature_scan::types::EnsembleSubcommandProvenance,
}

/// A diagnostic request position and reporting label, not a selected target.
/// Post-walk user arity advice reborrows its genuine original call/header/argv;
/// these fields cannot supply namespace identity, captured arguments or edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUserCallArity {
    /// Presentation only, including literal sigil bytes as originally reported.
    pub cmd_name: String,
    /// Original command-head request position; the shared source issuer must
    /// independently authenticate the complete current source/input at this site.
    pub call_off: u32,
}

/// Queued genuine source factory/call signature, with no runtime receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCtorArity {
    /// Complete source-call issuer and independent source-order requirement.
    pub original: std::sync::Arc<OriginalConstructorArityCall>,
    /// Reporting label, never a class/provider selection key.
    pub display_name: String,
}

/// A queued readonly next-chain signature question with exact source identity.
/// Declaration context and Registry source shape supply no entered receiver,
/// installed method, current dispatch or successful completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNextArity {
    /// Retained declaration/body and complete selected source invocation.
    pub original: std::sync::Arc<OriginalLexicalNextCall>,
    /// Reporting label used only in the eventual diagnostic.
    pub display_name: String,
}

/// How a proc parameter is used inside the proc body.
///
/// Drives optimisation, shimmer analysis, taint propagation, and
/// diagnostics — tells downstream passes how a parameter value
/// flows through the proc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcArgTrait {
    /// Argument is eval'd as a script (``eval`` / ``uplevel`` /
    /// ``subst``).
    Eval,
    /// Argument is used as a loop / control body.
    Body,
    /// Argument names a variable that the proc writes (upvar +
    /// set, or a registry-marked write site).
    VarWrite,
    /// Argument names a variable that the proc reads via
    /// ``upvar`` (read-only alias).
    VarRead,
    /// Argument is evaluated as an expression.
    Expr,
    /// Argument is used as the list in a ``foreach`` / ``lmap``.
    LoopList,
    /// The parameter's **value** is used as a variable *name* in the
    /// proc's **own** (callee-local) scope — e.g. ``set $p 1``,
    /// ``scan $s %d $p``, ``lassign $l $p``, ``regsub … $p``, or a
    /// registry ``VarWrite`` / ``VarRead`` role landing on a bare
    /// ``$param`` substitution.
    ///
    /// Distinct from [`VarWrite`](Self::VarWrite) /
    /// [`VarRead`](Self::VarRead): those imply the param *aliases* a
    /// caller-frame variable via ``upvar`` (so passing a literal name
    /// at the call site consumes the caller's variable).  This trait is
    /// callee-local only — ``f x`` does **not** consume the caller's
    /// ``x``; the callee merely uses the string ``x`` to name one of
    /// its own locals.  It is always emitted alongside `VarRead` (the
    /// param's string value *is* read), so consumers querying
    /// `VarRead` alone for "is the param used at all" still see it;
    /// the refinement only matters for caller-side dead-store /
    /// unused-variable suppression, which must skip a param that is
    /// `DynamicNameLocal` without also being a genuine `VarWrite`.
    DynamicNameLocal,
    /// The parameter's **value** is used as a **command name** — either the
    /// command word of an invocation (``$cmd arg1 arg2``) or a registry / stub
    /// ``CommandPrefix`` callback argument (a command prefix such as
    /// ``tcltest::customMatch``'s matcher, ``selection handle``'s handler, or a
    /// stub ``:command_prefix`` argument).  Passing a literal at the call site
    /// therefore names a command, which a consumer can resolve (call graph) or
    /// highlight as a command.
    Command,
}

impl ProcArgTrait {
    /// Stable lower-case name suitable for serialisation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProcArgTrait::Eval => "eval",
            ProcArgTrait::Body => "body",
            ProcArgTrait::VarWrite => "var_write",
            ProcArgTrait::VarRead => "var_read",
            ProcArgTrait::Expr => "expr",
            ProcArgTrait::LoopList => "loop_list",
            ProcArgTrait::DynamicNameLocal => "dynamic_name_local",
            ProcArgTrait::Command => "command",
        }
    }
}

/// Proc definition record.
///
/// Reuses the [`ParamDef`] type the signature scanner uses — same
/// param shape, same parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcDef {
    /// Exact authored publication geometry, independent of this declaration's report.
    pub source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    /// Proc name as written (no namespace qualifiers).
    pub name: String,
    /// Fully-qualified proc name with leading ``::``.
    pub qualified_name: String,
    /// Parameter list in declaration order.
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not
    /// "none" — see that field.
    pub params: Vec<ParamDef>,
    /// Original formal count owner or explicit authored metadata. Display
    /// parameter labels cannot recreate an original binding/count recipe.
    pub formal_count: crate::signature_scan::formal_count::SourceFormalCount,
    /// The parameter-list word was **computed**, so the proc's formals are
    /// unmodelled.
    ///
    /// `proc p [makeargs] {…}` / `proc q $params {…}` build the formal list at
    /// definition time from a value; which names it declares, and how many,
    /// are run-time facts (tclsh 9.0.4 / 8.6.16: with
    /// `proc makeargs {} {return {a b}}`, `proc p [makeargs] {…}` then
    /// `info args p` → `a b`, and `p 1 2` runs).  Reading the unresolved word
    /// as a one-parameter literal would register a `VarDef` whose *name is the
    /// source text* (`"[makeargs]"`) and make the call-site arity checker
    /// demand exactly one argument.  Nothing is claimed instead: no
    /// per-parameter `VarDef`, and the arity resolvers abstain
    /// (`0..unlimited`).
    pub params_computed: bool,
    /// Source span of the proc-name token.
    pub name_span: Span,
    /// Body geometry retained by this declaration's producer.
    /// Token-based declarations can retain group geometry; finite source
    /// reports can retain the complete delimited word. This is not a uniform
    /// delimiter-free content span. Content readers and editors must use the
    /// matching original body word's canonical content span.
    pub body_span: Span,
    /// Doc-comment text harvested from the line(s) above the
    /// ``proc`` statement, or empty when none was found.
    pub doc: String,
    /// Inferred parameter usage traits, keyed by parameter
    /// name.  Populated by ``infer_param_traits`` after the
    /// body walk.  Empty when no traits inferred (parameter
    /// unused, or proc body wasn't statically scannable).
    ///
    /// These are conditional May source-usage reports. They retain no actual
    /// alias, variable cell, quiet observer, entered frame, Native/Normal or
    /// completion grant. Consumers independently select the original callee,
    /// formals and caller-source template before navigation or display. This
    /// analyser type is separate from the IR-produced
    /// [`crate::interprocedural::ProcArgTrait`] used by optimiser summaries.
    pub param_traits: HashMap<String, std::collections::HashSet<ProcArgTrait>>,
    /// Parameters whose *value* names a variable in the **immediate caller's**
    /// frame — the `upvar 1 $param local` shape, and only that one.
    ///
    /// Strictly narrower than the [`ProcArgTrait::VarWrite`] /
    /// [`ProcArgTrait::VarRead`] entries in [`Self::param_traits`], which say
    /// a parameter's value is used as a variable *name* through an `upvar`
    /// but not which frame the alias lands in.  `upvar 0` aliases the
    /// callee's own frame, `upvar #0` the global one, `upvar 2` the caller's
    /// caller — none of them creates anything in the calling frame, so a
    /// call-site consumer (hover / go-to-definition / find-references on a
    /// caller-frame variable) must intersect the two rather than trust the
    /// trait alone.  Populated by
    /// [`super::super::param_traits::caller_frame_upvar_params`].
    /// A symbolic source recipe only: conditional formal reads and unknown
    /// observer/caller applicability cannot establish an entered caller alias.
    pub caller_frame_params: std::collections::HashSet<String>,
    /// **Literal** caller-frame names this proc binds in its immediate
    /// caller's frame — spelled in the proc's *own* body (`upvar 1 name
    /// name`), so no call-site argument word carries them.
    /// `name → written-through-alias`: `true` means
    /// the conditional callee template writes through the alias, `false` that
    /// it only reads through it. This proves no actual caller cell or completed
    /// write. Navigation requires a separate original call/allocation/frame
    /// receipt. Populated by
    /// [`super::super::param_traits::caller_frame_literal_targets`]; empty when
    /// the body binds none.
    /// Readonly source usage, never a successful caller store or physical cell.
    pub caller_frame_literals: std::collections::HashMap<String, bool>,
}

impl ProcDef {
    /// Callable source spelling only when both publication and lookup round-trip.
    #[must_use]
    pub fn source_spelling(&self) -> Option<String> {
        self.source_name.as_ref()?.source_spelling()
    }
    /// The proc's declared argument arity — or the **abstaining**
    /// `0..unlimited` when its parameter list was computed
    /// ([`Self::params_computed`]).
    ///
    /// A computed list declares an unknown number of formals with unknown
    /// names, so no call-site count can be wrong: `proc p [makeargs] {…}` with
    /// `makeargs` returning `{a b}` really does take two arguments (tclsh
    /// 9.0.4 / 8.6.16: `p 1 2` → `p got 1 2`), and reading the unresolved word
    /// as one literal parameter made `p 1 2` draw a false
    /// `E003 Too many arguments`.  Every arity consumer asks here rather than
    /// calling [`crate::signature_scan::arity::arity_of`] on `params`
    /// directly, so the abstention cannot be forgotten at one call site.
    #[must_use]
    pub fn arity(&self) -> tcl_registry::Arity {
        self.formal_count_projection().arity()
    }

    /// Stable descriptive header count, without source bytes or runtime grants.
    #[must_use]
    pub fn formal_count_projection(
        &self,
    ) -> crate::signature_scan::formal_count::SourceFormalCountProjection {
        self.formal_count
            .projection(&self.params, self.params_computed)
    }
}

/// A lightweight *named definition* introduced by a registry
/// symbol-definer command (a `tcltest::test NAME …` case, …).
///
/// Unlike [`ProcDef`] / [`ClassDef`] these carry no parameter list or member
/// table — just enough to list the name in the document / workspace outline and
/// jump to it.  The analyser records one per call to a command whose registry
/// spec declares a [`tcl_registry::SymbolDef`]; the argument index and category
/// come from that descriptor, so no command name is hardcoded here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinedSymbol {
    /// Independently produced original name under positioned Registry advice.
    /// None belongs only to explicitly logical declaration reporting; this is
    /// neither a command publication nor an editable computed name.
    pub original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    /// Definition name as resolved (constant-propagated from the name
    /// argument).  For a test this is the test-case label (`foo-1.1`).
    pub name: String,
    /// Fully-qualified name with leading ``::`` (the enclosing namespace
    /// applied), for workspace-symbol container grouping.
    pub qualified_name: String,
    /// The outline category, straight from the registry descriptor.
    pub kind: tcl_registry::DefinedSymbolKind,
    /// Source span of the name argument's token — the outline selection range.
    pub name_span: Span,
    /// Source span covering the whole call (name token through the last
    /// argument), used as the outline entry's fold range.
    pub full_span: Span,
    /// Short description harvested from the descriptor's detail argument when
    /// it resolves to a constant, else `None`.
    pub detail: Option<String>,
}

/// Analysis result from a user-defined ``unknown`` proc.
///
/// Populated when the analyser encounters ``proc unknown {cmd args}
/// { ... }``: the body is lowered to IR and inspected for
/// dispatch shapes (switch arms, ``exec``, ``auto_load``,
/// chains to a saved ``_original_unknown``, case-folding
/// dispatch).  The result gates the W123 (unresolved command)
/// emitter so commands handled by ``unknown`` aren't false-
/// positived.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
// `chains_original` / `empty_stub` / `case_insensitive` /
// `has_pattern_dispatch` / `has_exec` / `has_auto_load` are
// orthogonal facts detected about the body of an ``unknown``
// proc, each consumed independently by the W123 suppression
// logic, so they stay as separate bools rather than a bitflags set.
#[allow(clippy::struct_excessive_bools)]
pub struct UnknownProcInfo {
    /// Command names explicitly dispatched (e.g. switch arm
    /// labels).
    pub dispatch_targets: std::collections::BTreeSet<String>,
    /// Canonical possible source bodies with their option-selection and
    /// no-match residuals. These describe advisory syntax, independently of
    /// selected effects, current command occupancy or Native execution.
    pub case_body_residuals: Vec<tcl_registry::case_bodies::CaseBodyOperands>,
    /// Calls a renamed original ``unknown`` (e.g.
    /// ``_original_unknown``).
    pub chains_original: bool,
    /// Body is empty — nothing resolves at all.
    pub empty_stub: bool,
    /// Normalises case before dispatch (all known commands are
    /// valid).
    pub case_insensitive: bool,
    /// Uses glob or regexp switch dispatch — opaque match
    /// semantics.
    pub has_pattern_dispatch: bool,
    /// Calls ``exec`` — opaque external dispatch.
    pub has_exec: bool,
    /// Calls ``auto_load`` — dynamic package loading.
    pub has_auto_load: bool,
}
