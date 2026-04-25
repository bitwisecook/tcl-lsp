//! Analyser snapshot / restore — Rust port of
//! ``core/analysis/_analyser/_snapshot.py``.
//!
//! **C41a1 stub.** The full snapshot type + ``Analyser::snapshot``
//! / ``Analyser::restore`` ports of ``_core.py:105-184`` land in
//! **C41a3**. This file exists in C41a1 so the module structure
//! crystallises early — C41a3 fills in the dataclass-equivalent
//! struct, the deep-copy-of-result helper, and the
//! ``current_scope_path`` remap that tracks where in the
//! restored result tree the cursor should land.

use super::types::AnalysisResult;

/// Captured analyser state for chunked re-analysis.
///
/// **C41a1 stub.** The C41a3 strip will:
///
/// 1. Add the per-walk fields from ``AnalyserSnapshot`` in
///    ``_snapshot.py:9`` (``last_comment``, ``current_event``,
///    ``conditional_depth``, ``command_aliases``,
///    ``const_strings``, ``regex_vars``, ``var_command_sites``,
///    ``cmd_command_sites``, plus the ``current_scope_path``).
/// 2. Implement a ``copy_for_snapshot``-equivalent helper on
///    [`AnalysisResult`] that mirrors the Python deep-copy
///    fast-path (frozen records shared by reference, mutable
///    containers cloned).
/// 3. Wire ``Analyser::snapshot`` / ``Analyser::restore`` to
///    produce / consume this struct.
#[derive(Debug, Clone, Default)]
pub struct AnalyserSnapshot {
    /// Captured result tree (deep-copied).
    pub result: AnalysisResult,
}
