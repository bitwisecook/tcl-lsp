"""Tests recreated from Tcl's official tests/parseExpr.test.

These tests verify our ExprLexer's behaviour against the expression parsing
behaviours specified in the official Tcl test suite
(https://github.com/tcltk/tcl/blob/main/tests/parseExpr.test).

Since our ExprLexer is a tokeniser (not an evaluator), we adapt:
- Operator precedence tests become tests verifying correct token sequences
- Error tests verify no crashes and correct token production
- Lexeme recognition tests verify correct ExprTokenType classification
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.expr_lexer import ExprToken, ExprTokenType, tokenise_expr

# Helpers


def toks(source: str) -> list[ExprToken]:
    """All non-whitespace tokens."""
    return [t for t in tokenise_expr(source) if t.type != ExprTokenType.WHITESPACE]


def tok_types(source: str) -> list[ExprTokenType]:
    """Token types, excluding whitespace."""
    return [t.type for t in toks(source)]


def tok_texts(source: str) -> list[str]:
    """Token texts, excluding whitespace."""
    return [t.text for t in toks(source)]


def ops(source: str) -> list[str]:
    """Just operator texts."""
    return [t.text for t in toks(source) if t.type == ExprTokenType.OPERATOR]


def nums(source: str) -> list[str]:
    """Just number texts."""
    return [t.text for t in toks(source) if t.type == ExprTokenType.NUMBER]


def vars_(source: str) -> list[str]:
    """Just variable texts."""
    return [t.text for t in toks(source) if t.type == ExprTokenType.VARIABLE]


def funcs(source: str) -> list[str]:
    """Just function name texts."""
    return [t.text for t in toks(source) if t.type == ExprTokenType.FUNCTION]


# Group 1: Basic expression parsing (parseExpr-1.x)


class TestBasicParsing:
    """parseExpr-1.x: Basic expression parsing."""

    def test_1_1_string_with_null(self):
        """parseExpr-1.1: String with null byte."""
        tokens = toks("42\x00 + 1")
        assert any(t.type == ExprTokenType.NUMBER for t in tokens)

    def test_1_2_simple_number(self):
        """parseExpr-1.2: Simple number parsing."""
        tokens = toks("42")
        assert len(tokens) == 1
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "42"

    def test_1_3_large_number(self):
        """parseExpr-1.3: Large number."""
        tokens = toks("999999999999999999")
        assert tokens[0].type == ExprTokenType.NUMBER

    def test_1_4_empty_expression(self):
        """parseExpr-1.4: Empty expression."""
        tokens = toks("")
        assert tokens == []


# Group 2: Conditional (ternary) expressions (parseExpr-2.x)


class TestTernary:
    """parseExpr-2.x: Ternary operator parsing."""

    def test_2_1_valid_ternary(self):
        """parseExpr-2.1: 2>3 ? 1 : 0."""
        _ = toks("2>3 ? 1 : 0")
        assert ExprTokenType.TERNARY_Q in tok_types("2>3 ? 1 : 0")
        assert ExprTokenType.TERNARY_C in tok_types("2>3 ? 1 : 0")

    def test_2_4_ternary_components(self):
        """parseExpr-2.4: Complete ternary with all components."""
        tokens = toks("$x > 0 ? 1 : 0")
        assert ExprTokenType.VARIABLE in [t.type for t in tokens]
        assert ExprTokenType.TERNARY_Q in [t.type for t in tokens]
        assert ExprTokenType.TERNARY_C in [t.type for t in tokens]

    def test_2_6_minimal_ternary(self):
        """parseExpr-2.6: Simple ternary."""
        tokens = toks("1 ? 2 : 3")
        assert len([t for t in tokens if t.type == ExprTokenType.TERNARY_Q]) == 1
        assert len([t for t in tokens if t.type == ExprTokenType.TERNARY_C]) == 1

    def test_2_9_nested_ternary(self):
        """parseExpr-2.9: Ternary with logical operators in else branch."""
        _ = toks("$x > 0 ? 1 : $y && $z")
        assert "&&" in ops("$x > 0 ? 1 : $y && $z")

    def test_nested_ternary(self):
        """Nested ternary operators."""
        tokens = toks("$a ? $b ? 1 : 2 : 3")
        qs = [t for t in tokens if t.type == ExprTokenType.TERNARY_Q]
        cs = [t for t in tokens if t.type == ExprTokenType.TERNARY_C]
        assert len(qs) == 2
        assert len(cs) == 2


# Group 3: Logical OR (parseExpr-3.x)


class TestLogicalOr:
    """parseExpr-3.x: Logical OR (||) parsing."""

    def test_3_1_and_with_or(self):
        """parseExpr-3.1: Logical AND combined with OR."""
        assert ops("$a && $b || $c") == ["&&", "||"]

    def test_3_4_simple_or(self):
        """parseExpr-3.4: Simple OR operator."""
        assert "||" in ops("$a || $b")

    def test_3_6_multiple_or(self):
        """parseExpr-3.6: Multiple OR operators."""
        assert ops("$a || $b || $c") == ["||", "||"]


# Group 4: Logical AND (parseExpr-4.x)


class TestLogicalAnd:
    """parseExpr-4.x: Logical AND (&&) parsing."""

    def test_4_1_bitwise_or_with_and(self):
        """parseExpr-4.1: Bitwise OR combined with AND."""
        assert ops("$a | $b && $c") == ["|", "&&"]

    def test_4_4_simple_and(self):
        """parseExpr-4.4: Simple AND operator."""
        assert "&&" in ops("$a && $b")

    def test_4_6_multiple_and(self):
        """parseExpr-4.6: Multiple AND operators."""
        assert ops("$a && $b && $c") == ["&&", "&&"]


# Group 5: Bitwise OR (parseExpr-5.x)


class TestBitwiseOr:
    """parseExpr-5.x: Bitwise OR (|) parsing."""

    def test_5_1_xor_with_or(self):
        """parseExpr-5.1: XOR combined with bitwise OR."""
        assert ops("$a ^ $b | $c") == ["^", "|"]

    def test_5_4_simple_bitwise_or(self):
        """parseExpr-5.4: Simple bitwise OR."""
        assert "|" in ops("$a | $b")

    def test_5_6_multiple_bitwise_or(self):
        """parseExpr-5.6: Multiple bitwise OR."""
        assert ops("$a | $b | $c") == ["|", "|"]


# Group 6: Bitwise XOR (parseExpr-6.x)


class TestBitwiseXor:
    """parseExpr-6.x: Bitwise XOR (^) parsing."""

    def test_6_1_and_with_xor(self):
        """parseExpr-6.1: Bitwise AND combined with XOR."""
        assert ops("$a & $b ^ $c") == ["&", "^"]

    def test_6_4_simple_xor(self):
        """parseExpr-6.4: Simple XOR."""
        assert "^" in ops("$a ^ $b")


# Group 7: Bitwise AND (parseExpr-7.x)


class TestBitwiseAnd:
    """parseExpr-7.x: Bitwise AND (&) parsing."""

    def test_7_1_eq_with_and(self):
        """parseExpr-7.1: Equality combined with bitwise AND."""
        assert ops("$a == $b & $c") == ["==", "&"]

    def test_7_4_simple_bitwise_and(self):
        """parseExpr-7.4: Simple bitwise AND."""
        assert "&" in ops("$a & $b")


# Group 8: Equality (parseExpr-8.x)


class TestEquality:
    """parseExpr-8.x: Equality (==, !=) parsing."""

    def test_8_4_equality(self):
        """parseExpr-8.4: Equality operator."""
        assert "==" in ops("$a == $b")

    def test_8_5_inequality(self):
        """parseExpr-8.5: Inequality operator."""
        assert "!=" in ops("$a != $b")

    def test_8_7_multiple_equality(self):
        """parseExpr-8.7: Multiple equality operators."""
        assert ops("$a == $b != $c") == ["==", "!="]


# Group 9: Relational (parseExpr-9.x)


class TestRelational:
    """parseExpr-9.x: Relational operators (<, >, <=, >=)."""

    def test_9_4_less_than(self):
        """parseExpr-9.4: Less-than."""
        assert "<" in ops("$a < $b")

    def test_9_5_greater_than(self):
        """parseExpr-9.5: Greater-than."""
        assert ">" in ops("$a > $b")

    def test_9_6_less_equal(self):
        """parseExpr-9.6: Less-than-or-equal."""
        assert "<=" in ops("$a <= $b")

    def test_9_7_greater_equal(self):
        """parseExpr-9.7: Greater-than-or-equal."""
        assert ">=" in ops("$a >= $b")

    def test_9_9_multiple_relational(self):
        """parseExpr-9.9: Multiple relational operators."""
        assert ops("$a < $b > $c <= $d") == ["<", ">", "<="]


# Group 10: Shift (parseExpr-10.x)


class TestShift:
    """parseExpr-10.x: Shift operators (<<, >>)."""

    def test_10_4_left_shift(self):
        """parseExpr-10.4: Left shift."""
        assert "<<" in ops("$a << 2")

    def test_10_5_right_shift(self):
        """parseExpr-10.5: Right shift."""
        assert ">>" in ops("$a >> 2")

    def test_10_7_multiple_shifts(self):
        """parseExpr-10.7: Multiple shift operators."""
        assert ops("$a << $b >> $c") == ["<<", ">>"]


# Group 11-12: Addition/Subtraction (parseExpr-11.x, 12.x)


class TestAddition:
    """parseExpr-11.x/12.x: Addition and subtraction."""

    def test_11_4_addition(self):
        """parseExpr-11.4: Addition operator."""
        assert "+" in ops("$a + $b")

    def test_11_5_subtraction(self):
        """parseExpr-11.5: Subtraction operator (note: also could be unary)."""
        assert "-" in ops("$a - $b")

    def test_11_7_multiple_add_sub(self):
        """Multiple + and - operators."""
        assert ops("$a + $b - $c + $d") == ["+", "-", "+"]


# Group 13: Multiplication (parseExpr-13.x)


class TestMultiplication:
    """parseExpr-13.x: Multiplication, division, modulo."""

    def test_13_4_multiplication(self):
        """parseExpr-13.4: Multiplication."""
        assert "*" in ops("$a * $b")

    def test_13_5_division(self):
        """parseExpr-13.5: Division."""
        assert "/" in ops("$a / $b")

    def test_13_6_modulo(self):
        """parseExpr-13.6: Modulo."""
        assert "%" in ops("$a % $b")

    def test_13_8_multiple_multiply(self):
        """parseExpr-13.8: Multiple multiplication operators."""
        assert ops("$a * $b / $c % $d") == ["*", "/", "%"]


# Group 14: Unary (parseExpr-14.x)


class TestUnary:
    """parseExpr-14.x: Unary operators (+, -, ~, !)."""

    def test_14_1_unary_plus(self):
        """parseExpr-14.1: Unary plus."""
        tokens = toks("+$x")
        assert tokens[0].type == ExprTokenType.OPERATOR
        assert tokens[0].text == "+"

    def test_14_2_unary_minus(self):
        """parseExpr-14.2: Unary minus."""
        tokens = toks("-$x")
        assert tokens[0].type == ExprTokenType.OPERATOR
        assert tokens[0].text == "-"

    def test_14_3_bitwise_not(self):
        """parseExpr-14.3: Bitwise NOT."""
        tokens = toks("~$x")
        assert tokens[0].type == ExprTokenType.OPERATOR
        assert tokens[0].text == "~"

    def test_14_4_logical_not(self):
        """parseExpr-14.4: Logical NOT."""
        tokens = toks("!$x")
        assert tokens[0].type == ExprTokenType.OPERATOR
        assert tokens[0].text == "!"

    def test_14_7_nested_unary(self):
        """parseExpr-14.7: Multiple nested unary operators."""
        tokens = toks("--$x")
        op_tokens = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert len(op_tokens) == 2

    def test_14_11_parenthesized(self):
        """parseExpr-14.11: Parenthesized subexpression."""
        _ = toks("($x + 1)")
        assert ExprTokenType.PAREN_OPEN in tok_types("($x + 1)")
        assert ExprTokenType.PAREN_CLOSE in tok_types("($x + 1)")


# Group 15: Primary expressions (parseExpr-15.x)


class TestPrimary:
    """parseExpr-15.x: Primary expressions."""

    def test_15_1_parenthesized_division(self):
        """parseExpr-15.1: Parenthesized division."""
        tokens = toks("(4 / 2)")
        assert ExprTokenType.PAREN_OPEN in [t.type for t in tokens]
        assert "/" in ops("(4 / 2)")

    def test_15_3_nested_ternary_in_parens(self):
        """parseExpr-15.3: Ternary inside parentheses."""
        tokens = toks("(1 > 0 ? 2 + 3 : 4)")
        assert ExprTokenType.TERNARY_Q in [t.type for t in tokens]

    def test_15_6_integer_literal(self):
        """parseExpr-15.6: Integer literal."""
        assert toks("42")[0].type == ExprTokenType.NUMBER
        assert toks("42")[0].text == "42"

    def test_15_7_float_literal(self):
        """parseExpr-15.7: Floating-point literal."""
        assert toks("3.14")[0].type == ExprTokenType.NUMBER
        assert toks("3.14")[0].text == "3.14"

    def test_15_8_simple_variable(self):
        """parseExpr-15.8: Simple variable $a."""
        tokens = toks("$a")
        assert tokens[0].type == ExprTokenType.VARIABLE
        assert tokens[0].text == "$a"

    def test_15_9_array_variable(self):
        """parseExpr-15.9: Array variable $a(1)."""
        tokens = toks("$a(1)")
        assert tokens[0].type == ExprTokenType.VARIABLE

    def test_15_10_empty_array_index(self):
        """parseExpr-15.10: Array with empty index $a()."""
        tokens = toks("$a()")
        assert tokens[0].type == ExprTokenType.VARIABLE

    def test_15_12_quoted_string(self):
        """parseExpr-15.12: Quoted string with variable."""
        tokens = toks('"hello $x"')
        # Should contain STRING and/or VARIABLE tokens
        assert len(tokens) >= 1

    def test_15_14_quoted_with_command_and_var(self):
        """parseExpr-15.14: Quoted string with both command and variable."""
        tokens = toks('"[cmd] and $var"')
        assert len(tokens) >= 1

    def test_15_15_command_substitution(self):
        """parseExpr-15.15: Command substitution."""
        tokens = toks("[expr {1+1}]")
        assert tokens[0].type == ExprTokenType.COMMAND

    def test_15_16_multiple_commands(self):
        """parseExpr-15.16: Multiple commands in brackets."""
        tokens = toks("[cmd1; cmd2]")
        assert tokens[0].type == ExprTokenType.COMMAND

    def test_15_19_braced_string(self):
        """parseExpr-15.19: Braced string literal."""
        tokens = toks("{hello world}")
        assert tokens[0].type == ExprTokenType.STRING

    def test_15_21_braced_with_continuation(self):
        """parseExpr-15.21: Braced string with backslash continuation."""
        source = "{hello\\\nworld}"
        tokens = toks(source)
        assert tokens[0].type == ExprTokenType.STRING

    def test_15_22_function_call(self):
        """parseExpr-15.22: Math function call."""
        tokens = toks("sin(3.14)")
        assert tokens[0].type == ExprTokenType.FUNCTION
        assert tokens[0].text == "sin"

    def test_15_26_function_with_expr(self):
        """parseExpr-15.26: Function with multiplication in argument."""
        _ = toks("pow(2, 3 * 4)")
        assert "pow" in funcs("pow(2, 3 * 4)")
        assert "*" in ops("pow(2, 3 * 4)")

    def test_15_29_function_multiple_args(self):
        """parseExpr-15.29: Function with multiple arguments."""
        tokens = toks("atan2($y, $x)")
        assert "atan2" in funcs("atan2($y, $x)")
        commas = [t for t in tokens if t.type == ExprTokenType.COMMA]
        assert len(commas) == 1


# Group 16: Lexeme recognition (parseExpr-16.x)


class TestLexemeRecognition:
    """parseExpr-16.x: Individual lexeme recognition."""

    def test_16_4_integer_000(self):
        """parseExpr-16.4: Integer lexeme 000."""
        tokens = toks("000")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "000"

    def test_16_5_large_integer(self):
        """parseExpr-16.5: Large integer lexeme."""
        tokens = toks("123456789012345")
        assert tokens[0].type == ExprTokenType.NUMBER

    def test_16_7_float(self):
        """parseExpr-16.7: Float lexeme."""
        tokens = toks("1.5")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "1.5"

    def test_16_8_scientific(self):
        """parseExpr-16.8: Scientific notation."""
        tokens = toks("1.5e10")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "1.5e10"

    def test_16_13_command_bracket(self):
        """parseExpr-16.13: Command bracket lexeme."""
        tokens = toks("[cmd]")
        assert tokens[0].type == ExprTokenType.COMMAND

    def test_16_14_open_brace(self):
        """parseExpr-16.14: Open brace lexeme."""
        tokens = toks("{hello}")
        assert tokens[0].type == ExprTokenType.STRING

    def test_16_15_open_paren(self):
        """parseExpr-16.15: Open parenthesis."""
        tokens = toks("(1)")
        assert tokens[0].type == ExprTokenType.PAREN_OPEN

    def test_16_16_close_paren(self):
        """parseExpr-16.16: Close parenthesis."""
        tokens = toks("(1)")
        assert tokens[-1].type == ExprTokenType.PAREN_CLOSE

    def test_16_17_dollar_variable(self):
        """parseExpr-16.17: Dollar sign variable."""
        tokens = toks("$x")
        assert tokens[0].type == ExprTokenType.VARIABLE

    def test_16_18_quoted_string(self):
        """parseExpr-16.18: Quoted string."""
        tokens = toks('"hello"')
        assert tokens[0].type == ExprTokenType.STRING

    def test_16_19_comma(self):
        """parseExpr-16.19: Comma in function call."""
        tokens = toks("pow(2, 3)")
        commas = [t for t in tokens if t.type == ExprTokenType.COMMA]
        assert len(commas) == 1

    def test_16_20_multiply(self):
        """parseExpr-16.20: Multiplication operator."""
        assert ops("2 * 3") == ["*"]

    def test_16_21_divide(self):
        """parseExpr-16.21: Division operator."""
        assert ops("6 / 2") == ["/"]

    def test_16_22_modulo(self):
        """parseExpr-16.22: Modulo operator."""
        assert ops("7 % 3") == ["%"]

    def test_16_23_add(self):
        """parseExpr-16.23: Addition operator."""
        assert ops("1 + 2") == ["+"]

    def test_16_24_subtract(self):
        """parseExpr-16.24: Subtraction operator."""
        assert ops("3 - 1") == ["-"]

    def test_16_25_ternary_ops(self):
        """parseExpr-16.25: Ternary operators ? and :."""
        tokens = toks("1 ? 2 : 3")
        assert ExprTokenType.TERNARY_Q in [t.type for t in tokens]
        assert ExprTokenType.TERNARY_C in [t.type for t in tokens]

    def test_16_26_less_than(self):
        """parseExpr-16.26: Less-than."""
        assert ops("$a < $b") == ["<"]

    def test_16_27_left_shift(self):
        """parseExpr-16.27: Left shift."""
        assert ops("$a << 2") == ["<<"]

    def test_16_28_less_equal(self):
        """parseExpr-16.28: Less-than-or-equal."""
        assert ops("$a <= $b") == ["<="]

    def test_16_29_greater_than(self):
        """parseExpr-16.29: Greater-than."""
        assert ops("$a > $b") == [">"]

    def test_16_30_right_shift(self):
        """parseExpr-16.30: Right shift."""
        assert ops("$a >> 2") == [">>"]

    def test_16_31_greater_equal(self):
        """parseExpr-16.31: Greater-than-or-equal."""
        assert ops("$a >= $b") == [">="]

    def test_16_32_equality(self):
        """parseExpr-16.32: Equality."""
        assert ops("$a == $b") == ["=="]

    def test_16_34_inequality(self):
        """parseExpr-16.34: Inequality."""
        assert ops("$a != $b") == ["!="]

    def test_16_35_logical_not(self):
        """parseExpr-16.35: Logical NOT."""
        assert ops("!$a") == ["!"]

    def test_16_36_logical_and(self):
        """parseExpr-16.36: Logical AND."""
        assert ops("$a && $b") == ["&&"]

    def test_16_37_bitwise_and(self):
        """parseExpr-16.37: Bitwise AND."""
        assert ops("$a & $b") == ["&"]

    def test_16_38_bitwise_xor(self):
        """parseExpr-16.38: Bitwise XOR."""
        assert ops("$a ^ $b") == ["^"]

    def test_16_39_logical_or(self):
        """parseExpr-16.39: Logical OR."""
        assert ops("$a || $b") == ["||"]

    def test_16_40_bitwise_or(self):
        """parseExpr-16.40: Bitwise OR."""
        assert ops("$a | $b") == ["|"]

    def test_16_41_bitwise_not(self):
        """parseExpr-16.41: Bitwise NOT (~)."""
        assert ops("~$a") == ["~"]

    def test_16_42_function_name(self):
        """parseExpr-16.42: Function name recognition."""
        tokens = toks("sin(0)")
        assert tokens[0].type == ExprTokenType.FUNCTION
        assert tokens[0].text == "sin"

    def test_16_43_unknown_function(self):
        """parseExpr-16.43: Unknown function name."""
        tokens = toks("myfunc(0)")
        assert tokens[0].type == ExprTokenType.FUNCTION
        assert tokens[0].text == "myfunc"


# Group 17: Token array expansion (parseExpr-17.x)


class TestTokenExpansion:
    """parseExpr-17.x: Many tokens force expansion."""

    def test_17_1_many_subexpressions(self):
        """parseExpr-17.1: Many repeated subexpressions."""
        parts = " + ".join(f"$v{i}" for i in range(30))
        tokens = toks(parts)
        var_tokens = [t for t in tokens if t.type == ExprTokenType.VARIABLE]
        assert len(var_tokens) == 30
        op_tokens = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert len(op_tokens) == 29


# Group 19: Integer parsing edge cases (parseExpr-19.x, 20.x)


class TestIntegerParsing:
    """parseExpr-19.x/20.x: Integer parsing."""

    def test_19_1_hex_prefix(self):
        """parseExpr-19.1: 0x prefix."""
        tokens = toks("0xFF")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "0xFF"

    def test_20_1_large_18_digit(self):
        """parseExpr-20.1: 18-digit integer."""
        tokens = toks("123456789012345678")
        assert tokens[0].type == ExprTokenType.NUMBER

    def test_20_2_large_32_digit(self):
        """parseExpr-20.2: 32-digit integer."""
        num = "1" * 32
        tokens = toks(num)
        assert tokens[0].type == ExprTokenType.NUMBER

    def test_octal_prefix(self):
        """Octal 0o prefix."""
        tokens = toks("0o77")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "0o77"

    def test_binary_prefix(self):
        """Binary 0b prefix."""
        tokens = toks("0b1010")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "0b1010"

    def test_hex_uppercase(self):
        """Hex with uppercase prefix."""
        tokens = toks("0XFF")
        assert tokens[0].type == ExprTokenType.NUMBER

    def test_scientific_with_sign(self):
        """Scientific notation with +/- sign."""
        tokens = toks("1.5e+10")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "1.5e+10"

    def test_scientific_negative_exp(self):
        """Scientific notation with negative exponent."""
        tokens = toks("1.5e-10")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "1.5e-10"


# Group 21: Error messages (parseExpr-21.x) -- adapted for no-crash


class TestExprErrors:
    """parseExpr-21.x: Error handling -- verify no crashes."""

    def test_21_9_missing_quote(self):
        """parseExpr-21.9: Missing close-quote."""
        tokens = toks('"hello')
        assert len(tokens) >= 1

    def test_21_10_missing_close_brace(self):
        """parseExpr-21.10: Missing close-brace."""
        tokens = toks("{hello")
        assert len(tokens) >= 1

    def test_21_14_missing_close_bracket(self):
        """parseExpr-21.14: Missing close-bracket."""
        tokens = toks("[cmd")
        assert len(tokens) >= 1

    def test_21_16_empty_parens(self):
        """parseExpr-21.16: Empty parenthesized subexpression."""
        tokens = toks("()")
        assert ExprTokenType.PAREN_OPEN in [t.type for t in tokens]
        assert ExprTokenType.PAREN_CLOSE in [t.type for t in tokens]

    def test_21_19_empty_expression(self):
        """parseExpr-21.19: Empty expression."""
        tokens = toks("")
        assert tokens == []

    def test_21_invalid_character(self):
        """Invalid character in expression."""
        tokens = toks("@")
        # Should not crash, may produce no tokens (skipped) or function token
        assert isinstance(tokens, list)


# Group 22: Bug fixes (parseExpr-22.x)


class TestExprBugFixes:
    """parseExpr-22.x: Bug fixes."""

    def test_22_14_legacy_octal(self):
        """parseExpr-22.14: Legacy octal 07."""
        tokens = toks("07")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == "07"

    def test_leading_dot_number(self):
        """Number starting with decimal point."""
        tokens = toks(".5")
        assert tokens[0].type == ExprTokenType.NUMBER
        assert tokens[0].text == ".5"

    def test_leading_dot_plus(self):
        """.5 + .5 expression."""
        tokens = toks(".5 + .5")
        numbers = [t for t in tokens if t.type == ExprTokenType.NUMBER]
        assert len(numbers) == 2

    def test_power_operator(self):
        """** exponentiation operator."""
        assert ops("2 ** 8") == ["**"]

    def test_string_eq_operator(self):
        """eq string equality operator."""
        assert ops('"a" eq "b"') == ["eq"]

    def test_string_ne_operator(self):
        """ne string inequality operator."""
        assert ops('"a" ne "b"') == ["ne"]

    def test_in_operator(self):
        """in list membership operator."""
        assert ops('"x" in {a b c}') == ["in"]

    def test_ni_operator(self):
        """ni list non-membership operator."""
        assert ops('"x" ni {a b c}') == ["ni"]


# Boolean literals


class TestBooleanLiterals:
    """Boolean literal recognition."""

    def test_true(self):
        tokens = toks("true")
        assert tokens[0].type == ExprTokenType.BOOL
        assert tokens[0].text == "true"

    def test_false(self):
        tokens = toks("false")
        assert tokens[0].type == ExprTokenType.BOOL
        assert tokens[0].text == "false"

    def test_yes(self):
        tokens = toks("yes")
        assert tokens[0].type == ExprTokenType.BOOL

    def test_no(self):
        tokens = toks("no")
        assert tokens[0].type == ExprTokenType.BOOL

    def test_bool_in_expr(self):
        tokens = toks("true && false")
        bools = [t for t in tokens if t.type == ExprTokenType.BOOL]
        assert len(bools) == 2


# Math functions


class TestMathFunctions:
    """Math function recognition from parseExpr-15.22+."""

    @pytest.mark.parametrize(
        "func",
        [
            "abs",
            "acos",
            "asin",
            "atan",
            "atan2",
            "bool",
            "ceil",
            "cos",
            "cosh",
            "double",
            "entier",
            "exp",
            "floor",
            "fmod",
            "hypot",
            "int",
            "isqrt",
            "log",
            "log10",
            "max",
            "min",
            "pow",
            "rand",
            "round",
            "sin",
            "sinh",
            "sqrt",
            "srand",
            "tan",
            "tanh",
            "wide",
        ],
    )
    def test_known_function(self, func):
        """All known math functions are recognized."""
        tokens = toks(f"{func}(0)")
        assert tokens[0].type == ExprTokenType.FUNCTION
        assert tokens[0].text == func

    def test_nested_functions(self):
        """Nested function calls."""
        tokens = toks("abs(sin(cos($x)))")
        func_tokens = [t for t in tokens if t.type == ExprTokenType.FUNCTION]
        assert [f.text for f in func_tokens] == ["abs", "sin", "cos"]


# Complex expressions


class TestComplexExpressions:
    """Complex real-world expressions combining multiple features."""

    def test_arithmetic_with_functions(self):
        tokens = toks("($x + 1) * sin($y) > 0.5")
        types = {t.type for t in tokens}
        assert ExprTokenType.VARIABLE in types
        assert ExprTokenType.NUMBER in types
        assert ExprTokenType.OPERATOR in types
        assert ExprTokenType.FUNCTION in types
        assert ExprTokenType.PAREN_OPEN in types
        assert ExprTokenType.PAREN_CLOSE in types

    def test_bitwise_with_shifts(self):
        _ = toks("($mask & 0xFF) | ($flags << 8)")
        assert "&" in ops("($mask & 0xFF) | ($flags << 8)")
        assert "|" in ops("($mask & 0xFF) | ($flags << 8)")
        assert "<<" in ops("($mask & 0xFF) | ($flags << 8)")

    def test_string_comparison_chain(self):
        _ = toks('"foo" eq "bar" || "baz" ne "qux"')
        assert "eq" in ops('"foo" eq "bar" || "baz" ne "qux"')
        assert "ne" in ops('"foo" eq "bar" || "baz" ne "qux"')
        assert "||" in ops('"foo" eq "bar" || "baz" ne "qux"')

    def test_complex_ternary(self):
        expr = "$x > 0 ? ($x > 100 ? 100 : $x) : 0"
        tokens = toks(expr)
        qs = [t for t in tokens if t.type == ExprTokenType.TERNARY_Q]
        cs = [t for t in tokens if t.type == ExprTokenType.TERNARY_C]
        assert len(qs) == 2
        assert len(cs) == 2

    def test_wide_with_hex(self):
        _ = toks("wide(0xDEADBEEF)")
        assert "wide" in funcs("wide(0xDEADBEEF)")
        assert "0xDEADBEEF" in nums("wide(0xDEADBEEF)")

    def test_chained_comparison(self):
        expr = "$x > 0 && $x < 100"
        assert ops(expr) == [">", "&&", "<"]

    def test_nested_function_calls(self):
        expr = "int(ceil(sqrt($n)))"
        assert funcs(expr) == ["int", "ceil", "sqrt"]

    def test_empty_expression(self):
        assert toks("") == []

    def test_whitespace_only(self):
        assert toks("   \t  \n  ") == []
