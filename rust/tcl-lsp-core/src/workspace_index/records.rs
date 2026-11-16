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

//! The record types one document contributes: procedures, methods, symbols, variables, sources, packages and command links.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::Span;

/// One proc definition recorded in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProc {
    /// Exact original command publication receipt; report keys grant no identity.
    pub source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Document the proc is defined in (the `analyses` map key).
    pub uri: String,
    /// Simple (tail) name, e.g. `greet`.
    pub name: String,
    /// Fully-qualified name, e.g. `::myns::greet`.
    pub qualified_name: String,
    /// Declared parameter count (for completion detail).
    pub param_count: usize,
    /// The `(min, max)` argument arity this proc accepts, straight from
    /// [`tcl_compiler::analyser::ProcDef::arity`] — so a trailing `args` is
    /// unbounded, a defaulted parameter lowers the minimum, and a **computed**
    /// parameter list (`proc p $params {…}`) abstains to the fully-open
    /// `0..UNLIMITED` rather than reading as "takes no arguments".
    ///
    /// [`Self::param_count`] cannot answer an arity question: it is the raw
    /// formal count, which says nothing about defaults or `args`. Cross-file
    /// arity checking needs the real envelope, and it needs it
    /// from the same index that settles the call, so a caller is checked
    /// against the command navigation says it actually reaches.
    pub arity: tcl_registry::Arity,
    /// Byte span of the proc's name token in `uri`'s source.
    /// The server resolves this to an LSP range against the
    /// target document at query time.
    pub name_span: Span,
    /// Whether this proc's own declaration sits inside another proc's or
    /// class's body — i.e. it exists only conditionally, when and if that
    /// enclosing definition actually runs (the "rename a builtin away,
    /// install a same-named shadow proc, restore it" idiom). A nested
    /// definition must not permanently outrank a real registry builtin for
    /// workspace-wide command existence, mirroring the same judgement
    /// `tcl_compiler::analyser::AnalysisResult::offset_is_inside_any_definition_body`
    /// already applies same-file in `resolve_called_proc`.
    pub nested: bool,
}

/// One method a class record directly defines, as indexed for cross-file
/// dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMethod {
    /// Simple method name.
    pub name: String,
    /// Declaration kind: `"method"`, `"classmethod"`, or `"forward"`.
    pub kind: String,
    /// Effective `TclOO` export state at the end of the defining record's
    /// body: the family's name default (`[a-z]*` for `TclOO`) plus any
    /// explicit `export` / `unexport`, last writer wins (tclsh
    /// 9.0.4-pinned).  Externally callable iff `true`.
    pub exported: bool,
    /// `true` for a `TclOO` `private` definition — invisible to external
    /// dispatch *and* to subclasses; callable only via `my` within the
    /// declaring class's own methods.
    pub private: bool,
    /// `true` for a stock-`TclOO` `self method` (as opposed to `ooutil`'s
    /// `classmethod` keyword).  Both land in the class-receiver bucket
    /// (`kind == "classmethod"`), but a `self method` is visible **only**
    /// on the exact class object that declared it: `Gadget make` on a
    /// subclass of a class declaring `self method make` errors `unknown
    /// method "make"` under tclsh 8.6 and 9.0.4, whereas `classmethod`
    /// propagates to the subclass's own bound command through its
    /// `Delegate`-mixin machinery.  The single-document scan reads
    /// [`tcl_compiler::analyser::MethodDef::is_self_method`] for this;
    /// carrying it here is what lets a *cross-file* consumer scan tell the
    /// two apart.
    pub is_self_method: bool,
    /// Byte span of the method's name token in the declaring record's
    /// document.  Carried so `workspace/symbol` can locate a method in a file
    /// the editor has never opened; cross-file *dispatch* uses
    /// the name and the visibility flags and does not need it.
    pub name_span: Span,
}

