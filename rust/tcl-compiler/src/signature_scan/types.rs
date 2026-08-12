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

//! Public record types emitted by the signature scanner.
//!
//! Spans are [`tcl_lexer::Span`].

use std::collections::BTreeMap;

use tcl_lexer::Span;

/// A single Tcl proc parameter declaration.
///
/// The `default_value`
/// is the literal text following the parameter name inside a braced
/// `{name default}` form — whitespace before it is stripped, whitespace
/// inside the default text is preserved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamDef {
    /// Parameter name as written in the proc declaration.
    pub name: String,
    /// `true` when the parameter has a default value.
    pub has_default: bool,
    /// The default-value text when [`Self::has_default`] is `true`.
    pub default_value: Option<String>,
}

/// A `proc` definition recorded by the signature scanner.
///
/// Records a focused subset: name, qualified name, parameter
/// list, name-token range, body-token range. Diagnostics, scope-tree
/// references, and other heavy analyser fields are intentionally
/// absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureProc {
    /// Unqualified proc name (the trailing component of the qualified
    /// name).
    pub name: String,
    /// Fully-qualified proc name with leading `::`.
    pub qualified_name: String,
    /// Parsed parameter list.
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not "none"
    /// — see that field.
    pub params: Vec<ParamDef>,
    /// The parameter-list word was **computed**, so the proc's formals are
    /// unmodelled (issue #1107, the signature-scan twin of
    /// [`crate::analyser::ProcDef::params_computed`]).
    ///
    /// `proc p [makeargs] {…}` / `proc q $params {…}` build the formal list at
    /// definition time from a run-time value. Reading the unresolved word as a
    /// one-parameter literal recorded a parameter literally named
    /// `"[makeargs]"` and told the **cross-file** arity check the proc takes
    /// exactly one argument — a false `E003` on code both interpreters run
    /// (`proc makeargs {} {return {a b}}`; `proc p [makeargs] {…}`;
    /// `info args p` → `a b`; `p 1 2` runs). Consumers must ask
    /// [`Self::arity`] rather than computing an arity from `params`.
    pub params_computed: bool,
    /// Source span of the name argument.
    pub name_range: Span,
    /// Source span of the body argument.
    pub body_range: Span,
}

impl SignatureProc {
    /// The proc's declared argument arity — or the **abstaining**
    /// `0..unlimited` when its parameter list was computed
    /// ([`Self::params_computed`]).
    ///
    /// The signature-scan twin of [`crate::analyser::ProcDef::arity`]; both
    /// tiers abstain identically so a cross-file arity check cannot be
    /// stricter than the same-file one.
    #[must_use]
    pub fn arity(&self) -> tcl_registry::Arity {
        if self.params_computed {
            return tcl_registry::Arity::new(0, tcl_registry::Arity::UNLIMITED);
        }
        super::arity::arity_of(&self.params)
    }
}

/// A class definition recorded by the signature scanner.
///
/// Covers both `oo::class create NAME ?BODY?` and
/// `itcl::class NAME BODY` forms — the surface fields are identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureClass {
    /// Unqualified class name.
    pub name: String,
    /// Fully-qualified class name with leading `::`.
    pub qualified_name: String,
    /// Source span of the name argument.
    pub name_range: Span,
    /// Source span of the body argument (or the name span when the
    /// body is absent — e.g. `oo::class create NAME` without a body).
    pub body_range: Span,
}

