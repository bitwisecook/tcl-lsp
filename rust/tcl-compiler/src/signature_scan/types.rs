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