/// The access context of a method call site — `TclOO` dispatches an
/// external `$obj m` through exported methods only, while an internal
/// `my m` reaches unexported ones too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodAccess {
    /// `$obj m` / `objcmd m` — exported methods only.
    External,
    /// `my m` (or a declaration-side query from inside the class body) —
    /// exported and unexported methods; `private` methods only from the
    /// declaring class itself.
    Internal,
}

/// One **registry symbol-definer** definition recorded in the index — a
/// `tcltest::test` case, a `testConstraint`, a `customMatch` mode, or an
/// iRules `when EVENT` handler.
///
/// Cross-document identity, as the index requires of every table: each of
/// these is a *named* definition carrying its enclosing namespace, spellable
/// from another file exactly as a proc's qualified name is.  They are not
/// callable commands, so they stay out of `procs` and out of the command
/// existence oracle; the workspace-symbol picker is their consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDefinedSymbol {
    /// Document the definition is in.
    pub uri: String,
    /// Definition name as resolved (a test's case label, an event's name).
    pub name: String,
    /// Fully-qualified name, with the enclosing namespace applied.
    pub qualified_name: String,
    /// The registry's outline category for it.
    pub kind: tcl_registry::DefinedSymbolKind,
    /// Byte span of the name token in `uri`'s source.
    pub name_span: Span,
}

/// Original namespace-variable declaration with a checked C compatibility key.
/// Local frames and unrepresentable/Jim keys retain their separate typed
/// original inventory; this row supplies no runtime cell or publication proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariable {
    /// Document the declaration is in.
    pub uri: String,
    /// Simple (tail) name, e.g. `version`.
    pub name: String,
    /// `::`-rooted qualified name, e.g. `::tomato::version`.
    pub qualified_name: String,
    /// Byte span of the declaring name token in `uri`'s source.
    pub name_span: Span,
}

/// Original namespace-variable source occurrence with a checked outward C key.
/// Conditional local aliases remain possible source relationships; this row
/// supplies neither an entered link nor completed variable access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableRef {
    /// Document the occurrence is in.
    pub uri: String,
    /// Checked outward C source coordinate, never reconstructed from a label.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
}

/// One occurrence of a word naming a **namespace**, recorded workspace-wide —
/// the cross-document half of the namespace tier, lifted verbatim from
/// [`tcl_compiler::analyser::types::NamespaceRef`].
///
/// One table, not two, because a namespace's declaring site *is* one of its
/// spellings: the `::tomato` of `namespace eval ::tomato { … }` is both the
/// definition go-to-definition answers with and a word find-references
/// reports.  [`Self::declares`] is the discriminator, and it is registry data
/// ([`tcl_registry::Traits::DECLARES_NAMESPACE`]), not a spelling check.
///
/// Unlike [`WorkspaceVariable`] this table needs no qualified-only bound: the
/// analyser roots a relative namespace word against its own lexical namespace
/// before recording it, so every row already names one namespace absolutely,
/// spellable from any document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceRef {
    /// Original source operand and independently retained geometry.
    pub source: tcl_compiler::analyser::types::NamespaceRef,
    /// Document the occurrence is in.
    pub uri: String,
    /// `::`-rooted namespace the occurrence names.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
    /// `true` for a declaring `namespace eval` name word.
    pub declares: bool,
}

