//! Native port of `tests/lsp_e2e/test_hover_e2e.py` (command-hover subset).
//!
//! Open a document, wait for analysis, ask `textDocument/hover` at a position,
//! assert on the rendered markdown.

mod common;

use common::helpers::hover_text;
use common::{Lsp, unique_uri};

fn hover(lsp: &mut Lsp, uri: &str, line: u32, ch: u32) -> String {
    let h = lsp.hover(uri, line, ch);
    hover_text(&h)
}

#[test]
fn builtin_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\n");
    let text = hover(&mut lsp, &uri, 0, 1);
    assert!(text.contains("set"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("variable"), "hover: {text:?}");
}

#[test]
fn puts_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    assert!(hover(&mut lsp, &uri, 0, 2).contains("puts"));
}

#[test]
fn unknown_command_no_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "mycommand arg\n");
    assert!(hover_text(&lsp.hover(&uri, 0, 4)).is_empty());
}

#[test]
fn socket_hover_uses_registry_snippet() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket localhost 80\n");
    let text = hover(&mut lsp, &uri, 0, 1);
    assert!(text.contains("socket ?options? host port"), "hover: {text:?}");
    assert!(
        text.to_lowercase().contains("tcp client or server socket"),
        "hover: {text:?}"
    );
}
