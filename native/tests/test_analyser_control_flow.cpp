// Ported from tests/test_analyser.py — TestControlFlow.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("if body analysed", "[analyser][control]") {
    auto result = analyse("if {1} { set x 10 }");
    REQUIRE(result.global_scope().variables.contains("x"));
}

TEST_CASE("while body analysed", "[analyser][control]") {
    auto result = analyse("while {1} { set x 42 }");
    REQUIRE(result.global_scope().variables.contains("x"));
}

TEST_CASE("for body analysed", "[analyser][control]") {
    auto result = analyse("for {set i 0} {$i < 10} {incr i} { set x $i }");
    REQUIRE(result.global_scope().variables.contains("i"));
    REQUIRE(result.global_scope().variables.contains("x"));
}

TEST_CASE("if expr records variable reference", "[analyser][control]") {
    auto result = analyse("set x 1\nif {$x} { set y 1 }");
    auto it = result.global_scope().variables.find("x");
    REQUIRE(it != result.global_scope().variables.end());
    // x should have at least one reference (from the expr)
    REQUIRE(it->second.references.size() >= 1);
}

TEST_CASE("foreach defines variables", "[analyser][control]") {
    auto result = analyse("foreach item {a b c} { puts $item }");
    REQUIRE(result.global_scope().variables.contains("item"));
}

TEST_CASE("nested proc and if", "[analyser][control]") {
    auto result = analyse("proc test {flag} {\n  if {$flag} {\n    set x 1\n  }\n}");
    REQUIRE(result.find_proc("test") != nullptr);
    auto& children = result.global_scope().children;
    REQUIRE(children.size() == 1);
    auto* proc_scope = children[0].get();
    REQUIRE(proc_scope->variables.contains("flag"));
    REQUIRE(proc_scope->variables.contains("x"));
}

TEST_CASE("if/elseif/else all bodies analysed", "[analyser][control]") {
    auto src = R"(
if {1} {
    set x 10
} elseif {0} {
    set y 20
} else {
    set z 30
}
)";
    auto result = analyse(src);
    REQUIRE(result.global_scope().variables.contains("x"));
    REQUIRE(result.global_scope().variables.contains("y"));
    REQUIRE(result.global_scope().variables.contains("z"));
}

TEST_CASE("dict for body analysed", "[analyser][control]") {
    auto result = analyse("dict for {k v} $d { set result $k }");
    REQUIRE(result.global_scope().variables.contains("k"));
    REQUIRE(result.global_scope().variables.contains("v"));
    REQUIRE(result.global_scope().variables.contains("result"));
}

TEST_CASE("command subst inside expr analysed", "[analyser][control]") {
    auto result = analyse("if {[set x 1]} {}");
    REQUIRE(result.global_scope().variables.contains("x"));
}

TEST_CASE("catch body and result var", "[analyser][control]") {
    auto result = analyse("catch { set x 1 } result");
    REQUIRE(result.global_scope().variables.contains("x"));
    REQUIRE(result.global_scope().variables.contains("result"));
}

TEST_CASE("try with finally", "[analyser][control]") {
    auto result = analyse("try { set x 1 } finally { set y 2 }");
    REQUIRE(result.global_scope().variables.contains("x"));
    REQUIRE(result.global_scope().variables.contains("y"));
}

TEST_CASE("switch body forms", "[analyser][control]") {
    // Form 2: braced body
    auto result = analyse(R"(switch $val {
        a { set x 1 }
        b { set y 2 }
    })");
    REQUIRE(result.global_scope().variables.contains("x"));
    REQUIRE(result.global_scope().variables.contains("y"));
}
