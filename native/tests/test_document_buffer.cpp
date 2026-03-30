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

TEST_CASE("DocumentBuffer::offset_to_line_col hello_world", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld");

    CHECK(buf.offset_to_line_col(0) == std::pair{0, 0});
    CHECK(buf.offset_to_line_col(3) == std::pair{0, 3});
    CHECK(buf.offset_to_line_col(6) == std::pair{1, 0});
    CHECK(buf.offset_to_line_col(8) == std::pair{1, 2});
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

// ---------------------------------------------------------------------------
// compute_line_starts (tested via DocumentBuffer::line_starts() accessor)
// ---------------------------------------------------------------------------

TEST_CASE("compute_line_starts", "[document_buffer]") {
    SECTION("empty") {
        auto buf = DocumentBuffer::from_source("");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 1);
        CHECK(ls[0] == 0);
    }

    SECTION("single_line") {
        auto buf = DocumentBuffer::from_source("hello");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 1);
        CHECK(ls[0] == 0);
    }

    SECTION("two_lines") {
        auto buf = DocumentBuffer::from_source("hello\nworld");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 2);
        CHECK(ls[0] == 0);
        CHECK(ls[1] == 6);
    }

    SECTION("trailing_newline") {
        auto buf = DocumentBuffer::from_source("hello\n");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 2);
        CHECK(ls[0] == 0);
        CHECK(ls[1] == 6);
    }

    SECTION("multiple_lines") {
        auto buf = DocumentBuffer::from_source("a\nb\nc\nd");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 4);
        CHECK(ls[0] == 0);
        CHECK(ls[1] == 2);
        CHECK(ls[2] == 4);
        CHECK(ls[3] == 6);
    }

    SECTION("empty_lines") {
        auto buf = DocumentBuffer::from_source("\n\n\n");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 4);
        CHECK(ls[0] == 0);
        CHECK(ls[1] == 1);
        CHECK(ls[2] == 2);
        CHECK(ls[3] == 3);
    }

    SECTION("crlf") {
        // \r is treated as a regular character; only \n starts new lines
        auto buf = DocumentBuffer::from_source("a\r\nb\r\n");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 3);
        CHECK(ls[0] == 0);
        CHECK(ls[1] == 3);
        CHECK(ls[2] == 6);
    }
}

// ---------------------------------------------------------------------------
// DocumentBuffer::from_source — line_starts verification
// ---------------------------------------------------------------------------

TEST_CASE("DocumentBuffer::from_source line_starts", "[document_buffer]") {
    SECTION("basic") {
        auto buf = DocumentBuffer::from_source("hello\nworld", 1);
        CHECK(buf.source() == "hello\nworld");
        CHECK(buf.version() == 1);
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 2);
        CHECK(ls[0] == 0);
        CHECK(ls[1] == 6);
    }

    SECTION("empty") {
        auto buf = DocumentBuffer::from_source("");
        CHECK(buf.source() == "");
        auto& ls = buf.line_starts();
        REQUIRE(ls.size() == 1);
        CHECK(ls[0] == 0);
    }
}

// ---------------------------------------------------------------------------
// offset_to_position — individual cases matching Python tests
// ---------------------------------------------------------------------------

TEST_CASE("DocumentBuffer::offset_to_position detailed", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld");

    SECTION("start_of_file") {
        CHECK(buf.offset_to_position(0) == SourcePosition{0, 0, 0});
    }

    SECTION("middle_of_first_line") {
        CHECK(buf.offset_to_position(3) == SourcePosition{0, 3, 3});
    }

    SECTION("start_of_second_line") {
        CHECK(buf.offset_to_position(6) == SourcePosition{1, 0, 6});
    }

    SECTION("end_of_file") {
        CHECK(buf.offset_to_position(11) == SourcePosition{1, 5, 11});
    }

    SECTION("newline_char") {
        CHECK(buf.offset_to_position(5) == SourcePosition{0, 5, 5});
    }
}

TEST_CASE("DocumentBuffer::offset_to_position clamp_negative", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello");
    auto pos = buf.offset_to_position(-1);
    CHECK(pos.offset == 0);
}

TEST_CASE("DocumentBuffer::offset_to_position clamp_past_end", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello");
    auto pos = buf.offset_to_position(100);
    CHECK(pos.offset == 5);
}

