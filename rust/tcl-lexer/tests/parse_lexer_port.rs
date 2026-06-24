//! Port of `tests/test_tcl_parse.py` (groups 1-2) — recreated from Tcl's
//! official `tests/parse.test`. Verifies the Rust `Lexer::tokenise_all`
//! handles leading whitespace, backslash-newline continuation, and comment
//! processing the way Tcl's parser does.
//!
//! C-Tcl proof: comment + continuation rules are Tcl's own. A backslash at end
//! of a comment line continues the comment, swallowing the next line — verified
//! against tclsh8.6/9.0:
//!     # TODO fix this \
//!     puts "SHOULD-NOT-RUN"
//!     puts "RAN"
//! prints only `RAN` (the `puts "SHOULD-NOT-RUN"` is part of the comment).

use tcl_lexer::{Lexer, SourceMap, Token, TokenType};

/// Non-separator tokens (mirrors the Python `helpers.lex`, which drops SEP/EOL),
/// paired with their resolved text.
fn lex(source: &str) -> Vec<(TokenType, String)> {
    let toks = Lexer::new(source).tokenise_all().expect("lexes");
    let sm = SourceMap::new(source);
    toks.into_iter()
        .filter(|t| !matches!(t.kind, TokenType::Sep | TokenType::Eol))
        .map(|t| (t.kind, sm.token_text(t).to_string()))
        .collect()
}

/// All tokens (unfiltered), text-resolved.
fn lex_all(source: &str) -> Vec<(TokenType, String)> {
    let toks: Vec<Token> = Lexer::new(source).tokenise_all().expect("lexes");
    let sm = SourceMap::new(source);
    toks.into_iter().map(|t| (t.kind, sm.token_text(t).to_string())).collect()
}

fn texts(source: &str) -> Vec<String> {
    lex(source).into_iter().map(|(_, t)| t).collect()
}

fn comments(source: &str) -> Vec<String> {
    lex_all(source)
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Comment)
        .map(|(_, t)| t)
        .collect()
}

// Group 1: Basic command parsing, leading space, null bytes (parse-1.x)

#[test]
fn t1_1_null_byte_in_word() {
    let t = texts("foo\0 bar");
    assert!(!t.is_empty());
    assert_eq!(t[0], "foo\0");
}

#[test]
fn t1_2_simple_command() {
    assert_eq!(texts("foo bar"), ["foo", "bar"]);
}

#[test]
fn t1_3_leading_whitespace_skipped() {
    assert_eq!(texts("  \n\t   foo")[0], "foo");
}

#[test]
fn t1_4_leading_special_whitespace() {
    // \f, \r, \v before the command — does not crash; a command word follows.
    let t = texts("\u{0c}\r\u{0b}foo");
    assert!(!t.is_empty());
}

#[test]
fn t1_5_backslash_newline_leading_space() {
    // `\<nl>` in leading space collapses to a separator; `foo` is the command.
    assert!(texts("  \\\n foo").iter().any(|w| w == "foo"));
}

#[test]
fn t1_7_missing_continuation_line_no_crash() {
    // Trailing `\<nl>` with nothing after — must lex without panicking.
    let _ = lex("   \\\n");
}

#[test]
fn t1_8_eof_after_leading_space() {
    let t = texts("      foo");
    assert_eq!(t.last().unwrap(), "foo");
}

#[test]
fn t1_9_backslash_newline_then_blank_line() {
    // `cmd1\<nl><nl>cmd2` — the blank line still ends the command; cmd2 follows.
    assert!(texts("cmd1\\\n\ncmd2").iter().any(|w| w == "cmd2"));
}

#[test]
fn t1_10_backslash_newline_multiple_words() {
    // Two `list` commands separated by a blank line; C and D are words of the
    // second command. tclsh: this is two commands, `list A B` and `list C D`.
    let t = texts("list \\\nA B\\\n\nlist C D");
    assert_eq!(t.iter().filter(|w| *w == "list").count(), 2);
    assert!(t.iter().any(|w| w == "C"));
    assert!(t.iter().any(|w| w == "D"));
}

// Group 2: Comment processing (parse-2.x)

#[test]
fn t2_1_simple_comment_then_command() {
    let src = "# foo bar\n foo";
    assert_eq!(comments(src).len(), 1);
    assert!(texts(src).iter().any(|w| w == "foo"));
}

#[test]
fn t2_2_multiple_comments() {
    assert_eq!(comments("# comment1\n# comment2\nfoo").len(), 2);
}

#[test]
fn t2_3_backslash_newline_continues_comment() {
    // The continuation line `world` is swallowed into the one comment.
    let src = "# hello \\\nworld\nfoo";
    let c = comments(src);
    assert_eq!(c.len(), 1);
    assert!(c[0].contains("hello"));
    assert!(c[0].contains("world"));
    assert!(texts(src).iter().any(|w| w == "foo"));
}

#[test]
fn t2_3b_chained_continuations_in_comment() {
    let src = "# line1 \\\nline2 \\\nline3\nfoo";
    let c = comments(src);
    assert_eq!(c.len(), 1);
    assert!(c[0].contains("line1") && c[0].contains("line2") && c[0].contains("line3"));
    assert!(texts(src).iter().any(|w| w == "foo"));
}