/// One `source FILE` reference recorded in the index.
///
/// Tracks where a document loads another file so a file rename can
/// rewrite the dependent's `source` literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSource {
    /// Actual current-interpreter source association and its retained schema.
    /// This remains separate from child-interpreter source visibility advice.
    pub original_project_source_load: Option<
        std::sync::Arc<
            tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad,
        >,
    >,
    /// Compiler-issued current cut, conditional association or unknown debt
    /// from this row's actual parent analysis. This retains the full finite
    /// plan/body issuer, rather than promoting a point alone to a frontier.
    /// The host independently rejoins the parent's physical publication.
    pub original_source_frontier: Option<
        std::sync::Arc<
            tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier,
        >,
    >,
    /// Sealed conditional expression selected from genuine original words.
    /// It retains the full analysis/context and supplies no actual file load.
    pub original_path_expression:
        Option<tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
    /// Retained possible child source edge and its full source/input lineage.
    /// No actual file evaluation or completed execution is established.
    pub original_interpreter_source_load:
        Option<std::sync::Arc<tcl_compiler::analyser::OriginalInterpreterSourceLoad>>,
    /// Document containing the `source` statement.
    pub uri: String,
    /// Verbatim path text as written (with `${var}` / `[cmd]` markers
    /// preserved for substituted words).
    pub raw_path: String,
    /// Byte span of the path argument in `uri`'s source.
    pub range: Span,
    /// `true` when the path is a plain literal (no `$` / `[`).
    pub is_literal: bool,
    /// Command-resolution namespace at the `source` call site (a constructed
    /// `::`-rooted key).  `source` evaluates the file in the caller's current
    /// namespace, so the sourced document's definitions re-home under
    /// this namespace — see [`WorkspaceIndex::source_seed_map`].
    pub site_namespace: String,
    /// Span of the innermost proc/class body containing the `source`
    /// statement; `None` at load level.  See
    /// [`WorkspaceGlobImport::enclosing_body`] — the same execution-order
    /// fact, needed here so a cross-document interpreter-state question
    /// ("had this statement already run when the child was loaded?") applies
    /// the identical [`tcl_compiler::analyser::indirection::in_effect_within`]
    /// rule the single-document tier does.
    pub enclosing_body: Option<Span>,
}

/// One unconditional `package prefer latest` recorded in the index.
///
/// `package prefer` latches **interpreter-global** state, so a raise in a file
/// that runs before this one really does change this one's version selection.
/// Which file runs first is not knowable in general — but along the `source`
/// graph it is: a file that `source`s another runs the sourcing statement
/// before the sourced file loads at all.  See
/// [`WorkspaceIndex::source_ancestor_prefers_latest`].
///
/// Conditional raises (inside `if` / `catch` / `try`) are not recorded at all:
/// they may not run, and the abstention is toward the interpreter default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackagePrefer {
    /// Document containing the `package prefer latest` statement.
    pub uri: String,
    /// Byte offset of the `package` command word in `uri`'s source.
    pub at: u32,
    /// Span of the innermost proc/class body containing the raise; `None` at
    /// load level.
    pub enclosing_body: Option<Span>,
}

/// One `package require NAME` declaration recorded in the index.
///
/// Lets a module inherit the requires of the entry file(s) that `source` it,
/// so the workspace W120 refinement does not flag a command whose package is
/// required upstream (see [`crate::source_graph`]).
///
/// It is also an **ordered statement**: by the time it returns, the package's
/// files have run.  [`WorkspaceIndex::package_run_edges`] turns that into the
/// package half of the load order, which is why the position
/// fields are here and not only on the `source` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageRequire {
    /// Exact package-key projection; full original source ownership remains
    /// in the analyser and does not enter the body-free settlement summary.
    pub original_name: Option<tcl_registry::native_package::NativePackageNameKey>,
    /// Each alternative version word retained at this requirement.
    pub requirements: Vec<String>,
    /// Whether this requirement selected exact version matching.
    pub exact: bool,
    /// Local source preference at this requirement. The host independently
    /// supplies any interpreter or ancestor preference on entry.
    pub prefer: crate::package_resolver::PackagePrefer,
    /// Document containing the `package require` statement.
    pub uri: String,
    /// Required package name (the `NAME` argument).
    pub name: String,
    /// Byte offset of the `package` command word in `uri`'s source.
    pub at: u32,
    /// Span of the innermost proc/class body containing the require; `None`
    /// at load level.  Read by
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] exactly as
    /// [`WorkspaceSource::enclosing_body`] is.
    pub enclosing_body: Option<Span>,
    /// `true` when the require sits inside a guarded branch (`if` / `catch` /
    /// `try`) and so may never run — the standard optional-dependency idiom
    /// `if {[catch {package require Tk}]} { … }`.  It still counts as a
    /// *declared dependency* for W120, which is why the record is kept rather
    /// than dropped; it establishes no order.
    pub conditional: bool,
}

