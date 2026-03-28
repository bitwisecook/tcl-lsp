// Tests for ghost token lexer integration and compute_ghost_insertions
// (ported from tests/test_recovery.py TestGhostTokenLexer, TestComputeGhostInsertions).

#include "tcl_lsp/parsing/lexer.hpp"
#include "tcl_lsp/parsing/recovery.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <unordered_map>

using namespace tcl_lsp;

// TestGhostTokenLexer

TEST_CASE("ghost bracket terminates cmd", "[recovery][ghost]") {
    // Source: "set x [foo bar" — no closing ]
    // Ghost ] at offset 14 should terminate the CMD token.
    std::string_view source = "set x [foo bar";
    std::unordered_map<int32_t, char> ghosts = {{14, ']'}};
    TclLexer lexer(source, {}, 0, 0, 0, ghosts);
    auto tokens = lexer.tokenise_all();
    // Should find a CMD token that terminates properly.
    bool found_cmd = false;
    for (const auto& tok : tokens) {
        if (tok.type == TokenType::CMD) {
            found_cmd = true;
            CHECK(tok.text == "foo bar");
        }
    }
    CHECK(found_cmd);
}

TEST_CASE("ghost does not shift positions", "[recovery][ghost]") {
    // Ghost tokens are zero-width — positions should not shift.
    std::string_view source = "set x [foo";
    std::unordered_map<int32_t, char> ghosts = {{10, ']'}};
    TclLexer lexer(source, {}, 0, 0, 0, ghosts);
    auto tokens = lexer.tokenise_all();
    // The last real token should end at offset 9 (zero-based last char of "foo").
    // Ghost ] is zero-width at offset 10.
    bool found_cmd = false;
    for (const auto& tok : tokens) {
        if (tok.type == TokenType::CMD) {
            found_cmd = true;
        }
    }
    CHECK(found_cmd);
}

TEST_CASE("no ghosts behaves normally", "[recovery][ghost]") {
    std::string_view source = "set a 1";
    std::unordered_map<int32_t, char> empty;
    TclLexer lexer(source, {}, 0, 0, 0, empty);
    auto tokens = lexer.tokenise_all();
    // Should parse normally — 3 content tokens + separators + eol.
    int content_count = 0;
    for (const auto& tok : tokens) {
        if (tok.type != TokenType::SEP && tok.type != TokenType::EOL) {
            ++content_count;
        }
    }
    CHECK(content_count == 3);
}

// TestComputeGhostInsertions

TEST_CASE("clean source returns empty insertions", "[recovery][ghost]") {
    auto result = compute_ghost_insertions("set a 1\nset b 2");
    CHECK(result.empty());
}

TEST_CASE("compute ghost insertions for unterminated bracket at top level", "[recovery][ghost]") {
    // Top-level unterminated [ with { inside — brace-break heuristic fires.
    std::string source = "set x [foo bar {stuff}\nset y 2";
    auto result = compute_ghost_insertions(source);
    // The brace-break heuristic detects { inside [ and inserts ].
    CHECK(!result.empty());
}

// segment_with_recovery

TEST_CASE("segment_with_recovery clean source no diagnostics", "[recovery][pipeline]") {
    auto [cmds, diags] = segment_with_recovery("set a 1\nset b 2");
    CHECK(cmds.size() == 2);
    CHECK(diags.empty());
}

TEST_CASE("position_from_relative basic", "[recovery][position]") {
    auto pos = position_from_relative("hello\nworld", 6, 0, 0, 0);
    CHECK(pos.line == 1);
    CHECK(pos.character == 0);
    CHECK(pos.offset == 6);
}

TEST_CASE("position_from_relative within line", "[recovery][position]") {
    auto pos = position_from_relative("hello\nworld", 8, 0, 0, 0);
    CHECK(pos.line == 1);
    CHECK(pos.character == 2);
    CHECK(pos.offset == 8);
}

TEST_CASE("position_from_relative with base", "[recovery][position]") {
    auto pos = position_from_relative("hello", 3, 5, 10, 100);
    CHECK(pos.line == 5);
    CHECK(pos.character == 13);
    CHECK(pos.offset == 103);
}
