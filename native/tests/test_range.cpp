#include <catch2/catch_test_macros.hpp>
#include "tcl_lsp/core/range.hpp"

#include <unordered_set>

using tcl_lsp::Range;
using tcl_lsp::SourcePosition;

TEST_CASE("Range::zero()", "[range]") {
    auto r = Range::zero();
    CHECK(r.start == SourcePosition{0, 0, 0});
    CHECK(r.end == SourcePosition{0, 0, 0});
}

TEST_CASE("Range equality", "[range]") {
    auto a = Range{SourcePosition{0, 0, 0}, SourcePosition{1, 5, 10}};
    auto b = Range{SourcePosition{0, 0, 0}, SourcePosition{1, 5, 10}};
    auto c = Range{SourcePosition{0, 0, 0}, SourcePosition{2, 0, 20}};

    CHECK(a == b);
    CHECK(a != c);
}

TEST_CASE("Range ordering", "[range]") {
    auto a = Range{SourcePosition{0, 0, 0}, SourcePosition{0, 5, 5}};
    auto b = Range{SourcePosition{0, 0, 0}, SourcePosition{1, 0, 10}};
    auto c = Range{SourcePosition{1, 0, 10}, SourcePosition{2, 0, 20}};

    CHECK(a < b);
    CHECK(b < c);
}

TEST_CASE("Range hashing", "[range]") {
    auto a = Range{SourcePosition{0, 0, 0}, SourcePosition{1, 5, 10}};
    auto b = Range{SourcePosition{0, 0, 0}, SourcePosition{1, 5, 10}};
    auto c = Range{SourcePosition{2, 0, 20}, SourcePosition{3, 0, 30}};

    auto hash = Range::Hash{};
    CHECK(hash(a) == hash(b));
    CHECK(hash(a) != hash(c));
}

TEST_CASE("Range usable in unordered_set", "[range]") {
    std::unordered_set<Range, Range::Hash> ranges;
    ranges.insert(Range{SourcePosition{0, 0, 0}, SourcePosition{1, 0, 10}});
    ranges.insert(Range{SourcePosition{2, 0, 20}, SourcePosition{3, 0, 30}});
    ranges.insert(Range{SourcePosition{0, 0, 0}, SourcePosition{1, 0, 10}});  // duplicate

    CHECK(ranges.size() == 2);
}

TEST_CASE("Range to_string", "[range]") {
    auto r = Range{SourcePosition{1, 2, 3}, SourcePosition{4, 5, 6}};
    auto s = to_string(r);
    CHECK(s == "Range(start=SourcePosition(line=1, character=2, offset=3), "
               "end=SourcePosition(line=4, character=5, offset=6))");
}

TEST_CASE("Range::zero() is constexpr", "[range]") {
    constexpr auto r = Range::zero();
    static_assert(r.start.line == 0);
    static_assert(r.end.offset == 0);
}
