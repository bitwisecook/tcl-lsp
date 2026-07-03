//! Expression-lexer tests derived from Tcl's official `tests/parseExpr.test`.
//! Verifies the Rust `tokenise_expr` lexer classifies expression lexemes
//! according to Tcl's grammar.
//!
//! C-Tcl proof: every expression below is a real Tcl `expr` (validated against
//! tclsh8.6/9.0 — e.g. `2 ** 8`=256, `"a" eq "b"`=0, `"x" in {a b c}`=0,
//! `0xFF`=255, `0o77`=63, `0b1010`=10, `1.5e+10`=1.5e10, `.5 + .5`=1.0,
//! `sin(0)`=0.0). The lexer is a tokeniser (not an evaluator), so precedence
//! cases assert the operator *sequence* and lexeme cases assert token
//! classification, both matching Tcl's grammar.

use tcl_lexer::{ExprToken, ExprTokenType, tokenise_expr};

/// All non-whitespace tokens (the Rust lexer emits no trailing EOF token).
fn toks(source: &str) -> Vec<ExprToken> {
    tokenise_expr(source, None)
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .collect()
}

fn tok_kinds(source: &str) -> Vec<ExprTokenType> {
    toks(source).iter().map(|t| t.kind).collect()
}

fn of_kind(source: &str, kind: ExprTokenType) -> Vec<String> {
    toks(source)
        .into_iter()
        .filter(|t| t.kind == kind)
        .map(|t| t.text)
        .collect()
}

fn ops(source: &str) -> Vec<String> {
    of_kind(source, ExprTokenType::Operator)
}
fn nums(source: &str) -> Vec<String> {
    of_kind(source, ExprTokenType::Number)
}
fn funcs(source: &str) -> Vec<String> {
    of_kind(source, ExprTokenType::Function)
}

// Group 1: Basic expression parsing (parseExpr-1.x)

#[test]
fn t1_1_string_with_null() {
    let tokens = toks("42\0 + 1");
    assert!(tokens.iter().any(|t| t.kind == ExprTokenType::Number));
}

#[test]
fn t1_2_simple_number() {
    let tokens = toks("42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, ExprTokenType::Number);
    assert_eq!(tokens[0].text, "42");
}

#[test]
fn t1_3_large_number() {
    assert_eq!(toks("999999999999999999")[0].kind, ExprTokenType::Number);
}

#[test]
fn t1_4_empty_expression() {
    assert!(toks("").is_empty());
}

// Group 2: Ternary (parseExpr-2.x)

#[test]
fn t2_1_valid_ternary() {
    assert!(tok_kinds("2>3 ? 1 : 0").contains(&ExprTokenType::TernaryQ));
    assert!(tok_kinds("2>3 ? 1 : 0").contains(&ExprTokenType::TernaryC));
}

#[test]
fn t2_4_ternary_components() {
    let k = tok_kinds("$x > 0 ? 1 : 0");
    assert!(k.contains(&ExprTokenType::Variable));
    assert!(k.contains(&ExprTokenType::TernaryQ));
    assert!(k.contains(&ExprTokenType::TernaryC));
}

#[test]
fn t2_6_minimal_ternary() {
    let t = toks("1 ? 2 : 3");
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::TernaryQ)
            .count(),
        1
    );
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::TernaryC)
            .count(),
        1
    );
}

#[test]
fn t2_9_nested_ternary_logical_else() {
    assert!(ops("$x > 0 ? 1 : $y && $z").contains(&"&&".to_string()));
}

#[test]
fn t2_nested_ternary() {
    let t = toks("$a ? $b ? 1 : 2 : 3");
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::TernaryQ)
            .count(),
        2
    );
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::TernaryC)
            .count(),
        2
    );
}

// Groups 3-13: operator sequences (precedence as token order)

#[test]
fn t3_logical_or() {
    assert_eq!(ops("$a && $b || $c"), ["&&", "||"]);
    assert!(ops("$a || $b").contains(&"||".to_string()));
    assert_eq!(ops("$a || $b || $c"), ["||", "||"]);
}

