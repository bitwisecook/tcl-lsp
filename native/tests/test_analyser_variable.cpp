// Ported from tests/test_analyser.py — TestVariableAnalysis.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("set defines variable", "[analyser][variable]") {
    auto result = analyse("set x 10");
    REQUIRE(result.global_scope().variables.contains("x"));
}

TEST_CASE("multiple sets", "[analyser][variable]") {
    auto result = analyse("set x 10\nset y 20");
    REQUIRE(result.global_scope().variables.contains("x"));
    REQUIRE(result.global_scope().variables.contains("y"));
}

TEST_CASE("incr defines variable", "[analyser][variable]") {
    auto result = analyse("incr counter");
    REQUIRE(result.global_scope().variables.contains("counter"));
}

TEST_CASE("array variable uses base name", "[analyser][variable]") {
    auto result = analyse("set arr(key) value");
    REQUIRE(result.global_scope().variables.contains("arr"));
    REQUIRE_FALSE(result.global_scope().variables.contains("arr(key)"));
}

TEST_CASE("var in proc scope not in global", "[analyser][variable]") {
    auto result = analyse("proc test {} { set local 42 }");
    REQUIRE_FALSE(result.global_scope().variables.contains("local"));
    auto& children = result.global_scope().children;
    REQUIRE(children.size() == 1);
    REQUIRE(children[0]->variables.contains("local"));
}

TEST_CASE("set with one arg is read-only", "[analyser][variable]") {
    auto result = analyse("set x");
    // One-arg set is a read, not a definition.
    REQUIRE_FALSE(result.global_scope().variables.contains("x"));
}

TEST_CASE("variable command alternating pairs", "[analyser][variable]") {
    // 'variable port 8080' — port is defined, 8080 is the value (not a variable)
    auto result = analyse("variable port 8080");
    REQUIRE(result.global_scope().variables.contains("port"));
    REQUIRE_FALSE(result.global_scope().variables.contains("8080"));
}

TEST_CASE("global command defines variables", "[analyser][variable]") {
    auto result = analyse("proc test {} { global gvar\nset gvar 10 }");
    auto& children = result.global_scope().children;
    REQUIRE(children.size() == 1);
    REQUIRE(children[0]->variables.contains("gvar"));
}
