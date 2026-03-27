#include "tcl_lsp/core/document_buffer.hpp"

#include <catch2/catch_test_macros.hpp>

using tcl_lsp::DocumentBuffer;
using tcl_lsp::SourcePosition;

TEST_CASE("DocumentBuffer::from_source basic", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld");

    CHECK(buf.source() == "hello\nworld");
    CHECK(buf.version() == std::nullopt);
}

TEST_CASE("DocumentBuffer with version", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc", 42);
    CHECK(buf.version() == 42);
}

TEST_CASE("DocumentBuffer::lines() splits correctly", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("line1\nline2\nline3");
    auto lns = buf.lines();

    REQUIRE(lns.size() == 3);
    CHECK(lns[0] == "line1");
    CHECK(lns[1] == "line2");
    CHECK(lns[2] == "line3");
}

TEST_CASE("DocumentBuffer::lines() single line", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("only one line");
    auto lns = buf.lines();

    REQUIRE(lns.size() == 1);
    CHECK(lns[0] == "only one line");
}

TEST_CASE("DocumentBuffer::lines() empty source", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("");
    auto lns = buf.lines();

    REQUIRE(lns.size() == 1);
    CHECK(lns[0] == "");
}

TEST_CASE("DocumentBuffer::lines() trailing newline", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("a\nb\n");
    auto lns = buf.lines();

    REQUIRE(lns.size() == 3);
    CHECK(lns[0] == "a");
    CHECK(lns[1] == "b");
    CHECK(lns[2] == "");
}

TEST_CASE("DocumentBuffer::offset_to_position", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc\ndef\nghi");
    //  offsets: a=0 b=1 c=2 \n=3 d=4 e=5 f=6 \n=7 g=8 h=9 i=10

    CHECK(buf.offset_to_position(0) == SourcePosition{0, 0, 0});
    CHECK(buf.offset_to_position(2) == SourcePosition{0, 2, 2});
    CHECK(buf.offset_to_position(4) == SourcePosition{1, 0, 4});
    CHECK(buf.offset_to_position(6) == SourcePosition{1, 2, 6});
    CHECK(buf.offset_to_position(8) == SourcePosition{2, 0, 8});
    CHECK(buf.offset_to_position(10) == SourcePosition{2, 2, 10});
}

TEST_CASE("DocumentBuffer::offset_to_position clamping", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("ab");

    // Negative offset clamps to 0.
    CHECK(buf.offset_to_position(-5) == SourcePosition{0, 0, 0});
    // Beyond end clamps to source length.
    CHECK(buf.offset_to_position(100) == SourcePosition{0, 2, 2});
}

TEST_CASE("DocumentBuffer::position_to_offset", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc\ndef\nghi");

    CHECK(buf.position_to_offset(0, 0) == 0);
    CHECK(buf.position_to_offset(0, 2) == 2);
    CHECK(buf.position_to_offset(1, 0) == 4);
    CHECK(buf.position_to_offset(1, 2) == 6);
    CHECK(buf.position_to_offset(2, 0) == 8);
}

TEST_CASE("DocumentBuffer::position_to_offset clamping", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc\ndef");

    // Character beyond line length clamps to line end.
    CHECK(buf.position_to_offset(0, 100) == 3);
    // Line beyond last line clamps to last line.
    CHECK(buf.position_to_offset(100, 0) == 4);
    // Negative values clamp to 0.
    CHECK(buf.position_to_offset(-1, 0) == 0);
}

TEST_CASE("DocumentBuffer::offset_to_line_col", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc\ndef");

    CHECK(buf.offset_to_line_col(0) == std::pair{0, 0});
    CHECK(buf.offset_to_line_col(2) == std::pair{0, 2});
    CHECK(buf.offset_to_line_col(4) == std::pair{1, 0});
    CHECK(buf.offset_to_line_col(6) == std::pair{1, 2});
}

TEST_CASE("DocumentBuffer::range_from_offsets", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc\ndef");

    auto r = buf.range_from_offsets(0, 6);
    CHECK(r.start == SourcePosition{0, 0, 0});
    CHECK(r.end == SourcePosition{1, 2, 6});
}

TEST_CASE("DocumentBuffer::range_from_offsets empty source", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("");

    auto r = buf.range_from_offsets(0, 0);
    CHECK(r.start == SourcePosition{0, 0, 0});
    CHECK(r.end == SourcePosition{0, 0, 0});
}

TEST_CASE("DocumentBuffer::chunk_line_range", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("abc\ndef\nghi");

    auto [sl, sc, el, ec] = buf.chunk_line_range(0, 10);
    CHECK(sl == 0);
    CHECK(sc == 0);
    CHECK(el == 2);
    CHECK(ec == 2);
}

TEST_CASE("DocumentBuffer::position_to_offset empty source", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("");
    CHECK(buf.position_to_offset(0, 0) == 0);
}

TEST_CASE("DocumentBuffer round-trip offset->position->offset", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld\n!");

    for (int32_t off = 0; off <= 12; ++off) {
        auto pos = buf.offset_to_position(off);
        auto back = buf.position_to_offset(pos.line, pos.character);
        CHECK(back == off);
    }
}