#[test]
fn t4_logical_and() {
    assert_eq!(ops("$a | $b && $c"), ["|", "&&"]);
    assert!(ops("$a && $b").contains(&"&&".to_string()));
    assert_eq!(ops("$a && $b && $c"), ["&&", "&&"]);
}

#[test]
fn t5_bitwise_or() {
    assert_eq!(ops("$a ^ $b | $c"), ["^", "|"]);
    assert!(ops("$a | $b").contains(&"|".to_string()));
    assert_eq!(ops("$a | $b | $c"), ["|", "|"]);
}

#[test]
fn t6_bitwise_xor() {
    assert_eq!(ops("$a & $b ^ $c"), ["&", "^"]);
    assert!(ops("$a ^ $b").contains(&"^".to_string()));
}

#[test]
fn t7_bitwise_and() {
    assert_eq!(ops("$a == $b & $c"), ["==", "&"]);
    assert!(ops("$a & $b").contains(&"&".to_string()));
}

#[test]
fn t8_equality() {
    assert!(ops("$a == $b").contains(&"==".to_string()));
    assert!(ops("$a != $b").contains(&"!=".to_string()));
    assert_eq!(ops("$a == $b != $c"), ["==", "!="]);
}

#[test]
fn t9_relational() {
    assert!(ops("$a < $b").contains(&"<".to_string()));
    assert!(ops("$a > $b").contains(&">".to_string()));
    assert!(ops("$a <= $b").contains(&"<=".to_string()));
    assert!(ops("$a >= $b").contains(&">=".to_string()));
    assert_eq!(ops("$a < $b > $c <= $d"), ["<", ">", "<="]);
}

#[test]
fn t10_shift() {
    assert!(ops("$a << 2").contains(&"<<".to_string()));
    assert!(ops("$a >> 2").contains(&">>".to_string()));
    assert_eq!(ops("$a << $b >> $c"), ["<<", ">>"]);
}

#[test]
fn t11_add_sub() {
    assert!(ops("$a + $b").contains(&"+".to_string()));
    assert!(ops("$a - $b").contains(&"-".to_string()));
    assert_eq!(ops("$a + $b - $c + $d"), ["+", "-", "+"]);
}

#[test]
fn t13_mul_div_mod() {
    assert!(ops("$a * $b").contains(&"*".to_string()));
    assert!(ops("$a / $b").contains(&"/".to_string()));
    assert!(ops("$a % $b").contains(&"%".to_string()));
    assert_eq!(ops("$a * $b / $c % $d"), ["*", "/", "%"]);
}

// Group 14: Unary (parseExpr-14.x)

#[test]
fn t14_unary_ops() {
    assert_eq!(toks("+$x")[0].text, "+");
    assert_eq!(toks("+$x")[0].kind, ExprTokenType::Operator);
    assert_eq!(toks("-$x")[0].text, "-");
    assert_eq!(toks("~$x")[0].text, "~");
    assert_eq!(toks("!$x")[0].text, "!");
}

#[test]
fn t14_7_nested_unary() {
    let t = toks("--$x");
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::Operator)
            .count(),
        2
    );
}

#[test]
fn t14_11_parenthesized() {
    let k = tok_kinds("($x + 1)");
    assert!(k.contains(&ExprTokenType::ParenOpen));
    assert!(k.contains(&ExprTokenType::ParenClose));
}

// Group 15: Primary expressions (parseExpr-15.x)

#[test]
fn t15_1_parenthesized_division() {
    assert!(tok_kinds("(4 / 2)").contains(&ExprTokenType::ParenOpen));
    assert!(ops("(4 / 2)").contains(&"/".to_string()));
}

#[test]
fn t15_3_nested_ternary_in_parens() {
    assert!(tok_kinds("(1 > 0 ? 2 + 3 : 4)").contains(&ExprTokenType::TernaryQ));
}

