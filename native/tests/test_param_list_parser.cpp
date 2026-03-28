#include "tcl_lsp/analysis/param_list_parser.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("parse_param_list: simple params", "[param_list_parser]") {
    auto params = parse_param_list("a b c");
    REQUIRE(params.size() == 3);
    REQUIRE(params[0].name == "a");
    REQUIRE(params[1].name == "b");
    REQUIRE(params[2].name == "c");
    for (const auto& p : params) {
        REQUIRE(p.has_default == false);
        REQUIRE(p.default_value.empty());
    }
}

TEST_CASE("parse_param_list: param with default", "[param_list_parser]") {
    auto params = parse_param_list("{name World}");
    REQUIRE(params.size() == 1);
    REQUIRE(params[0].name == "name");
    REQUIRE(params[0].has_default == true);
    REQUIRE(params[0].default_value == "World");
}

TEST_CASE("parse_param_list: mixed params", "[param_list_parser]") {
    auto params = parse_param_list("a {b 42} c");
    REQUIRE(params.size() == 3);
    REQUIRE(params[0].name == "a");
    REQUIRE(params[0].has_default == false);
    REQUIRE(params[1].name == "b");
    REQUIRE(params[1].has_default == true);
    REQUIRE(params[1].default_value == "42");
    REQUIRE(params[2].name == "c");
    REQUIRE(params[2].has_default == false);
}

TEST_CASE("parse_param_list: empty string", "[param_list_parser]") {
    auto params = parse_param_list("");
    REQUIRE(params.empty());
}

TEST_CASE("parse_param_list: whitespace only", "[param_list_parser]") {
    auto params = parse_param_list("   \t\n  ");
    REQUIRE(params.empty());
}

TEST_CASE("parse_param_list: braced param without default", "[param_list_parser]") {
    auto params = parse_param_list("{x}");
    REQUIRE(params.size() == 1);
    REQUIRE(params[0].name == "x");
    REQUIRE(params[0].has_default == false);
}

TEST_CASE("parse_param_list: empty default value", "[param_list_parser]") {
    auto params = parse_param_list("{a {}}");
    REQUIRE(params.size() == 1);
    REQUIRE(params[0].name == "a");
    REQUIRE(params[0].has_default == true);
    REQUIRE(params[0].default_value == "{}");
}

TEST_CASE("parse_param_list: args parameter", "[param_list_parser]") {
    auto params = parse_param_list("x y args");
    REQUIRE(params.size() == 3);
    REQUIRE(params[2].name == "args");
    REQUIRE(params[2].has_default == false);
}

TEST_CASE("parse_param_list: leading and trailing whitespace", "[param_list_parser]") {
    auto params = parse_param_list("  x  y  ");
    REQUIRE(params.size() == 2);
    REQUIRE(params[0].name == "x");
    REQUIRE(params[1].name == "y");
}

TEST_CASE("parse_param_list: tab-separated", "[param_list_parser]") {
    auto params = parse_param_list("a\tb\tc");
    REQUIRE(params.size() == 3);
}

TEST_CASE("parse_param_list: newline-separated", "[param_list_parser]") {
    auto params = parse_param_list("a\nb\nc");
    REQUIRE(params.size() == 3);
}

TEST_CASE("parse_param_list: multiple defaults", "[param_list_parser]") {
    auto params = parse_param_list("{a 1} {b 2} {c 3}");
    REQUIRE(params.size() == 3);
    for (int i = 0; i < 3; ++i) {
        REQUIRE(params[static_cast<std::size_t>(i)].has_default == true);
    }
    REQUIRE(params[0].default_value == "1");
    REQUIRE(params[1].default_value == "2");
    REQUIRE(params[2].default_value == "3");
}
