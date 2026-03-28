// Tests for E202 (unterminated ") recovery heuristics
// (ported from tests/test_recovery.py TestE202UnterminatedQuote).

#include "tcl_lsp/parsing/recovery.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <unordered_set>

using namespace tcl_lsp;

TEST_CASE("e202 suspicious quote detected", "[recovery][e202]") {
    // ESC token from unterminated " at EOL
    Token tok{TokenType::ESC, "\nset y 2\nset z 3", {0, 6, 6}, {2, 5, 22}};
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {tok};
    std::string_view source = "set x \"\nset y 2\nset z 3";
    CHECK(is_suspicious_quote(tok, cmd, source, 0));
}

TEST_CASE("e202 not suspicious for properly closed quote", "[recovery][e202]") {
    Token tok{TokenType::ESC, "hello", {0, 6, 6}, {0, 11, 11}};
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {tok};
    std::string_view source = "set x \"hello\" more";
    CHECK(!is_suspicious_quote(tok, cmd, source, 0));
}

TEST_CASE("e202 quote recovery produces ghost token", "[recovery][e202]") {
    std::unordered_set<std::string> known = {"set", "puts"};
    Token tok{TokenType::ESC, "\nset y 2", {0, 6, 6}, {1, 5, 14}};
    auto vt = detect_missing_quote_at_newline(tok, "", 0, known);
    REQUIRE(vt.has_value());
    CHECK(vt->ch == '"');
    CHECK(vt->diagnostic.code == "E202");
    CHECK(vt->diagnostic.message == "missing \"");
}

TEST_CASE("e202 codefix inserts close quote", "[recovery][e202]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::ESC, "\nset y 2", {0, 6, 6}, {1, 5, 14}};
    auto vt = detect_missing_quote_at_newline(tok, "", 0, known);
    REQUIRE(vt.has_value());
    REQUIRE(!vt->diagnostic.fixes.empty());
    CHECK(vt->diagnostic.fixes[0].new_text == "\"");
}

TEST_CASE("e202 skips blank lines", "[recovery][e202]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::ESC, "\n\nset y 2", {0, 6, 6}, {2, 5, 15}};
    auto vt = detect_missing_quote_at_newline(tok, "", 0, known);
    REQUIRE(vt.has_value());
}

TEST_CASE("e202 no known command no recovery", "[recovery][e202]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::ESC, "\nunknown_cmd y", {0, 6, 6}, {1, 12, 19}};
    auto vt = detect_missing_quote_at_newline(tok, "", 0, known);
    CHECK(!vt.has_value());
}

TEST_CASE("e202 no heuristic fallback", "[recovery][e202]") {
    Token tok{TokenType::ESC, "\nstuff", {0, 6, 6}, {1, 4, 11}};
    auto diag = detect_missing_quote_no_heuristic(tok, "", 0);
    CHECK(diag.code == "E202");
    CHECK(diag.message == "missing \"");
    CHECK(diag.fixes.empty());
}

TEST_CASE("e202 not suspicious when not starting with newline", "[recovery][e202]") {
    Token tok{TokenType::ESC, "hello world", {0, 6, 6}, {0, 17, 17}};
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {tok};
    std::string_view source = "set x \"hello world";
    CHECK(!is_suspicious_quote(tok, cmd, source, 0));
}
