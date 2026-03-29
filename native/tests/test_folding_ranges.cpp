#include <catch2/catch_test_macros.hpp>

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/lsp/folding_ranges.hpp"

using namespace tcl_lsp;

namespace {

auto analyse_source(std::string_view source) -> AnalysisResult {
    Analyser analyser;
    return analyser.analyse(source);
}

auto has_fold(const std::vector<FoldingRangeInfo>& ranges,
              int start, int end, FoldingRangeKind kind) -> bool {
    for (auto& r : ranges) {
        if (r.start_line == start && r.end_line == end && r.kind == kind)
            return true;
    }
    return false;
}

} // anonymous namespace

TEST_CASE("folding: empty source", "[folding]") {
    auto ranges = get_folding_ranges("");
    REQUIRE(ranges.empty());
}

TEST_CASE("folding: comment block", "[folding]") {
    auto source =
        "# line 1\n"
        "# line 2\n"
        "# line 3\n"
        "set x 1\n";
    auto ranges = get_folding_ranges(source);
    REQUIRE(has_fold(ranges, 0, 2, FoldingRangeKind::COMMENT));
}

TEST_CASE("folding: comment block too short", "[folding]") {
    auto source = "# line 1\n# line 2\nset x 1\n";
    auto ranges = get_folding_ranges(source);
    // 2 lines is not enough for a fold.
    bool has_comment_fold = false;
    for (auto& r : ranges) {
        if (r.kind == FoldingRangeKind::COMMENT) has_comment_fold = true;
    }
    REQUIRE_FALSE(has_comment_fold);
}

TEST_CASE("folding: proc body", "[folding]") {
    auto source =
        "proc foo {} {\n"
        "    set x 1\n"
        "    set y 2\n"
        "}\n";
    auto analysis = analyse_source(source);
    auto ranges = get_folding_ranges(source, &analysis);
    // Should have a region fold for the proc body.
    bool has_region = false;
    for (auto& r : ranges) {
        if (r.kind == FoldingRangeKind::REGION && r.start_line == 0)
            has_region = true;
    }
    REQUIRE(has_region);
}

TEST_CASE("folding: single-line proc no fold", "[folding]") {
    auto source = "proc foo {} { return 1 }\n";
    auto analysis = analyse_source(source);
    auto ranges = get_folding_ranges(source, &analysis);
    // Single-line proc should not generate a fold.
    bool has_region = false;
    for (auto& r : ranges) {
        if (r.kind == FoldingRangeKind::REGION) has_region = true;
    }
    REQUIRE_FALSE(has_region);
}

TEST_CASE("folding: namespace body", "[folding]") {
    auto source =
        "namespace eval ns {\n"
        "    proc foo {} {}\n"
        "    proc bar {} {}\n"
        "}\n";
    auto analysis = analyse_source(source);
    auto ranges = get_folding_ranges(source, &analysis);
    // Should have at least one region fold for the namespace.
    bool has_ns_fold = false;
    for (auto& r : ranges) {
        if (r.kind == FoldingRangeKind::REGION) has_ns_fold = true;
    }
    REQUIRE(has_ns_fold);
}

TEST_CASE("folding: mixed comments and code", "[folding]") {
    auto source =
        "# Header comment\n"
        "# more header\n"
        "# and more\n"
        "\n"
        "proc foo {} {\n"
        "    return 1\n"
        "}\n";
    auto analysis = analyse_source(source);
    auto ranges = get_folding_ranges(source, &analysis);
    REQUIRE(has_fold(ranges, 0, 2, FoldingRangeKind::COMMENT));
}
