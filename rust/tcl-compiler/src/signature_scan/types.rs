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
//! Spans are [`tcl_lexer::Span`]; the `PyO3` binding in
//! `rust/tcl-lsp-rust/src/signature_scan.rs` flattens them to
//! `(start, end)` `u32` tuples for the materialiser on the Python
//! side.

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
    pub params: Vec<ParamDef>,
    /// Source span of the name argument.
    pub name_range: Span,
    /// Source span of the body argument.
    pub body_range: Span,
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
    /// Source span of the name argument.
    pub range: Span,
    /// `true` when the call is inside a guarded branch.
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
}

/// An `auto_path` mutation recorded by the signature scanner.
///
/// Covers both `lappend auto_path …` and `set auto_path …` forms.
/// Each path element gets one record; resolution to absolute paths
/// happens later in the analyser pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureAutoPathEntry {
    /// Verbatim path-element text as reconstructed by the segmenter.
    pub raw: String,
    /// Source span of the path-element argument.
    pub range: Span,
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
    /// Every `auto_path` mutation (one record per path element).
    pub auto_path_entries: Vec<SignatureAutoPathEntry>,
    /// Every command invocation visited (lightweight: name + range).
    pub command_invocations: Vec<SignatureCommandInvocation>,
}