/// A `package require` invocation recorded by the signature scanner.
///
/// `version` is `None` when the call supplied no version constraint.
/// `conditional` is `true` when the call lives inside a guarded
/// branch (an `if`/`elseif`/`else` body, a `catch` script, or a
/// `try`/`on`/`trap`/`finally` clause) so workspace-level Tcl-version
/// inference does not promote a guarded `package require Tcl 8.6` to
/// an unconditional minimum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePackageRequire {
    /// Package name (the `NAME` argument to `package require`).
    pub name: String,
    /// Optional version constraint (the `VERSION` argument); `None`
    /// when no version is supplied.
    pub version: Option<String>,
    /// `true` when the call carried the `-exact` flag, which turns
    /// `version` from the ranged requirement `V` (`[V, next major)`)
    /// into the degenerate range `V-V` — see
    /// [`tcl_dialect::exact_requirement`].  Meaningless without a
    /// `version`: `package require -exact NAME` is a syntax error in
    /// real Tcl, and consumers treat the pair as unconstrained.
    pub exact: bool,
    /// Source span of the name argument.
    pub range: Span,
    /// `true` when the call is inside a guarded branch.
    pub conditional: bool,
}

/// A `package prefer latest` invocation — the one form of `package
/// prefer` that changes anything (issue #1126 item 1).
///
/// `package prefer` sets the interpreter-global rule `package
/// require` uses to choose between the highest acceptable version
/// and the highest acceptable **stable** one.  Only the raise to
/// `latest` is recorded, because it is the only transition that
/// exists:
///
/// * the default is `stable`, so `package prefer stable` from the
///   default changes nothing;
/// * `latest` is sticky — the manpage says an attempt to set it back
///   is silently ineffective, and the interpreter agrees.
///
// tclsh-proof: on tclsh8.6 (8.6.14) `package prefer` → `stable`;
// `package prefer latest` → `latest`; a following `package prefer
// stable` returns `latest` with no error, and `package prefer` still
// answers `latest`.
///
/// So the state at any point is the monotone predicate "has a
/// `package prefer latest` already run", which is why this record
/// carries no mode field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePackagePrefer {
    /// Source span of the `package` command word, matching
    /// [`SignaturePackageRequire::range`]'s anchoring.
    pub range: Span,
    /// `true` when the call is inside a guarded branch (an
    /// `if`/`elseif`/`else` body, a `catch` script, or a
    /// `try`/`on`/`trap`/`finally` clause), so a consumer can refuse
    /// to flip the selection rule on a statement that may not run.
    pub conditional: bool,
}

/// A `source` invocation recorded by the signature scanner.
///
/// `is_literal` is `true` when the path argument contains no `$` or
/// `[` — the segmenter reconstructs substituted words with the
/// `${var}` / `[cmd]` markers preserved, so their absence is reliable
/// evidence the path is a plain literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureSource {
    /// Verbatim path text as reconstructed by the segmenter (with
    /// `${var}` / `[cmd]` markers preserved for substituted words).
    pub raw_path: String,
    /// Source span of the path argument.
    pub range: Span,
    /// `true` when the path is a plain literal (no `$` or `[`).
    pub is_literal: bool,
    /// Command-resolution namespace at the `source` call site (a constructed
    /// `::`-rooted key).  `source` evaluates the file **in the caller's
    /// current namespace** (M9): a bare `proc helper` in a file sourced
    /// inside `namespace eval ::x` lands in `::x::helper`, so the workspace
    /// index re-homes the sourced document's definitions under this
    /// namespace.
    pub site_namespace: String,
}

/// A local-interpreter `interp alias` recorded by the signature scanner.
///
/// Only the form `interp alias {} ALIAS {} TARGET ?ARG…?` (both
/// slave and target paths empty) is recorded — cross-interpreter
/// aliases do not affect command resolution in the current
/// workspace and are skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureCommandAlias {
    /// Fully-qualified alias name (the `ALIAS` argument with leading
    /// `::` applied).
    pub qualified_name: String,
    /// The target command name (the `TARGET` argument).
    pub target: String,
    /// The optional pre-bound arguments appended after `TARGET`.
    pub extras: Vec<String>,
}

