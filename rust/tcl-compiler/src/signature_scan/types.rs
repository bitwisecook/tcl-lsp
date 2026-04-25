//! Public record types emitted by the signature scanner.
//!
//! Mirrors the subset of `core.analysis.semantic_model` populated by
//! `signature_scan.py`. The remaining record types and the
//! [`SignatureScanResult`] aggregator are added by later C40 sub-strips.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

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