impl WorkspacePackageRequire {
    /// Project advice without rebuilding a package key from its report label.
    /// Interpreter preference is a caller fact; local `latest` is a latch.
    #[must_use]
    pub fn original_advice(
        &self,
        default: crate::package_resolver::PackagePrefer,
    ) -> crate::package_resolver::PackageRequirementAdvice {
        use crate::package_resolver::{
            PackagePrefer, PackageRequirementAdvice, PackageRequirementAdviceKey,
        };
        let key = self
            .original_name
            .as_ref()
            .map_or(PackageRequirementAdviceKey::Unknown, |name| {
                PackageRequirementAdviceKey::Original(name.clone())
            });
        let prefer = if default == PackagePrefer::Latest || self.prefer == PackagePrefer::Latest {
            PackagePrefer::Latest
        } else {
            PackagePrefer::Stable
        };
        PackageRequirementAdvice::new(key, self.requirements.clone(), self.exact, prefer)
    }
}

/// One `package provide NAME` declaration recorded in the index.
///
/// The other end of a [`WorkspacePackageRequire`]: the document that, when it
/// runs, makes the package available.  A `package require` whose name this
/// resolves to a **single** indexed document is the load order's evidence that
/// that document had run by the require.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageProvide {
    /// Exact original package-key projection, independent of its reporting label.
    pub original_name: Option<tcl_registry::native_package::NativePackageNameKey>,
    /// Document containing the `package provide` statement.
    pub uri: String,
    /// Provided package name (the `NAME` argument).
    pub name: String,
    /// Byte offset of the `package` command word in `uri`'s source.
    pub at: u32,
    /// `true` when the provide sits inside a guarded branch and so may never
    /// run — the shim idiom
    /// [`tcl_compiler::analyser::types::PackageProvide::conditional`]
    /// documents.  Such a document is not this package's provider.
    pub conditional: bool,
}

/// One `package ifneeded NAME VERSION SCRIPT` registration recorded in the
/// index.
///
/// A registration is a statement that the package's loading is *this script's*
/// business — and the script is arbitrary, runs later, and runs in the global
/// namespace.  The load order reads the mere existence of one as "which
/// statements a `package require NAME` runs is not static here" and builds no
/// package edge for that name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageIfneeded {
    /// Original registered package key; registration does not prove loading.
    pub original_name: Option<tcl_registry::native_package::NativePackageNameKey>,
    /// Document containing the `package ifneeded` statement.
    pub uri: String,
    /// Package the load script is registered for.
    pub name: String,
}