/// A `rename OLD NEW` recorded by the signature scanner.
///
/// `NEW` becomes a callable command name subject to ordinary command-name
/// resolution — like `proc`, a bare `NEW` inside a `namespace eval` is
/// namespace-relative, unlike `interp alias`'s always-global aliasName. Only
/// recorded when `NEW` is non-empty: `rename OLD {}` deletes `OLD` rather
/// than introducing a new name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureRename {
    /// Fully-qualified new command name (with leading `::`).
    pub qualified_name: String,
    /// The old command name text as written at the call site.
    pub target: String,
}

/// A `namespace import` recorded by the signature scanner.
///
/// Records both direct `namespace import PATTERN` calls and the
/// tcllib `<NS>::import <ALIAS>` wrapper idiom. The latter sets
/// `conjectured` to `true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureNamespaceImport {
    /// Importing namespace, with leading `::`.
    pub ns: String,
    /// Imported pattern, fully-qualified (relative patterns are
    /// resolved against `ns`).
    pub pattern: String,
    /// Source span of the pattern argument.
    pub range: Span,
    /// `true` when the import is inferred from a tcllib-style
    /// `<NS>::import <ALIAS>` call rather than a direct `namespace
    /// import` invocation.
    pub conjectured: bool,
    /// `true` when the call consumed its leading option word — i.e. the
    /// import is `namespace import -force …`.
    ///
    /// The distinction is not cosmetic: it decides what the import does to a
    /// command of the same name that the importing namespace *already* holds
    /// (oracle tclsh 8.6.14 / 9.0.4, issue #1103).
    ///
    /// - Without `-force`, the import **fails**: `namespace eval ::dst {proc
    ///   p {} {return LOCAL}}` then `namespace eval ::dst {namespace import
    ///   ::src::*}` raises `can't import command "p": already exists`, the
    ///   rest of that script never runs, and `::dst::p` still runs `LOCAL`
    ///   (`namespace origin ::dst::p` → `::dst::p`). The import installed
    ///   nothing.
    /// - With `-force`, the import silently **replaces** the local command:
    ///   `::dst::p` then runs `SRC` and `namespace origin ::dst::p` →
    ///   `::src::p`.
    ///
    /// Which leading words are options, and how many are consumed, is
    /// registry data (`IMPORT_OPTIONS` + `SubCommand::max_leading_option_words`
    /// = 1) — this flag is "the declared leading option word was consumed",
    /// not a `-force` string match, exactly as `namespace export`'s
    /// [`SignatureNamespaceExport::clears`] is.
    pub forced: bool,
}

/// A `namespace forget` **event** recorded by the signature scanner.
///
/// `namespace import` does not create a permanent name: `namespace forget`
/// removes the alias again, and a later bare call is `invalid command name`
/// (oracle tclsh 8.6.14 / 9.0.4, byte-identical — issue #1103):
///
/// ```tcl
/// namespace eval ::src { proc p {} {return P}; namespace export p }
/// namespace eval ::dst { namespace import ::src::* }
/// namespace eval ::dst { p }                  ;# → P
/// namespace eval ::dst { namespace forget ::src::p }
/// namespace eval ::dst { p }                  ;# → invalid command name "p"
/// ```
///
/// So the import edge is not a static name-visibility fact but a link with a
/// lifecycle, and these records are the *removal* half of its ordered event
/// log — the exact counterpart of [`SignatureNamespaceExport`]'s `-clear`
/// tombstones, consumed the same way (latest visible event wins, order-gated
/// by `analyser::indirection::in_effect`) through
/// `tcl_lsp_core::namespace_import::alias_live_at`.
///
/// # Two pattern shapes
///
/// `Tcl_ForgetImport` (`tclNamesp.c`) branches on whether the pattern is
/// qualified, and both forms are oracle-confirmed:
///
/// - **Qualified** (`namespace forget ::src::p`, `::src::*`) — every command
///   in `::src` matching the tail pattern has its *import* in the current
///   namespace removed. [`Self::source_ns`] is `Some("::src")`.
/// - **Simple** (`namespace forget p`, `*`) — every *imported* command of the
///   current namespace whose own name matches the pattern is removed,
///   whatever it was imported from. [`Self::source_ns`] is `None`.
///   (Oracle: with `::dst::p` imported from `::src`, `namespace eval ::dst
///   {namespace forget p}` leaves `info commands ::dst::*` empty and raises
///   no error.)
///
/// Forgetting a name that was never imported is a silent no-op, not an error
/// — only an unknown *namespace* in a qualified pattern errors (`unknown
/// namespace in namespace forget pattern "::nope::x"`), which is a
/// diagnostics question, not a resolution one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureNamespaceForget {
    /// The namespace the forget runs in — the one losing the aliases, with
    /// leading `::`.
    pub ns: String,
    /// The pattern's source namespace with leading `::` when the pattern was
    /// qualified (`::src` for `::src::p`), `None` for a simple pattern.
    ///
    /// `None` matches *any* source: a simple pattern is matched against the
    /// forgetting namespace's own imported command names.
    pub source_ns: Option<String>,
    /// The pattern's final `::`-segment, exactly as written (`p`, `*`,
    /// `get*`) — matched against a command's bare name with Tcl glob
    /// semantics.
    pub pattern: String,
    /// Source span of the pattern argument.
    pub range: Span,
}

