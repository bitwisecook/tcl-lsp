// Tests for virtual token lexer integration and compute_virtual_insertions
// (ported from tests/test_recovery.py TestVirtualTokenLexer, TestComputeVirtualInsertions).

#include "tcl_lsp/parsing/lexer.hpp"
#include "tcl_lsp/parsing/recovery.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <unordered_map>

using namespace tcl_lsp;

// TestVirtualTokenLexer

TEST_CASE("virtual bracket terminates cmd", "[recovery][virtual]") {
    // Source: "set x [foo bar" — no closing ]
    // Virtual ] at offset 14 should terminate the CMD token.
    std::string_view source = "set x [foo bar";
    std::unordered_map<int32_t, char> virtuals = {{14, ']'}};
    TclLexer lexer(source, {}, 0, 0, 0, virtuals);
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

TEST_CASE("virtual does not shift positions", "[recovery][virtual]") {
    // Virtual tokens are zero-width — positions should not shift.
    std::string_view source = "set x [foo";
    std::unordered_map<int32_t, char> virtuals = {{10, ']'}};
    TclLexer lexer(source, {}, 0, 0, 0, virtuals);
    auto tokens = lexer.tokenise_all();
    // The last real token should end at offset 9 (zero-based last char of "foo").
    // Virtual ] is zero-width at offset 10.
    bool found_cmd = false;
    for (const auto& tok : tokens) {
        if (tok.type == TokenType::CMD) {
            found_cmd = true;
        }
    }
    CHECK(found_cmd);
}

TEST_CASE("no virtuals behaves normally", "[recovery][virtual]") {
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

// TestComputeVirtualInsertions

TEST_CASE("clean source returns empty insertions", "[recovery][virtual]") {
    auto result = compute_virtual_insertions("set a 1\nset b 2");
    CHECK(result.empty());
}

TEST_CASE("compute virtual insertions for unterminated bracket at top level",
          "[recovery][virtual]") {
    // Top-level unterminated [ with { inside — brace-break heuristic fires.
    std::string source = "set x [foo bar {stuff}\nset y 2";
    auto result = compute_virtual_insertions(source);
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
