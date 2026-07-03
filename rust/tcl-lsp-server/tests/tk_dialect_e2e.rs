//! Native port of `tests/lsp_e2e/test_tk_dialect_e2e.py`.
//!
//! Tk command availability, end-to-end over LSP. Tk widget/window commands
//! (`button`, `pack`, `wm`, the `ttk::` forms, …) are gated two ways, verified
//! here through completion:
//!
//! 1. **Loaded** — only offered once the file makes Tk available (a
//!    `# tcl-dialect: tk` document or a `package require Tk`). A plain `.tcl`
//!    script must not be offered Tk commands.
//! 2. **Dialect** — never offered in the restricted embedded dialects
//!    (F5 iRules / iApps), even with a stray `package require Tk`.
//!
//! Tk load-gating is a native-server feature, so the pytest suite skips unless
//! `TCL_LSP_SERVER_KIND=rust`; this native port always exercises the Rust
//! server. The Tk-availability gating is driven by `# tcl-dialect:` directives
//! in ordinary `tcl`-language documents (not a `tk` language id).

mod common;

use common::helpers::*;
use common::{Lsp, unique_uri};

/// Open `src`, then return the completion labels at `(line, char)`.
fn complete(lsp: &mut Lsp, uri: &str, src: &str, line: u32, ch: u32) -> std::collections::BTreeSet<String> {
    lsp.open_ready(uri, src);
    completion_labels(&lsp.completion(uri, line, ch)).into_iter().collect()
}

// -- TestTkLoadedGating --------------------------------------------------

#[test]
fn button_absent_in_plain_tcl() {
    // No `package require Tk` — Tk commands must not surface.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(&mut lsp, &uri, "# tcl-dialect: tcl8.6\nbutt\n", 1, 4);
    assert!(!labels.contains("button"), "{labels:?}");
}

#[test]
fn button_offered_after_package_require_tk() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\npackage require Tk\nbutt\n",
        2,
        4,
    );
    assert!(labels.contains("button"), "{labels:?}");
}

// -- TestTkDialectGating -------------------------------------------------

#[test]
fn button_never_offered_in_irules() {
    // Even a stray `package require Tk` cannot make Tk valid in iRules.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(
        &mut lsp,
        &uri,
        "# tcl-dialect: f5-irules\npackage require Tk\nbutt\n",
        2,
        4,
    );
    assert!(!labels.contains("button"), "{labels:?}");
}

#[test]
fn ttk_widget_absent_in_iapps() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(
        &mut lsp,
        &uri,
        "# tcl-dialect: f5-iapps\npackage require Tk\nttk::butt\n",
        2,
        9,
    );
    assert!(!labels.contains("ttk::button"), "{labels:?}");
}