/// One `namespace export` **event** recorded by the signature scanner.
///
/// Gates which commands a wildcard `namespace import NS::*` elsewhere may
/// actually reach: real Tcl only imports names `NS` has exported
/// (`Tcl_Export`, `tclNamesp.c`) — an unexported sibling command living in
/// `NS` is not reachable through the import at all (tclsh9.0/8.6-verified:
/// `invalid command name` calling it bare). Issue #923 idx 18.
///
/// # Why an event log, not a set
///
/// A namespace's export list is *mutable state on a timeline*, and
/// `namespace import` snapshots it at the instant the import runs — later
/// changes never reach back (issue #1027). Both directions are observable
/// (oracle, tclsh 8.6.14 / 9.0.4):
///
/// - `namespace export -clear` **after** an import does not revoke the alias
///   the import already installed: with `::src` exporting `p`, `namespace
///   import ::src::*` in `::dst`, then `namespace export -clear` in `::src`,
///   `::dst::p` still runs `::src::p`.
/// - Exporting a name **after** an import does not add it retroactively:
///   importing `::src::*` while nothing is exported, then `namespace export
///   p`, leaves `::dst::p` an `invalid command name`. Only a *later* import
///   picks the new name up.
///
/// So the records are kept as an ordered, append-only log — one entry per
/// pattern word, plus a [`Self::clears`] tombstone for each `-clear` — and
/// consumers reconstruct the export set *as of a given offset* rather than
/// reading a flat final set. Collapsing `-clear` eagerly (dropping the
/// namespace's earlier entries, as this type's first implementation did)
/// destroys exactly the ordering the snapshot needs.
///
/// Note that an export pattern is a *name pattern*, not a reference to a
/// command: `namespace export p` before `proc p` is written still exports
/// `p` (oracle-verified), so nothing here is gated on the command existing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureNamespaceExport {
    /// Exporting namespace, with leading `::`.
    pub ns: String,
    /// Exported pattern text, exactly as written. Always relative to `ns`
    /// (Tcl's `namespace export` patterns are simple, unqualified glob
    /// patterns matched against a command's tail name — never `::`-qualified;
    /// a `::`-qualified one is a hard error in real Tcl: `invalid export
    /// pattern "::foo": pattern can't specify a namespace`).
    ///
    /// Empty when this record is a [`Self::clears`] tombstone, which carries
    /// no pattern of its own.
    pub pattern: String,
    /// Source span of the pattern argument — of the `-clear` word itself for
    /// a [`Self::clears`] tombstone.
    pub range: Span,
    /// `true` when this record is the `-clear` tombstone of a `namespace
    /// export -clear ?pattern …?` call rather than an exported pattern.
    ///
    /// A tombstone revokes every pattern this namespace recorded *before* it;
    /// the same call's own patterns are recorded after it and survive
    /// (`namespace export a b; namespace export -clear p` leaves exactly `p`
    /// exported — oracle-verified).
    pub clears: bool,
}