#[test]
fn t15_6_7_int_and_float_literal() {
    assert_eq!(toks("42")[0].kind, ExprTokenType::Number);
    assert_eq!(toks("42")[0].text, "42");
    assert_eq!(toks("3.14")[0].kind, ExprTokenType::Number);
    assert_eq!(toks("3.14")[0].text, "3.14");
}

#[test]
fn t15_8_to_10_variables() {
    assert_eq!(toks("$a")[0].kind, ExprTokenType::Variable);
    assert_eq!(toks("$a")[0].text, "$a");
    assert_eq!(toks("$a(1)")[0].kind, ExprTokenType::Variable);
    assert_eq!(toks("$a()")[0].kind, ExprTokenType::Variable);
}

#[test]
fn t15_12_14_quoted_with_var_and_command() {
    assert!(!toks("\"hello $x\"").is_empty());
    assert!(!toks("\"[cmd] and $var\"").is_empty());
}

#[test]
fn t15_15_16_command_substitution() {
    assert_eq!(toks("[expr {1+1}]")[0].kind, ExprTokenType::Command);
    assert_eq!(toks("[cmd1; cmd2]")[0].kind, ExprTokenType::Command);
}

#[test]
fn t15_19_21_braced_string() {
    assert_eq!(toks("{hello world}")[0].kind, ExprTokenType::String);
    assert_eq!(toks("{hello\\\nworld}")[0].kind, ExprTokenType::String);
}

#[test]
fn t15_22_26_29_functions() {
    assert_eq!(toks("sin(3.14)")[0].kind, ExprTokenType::Function);
    assert_eq!(toks("sin(3.14)")[0].text, "sin");
    assert!(funcs("pow(2, 3 * 4)").contains(&"pow".to_string()));
    assert!(ops("pow(2, 3 * 4)").contains(&"*".to_string()));
    assert!(funcs("atan2($y, $x)").contains(&"atan2".to_string()));
    assert_eq!(
        toks("atan2($y, $x)")
            .iter()
            .filter(|t| t.kind == ExprTokenType::Comma)
            .count(),
        1
    );
}

// Group 16: Lexeme recognition (parseExpr-16.x)

#[test]
fn t16_integer_and_float_lexemes() {
    assert_eq!(toks("000")[0].text, "000");
    assert_eq!(toks("000")[0].kind, ExprTokenType::Number);
    assert_eq!(toks("123456789012345")[0].kind, ExprTokenType::Number);
    assert_eq!(toks("1.5")[0].text, "1.5");
    assert_eq!(toks("1.5e10")[0].text, "1.5e10");
    assert_eq!(toks("1.5e10")[0].kind, ExprTokenType::Number);
}

#[test]
fn t16_brackets_braces_parens() {
    assert_eq!(toks("[cmd]")[0].kind, ExprTokenType::Command);
    assert_eq!(toks("{hello}")[0].kind, ExprTokenType::String);
    assert_eq!(toks("(1)")[0].kind, ExprTokenType::ParenOpen);
    assert_eq!(toks("(1)").last().unwrap().kind, ExprTokenType::ParenClose);
    assert_eq!(toks("$x")[0].kind, ExprTokenType::Variable);
    assert_eq!(toks("\"hello\"")[0].kind, ExprTokenType::String);
}

#[test]
fn t16_19_comma() {
    assert_eq!(
        toks("pow(2, 3)")
            .iter()
            .filter(|t| t.kind == ExprTokenType::Comma)
            .count(),
        1
    );
}

