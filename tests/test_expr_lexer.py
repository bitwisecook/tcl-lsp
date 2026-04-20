"""Tests for the expression sub-lexer."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.expr_lexer import ExprTokenType, tokenise_expr


class TestNumbers:
    def test_integer(self):
        tokens = tokenise_expr("42")
        nums = [t for t in tokens if t.type == ExprTokenType.NUMBER]
        assert len(nums) == 1
        assert nums[0].text == "42"

    def test_float(self):
        tokens = tokenise_expr("3.14")
        nums = [t for t in tokens if t.type == ExprTokenType.NUMBER]
        assert len(nums) == 1
        assert nums[0].text == "3.14"

    def test_scientific(self):
        tokens = tokenise_expr("1.5e10")
        nums = [t for t in tokens if t.type == ExprTokenType.NUMBER]
        assert len(nums) == 1
        assert nums[0].text == "1.5e10"

    def test_hex(self):
        tokens = tokenise_expr("0xFF")
        nums = [t for t in tokens if t.type == ExprTokenType.NUMBER]
        assert len(nums) == 1
        assert nums[0].text == "0xFF"

    def test_octal(self):
        tokens = tokenise_expr("0o77")
        nums = [t for t in tokens if t.type == ExprTokenType.NUMBER]
        assert len(nums) == 1
        assert nums[0].text == "0o77"


class TestOperators:
    def test_arithmetic(self):
        tokens = tokenise_expr("1 + 2 * 3")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert [o.text for o in ops] == ["+", "*"]

    def test_comparison(self):
        tokens = tokenise_expr("$x <= 10")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "<="

    def test_logical(self):
        tokens = tokenise_expr("$a && $b || !$c")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert [o.text for o in ops] == ["&&", "||", "!"]

    def test_string_operators(self):
        tokens = tokenise_expr('"hello" eq "world"')
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "eq"

    def test_power(self):
        tokens = tokenise_expr("2 ** 8")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "**"


class TestVariables:
    def test_simple_var(self):
        tokens = tokenise_expr("$x + 1")
        vars_ = [t for t in tokens if t.type == ExprTokenType.VARIABLE]
        assert len(vars_) == 1
        assert vars_[0].text == "$x"

    def test_namespace_var(self):
        tokens = tokenise_expr("$ns::val")
        vars_ = [t for t in tokens if t.type == ExprTokenType.VARIABLE]
        assert vars_[0].text == "$ns::val"

    def test_braced_var(self):
        tokens = tokenise_expr("${my var}")
        vars_ = [t for t in tokens if t.type == ExprTokenType.VARIABLE]
        assert vars_[0].text == "${my var}"

    def test_array_var(self):
        tokens = tokenise_expr("$arr(key)")
        vars_ = [t for t in tokens if t.type == ExprTokenType.VARIABLE]
        assert vars_[0].text == "$arr(key)"


class TestFunctions:
    def test_math_function(self):
        tokens = tokenise_expr("sin(3.14)")
        funcs = [t for t in tokens if t.type == ExprTokenType.FUNCTION]
        assert len(funcs) == 1
        assert funcs[0].text == "sin"

    def test_multi_arg_function(self):
        tokens = tokenise_expr("pow(2, 8)")
        funcs = [t for t in tokens if t.type == ExprTokenType.FUNCTION]
        assert funcs[0].text == "pow"
        commas = [t for t in tokens if t.type == ExprTokenType.COMMA]
        assert len(commas) == 1

    def test_nested_functions(self):
        tokens = tokenise_expr("abs(sin($x))")
        funcs = [t for t in tokens if t.type == ExprTokenType.FUNCTION]
        assert [f.text for f in funcs] == ["abs", "sin"]


class TestBooleans:
    def test_true_false(self):
        tokens = tokenise_expr("true && false")
        bools = [t for t in tokens if t.type == ExprTokenType.BOOL]
        assert [b.text for b in bools] == ["true", "false"]


class TestTernary:
    def test_ternary(self):
        tokens = tokenise_expr("$x > 0 ? 1 : -1")
        assert any(t.type == ExprTokenType.TERNARY_Q for t in tokens)
        assert any(t.type == ExprTokenType.TERNARY_C for t in tokens)


class TestCommandSubstitution:
    def test_command_in_expr(self):
        tokens = tokenise_expr("[llength $list] > 0")
        cmds = [t for t in tokens if t.type == ExprTokenType.COMMAND]
        assert len(cmds) == 1
        assert "[llength $list]" in cmds[0].text


class TestStrings:
    def test_quoted_string(self):
        tokens = tokenise_expr('"hello"')
        strs = [t for t in tokens if t.type == ExprTokenType.STRING]
        assert len(strs) == 1
        assert strs[0].text == '"hello"'

    def test_escaped_quote(self):
        tokens = tokenise_expr(r'"say \"hi\""')
        strs = [t for t in tokens if t.type == ExprTokenType.STRING]
        assert len(strs) == 1


class TestComplexExpressions:
    def test_full_expression(self):
        tokens = tokenise_expr("($x + 1) * sin($y) > 0.5")
        # Should have parens, operators, numbers, vars, functions
        types_found = {t.type for t in tokens}
        assert ExprTokenType.PAREN_OPEN in types_found
        assert ExprTokenType.PAREN_CLOSE in types_found
        assert ExprTokenType.OPERATOR in types_found
        assert ExprTokenType.NUMBER in types_found
        assert ExprTokenType.VARIABLE in types_found
        assert ExprTokenType.FUNCTION in types_found

    def test_empty_expression(self):
        tokens = tokenise_expr("")
        assert tokens == []


class TestIRulesOperators:
    """iRules-specific expression operators, gated on dialect."""

    def test_contains_is_operator_in_irules(self):
        tokens = tokenise_expr('$uri contains "/api"', dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert len(ops) == 1
        assert ops[0].text == "contains"

    def test_contains_is_function_in_standard_tcl(self):
        tokens = tokenise_expr('$uri contains "/api"')
        funcs = [t for t in tokens if t.type == ExprTokenType.FUNCTION]
        assert any(f.text == "contains" for f in funcs)
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert not any(o.text == "contains" for o in ops)

    def test_starts_with(self):
        tokens = tokenise_expr('$path starts_with "/v2"', dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "starts_with"

    def test_ends_with(self):
        tokens = tokenise_expr('$host ends_with ".com"', dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "ends_with"

    def test_equals(self):
        tokens = tokenise_expr('$method equals "GET"', dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "equals"

    def test_matches_glob(self):
        tokens = tokenise_expr('$path matches_glob "/api/*"', dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "matches_glob"

    def test_matches_regex(self):
        tokens = tokenise_expr('$uri matches_regex "^/v[0-9]+"', dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "matches_regex"

    def test_word_and(self):
        tokens = tokenise_expr("$a and $b", dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "and"

    def test_word_or(self):
        tokens = tokenise_expr("$a or $b", dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "or"

    def test_word_not(self):
        tokens = tokenise_expr("not $flag", dialect="f5-irules")
        ops = [t for t in tokens if t.type == ExprTokenType.OPERATOR]
        assert ops[0].text == "not"

    def test_and_is_function_without_dialect(self):
        tokens = tokenise_expr("$a and $b")
        funcs = [t for t in tokens if t.type == ExprTokenType.FUNCTION]
        assert any(f.text == "and" for f in funcs)

    def test_boolean_not_shadowed_by_irules(self):
        """true/false remain BOOL even in iRules dialect."""
        tokens = tokenise_expr("true and false", dialect="f5-irules")
        bools = [t for t in tokens if t.type == ExprTokenType.BOOL]
        assert len(bools) == 2