TEST_CASE("DocumentBuffer::offset_to_position empty_source", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("");
    auto pos = buf.offset_to_position(0);
    CHECK(pos == SourcePosition{0, 0, 0});
}

TEST_CASE("DocumentBuffer::offset_to_position multiline", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("line1\nline2\nline3");

    SECTION("start_of_line3") {
        auto pos = buf.offset_to_position(12);
        CHECK(pos == SourcePosition{2, 0, 12});
    }

    SECTION("middle_of_line3") {
        auto pos = buf.offset_to_position(15);
        CHECK(pos == SourcePosition{2, 3, 15});
    }
}

// ---------------------------------------------------------------------------
// position_to_offset — individual cases matching Python tests
// ---------------------------------------------------------------------------

TEST_CASE("DocumentBuffer::position_to_offset detailed", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld");

    SECTION("start") {
        CHECK(buf.position_to_offset(0, 0) == 0);
    }

    SECTION("middle") {
        CHECK(buf.position_to_offset(0, 3) == 3);
    }

    SECTION("second_line") {
        CHECK(buf.position_to_offset(1, 0) == 6);
    }

    SECTION("clamp_line") {
        // Line 5 doesn't exist, clamps to last line
        CHECK(buf.position_to_offset(5, 0) == 6);
    }

    SECTION("clamp_character") {
        // Character 100 on line 0 clamps to end of line
        CHECK(buf.position_to_offset(0, 100) == 5);
    }
}

TEST_CASE("DocumentBuffer::position_to_offset roundtrip proc", "[document_buffer]") {
    std::string source = "proc foo {bar} {\n    set x 1\n    return $x\n}\n";
    auto buf = DocumentBuffer::from_source(std::string(source));

    for (int32_t off = 0; off < static_cast<int32_t>(source.size()); ++off) {
        auto pos = buf.offset_to_position(off);
        auto back = buf.position_to_offset(pos.line, pos.character);
        CHECK(back == off);
    }
}

// ---------------------------------------------------------------------------
// chunk_line_range — multiple chunks
// ---------------------------------------------------------------------------

TEST_CASE("DocumentBuffer::chunk_line_range single_chunk", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("set x 1\n");
    auto [sl, sc, el, ec] = buf.chunk_line_range(0, 8);
    CHECK(sl == 0);
    CHECK(sc == 0);
    CHECK(el == 1);
    CHECK(ec == 0);
}

TEST_CASE("DocumentBuffer::chunk_line_range multiple_chunks", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("set x 1\nset y 2\n");

    SECTION("first_chunk") {
        auto [sl, sc, el, ec] = buf.chunk_line_range(0, 8);
        CHECK(sl == 0);
        CHECK(sc == 0);
        CHECK(el == 1);
        CHECK(ec == 0);
    }

    SECTION("second_chunk") {
        auto [sl, sc, el, ec] = buf.chunk_line_range(8, 16);
        CHECK(sl == 1);
        CHECK(sc == 0);
        CHECK(el == 2);
        CHECK(ec == 0);
    }
}

// ---------------------------------------------------------------------------
// range_from_offsets — cross_line and basic with hello\nworld
// ---------------------------------------------------------------------------

TEST_CASE("DocumentBuffer::range_from_offsets basic hello", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld");
    auto r = buf.range_from_offsets(0, 4);
    CHECK(r.start.line == 0);
    CHECK(r.start.character == 0);
    CHECK(r.end.line == 0);
    CHECK(r.end.character == 4);
}

TEST_CASE("DocumentBuffer::range_from_offsets cross_line", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello\nworld");
    auto r = buf.range_from_offsets(3, 8);
    CHECK(r.start.line == 0);
    CHECK(r.start.character == 3);
    CHECK(r.end.line == 1);
    CHECK(r.end.character == 2);
}

TEST_CASE("DocumentBuffer epoch defaults to zero", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello");
    CHECK(buf.epoch() == 0);
}

TEST_CASE("DocumentBuffer epoch can be set", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("hello", 1, 42);
    CHECK(buf.epoch() == 42);
    CHECK(buf.version() == 1);
    CHECK(buf.source() == "hello");
}

TEST_CASE("DocumentBuffer is move-constructible", "[document_buffer]") {
    auto buf = DocumentBuffer::from_source("test", 3, 7);
    auto moved = std::move(buf);
    CHECK(moved.source() == "test");
    CHECK(moved.version() == 3);
    CHECK(moved.epoch() == 7);
}