/// An `auto_path` mutation recorded by the signature scanner.
///
/// Covers both `lappend auto_path …` and `set auto_path …` forms.  One record
/// per argument **word**; resolution to absolute paths happens later in the
/// analyser pipeline.
///
/// Note that a word is not always one directory: `set auto_path` assigns a
/// Tcl *list*, so its single word may name several.  The analyser's own
/// [`crate::analyser::types::AutoPathEntry`] tags which form wrote it, and
/// [`crate::auto_path_eval::evaluate_auto_path_entry`] applies the
/// distinction; this scanner-side record is raw text with no such tag, so a
/// consumer must not read it as a directory list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureAutoPathEntry {
    /// Verbatim path-element text as reconstructed by the segmenter.
    pub raw: String,
    /// Source span of the path-element argument.
    pub range: Span,
}

/// How a `namespace ensemble` subcommand got its name — the option that
/// declared it (issue #1281).
///
/// The two options bind the subcommand word to its target in opposite ways,
/// and a consumer that **rewrites source text** has to tell them apart
/// (tclsh 8.6.14 / 9.0.4, identical):
///
/// - `-map {show ::app::widget::Show}` — the key is an *arbitrary* name with
///   no required relationship to the target. `::app::widget show` runs
///   `::app::widget::Show`, and `::app::widget Show` is `unknown or ambiguous
///   subcommand "Show": must be show`. Renaming the target therefore must
///   **not** touch the subcommand word; the pairing lives in the `-map`
///   value, which is a separate reference the rename does rewrite.
/// - `-subcommands {alpha}` — the entry *is* the target's tail; the ensemble
///   derives `::app::widget::alpha` from it. Leaving the word behind when the
///   proc is renamed gives `invalid command name "alpha"`, so renaming the
///   target **must** rewrite the dispatch word (and the `-subcommands` entry)
///   along with the declaration.
///
/// A dynamic value (`-map [list …]`) records no mapping at all, so it has no
/// provenance to carry and every consumer abstains — see
/// [`crate::analyser::types::AnalysisResult::ensemble_subcommand_targets`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnsembleSubcommandProvenance {
    /// Declared by `-map {sub target …}`: the subcommand word is arbitrary
    /// and independent of the target's name.
    Map,
    /// Declared by `-subcommands {name …}`: the subcommand word *is* the
    /// target command's tail.
    Subcommands,
}

