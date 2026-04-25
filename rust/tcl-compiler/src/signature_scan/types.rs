//! Public record types emitted by the signature scanner.
//!
//! Mirrors the subset of `core.analysis.semantic_model` populated by
//! `signature_scan.py`. The remaining record types and the
//! [`SignatureScanResult`] aggregator are added by later C40 sub-strips.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

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
