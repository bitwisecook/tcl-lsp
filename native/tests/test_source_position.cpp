#include <catch2/catch_test_macros.hpp>
#include "tcl_lsp/core/source_position.hpp"

#include <unordered_set>

using tcl_lsp::SourcePosition;

TEST_CASE("SourcePosition default-constructed fields", "[source_position]") {
    SourcePosition p{0, 0, 0};
    CHECK(p.line == 0);
    CHECK(p.character == 0);
    CHECK(p.offset == 0);
}

TEST_CASE("SourcePosition equality", "[source_position]") {
    SourcePosition a{1, 5, 42};
    SourcePosition b{1, 5, 42};
    SourcePosition c{2, 0, 50};

    CHECK(a == b);
    CHECK(a != c);
}

TEST_CASE("SourcePosition ordering", "[source_position]") {
    SourcePosition a{0, 0, 0};
    SourcePosition b{0, 5, 5};
    SourcePosition c{1, 0, 10};

    CHECK(a < b);
    CHECK(b < c);
    CHECK(a < c);
    CHECK(c > a);
}

TEST_CASE("SourcePosition hashing", "[source_position]") {
    SourcePosition a{1, 2, 3};
    SourcePosition b{1, 2, 3};
    SourcePosition c{4, 5, 6};

    auto hash = SourcePosition::Hash{};
    CHECK(hash(a) == hash(b));
    // Different positions should (very likely) have different hashes.
    CHECK(hash(a) != hash(c));
}

TEST_CASE("SourcePosition usable in unordered_set", "[source_position]") {
    std::unordered_set<SourcePosition, SourcePosition::Hash> positions;
    positions.insert({0, 0, 0});
    positions.insert({1, 5, 10});
    positions.insert({0, 0, 0});  // duplicate

    CHECK(positions.size() == 2);
}

TEST_CASE("SourcePosition to_string", "[source_position]") {
    SourcePosition p{3, 7, 42};
    auto s = to_string(p);
    CHECK(s == "SourcePosition(line=3, character=7, offset=42)");
}