/// A single command invocation recorded by the signature scanner.
///
/// One record per command in the source — populated for every
/// non-partial command the walker visits. Used by
/// `WorkspaceIndex.command_usage_counts()` so background-scanned
/// files still contribute to cross-file command-usage statistics.
///
/// `resolved_qualified_name` is `None` when populated by the
/// signature scanner (the full scope walk required to resolve
/// it is what `signature_scan` skips for background files); the
/// full analyser populates it during its body walk so the LSP
/// references / document-highlight providers can match call
/// sites against a proc's qualified name even when the call
/// site uses a relative form.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent per-call-site flags, not a state machine
pub struct SignatureCommandInvocation {
    /// Command head as written at the call site (no namespace
    /// resolution performed).
    pub name: String,
    /// Source span of the command-head token.
    pub range: Span,
    /// Scope-resolved qualified name, if the analyser was able
    /// to compute one.  `None` from background-file scans
    /// (which skip the scope walk); `Some("::ns::name")` from
    /// the full analyser.
    pub resolved_qualified_name: Option<String>,
    /// `true` when [`Self::resolved_qualified_name`] was proved, for this
    /// call's own execution position, to be a proc or class definition in the
    /// analysed document.  This is deliberately narrower than "known": a
    /// registry builtin or command-name link has no direct declaration for a
    /// references consumer to attach to.
    ///
    /// The distinction matters after a later load-level `rename NAME {}`:
    /// the document's final command table no longer contains `NAME`, but a
    /// body that was provably invoked before the deletion still called that
    /// definition.  Workspace indexing carries this per-site fact instead of
    /// trying to reconstruct execution order from the final live-name set.
    pub resolved_user_definition: bool,
    /// The full ordered command-resolution candidate list for this call —
    /// every qualified name it could name, in Tcl priority order (caller
    /// namespace, then each `namespace path` entry, then global), as produced
    /// by `command_resolution_candidates`.  Populated by
    /// `finalise_invocation_resolutions` once the namespace/path context is
    /// known; empty from background scans.  A cross-document consumer runs
    /// these through a *workspace-wide* existence check to settle a call the
    /// single file could not (the local-first `resolved_qualified_name` is only
    /// a within-file guess).
    pub resolution_candidates: Vec<String>,
    /// Number of **argument** words at the call site (the words after the command
    /// head).  Used by cross-file arity checking: a call
    /// to a workspace-defined proc whose `argc` fits none of that proc's
    /// arities is a wrong-argument-count error.  `{*}`-expanded args make the true
    /// count unknown, recorded as `None` so arity checking conservatively skips.
    /// Always `None` for a **command-prefix callback head** ([`Self::callback_arity`]
    /// `.is_some()`) — a callback isn't literally invoked with N arguments *at
    /// this span*, so it must stay invisible to this legacy direct-call check;
    /// [`Self::callback_baked_args`] is the field the callback-arity check reads.
    pub argc: Option<usize>,
    /// `Some(arity)` when this invocation is a **command-prefix callback head**
    /// (`lsort -command myCompare` records `myCompare` with the appended
    /// arity), `None` for an ordinary direct call.
    pub callback_arity: Option<tcl_registry::AppendedArity>,
    /// For a callback head ([`Self::callback_arity`] `.is_some()`), the count
    /// of *baked* prefix args already present in the prefix literal — `0` for
    /// a bare word (`-command cb`), `N` for a braced multi-word prefix
    /// (`-command {cb a b}` bakes 2). Meaningless (and unread) when
    /// `callback_arity` is `None`. The callback arity check validates
    /// `baked + appended` against the referenced proc.
    pub callback_baked_args: usize,
    /// The span does **not** carry the written command name (M7): the site
    /// invokes the command *indirectly* — a constant `$cmd` head, a dispatch-
    /// table literal consumed elsewhere — so navigation (references,
    /// go-to-definition, call hierarchy) may use it, but **rename and every
    /// other span-rewriting consumer must skip it** (rewriting the span would
    /// splice the new command name over unrelated source text).
    pub indirect: bool,
    /// `false` when renaming this invocation's resolved command cannot be
    /// completed soundly from source edits alone: the site reaches the
    /// command through a value at least one of whose contributing constant
    /// definitions has **no exact writable source span** (a synthesised /
    /// folded constant, a list element the harvester cannot span).  Rename
    /// must abstain for the whole symbol rather than emit an edit set that
    /// leaves this site dispatching the old name (issue #945 fault 1: a
    /// "successful" rename that produces broken Tcl).  `true` for every
    /// ordinary direct call (the span *is* the written name) and for
    /// indirect sites whose full contributor set is writable (their
    /// literal-anchored twin references carry the edits).
    pub rename_safe: bool,
    /// `true` when this reference comes from an existence **probe**
    /// ([`tcl_registry::ArgRole::CommandNameProbe`] — `namespace which
    /// -command NAME`, an exact `info commands NAME`): the named command
    /// legitimately may not exist, so the W123 unresolved-command pass
    /// must skip this record — reference identity and existence assertion
    /// are orthogonal (issue #945 fault 9).  Navigation and rename treat
    /// the record exactly like any other direct reference.
    pub existence_probe: bool,
    /// `true` for an `expr` math-function call (`sin($x)`, `max($a, $b)`,
    /// recorded by `record_expr_function_invocations`). `name` is the bare
    /// function word and `resolved_qualified_name` is the *local-first*
    /// `{ns}::tcl::mathfunc::name` candidate for the call's own namespace —
    /// unlike every other invocation, whose resolved name has the ordinary
    /// `{ns}::{name}` shape (`ns` a single hop from `name`), a mathfunc
    /// invocation always carries the fixed two-segment `tcl::mathfunc`
    /// dispatch prefix between them. This flag — not a shape guess re-derived
    /// from the resolved string — is what tells
    /// [`crate::analyser::Analyser::finalise_invocation_resolutions`] to
    /// settle it against the dedicated two-candidate mathfunc rule instead of
    /// the generic one-hop suffix-strip, which would otherwise misparse
    /// `tcl::mathfunc` as if it were the calling namespace.
    pub is_mathfunc_call: bool,
    /// `Some(provenance)` when this record is the **subcommand word** of an
    /// `<ensemble> <sub> …` dispatch (issue #923 idx 106 / idx 85), carrying
    /// the option that declared the mapping; `None` for every other
    /// invocation, including the `-map` target and the `-subcommands` entry
    /// inside the ensemble declaration itself (those spans *do* carry the
    /// target's own name and are ordinary references).
    ///
    /// The flag rides on the invocation rather than being re-derived from
    /// [`crate::analyser::types::AnalysisResult::ensemble_subcommand_targets`]
    /// by name at the consumer, because only the recording site knows *this
    /// span* is a dispatch word: a `-map {Show ::app::widget::Show}` whose
    /// key happens to equal the target's tail makes a name-based test
    /// indistinguishable from an ordinary bare call to the target.
    ///
    /// Navigation consumers (references, go-to-definition, call hierarchy)
    /// ignore it — a dispatch site is a genuine reference under either
    /// provenance. Rename reads it: see
    /// [`EnsembleSubcommandProvenance`] for the oracle that says why a `-map`
    /// dispatch word must survive a rename of its target unchanged.
    pub ensemble_dispatch: Option<EnsembleSubcommandProvenance>,
}

