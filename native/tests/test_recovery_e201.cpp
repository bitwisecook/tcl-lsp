// Tests for E201 (unterminated [) recovery heuristics
// (ported from tests/test_recovery.py TestE201CommentBreak, TestE201CommandBreak,
// TestE201BraceBreak).

#include "tcl_lsp/parsing/recovery.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <unordered_set>

using namespace tcl_lsp;

// E201 comment-break heuristic

TEST_CASE("e201 comment break basic detection", "[recovery][e201]") {
    // CMD token: "ACCESS::policy agent_id\n# comment"
    Token tok{TokenType::CMD, "ACCESS::policy agent_id\n# comment", {1, 15, 47}, {2, 8, 80}};
    auto vt = detect_missing_bracket_at_comment(tok, "", 0);
    REQUIRE(vt.has_value());
    CHECK(vt->ch == ']');
    CHECK(vt->diagnostic.code == DiagCode::E201);
    CHECK(vt->diagnostic.message == "missing close-bracket");
}

TEST_CASE("e201 comment break codefix inserts bracket", "[recovery][e201]") {
    Token tok{TokenType::CMD, "ACCESS::policy agent_id\n# comment", {1, 15, 47}, {2, 8, 80}};
    auto vt = detect_missing_bracket_at_comment(tok, "", 0);
    REQUIRE(vt.has_value());
    REQUIRE(!vt->diagnostic.fixes.empty());
    CHECK(vt->diagnostic.fixes[0].new_text == "]");
}

TEST_CASE("e201 comment break no false positive on single line", "[recovery][e201]") {
    Token tok{TokenType::CMD, "string length $x", {0, 5, 5}, {0, 21, 21}};
    auto vt = detect_missing_bracket_at_comment(tok, "", 0);
    CHECK(!vt.has_value());
}

// E201 command-break heuristic

TEST_CASE("e201 command break basic recovery", "[recovery][e201]") {
    std::unordered_set<std::string> known = {"set", "puts"};
    Token tok{TokenType::CMD, "foo bar\nset x 1", {0, 5, 5}, {1, 5, 18}};
    auto vt = detect_missing_bracket_at_command(tok, "", 0, known);
    REQUIRE(vt.has_value());
    CHECK(vt->ch == ']');
    CHECK(vt->diagnostic.code == DiagCode::E201);
}

TEST_CASE("e201 command break skips blank lines", "[recovery][e201]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::CMD, "foo bar\n\nset x 1", {0, 5, 5}, {2, 5, 19}};
    auto vt = detect_missing_bracket_at_command(tok, "", 0, known);
    REQUIRE(vt.has_value());
}

TEST_CASE("e201 command break non command stops", "[recovery][e201]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::CMD, "foo bar\nunknown_cmd x", {0, 5, 5}, {1, 13, 25}};
    auto vt = detect_missing_bracket_at_command(tok, "", 0, known);
    CHECK(!vt.has_value());
}

TEST_CASE("e201 comment takes priority over command", "[recovery][e201]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::CMD, "foo bar\n# comment here\nset x 1", {0, 5, 5}, {2, 5, 33}};
    // Comment-break should fire before command-break.
    auto vt_comment = detect_missing_bracket_at_comment(tok, "", 0);
    CHECK(vt_comment.has_value());
}

// E201 brace-break heuristic

TEST_CASE("e201 brace break basic", "[recovery][e201]") {
    Token tok{TokenType::CMD, "ACCESS::policy agent_id {", {0, 5, 5}, {0, 29, 29}};
    auto vt = detect_missing_bracket_at_brace(tok, "", 0);
    REQUIRE(vt.has_value());
    CHECK(vt->ch == ']');
    CHECK(vt->diagnostic.code == DiagCode::E201);
}

TEST_CASE("e201 brace break codefix", "[recovery][e201]") {
    Token tok{TokenType::CMD, "ACCESS::policy agent_id {", {0, 5, 5}, {0, 29, 29}};
    auto vt = detect_missing_bracket_at_brace(tok, "", 0);
    REQUIRE(vt.has_value());
    REQUIRE(!vt->diagnostic.fixes.empty());
    CHECK(vt->diagnostic.fixes[0].new_text == "]");
}

TEST_CASE("e201 no brace break for valid cmd", "[recovery][e201]") {
    Token tok{TokenType::CMD, "string length $x", {0, 5, 5}, {0, 21, 21}};
    auto vt = detect_missing_bracket_at_brace(tok, "", 0);
    CHECK(!vt.has_value());
}

TEST_CASE("e201 no heuristic fallback", "[recovery][e201]") {
    Token tok{TokenType::CMD, "foo", {0, 5, 5}, {0, 8, 8}};
    auto diag = detect_missing_bracket_no_heuristic(tok);
    CHECK(diag.code == DiagCode::E201);
    CHECK(diag.message == "missing close-bracket");
    CHECK(diag.fixes.empty());
}

// is_unterminated_cmd

TEST_CASE("unterminated cmd detected", "[recovery][e201]") {
    // Source: "set x [foo" — no closing ]
    std::string_view source = "set x [foo";
    Token tok{TokenType::CMD, "foo", {0, 6, 6}, {0, 9, 9}};
    CHECK(is_unterminated_cmd(tok, source, 0));
}

TEST_CASE("terminated cmd not flagged", "[recovery][e201]") {
    std::string_view source = "set x [foo]";
    Token tok{TokenType::CMD, "foo", {0, 6, 6}, {0, 9, 9}};
    CHECK(!is_unterminated_cmd(tok, source, 0));
}
