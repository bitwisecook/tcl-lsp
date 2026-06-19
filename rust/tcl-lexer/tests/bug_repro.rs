use tcl_lexer::BracketIndex;

#[test]
fn repro_bug_close_bracket_balances_unterminated_brace_eof() {
    // Unterminated brace word at EOF: the ] is absorbed by the unterminated
    // brace word, becoming literal. It cannot balance the outer [.
    let src = "[foo {bar";
    let idx = BracketIndex::build(src);
    let at_eof = u32::try_from(src.len()).expect("offset fits u32");
    assert!(
        !idx.close_bracket_balances(at_eof),
        "BUG: close_bracket_balances({at_eof}) returned true for '{src}', \
         but ] at EOF is inside the unterminated brace word and is literal"
    );
}

#[test]
fn regression_terminated_brace_at_eof_still_balances() {
    // Properly closed brace word at EOF: ] after the brace word closes the [.
    let src = "[foo {bar}";
    let idx = BracketIndex::build(src);
    let at_eof = u32::try_from(src.len()).expect("offset fits u32");
    assert!(
        idx.close_bracket_balances(at_eof),
        "REGRESSION: close_bracket_balances({at_eof}) returned false for '{src}', \
         but ] at EOF after a closed brace word should balance"
    );
}