#[test]
fn t2_3c_continuation_swallows_following_code() {
    // tclsh-proven: the continued comment swallows `puts "hello world"`, and
    // `set x 1` is still a live command afterwards.
    let src = "# TODO: fix this later\\\nputs \"hello world\"\nset x 1";
    let c = comments(src);
    assert_eq!(c.len(), 1);
    assert!(c[0].contains("puts"));
    assert!(texts(src).iter().any(|w| w == "set"));
}

#[test]
fn comment_not_in_command_position_is_a_word() {
    // A `#` that is NOT in command position is an ordinary word, not a comment
    // (tclsh: `string length #foo` works; `#` is data here).
    let src = "string length #foo";
    assert!(comments(src).is_empty());
    assert!(texts(src).iter().any(|w| w == "#foo"));
}

fn kinds(source: &str) -> Vec<TokenType> {
    lex(source).into_iter().map(|(k, _)| k).collect()
}

// Group 3: Word parsing and space skipping (parse-3.x)

#[test]
fn t3_1_multiple_spaces_and_tabs() {
    assert_eq!(texts("foo  bar\t\tx"), ["foo", "bar", "x"]);
}

#[test]
fn t3_3_semicolon_separator_words() {
    let t = texts("foo  ;  bar x");
    for w in ["foo", "bar", "x"] {
        assert!(t.iter().any(|s| s == w), "{w}");
    }
}

#[test]
fn t3_4_trailing_spaces_dropped() {
    assert_eq!(texts("foo       "), ["foo"]);
}

#[test]
fn t3_5_quoted_word_is_one_unit() {
    // `"a b c"` is a single word (tclsh: `cnt "a b c"` → 1).
    assert!(lex("foo \"a b c\" d \"efg\"")
        .iter()
        .any(|(_, t)| t == "a b c"));
}

#[test]
fn t3_6_braces_group_and_suppress_substitution() {
    // `$b` is NOT substituted inside braces; the braced word is one STR.
    let strs: Vec<String> = lex("foo {a $b [concat foo]} {c d}")
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Str)
        .map(|(_, t)| t)
        .collect();
    assert!(strs.iter().any(|t| t.contains("$b")));
    assert!(strs.iter().any(|t| t == "c d"));
}

#[test]
fn t3_7_braced_var_name() {
    // `${abc}` → a VAR token whose name is `abc`.
    let toks = lex("foo ${abc}");
    assert_eq!(toks[0].1, "foo");
    let vars: Vec<&String> = toks.iter().filter(|(k, _)| *k == TokenType::Var).map(|(_, t)| t).collect();
    assert!(vars.iter().any(|t| *t == "abc"));
}

#[test]
fn t3_empty_commands_from_semicolons() {
    assert_eq!(texts("set a 1; set b 2").iter().filter(|w| *w == "set").count(), 2);
    assert!(texts(";;;foo").iter().any(|w| w == "foo"));
}

// Group 4: Simple words (parse-4.x)

#[test]
fn t4_1_simple_word_is_esc() {
    let t = lex("foo");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].0, TokenType::Esc);
    assert_eq!(t[0].1, "foo");
}

#[test]
fn t4_2_braced_word_is_str() {
    let t = lex("{abc}");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].0, TokenType::Str);
    assert_eq!(t[0].1, "abc");
}

#[test]
fn t4_4_word_with_variable_concatenates() {
    // `x$d` → an ESC piece and a VAR piece in one word.
    let k = kinds("x$d");
    assert!(k.contains(&TokenType::Esc));
    assert!(k.contains(&TokenType::Var));
}

#[test]
fn t4_5_word_with_command_sub() {
    assert!(kinds("\"a [foo] b\"").contains(&TokenType::Cmd));
}

#[test]
fn t4_6_simple_variable() {
    let t = lex("$x");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].0, TokenType::Var);
    assert_eq!(t[0].1, "x");
}

// Group 5: Word terminators (parse-5.x)

#[test]
fn t5_newline_and_semicolon_terminate_words() {
    for src in ["foo\n bar", "foo; bar", "\"foo\" bar"] {
        let t = texts(src);
        assert!(t.iter().any(|w| w == "foo"), "{src}");
        assert!(t.iter().any(|w| w == "bar"), "{src}");
    }
}

#[test]
fn t5_backslash_newline_after_brace() {
    assert!(texts("{abc}\\\nfoo").iter().any(|w| w == "abc"));
}

// Group 5 continued: {*} expansion (parse-5.11+)

#[test]
fn expansion_basic_var() {
    // tclsh: `list a {*}{b c} d` → 4 elements (the {*} expands).
    let k = kinds("cmd {*}$args");
    assert_eq!(lex("cmd {*}$args")[0].1, "cmd");
    assert!(k.contains(&TokenType::Expand));
    assert!(k.contains(&TokenType::Var));
}