/// The full result returned by `extract_signatures`.
///
/// Procs / classes / aliases
/// use `BTreeMap` keyed by qualified name so iteration is
/// deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureScanResult {
    /// Every proc definition discovered, keyed by qualified name.
    pub procs: BTreeMap<String, SignatureProc>,
    /// Every class definition discovered, keyed by qualified name.
    pub classes: BTreeMap<String, SignatureClass>,
    /// Every `package require` invocation.
    pub package_requires: Vec<SignaturePackageRequire>,
    /// Every `source` invocation.
    pub source_targets: Vec<SignatureSource>,
    /// Every local-interpreter `interp alias`, keyed by alias
    /// qualified name.
    pub command_aliases: BTreeMap<String, SignatureCommandAlias>,
    /// Every `rename OLD NEW`, keyed by the new qualified name.
    pub renames: BTreeMap<String, SignatureRename>,
    /// Every recorded `namespace import` (direct + conjectured).
    pub namespace_imports: Vec<SignatureNamespaceImport>,
    /// Every recorded `namespace forget` event — the removal half of the
    /// import edge's lifecycle log (issue #1103).
    pub namespace_forgets: Vec<SignatureNamespaceForget>,
    /// Every `auto_path` mutation (one record per path element).
    pub auto_path_entries: Vec<SignatureAutoPathEntry>,
    /// Every command invocation visited (lightweight: name + range).
    pub command_invocations: Vec<SignatureCommandInvocation>,
}
