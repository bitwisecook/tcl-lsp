"""Tests for proc argument trait inference."""

from core.analysis.proc_arg_traits import (
    infer_param_traits,
    infer_param_traits_deep,
    merge_traits,
)
from core.analysis.semantic_model import ProcArgTrait


class TestEvalTrait:
    def test_eval_simple(self):
        traits = infer_param_traits(("script",), "eval $script")
        assert ProcArgTrait.EVAL in traits["script"]

    def test_uplevel_last_arg(self):
        traits = infer_param_traits(("script",), "uplevel 1 $script")
        assert ProcArgTrait.EVAL in traits["script"]

    def test_subst(self):
        traits = infer_param_traits(("template",), "subst $template")
        assert ProcArgTrait.EVAL in traits["template"]


class TestBodyTrait:
    def test_param_as_foreach_body_arg(self):
        """When $body is passed directly as the foreach body argument."""
        body = "foreach item $items $body"
        traits = infer_param_traits(("items", "body"), body)
        assert ProcArgTrait.BODY in traits.get("body", frozenset())


class TestVarWriteTrait:
    def test_upvar_write(self):
        body = "upvar 1 $varName var\nset var 42"
        traits = infer_param_traits(("varName",), body)
        assert ProcArgTrait.VAR_WRITE in traits["varName"]

    def test_set_with_param_as_varname(self):
        body = "set $varName value"
        traits = infer_param_traits(("varName",), body)
        assert ProcArgTrait.VAR_WRITE in traits["varName"]

    def test_incr_with_param_as_varname(self):
        body = "incr $counter"
        traits = infer_param_traits(("counter",), body)
        assert ProcArgTrait.VAR_WRITE in traits["counter"]

    def test_upvar_alias_write(self):
        body = "upvar 1 $varName localVar\nset localVar 42"
        traits = infer_param_traits(("varName",), body)
        assert ProcArgTrait.VAR_WRITE in traits["varName"]


class TestLoopListTrait:
    def test_foreach_list(self):
        body = "foreach item $items {\n    puts $item\n}"
        traits = infer_param_traits(("items",), body)
        assert ProcArgTrait.LOOP_LIST in traits["items"]


class TestWhileForTraits:
    def test_while_body(self):
        body = "while {$cond} $body"
        traits = infer_param_traits(("cond", "body"), body)
        assert ProcArgTrait.EXPR in traits.get("cond", frozenset())
        assert ProcArgTrait.BODY in traits.get("body", frozenset())

    def test_while_braced_condition(self):
        """When condition is braced literal, only body param gets trait."""
        body = "while {1} $loopBody"
        traits = infer_param_traits(("loopBody",), body)
        assert ProcArgTrait.BODY in traits["loopBody"]

    def test_for_all_parts(self):
        body = "for $init $cond $next $body"
        traits = infer_param_traits(("init", "cond", "next", "body"), body)
        assert ProcArgTrait.BODY in traits.get("init", frozenset())
        assert ProcArgTrait.EXPR in traits.get("cond", frozenset())
        assert ProcArgTrait.BODY in traits.get("next", frozenset())
        assert ProcArgTrait.BODY in traits.get("body", frozenset())

    def test_for_body_only(self):
        body = "for {set i 0} {$i < 10} {incr i} $loopBody"
        traits = infer_param_traits(("loopBody",), body)
        assert ProcArgTrait.BODY in traits["loopBody"]


class TestCombinedTraits:
    def test_foreach_in_collection_pattern(self):
        """The classic EDA pattern: varName for upvar, collection for loop, body for eval."""
        body = "upvar 1 $varName var\nforeach var $collection $body\n"
        traits = infer_param_traits(("varName", "collection", "body"), body)
        assert ProcArgTrait.VAR_WRITE in traits.get("varName", frozenset())
        assert ProcArgTrait.LOOP_LIST in traits.get("collection", frozenset())
        assert ProcArgTrait.BODY in traits.get("body", frozenset())

    def test_no_traits_for_unused_param(self):
        traits = infer_param_traits(("a", "b"), "puts hello")
        assert "a" not in traits
        assert "b" not in traits

    def test_empty_body(self):
        traits = infer_param_traits(("x",), "")
        assert traits == {}

    def test_no_params(self):
        traits = infer_param_traits((), "set x 1")
        assert traits == {}


class TestDeepAnalysis:
    def test_eval_inside_foreach_body(self):
        """Deep analysis catches $body usage inside a braced foreach body."""
        body = "foreach item $items {\n    uplevel 1 $body\n}"
        shallow = infer_param_traits(("items", "body"), body)
        # Shallow only sees $items as LOOP_LIST, not $body
        assert ProcArgTrait.LOOP_LIST in shallow.get("items", frozenset())
        assert "body" not in shallow

        deep = infer_param_traits_deep(("items", "body"), body)
        # Deep catches $body EVAL inside the braced body
        assert ProcArgTrait.EVAL in deep.get("body", frozenset())

    def test_merge_shallow_and_deep(self):
        body = "foreach item $items {\n    uplevel 1 $body\n}"
        shallow = infer_param_traits(("items", "body"), body)
        deep = infer_param_traits_deep(("items", "body"), body)
        merged = merge_traits(shallow, deep)
        assert ProcArgTrait.LOOP_LIST in merged.get("items", frozenset())
        assert ProcArgTrait.EVAL in merged.get("body", frozenset())

    def test_nested_if_body(self):
        body = "if {$cond} {\n    set $varName value\n}"
        deep = infer_param_traits_deep(("cond", "varName"), body)
        assert ProcArgTrait.VAR_WRITE in deep.get("varName", frozenset())

    def test_max_depth_guard(self):
        """Deep analysis should not crash on deeply nested bodies."""
        body = "if {1} { " * 20 + "eval $x" + " }" * 20
        # Should not raise, even though nesting is deep
        infer_param_traits_deep(("x",), body)
        # Should not raise regardless of depth
