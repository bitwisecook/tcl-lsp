//! Pure analyser helpers — Rust port of
//! ``core/analysis/_analyser/_utils.py``.
//!
//! **C41a1 stub.** Filled in by **C41a2**:
//!
//! - ``parse_file_suppression(source) -> HashSet<String>`` —
//!   parses top-of-file ``# tcl-lsp: disable=CODE,CODE`` directives
//!   from the leading comment block.
//! - ``format_literal_for_message(text) -> String`` — quotes a
//!   literal for inclusion in a diagnostic message, escaping
//!   Tcl-significant characters.
//! - ``argv_with_word_spans(argv, all_tokens) -> Vec<Token>`` —
//!   widens each argv token to the source span of the whole word
//!   it sits in (mirrors the segmenter's argv-widening fix from
//!   **Seg1** but applied to alread-segmented commands).
//! - ``possible_paste_fingerprint(stmt) -> Option<(String, String)>`` —
//!   extracts ``(cmd_head, dependency)`` pairs the W214
//!   paste-error heuristic checks against.
//! - ``irules_top_level_only() -> &'static [&'static str]`` —
//!   the static list of iRules commands that may only appear at
//!   the top level of an event handler.
//!
//! ``parse_param_list`` is already ported under
//! ``signature_scan::params``; **C41a2** re-exports it through
//! this module so the analyser can keep its imports flat.
