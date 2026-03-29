#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/lsp/definition.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

namespace {

auto analyse_source(std::string_view source) -> AnalysisResult {
    Analyser analyser;
    return analyser.analyse(source);
}

} // anonymous namespace

TEST_CASE("definition: proc name", "[definition]") {
    auto source = "proc foo {} {}\nfoo";
    auto analysis = analyse_source(source);
    auto result = find_definition(source, 1, 0, analysis);
    REQUIRE(result.has_value());
    // Should point to the proc definition.
    REQUIRE(result->start.line == 0);
}

TEST_CASE("definition: variable", "[definition]") {
    auto source = "set myvar 42\nputs $myvar";
    auto analysis = analyse_source(source);
    // Position on $myvar (line 1, col 6 = 'm' of myvar after $).
    auto result = find_definition(source, 1, 6, analysis);
    REQUIRE(result.has_value());
    // Should point to the set definition.
    REQUIRE(result->start.line == 0);
}

TEST_CASE("definition: not found", "[definition]") {
    auto source = "puts hello";
    auto analysis = analyse_source(source);
    auto result = find_definition(source, 0, 5, analysis);
    // 'hello' is not a defined variable or proc.
    REQUIRE_FALSE(result.has_value());
}

TEST_CASE("references: variable", "[references]") {
    auto source = "set x 1\nputs $x\nset y $x";
    auto analysis = analyse_source(source);
    // Cursor on '$x' at line 1, col 6 (the 'x' in '$x').
    auto refs = find_references(source, 1, 6, analysis);
    // Should find definition + at least one reference.
    REQUIRE(refs.size() >= 1);
}

TEST_CASE("references: proc", "[references]") {
    auto source = "proc foo {} {}\nfoo\nfoo";
    auto analysis = analyse_source(source);
    auto refs = find_references(source, 0, 5, analysis);
    // Should find the definition and call sites.
    REQUIRE(refs.size() >= 1);
}