/// One command name-link recorded in the index.
///
/// A `namespace import`, `interp alias`, or `rename` introduces a *new*
/// callable name that resolves to another command: an imported `helper`
/// runs the exporting namespace's `helper`, an alias runs its target, a
/// `rename OLD NEW` makes `NEW` run what `OLD` denoted.  A call reaching the
/// new name is a reference to the ultimate target; the token that *names*
/// the target in the declaration (the import pattern, the alias `TARGET`
/// word, the `rename` `OLD` word) is itself a reference and a rename must
/// rewrite it.  Ground truth: the VM re-resolves an alias from `::` at call
/// time ([`tcl_vm::exec`]); a rename is a pure name move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCommandLink {
    /// Document the link declaration is in.
    pub uri: String,
    /// Fully-qualified name the link introduces (`::`-rooted): the imported
    /// `<ns>::<tail>`, the alias name, or the `rename` `NEW`.
    pub linked_qname: String,
    /// Exact authored alias publication slot, when supplied by its original
    /// declaration. Its reported full name is not new lookup input.
    pub linked_source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Fully-qualified name (`::`-rooted) the link resolves *to*: the import
    /// pattern's source, the alias `TARGET`, or the `rename` `OLD` — the
    /// command whose references a call through `linked_qname` joins.
    pub target_qname: String,
    /// Exact authored global alias target slot. Its report need not be a
    /// globally callable spelling; positioned source references resolve it.
    /// Imports and renames retain their existing declaration carriers.
    pub target_source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Byte span of the token naming the target in the declaration (import
    /// pattern, alias `TARGET`, `rename` `OLD`).  A reference to the target;
    /// rename rewrites it.  `None` when the source scan did not record a span
    /// for this link kind.
    pub target_span: Option<Span>,
    /// Whether this link's own declaration (the `namespace import` /
    /// `interp alias` / `rename` command) sits inside another proc's or
    /// class's body — i.e. it takes effect only conditionally, when and if
    /// that enclosing definition actually runs (the same "rename a builtin
    /// away, install a same-named shadow, restore it" idiom
    /// [`WorkspaceProc::nested`] guards against, extended to the alias /
    /// rename / import forms of introducing a name). Mirrors
    /// [`WorkspaceProc::nested`] exactly, including its consumer:
    /// [`WorkspaceIndex::workspace_command_exists_for_call`] excludes only a
    /// *nested* link when a same-named builtin is in play, so an
    /// unconditional (top-level) alias/rename/import still counts as
    /// existing.
    pub nested: bool,
    /// For a link introduced by an **exact** `namespace import ::src::p`: the
    /// export snapshot the import must pass before it installs anything.
    /// `None` for an `interp alias` / `rename` link, and for a *conjectured*
    /// import — neither is gated by `namespace export`.
    ///
    /// An exact import is no less gated than a glob one: real Tcl installs
    /// **no** alias, silently, when `p` is not exported at the moment the
    /// import runs (oracle tclsh 8.6.14 / 9.0.4 — `namespace eval ::src {proc
    /// p {} {}}; namespace eval ::dst {namespace import ::src::p}` leaves
    /// `info commands ::dst::*` empty and raises no error; with `namespace
    /// export p` first it binds). Recording the site here, rather than
    /// resolving the gate when the link is created, is what keeps the answer
    /// correct across edits: the export usually lives in a *different*
    /// document, re-indexed independently of this one, so a decision frozen
    /// at creation time would go stale the moment that file changes.
    /// [`WorkspaceIndex::live_command_links`] applies it against the
    /// current index, cached per [`WorkspaceIndex::generation`].
    pub import_gate: Option<WorkspaceImportGate>,
}

/// The export-snapshot condition an exact `namespace import` link must satisfy
/// to be installed at all — see [`WorkspaceCommandLink::import_gate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceImportGate {
    /// Genuine whole original pattern and caller scope; optional display fields
    /// do not recover this receipt when unavailable.
    pub original_pattern:
        Option<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    /// Exact original selected namespace geometry; never parsed from its display.
    pub native_source: Option<tcl_syntax::naming::NativeNamespacePatternSource>,
    /// The pattern's source namespace, with leading `::` (`::src` for
    /// `::src::p`).
    pub source_ns: String,
    /// The bare name the import binds (`p`).
    pub name: String,
    /// Byte offset of the import's pattern word within the link's own `uri`.
    pub at: u32,
    /// Span of the innermost proc/class body containing the import; `None` at
    /// load level. See [`WorkspaceGlobImport::enclosing_body`].
    pub enclosing_body: Option<Span>,
    /// `true` when the import carried its declared leading option word —
    /// `namespace import -force`. See [`WorkspaceGlobImport::forced`].
    pub forced: bool,
}