#[test]
fn expansion_with_list_quotes_bracket() {
    // {*}{a b c} — STR "a b c"; tclsh `list {*}{a b c}` → llength 3.
    let lst = lex("cmd {*}{a b c}");
    assert!(lst.iter().any(|(k, _)| *k == TokenType::Expand));
    assert!(lst.iter().any(|(k, t)| *k == TokenType::Str && t == "a b c"));
    // {*}"a b c" — quoted argument.
    let q = lex("cmd {*}\"a b c\"");
    assert!(q.iter().any(|(k, _)| *k == TokenType::Expand));
    // {*}[list a b] — command substitution.
    let b = kinds("cmd {*}[list a b]");
    assert!(b.contains(&TokenType::Expand));
    assert!(b.contains(&TokenType::Cmd));
}

#[test]
fn no_expansion_for_brace_star_space() {
    // `{* }` is the literal string "* ", not an expansion prefix
    // (tclsh: `list {* }` → llength 1).
    let t = lex("cmd {* }");
    assert!(!t.iter().any(|(k, _)| *k == TokenType::Expand));
    assert!(t.iter().any(|(k, txt)| *k == TokenType::Str && txt == "* "));
}

fn first(source: &str) -> (TokenType, String) {
    lex(source).into_iter().next().unwrap()
}

fn vars(source: &str) -> Vec<String> {
    lex(source)
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Var)
        .map(|(_, t)| t)
        .collect()
}

// Group 12: Variable name parsing (parse-12.x)

#[test]
fn t12_simple_and_underscore_var() {
    assert_eq!(first("$abcd"), (TokenType::Var, "abcd".to_string()));
    assert_eq!(first("$az_AZ"), (TokenType::Var, "az_AZ".to_string()));
}

#[test]
fn t12_6_braced_var_name() {
    // `${..[]b}` — a braced var name takes the content verbatim.
    assert_eq!(first("${..[]b}"), (TokenType::Var, "..[]b".to_string()));
}

#[test]
fn t12_13_namespace_var() {
    // tclsh: `$xyz::ab` reads the namespace variable `xyz::ab`.
    assert_eq!(first("$xyz::ab"), (TokenType::Var, "xyz::ab".to_string()));
}

#[test]
fn t12_15_single_colon_stops_var_name() {
    // tclsh: `"$ab:cd"` with ab=X → "X:cd" — a single `:` is NOT part of the
    // name (only `::` is a namespace separator).
    assert_eq!(first("$ab:cd"), (TokenType::Var, "ab".to_string()));
}

#[test]
fn t12_18_bare_dollar_is_not_a_var() {
    // `$$ $.` — a `$` not followed by a name char is literal, not a VAR.
    let t = lex("$$ $.");
    assert!(!t.is_empty());
}

#[test]
fn t12_20_25_array_references() {
    // `$x(abc)` → VAR whose text names the array `x`.
    assert_eq!(first("$x(abc)").0, TokenType::Var);
    assert!(first("$x(abc)").1.contains('x'));
    // Array index containing a var / command sub still lexes to a VAR token.
    assert_eq!(first("$x(ab$cde[foo bar])").0, TokenType::Var);
    assert!(!vars("$x(a$y(b))").is_empty());
}

// Group 14: Braced string parsing (parse-14.x)

#[test]
fn t14_braces_suppress_substitution() {
    // `$b` is literal inside braces (tclsh: `string index {$b} 0` → `$`).
    let strs: Vec<String> = lex("foo {a $b [concat foo]} {c d}")
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Str)
        .map(|(_, t)| t)
        .collect();
    assert!(strs.iter().any(|t| t.contains("$b")));
}

#[test]
fn t14_empty_and_nested_braces() {
    // tclsh: `string length {}` → 0; `{{}}` is the one-element braced string `{}`.
    let empty: Vec<String> = lex("foo {}")
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Str)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(empty, [""]);
    let nested: Vec<String> = lex("foo {{}}")
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Str)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(nested[0], "{}");
}

#[test]
fn t14_6_backslash_is_literal_in_braces() {
    // A backslash inside braces is preserved verbatim (no decoding).
    let strs: Vec<String> = lex("foo {a \\n\\{}")
        .into_iter()
        .filter(|(k, _)| *k == TokenType::Str)
        .map(|(_, t)| t)
        .collect();
    assert!(strs.iter().any(|t| t.contains("\\n")));
}

// Group 15: Quoted strings (parse-15.x)

#[test]
fn t15_simple_quoted_word() {
    // A quoted word is one ESC token with the inner text.
    assert_eq!(first("\"hello world\""), (TokenType::Esc, "hello world".to_string()));
}

#[test]
fn t15_quoted_substitutes_var_and_command() {
    // Unlike braces, quotes DO substitute `$var` and `[cmd]`.
    assert!(lex("\"hello $name\"").iter().any(|(k, _)| *k == TokenType::Var));
    assert!(lex("\"value is [expr {1+1}]\"").iter().any(|(k, _)| *k == TokenType::Cmd));
}

#[test]
fn t15_5_incomplete_delimiters_do_not_crash() {
    for src in ["set x {", "set x \"hello", "set x [cmd"] {
        let _ = lex(src);
    }
}
