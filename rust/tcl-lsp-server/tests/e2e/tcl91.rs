//! Native port of `tests/lsp_e2e/test_tcl91_e2e.py`.
//!
//! Tcl 9.1 dialect support, end-to-end over LSP. Oracle: C Tcl 9.1b0 source. The
//! dialect is pinned with a `# tcl-dialect: tcl9.1` directive (server-side source
//! detection), and behaviour is observed through completion + diagnostics.


use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

use std::collections::BTreeSet;

/// The set of completion labels from a completion result.
fn labels(result: &Value) -> BTreeSet<String> {
    completion_labels(result).into_iter().collect()
}

/// The set of `code` strings carried by `diags`.
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// Open a buffer pinned to `dialect` and complete a command-position word.
fn complete_cmd(lsp: &mut Lsp, dialect: &str, partial: &str) -> BTreeSet<String> {
    let uri = unique_uri("tcl");
    let src = format!("# tcl-dialect: {dialect}\n{partial}\n");
    lsp.open_ready(&uri, &src);
    labels(&lsp.completion(&uri, 1, partial.len() as u32))
}

// -- TestTcl91Completion -------------------------------------------------

#[test]
fn unicode_and_timer_offered_in_91() {
    // doc/unicode.n, doc/timer.n — both are new commands in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "unic").contains("unicode"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "time").contains("timer"));
}

#[test]
fn unicode_and_timer_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "unic").contains("unicode"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "time").contains("timer"));
}

#[test]
fn math_and_lfilter_offered_in_91() {
    // doc/divmod.n, doc/lfilter.n — new commands in 9.1 (C tclBasic.c).
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "divm").contains("divmod"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lfil").contains("lfilter"));
}

#[test]
fn math_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "divm").contains("divmod"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "lfil").contains("lfilter"));
}

#[test]
fn commands_90_still_offered_in_91() {
    // A `.1` release is additive: `lseq` (9.0) persists in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lseq").contains("lseq"));
}

// -- TestTcl91Operators --------------------------------------------------
// doc/expr.n — the `lt`/`le`/`gt`/`ge` string operators (TIP 461) are 9.0+.

#[test]
fn lt_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a lt $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn lt_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a lt $b}\n");
    assert!(codes(&diags).contains("W003"));
}
