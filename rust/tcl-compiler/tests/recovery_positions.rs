//! Prove that ghost-token recovery does NOT shift LSP positions.
//!
//! Ghost closing delimiters are zero-width — they are seen by the lexer
//! at a byte offset but do not advance the position counter.  After the
//! recovered token stream is parsed, every proc definition, variable
//! reference, and diagnostic must report spans that are absolute byte
//! offsets into the ORIGINAL source text.
//!
//! This suite asserts two contracts at the analyser level:
//!   - recovery is non-fatal (procs after the break are still found)
//!   - in-document positions (all spans match original source text)

use std::collections::HashMap;
use tcl_compiler::analyser::{Analyser, ProcDef};

fn find_proc<'a>(procs: &'a HashMap<String, ProcDef>, name: &str) -> &'a ProcDef {
    procs
        .values()
        .find(|p| p.name == name)
        .expect("proc not found")
}

#[test]
fn proc_after_unterminated_bracket_has_correct_positions() {
    // recovery is non-fatal — a proc after the break must still
    // be a document symbol with correct source positions.
    //
    // "set x [bad"  → unterminated [, ghost ] inserted at offset 11
    // "\nproc greet" → separate command after newline
    let src = "set x [bad\nproc greet {} {\n    puts hi\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");

    let proc = find_proc(&r.all_procs, "greet");
    assert!(r.all_procs.contains_key("::greet"));

    // "set x [bad\n" = 11 bytes. "proc " at 11, "greet" at 16.
    let byte_before = "set x [bad\n".len();
    let name_start = byte_before + "proc ".len();
    let name_end = name_start + "greet".len();

    assert_eq!(proc.name_span.start() as usize, name_start);
    assert_eq!(proc.name_span.end() as usize, name_end);
    assert_eq!(
        &src[proc.name_span.start() as usize..proc.name_span.end() as usize],
        "greet"
    );

    // Body must contain source text, not shifted garbage.
    let body = &src[proc.body_span.start() as usize..proc.body_span.end() as usize];
    assert!(body.contains("puts hi"), "body: {body:?}");

    // Recovery diagnostic must exist.
    let has_e = r
        .diagnostics
        .iter()
        .any(|d| d.code.as_str().starts_with("E20"));
    assert!(has_e, "expected recovery diagnostic");
}

#[test]
fn proc_after_unterminated_brace_has_correct_positions() {
    // Unterminated brace word: ghost-} must not shift proc positions.
    let src = "set x {unterminated\nproc greet {} {\n    puts hi\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");

    assert!(r.all_procs.contains_key("::greet"));
    let proc = find_proc(&r.all_procs, "greet");

    let byte_before = "set x {unterminated\n".len();
    let name_start = byte_before + "proc ".len();
    assert_eq!(proc.name_span.start() as usize, name_start);
    assert_eq!(
        &src[proc.name_span.start() as usize..proc.name_span.end() as usize],
        "greet"
    );
}

#[test]
fn invocation_spans_after_recovery_use_original_offsets() {
    // Command invocation spans (used for references, highlights,
    // semantic tokens) must use source offsets, not ghost-shifted ones.
    let src = "set x [bad\nputs hi\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");

    let invoc = r
        .command_invocations
        .iter()
        .find(|inv| inv.name == "puts")
        .expect("puts must be found after recovery");

    // "set x [bad\n" = 11 bytes. "puts" starts at byte 11.
    let puts_start: usize = "set x [bad\n".len();
    assert_eq!(invoc.range.start() as usize, puts_start);
    assert_eq!(
        &src[invoc.range.start() as usize..invoc.range.end() as usize],
        "puts"
    );
}

#[test]
fn ghost_token_does_not_shift_body_proc_positions() {
    // Unterminated [ INSIDE a proc body — the body's own position
    // must be correct.
    let src = "proc p {} {\n    set x [bad\n    puts hi\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");

    assert!(r.all_procs.contains_key("::p"));
    let proc = find_proc(&r.all_procs, "p");

    assert_eq!(proc.name_span.start(), 5); // after "proc "
    assert_eq!(&src[5..6], "p");

    let body = &src[proc.body_span.start() as usize..proc.body_span.end() as usize];
    assert!(body.contains("set x [bad"));
    assert!(body.contains("puts hi"));
}

#[test]
fn diagnostic_spans_always_point_into_original_source() {
    // Every diagnostic span must be a valid, in-range slice of the
    // original source — never shifted past EOF by ghost tokens.
    let src = "set x [bad\nputs hi\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");

    for d in &r.diagnostics {
        let s = d.span.start() as usize;
        let e = d.span.end() as usize;
        assert!(
            s <= e && e <= src.len(),
            "diagnostic {} span ({s},{e}) out of range for source len {}",
            d.code,
            src.len(),
        );
        // Span must land on UTF-8 char boundaries.
        assert!(
            src.is_char_boundary(s),
            "diag {code} start {s} not char boundary",
            code = d.code
        );
        assert!(
            src.is_char_boundary(e),
            "diag {code} end {e} not char boundary",
            code = d.code
        );
    }
}