#[test]
fn t16_all_single_and_multi_operators() {
    assert_eq!(ops("2 * 3"), ["*"]);
    assert_eq!(ops("6 / 2"), ["/"]);
    assert_eq!(ops("7 % 3"), ["%"]);
    assert_eq!(ops("1 + 2"), ["+"]);
    assert_eq!(ops("3 - 1"), ["-"]);
    assert_eq!(ops("$a < $b"), ["<"]);
    assert_eq!(ops("$a << 2"), ["<<"]);
    assert_eq!(ops("$a <= $b"), ["<="]);
    assert_eq!(ops("$a > $b"), [">"]);
    assert_eq!(ops("$a >> 2"), [">>"]);
    assert_eq!(ops("$a >= $b"), [">="]);
    assert_eq!(ops("$a == $b"), ["=="]);
    assert_eq!(ops("$a != $b"), ["!="]);
    assert_eq!(ops("!$a"), ["!"]);
    assert_eq!(ops("$a && $b"), ["&&"]);
    assert_eq!(ops("$a & $b"), ["&"]);
    assert_eq!(ops("$a ^ $b"), ["^"]);
    assert_eq!(ops("$a || $b"), ["||"]);
    assert_eq!(ops("$a | $b"), ["|"]);
    assert_eq!(ops("~$a"), ["~"]);
}

#[test]
fn t16_25_ternary_ops() {
    let k = tok_kinds("1 ? 2 : 3");
    assert!(k.contains(&ExprTokenType::TernaryQ));
    assert!(k.contains(&ExprTokenType::TernaryC));
}

#[test]
fn t16_42_43_function_names() {
    assert_eq!(toks("sin(0)")[0].kind, ExprTokenType::Function);
    assert_eq!(toks("sin(0)")[0].text, "sin");
    // An unknown identifier still classifies as Function (default fallback).
    assert_eq!(toks("myfunc(0)")[0].kind, ExprTokenType::Function);
    assert_eq!(toks("myfunc(0)")[0].text, "myfunc");
}

// Group 17: Token array expansion (parseExpr-17.x)

#[test]
fn t17_1_many_subexpressions() {
    let parts: Vec<String> = (0..30).map(|i| format!("$v{i}")).collect();
    let src = parts.join(" + ");
    let t = toks(&src);
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::Variable)
            .count(),
        30
    );
    assert_eq!(
        t.iter()
            .filter(|x| x.kind == ExprTokenType::Operator)
            .count(),
        29
    );
}

// Groups 19-20, 22: integer/number parsing edge cases

#[test]
fn t19_20_radix_and_large_integers() {
    assert_eq!(toks("0xFF")[0].text, "0xFF");
    assert_eq!(toks("0xFF")[0].kind, ExprTokenType::Number);
    assert_eq!(toks("123456789012345678")[0].kind, ExprTokenType::Number);
    assert_eq!(toks(&"1".repeat(32))[0].kind, ExprTokenType::Number);
    assert_eq!(toks("0o77")[0].text, "0o77");
    assert_eq!(toks("0b1010")[0].text, "0b1010");
    assert_eq!(toks("0XFF")[0].kind, ExprTokenType::Number);
}

#[test]
fn t20_scientific_with_signs() {
    assert_eq!(toks("1.5e+10")[0].text, "1.5e+10");
    assert_eq!(toks("1.5e+10")[0].kind, ExprTokenType::Number);
    assert_eq!(toks("1.5e-10")[0].text, "1.5e-10");
}

#[test]
fn t22_14_legacy_octal_and_leading_dot() {
    assert_eq!(toks("07")[0].text, "07");
    assert_eq!(toks("07")[0].kind, ExprTokenType::Number);
    assert_eq!(toks(".5")[0].text, ".5");
    assert_eq!(toks(".5")[0].kind, ExprTokenType::Number);
    assert_eq!(nums(".5 + .5").len(), 2);
}

#[test]
fn t22_word_and_power_operators() {
    // tclsh: 2**8=256; "a" eq "b"=0; "a" ne "b"=1; "x" in {a b c}=0; "x" ni …=1.
    assert_eq!(ops("2 ** 8"), ["**"]);
    assert_eq!(ops("\"a\" eq \"b\""), ["eq"]);
    assert_eq!(ops("\"a\" ne \"b\""), ["ne"]);
    assert_eq!(ops("\"x\" in {a b c}"), ["in"]);
    assert_eq!(ops("\"x\" ni {a b c}"), ["ni"]);
}

// Group 21: Error handling — no crash, tokens still produced