/// One wildcard `namespace import NS::*` recorded in the index.
///
/// Unlike [`WorkspaceCommandLink`], a glob pattern names no single command —
/// [`WorkspaceIndex::index_command_links`] deliberately skips it — so it
/// cannot resolve a bare call to a fixed `target_qname` on its own. Instead
/// each recorded entry is consulted per-call, against whichever bare `word`
/// the invocation actually writes:
/// [`WorkspaceIndex::resolve_wildcard_import`] glob-matches `word` against
/// [`Self::tail_pattern`] and requires [`Self::source_ns`] to have exported
/// a covering pattern (`WildcardImportIndex::exports_name`) before
/// resolving — see [`WorkspaceIndex::index_command_links`]'s doc comment for
/// why an exact pattern still takes the `WorkspaceCommandLink` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceGlobImport {
    /// Genuine whole original pattern and caller scope; optional display fields
    /// do not recover this receipt when unavailable.
    pub original_pattern:
        Option<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    /// Exact original selected namespace geometry; never parsed from its display.
    pub native_source: Option<tcl_syntax::naming::NativeNamespacePatternSource>,
    /// Document the `namespace import` is in.
    pub uri: String,
    /// Importing namespace, with leading `::` — the namespace a bare call
    /// must resolve *from* for this import to be in scope (the same
    /// candidate-namespace order ordinary command resolution already uses).
    pub ns: String,
    /// The pattern's source namespace, with leading `::` (`::Foo` for a
    /// `::Foo::*` / `::Foo::b*` pattern).
    pub source_ns: String,
    /// The pattern's final `::`-segment, exactly as written (`*`, `b*`,
    /// or a literal tail) — matched against a call's bare name with Tcl
    /// glob semantics ([`tcl_syntax::glob::string_match`]).
    pub tail_pattern: String,
    /// Byte offset of the import's pattern word within [`Self::uri`].
    ///
    /// The import's position on its own document's timeline: an import binds
    /// the names its source namespace exported *when the import ran*, so an
    /// export declared in the same file is judged against this offset.
    /// Meaningless against another file's offsets — which document
    /// loads first is not a static fact — so
    /// [`WildcardImportIndex::exports_name_at`] only compares within one URI.
    pub at: u32,
    /// Span of the innermost proc/class **body** containing this import,
    /// within [`Self::uri`]; `None` when the import is at load level.
    ///
    /// Ordering an event against this import is not a plain offset compare:
    /// an import written *inside a body* observes every top-level statement
    /// of its own file, wherever written, because the whole file loads before
    /// any body runs. This is the one fact
    /// [`tcl_compiler::analyser::indirection::in_effect`] reads out of an
    /// `AnalysisResult`, stored per row so the cross-document tier can apply
    /// that identical rule via
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] instead of a
    /// weaker `at <= import_at`, which would reject such an export and lose a
    /// real imported alias.
    pub enclosing_body: Option<Span>,
    /// `true` when the import carried its declared leading option word —
    /// `namespace import -force`.
    ///
    /// Decides what happens when the importing namespace already holds a
    /// command of the imported name (oracle tclsh 8.6.14 / 9.0.4): without
    /// `-force` the import raises `can't import command "p":
    /// already exists` and installs **nothing**, so a bare call still reaches
    /// the local definition; with `-force` it silently replaces it and the
    /// call reaches the source (`namespace origin` → `::src::p`). See
    /// [`tcl_compiler::signature_scan::types::SignatureNamespaceImport::forced`]
    /// for why this is registry data rather than a `-force` name match.
    pub forced: bool,
}

/// One `namespace forget` **event** recorded in the index — the removal half
/// of the import edge's lifecycle log.
///
/// Aggregated workspace-wide for the same reason
/// [`WorkspaceNamespaceExport`] is: the forget and the import it undoes need
/// not live in the same file. Ordering only means something *within* a
/// document, which is why [`Self::uri`] and [`Self::at`] are always read
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceForget {
    /// Document the `namespace forget` is in.
    pub uri: String,
    /// The namespace losing the aliases, with leading `::`.
    pub ns: String,
    /// Source namespace named by a *qualified* pattern (`::src` for
    /// `namespace forget ::src::p`), `None` for a simple pattern — which
    /// matches this namespace's own imported command names whatever their
    /// origin. See
    /// [`tcl_compiler::signature_scan::types::SignatureNamespaceForget`].
    pub source_ns: Option<String>,
    /// The pattern's final `::`-segment, exactly as written, matched against
    /// a command's bare name with Tcl glob semantics.
    pub pattern: String,
    /// Byte offset of the event within [`Self::uri`].
    pub at: u32,
}

