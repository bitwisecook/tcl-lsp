// Ported from tests/test_proc_arg_traits.py — shallow trait inference.

#include "tcl_lsp/analysis/proc_arg_traits.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <vector>

using namespace tcl_lsp;

// Helper: infer traits for a single-param proc body.
static auto traits_for(const std::string& param, const std::string& body) -> ProcArgTraits {
    std::vector<std::string> params{param};
    auto result = infer_param_traits_shallow(params, body);
    auto it = result.find(param);
    if (it == result.end())
        return 0;
    return it->second;
}

TEST_CASE("eval $param -> EVAL", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("script", "eval $script"), ProcArgTrait::EVAL));
}

TEST_CASE("uplevel 1 $param -> EVAL", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("body", "uplevel 1 $body"), ProcArgTrait::EVAL));
}

TEST_CASE("subst $param -> EVAL", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("tmpl", "subst $tmpl"), ProcArgTrait::EVAL));
}

TEST_CASE("upvar 1 $param local -> VAR_READ", "[proc_arg_traits]") {
    auto t = traits_for("varName", "upvar 1 $varName local");
    CHECK(has_trait(t, ProcArgTrait::VAR_READ));
}

TEST_CASE("upvar + set through alias -> VAR_WRITE", "[proc_arg_traits]") {
    auto t = traits_for("varName", "upvar 1 $varName local\nset local value");
    CHECK(has_trait(t, ProcArgTrait::VAR_WRITE));
}

TEST_CASE("foreach x $list body -> LOOP_LIST", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("items", "foreach x $items { puts $x }"), ProcArgTrait::LOOP_LIST));
}

TEST_CASE("while $cond $body -> EXPR + BODY", "[proc_arg_traits]") {
    std::vector<std::string> params{"cond", "body"};
    auto result = infer_param_traits_shallow(params, "while $cond $body");
    CHECK(has_trait(result["cond"], ProcArgTrait::EXPR));
    CHECK(has_trait(result["body"], ProcArgTrait::BODY));
}

TEST_CASE("for $init $cond $next $body -> BODY + EXPR", "[proc_arg_traits]") {
    std::vector<std::string> params{"init", "cond", "next", "body"};
    auto result = infer_param_traits_shallow(params, "for $init $cond $next $body");
    CHECK(has_trait(result["init"], ProcArgTrait::BODY));
    CHECK(has_trait(result["cond"], ProcArgTrait::EXPR));
    CHECK(has_trait(result["next"], ProcArgTrait::BODY));
    CHECK(has_trait(result["body"], ProcArgTrait::BODY));
}

TEST_CASE("after ms $script -> EVAL", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("cb", "after 1000 $cb"), ProcArgTrait::EVAL));
}

TEST_CASE("scan $str $fmt $var -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("result", "scan $str %d $result"), ProcArgTrait::VAR_WRITE));
}

TEST_CASE("set $param value -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("v", "set $v 42"), ProcArgTrait::VAR_WRITE));
}

TEST_CASE("no traits for unused param", "[proc_arg_traits]") {
    std::vector<std::string> params{"x", "y"};
    auto result = infer_param_traits_shallow(params, "puts hello");
    CHECK(result.empty());
}

TEST_CASE("empty body -> no traits", "[proc_arg_traits]") {
    std::vector<std::string> params{"x"};
    auto result = infer_param_traits_shallow(params, "   ");
    CHECK(result.empty());
}

TEST_CASE("after idle $script -> EVAL", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("cb", "after idle $cb"), ProcArgTrait::EVAL));
}

TEST_CASE("after cancel $id -> no trait", "[proc_arg_traits]") {
    std::vector<std::string> params{"id"};
    auto result = infer_param_traits_shallow(params, "after cancel $id");
    CHECK(result.empty());
}

TEST_CASE("incr $counter -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("counter", "incr $counter"), ProcArgTrait::VAR_WRITE));
}

TEST_CASE("lassign $data $first $second -> VAR_WRITE", "[proc_arg_traits]") {
    std::vector<std::string> params{"first", "second"};
    auto result = infer_param_traits_shallow(params, "lassign {a b} $first $second");
    CHECK(has_trait(result["first"], ProcArgTrait::VAR_WRITE));
    CHECK(has_trait(result["second"], ProcArgTrait::VAR_WRITE));
}

TEST_CASE("regexp match vars -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("m", R"(regexp {(\w+)} $str $m)"), ProcArgTrait::VAR_WRITE));
}

TEST_CASE("regexp with options match vars -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("m", R"(regexp -nocase -- {(\w+)} $str $m)"),
                    ProcArgTrait::VAR_WRITE));
}

TEST_CASE("regsub var -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("out", R"(regsub {old} $str new $out)"), ProcArgTrait::VAR_WRITE));
}

TEST_CASE("regsub with options var -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(has_trait(traits_for("out", R"(regsub -all -- {old} $str new $out)"),
                    ProcArgTrait::VAR_WRITE));
}

TEST_CASE("binary scan -> VAR_WRITE", "[proc_arg_traits]") {
    CHECK(
        has_trait(traits_for("intVar", R"(binary scan $data I $intVar)"), ProcArgTrait::VAR_WRITE));
}

TEST_CASE("no params -> empty map", "[proc_arg_traits]") {
    std::vector<std::string> params;
    auto result = infer_param_traits_shallow(params, "set x 1");
    CHECK(result.empty());
}
