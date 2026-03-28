// Tests for TopLevelChunk and find_first_dirty_chunk
// (ported from tests/test_command_segmenter.py TestTopLevelChunks, TestFindFirstDirtyChunk).

#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("one chunk per command", "[segmenter][chunks]") {
    auto chunks = segment_top_level_chunks("set a 1\nset b 2\nset c 3");
    CHECK(chunks.size() == 3);
}

TEST_CASE("chunk indices sequential", "[segmenter][chunks]") {
    auto chunks = segment_top_level_chunks("set a 1\nset b 2");
    REQUIRE(chunks.size() == 2);
    CHECK(chunks[0].index == 0);
    CHECK(chunks[1].index == 1);
}

TEST_CASE("chunks tile source", "[segmenter][chunks]") {
    std::string source = "set a 1\nset b 2\nset c 3";
    auto chunks = segment_top_level_chunks(source);
    REQUIRE(chunks.size() == 3);
    // First chunk starts at 0.
    CHECK(chunks[0].start_offset == 0);
    // Each chunk's end matches the next chunk's start.
    for (std::size_t i = 0; i + 1 < chunks.size(); ++i) {
        CHECK(chunks[i].end_offset == chunks[i + 1].start_offset);
    }
    // Last chunk ends at source length.
    CHECK(chunks.back().end_offset == static_cast<int32_t>(source.size()));
}

TEST_CASE("hash changes with content", "[segmenter][chunks]") {
    // Use "99" vs "2" so the text slice lengths differ (the range end is
    // the offset of the last char, and source[start:end] excludes it — but
    // different-length tokens produce different-length slices).
    auto chunks_a = segment_top_level_chunks("set a 1\nset b 2");
    auto chunks_b = segment_top_level_chunks("set a 1\nset b 99");
    REQUIRE(chunks_a.size() == 2);
    REQUIRE(chunks_b.size() == 2);
    CHECK(chunks_a[0].source_hash == chunks_b[0].source_hash);
    CHECK(chunks_a[1].source_hash != chunks_b[1].source_hash);
}

TEST_CASE("empty source chunks", "[segmenter][chunks]") {
    auto chunks = segment_top_level_chunks("");
    CHECK(chunks.empty());
}

TEST_CASE("proc is single chunk", "[segmenter][chunks]") {
    auto chunks = segment_top_level_chunks("proc foo {} {\n    set a 1\n    return $a\n}");
    CHECK(chunks.size() == 1);
}

// find_first_dirty_chunk

TEST_CASE("identical sources", "[segmenter][dirty_chunk]") {
    auto a = segment_top_level_chunks("set a 1\nset b 2");
    auto b = segment_top_level_chunks("set a 1\nset b 2");
    CHECK(find_first_dirty_chunk(a, b) == static_cast<int32_t>(a.size()));
}

TEST_CASE("last chunk changed", "[segmenter][dirty_chunk]") {
    auto a = segment_top_level_chunks("set a 1\nset b 2");
    auto b = segment_top_level_chunks("set a 1\nset b 99");
    CHECK(find_first_dirty_chunk(a, b) == 1);
}

TEST_CASE("first chunk changed", "[segmenter][dirty_chunk]") {
    auto a = segment_top_level_chunks("set a 1\nset b 2");
    auto b = segment_top_level_chunks("set a 99\nset b 2");
    CHECK(find_first_dirty_chunk(a, b) == 0);
}

TEST_CASE("chunk added", "[segmenter][dirty_chunk]") {
    auto a = segment_top_level_chunks("set a 1");
    auto b = segment_top_level_chunks("set a 1\nset b 2");
    CHECK(find_first_dirty_chunk(a, b) == 1);
}

TEST_CASE("chunk removed", "[segmenter][dirty_chunk]") {
    auto a = segment_top_level_chunks("set a 1\nset b 2");
    auto b = segment_top_level_chunks("set a 1");
    CHECK(find_first_dirty_chunk(a, b) == 1);
}

TEST_CASE("both empty", "[segmenter][dirty_chunk]") {
    std::vector<TopLevelChunk> a;
    std::vector<TopLevelChunk> b;
    CHECK(find_first_dirty_chunk(a, b) == 0);
}