/// One straight-line command **deletion** recorded in the index — `rename
/// OLD {}` / `interp alias {} NAME {}`.
///
/// A deletion is a lifecycle event on every import edge pointing at the
/// deleted command: the alias holds the command object, so deleting the
/// source kills the alias too (oracle on
/// [`tcl_compiler::analyser::AnalysisResult::deleted_commands`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCommandDeletion {
    /// Document the deletion is in.
    pub uri: String,
    /// The `::`-normalised qualified name that was deleted.
    pub qualified_name: String,
    /// Byte offset of the deleting statement within [`Self::uri`].
    pub at: u32,
}

/// One straight-line command-name retirement recorded in the index.
///
/// `rename OLD NEW` preserves the command object (and therefore any import
/// aliases which hold it), but it stops `OLD` being a callable name.  A
/// destruction has the same consequence for the name as well as ending the
/// command object.  Keeping that narrower name-liveness fact apart from
/// [`WorkspaceCommandDeletion`] lets the import lifecycle retain Tcl's
/// object-identity rule while all direct command lookups consistently hide a
/// definition after its name has been moved or destroyed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceCommandRetirement {
    /// Document containing the lifecycle event.
    pub uri: String,
    /// The `::`-normalised qualified name which stopped being callable.
    pub qualified_name: String,
    /// Byte offset of the lifecycle statement within [`Self::uri`].
    pub at: u32,
}

/// One `namespace export` **event** recorded in the index.
///
/// Aggregated workspace-wide (unlike
/// [`tcl_compiler::analyser::AnalysisResult::namespace_exports`], which is
/// per-document) so a wildcard import in one file can be checked against an
/// export declared in *another* file.
///
/// An *event*, not a member of a set: `-clear` tombstones and ordering are
/// carried through so the cross-document tier applies the same per-import-site
/// snapshot the same-document one does (see
/// [`crate::namespace_import`]). Ordering only means something *within* a
/// document, which is why [`Self::uri`] and [`Self::at`] are always read
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceExport {
    /// Original counted event, retained independently of the reporting label.
    pub original: Option<tcl_compiler::signature_scan::original_name::SourceNamespaceExport>,
    /// Unrepresented source operand; this is a May barrier, never a name value.
    pub unknown: bool,
    /// Document the `namespace export` is in.
    pub uri: String,
    /// Exporting namespace, with leading `::`.
    pub ns: String,
    /// Exported pattern text, exactly as written (relative to `ns`). Empty
    /// for a [`Self::clears`] tombstone.
    pub pattern: String,
    /// Byte offset of the event within [`Self::uri`].
    pub at: u32,
    /// Genuine original declaration-body geometry for source ordering.
    pub enclosing_body: Option<Span>,
    /// `true` for a `namespace export -clear` tombstone.
    pub clears: bool,
}

/// Exact source/configuration/Registry currency and selected declaration advice.
/// This readonly invalidation context supplies no lookup, publication, Normal
/// completion, execution frame or native command-table capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDiagnosticSourceContext {
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) registry: tcl_registry::RegistrySemanticKey,
    pub(super) original_names: bool,
}

impl WorkspaceDiagnosticSourceContext {
    /// Retain complete source/configuration, Registry identity and advice mode.
    /// This readonly currency does not issue a lookup or native capability.
    #[must_use]
    pub fn for_analysis(analysis: &AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let registry = analysis.resolved_registry()?.snapshot().semantic_key();
        let image = analysis
            .retained_command_realm()?
            .original_source_image()?
            .clone();
        analysis
            .matches_original_source_image(&image, config)
            .then_some(Self {
                image,
                config,
                registry,
                original_names: !analysis.allows_lexical_declaration_advice(),
            })
    }
    /// Complete actual source bytes and input channel retained at ingress.
    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }
    /// Full scanner configuration, without profile-name reconstruction.
    #[must_use]
    pub const fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
    /// Independently selected original versus logical compatibility advice.
    #[must_use]
    pub const fn uses_original_names(&self) -> bool {
        self.original_names
    }
}