#[test]
fn t21_unterminated_delimiters_do_not_crash() {
    assert!(!toks("\"hello").is_empty());
    assert!(!toks("{hello").is_empty());
    assert!(!toks("[cmd").is_empty());
}

#[test]
fn t21_16_empty_parens() {
    let k = tok_kinds("()");
    assert!(k.contains(&ExprTokenType::ParenOpen));
    assert!(k.contains(&ExprTokenType::ParenClose));
}

#[test]
fn t21_19_empty_and_invalid_char() {
    assert!(toks("").is_empty());
    // An unknown character is skipped (no token), without panicking.
    assert!(toks("@").is_empty());
}

// Boolean literals (case-insensitive per Tcl_GetBoolean)

#[test]
fn boolean_literals() {
    assert_eq!(toks("true")[0].kind, ExprTokenType::Bool);
    assert_eq!(toks("true")[0].text, "true");
    assert_eq!(toks("false")[0].kind, ExprTokenType::Bool);
    assert_eq!(toks("yes")[0].kind, ExprTokenType::Bool);
    assert_eq!(toks("no")[0].kind, ExprTokenType::Bool);
    let t = toks("true && false");
    assert_eq!(
        t.iter().filter(|x| x.kind == ExprTokenType::Bool).count(),
        2
    );
}

// Math functions (the documented `::tcl::mathfunc` set)

#[test]
fn known_math_functions_classify_as_function() {
    for f in [
        "abs", "acos", "asin", "atan", "atan2", "bool", "ceil", "cos", "cosh", "double", "entier",
        "exp", "floor", "fmod", "hypot", "int", "isqrt", "log", "log10", "max", "min", "pow",
        "rand", "round", "sin", "sinh", "sqrt", "srand", "tan", "tanh", "wide",
    ] {
        let src = format!("{f}(0)");
        let t = toks(&src);
        assert_eq!(t[0].kind, ExprTokenType::Function, "{f}");
        assert_eq!(t[0].text, f);
    }
}

#[test]
fn nested_functions() {
    assert_eq!(funcs("abs(sin(cos($x)))"), ["abs", "sin", "cos"]);
    assert_eq!(funcs("int(ceil(sqrt($n)))"), ["int", "ceil", "sqrt"]);
}

// Complex real-world expressions

#[test]
fn complex_expressions() {
    let t = tok_kinds("($x + 1) * sin($y) > 0.5");
    for want in [
        ExprTokenType::Variable,
        ExprTokenType::Number,
        ExprTokenType::Operator,
        ExprTokenType::Function,
        ExprTokenType::ParenOpen,
        ExprTokenType::ParenClose,
    ] {
        assert!(t.contains(&want), "{want:?}");
    }

    let e = "($mask & 0xFF) | ($flags << 8)";
    assert!(ops(e).contains(&"&".to_string()));
    assert!(ops(e).contains(&"|".to_string()));
    assert!(ops(e).contains(&"<<".to_string()));

    let s = "\"foo\" eq \"bar\" || \"baz\" ne \"qux\"";
    assert!(ops(s).contains(&"eq".to_string()));
    assert!(ops(s).contains(&"ne".to_string()));
    assert!(ops(s).contains(&"||".to_string()));

    let ct = toks("$x > 0 ? ($x > 100 ? 100 : $x) : 0");
    assert_eq!(
        ct.iter()
            .filter(|x| x.kind == ExprTokenType::TernaryQ)
            .count(),
        2
    );
    assert_eq!(
        ct.iter()
            .filter(|x| x.kind == ExprTokenType::TernaryC)
            .count(),
        2
    );

    assert!(funcs("wide(0xDEADBEEF)").contains(&"wide".to_string()));
    assert!(nums("wide(0xDEADBEEF)").contains(&"0xDEADBEEF".to_string()));

    assert_eq!(ops("$x > 0 && $x < 100"), [">", "&&", "<"]);
}

#[test]
fn empty_and_whitespace_only() {
    assert!(toks("").is_empty());
    assert!(toks("   \t  \n  ").is_empty());
}
