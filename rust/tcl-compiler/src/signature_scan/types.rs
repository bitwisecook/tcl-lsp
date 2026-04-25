//! Public record types emitted by the signature scanner.
//!
//! Mirrors the subset of `core.analysis.semantic_model` populated by
//! `signature_scan.py`. The remaining record types and the
//! [`SignatureScanResult`] aggregator are added by later C40 sub-strips.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

use std::collections::BTreeMap;

use tcl_lexer::Span;

/// A single Tcl proc parameter declaration.
///
/// Mirrors `core.analysis.semantic_model.ParamDef`. The `default_value`
/// is the literal text following the parameter name inside a braced
/// `{name default}` form — whitespace before it is stripped, whitespace
/// inside the default text is preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
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
/// Mirrors `core.analysis.semantic_model.ProcDef` for the subset
/// `signature_scan.py` populates: name, qualified name, parameter
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
/// `resolved_qualified_name` is intentionally omitted (the full scope
/// walk required to resolve it is what `signature_scan` skips for
/// background files).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureCommandInvocation {
    /// Command head as written at the call site (no namespace
    /// resolution performed).
    pub name: String,
    /// Source span of the command-head token.
    pub range: Span,
}

/// The full result returned by `extract_signatures`.
///
/// Mirrors the subset of `core.analysis.semantic_model.AnalysisResult`
/// that `signature_scan.py` populates. Procs / classes / aliases
/// use `BTreeMap` keyed by qualified name so iteration is
/// deterministic — important for differential parity testing against
/// the Python implementation.
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
    /// Every recorded `namespace import` (direct + conjectured).
    pub namespace_imports: Vec<SignatureNamespaceImport>,
    /// Every `auto_path` mutation (one record per path element).
    pub auto_path_entries: Vec<SignatureAutoPathEntry>,
    /// Every command invocation visited (lightweight: name + range).
    pub command_invocations: Vec<SignatureCommandInvocation>,
}
