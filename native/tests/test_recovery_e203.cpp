// Tests for E203 (unterminated {) recovery heuristics
// (ported from tests/test_recovery.py TestE203UnterminatedBrace).

#include "tcl_lsp/parsing/recovery.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <unordered_set>

using namespace tcl_lsp;

TEST_CASE("e203 suspicious str detected", "[recovery][e203]") {
    // STR token from unterminated { spanning 3+ lines, reaching EOF, no inner }
    Token tok{TokenType::STR, "\n    set inner 1\n    set inner2 2\n", {0, 15, 15}, {3, 0, 49}};
    std::string_view source = "proc broken {} {\n    set inner 1\n    set inner2 2\n";
    CHECK(is_suspicious_str(tok, source, 0));
}

TEST_CASE("e203 not suspicious when properly closed", "[recovery][e203]") {
    Token tok{TokenType::STR, "\n    set a 1\n", {0, 15, 15}, {2, 0, 28}};
    // Source has } after the token
    std::string_view source = "proc foo {} {\n    set a 1\n}";
    CHECK(!is_suspicious_str(tok, source, 0));
}

TEST_CASE("e203 not suspicious when contains close brace", "[recovery][e203]") {
    Token tok{TokenType::STR, "\n    set a 1\n}\nmore stuff\n", {0, 15, 15}, {4, 0, 41}};
    std::string_view source = "proc foo {} {\n    set a 1\n}\nmore stuff\n";
    // Contains } in text — this is E103 territory, not E203.
    CHECK(!is_suspicious_str(tok, source, 0));
}

TEST_CASE("e203 brace recovery with de-indented command", "[recovery][e203]") {
    std::unordered_set<std::string> known = {"proc", "set", "puts"};
    // Token text: body of an unterminated proc
    Token tok{
        TokenType::STR, "\n    set inner 1\n    set inner2 2\nset x 1", {0, 15, 15}, {3, 5, 52}};
    auto vt = detect_missing_brace_at_command(tok, "", 0, known);
    REQUIRE(vt.has_value());
    CHECK(vt->ch == '}');
    CHECK(vt->diagnostic.code == "E203");
    CHECK(vt->diagnostic.message == "missing close-brace");
}

TEST_CASE("e203 codefix inserts close brace", "[recovery][e203]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::STR, "\n    set inner 1\nset x 1", {0, 15, 15}, {2, 5, 37}};
    auto vt = detect_missing_brace_at_command(tok, "", 0, known);
    REQUIRE(vt.has_value());
    REQUIRE(!vt->diagnostic.fixes.empty());
    CHECK(vt->diagnostic.fixes[0].new_text == "}");
}

TEST_CASE("e203 no recovery when no de-indented command", "[recovery][e203]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::STR, "\n    set inner 1\n    set inner2 2", {0, 15, 15}, {2, 14, 44}};
    // All lines have same indentation — no de-indented command found.
    auto vt = detect_missing_brace_at_command(tok, "", 0, known);
    CHECK(!vt.has_value());
}

TEST_CASE("e203 no heuristic fallback", "[recovery][e203]") {
    Token tok{TokenType::STR, "\nstuff", {0, 5, 5}, {1, 4, 10}};
    auto diag = detect_missing_brace_no_heuristic(tok, "", 0);
    CHECK(diag.code == "E203");
    CHECK(diag.message == "missing close-brace");
    CHECK(diag.fixes.empty());
}

TEST_CASE("e203 needs at least 3 lines", "[recovery][e203]") {
    std::unordered_set<std::string> known = {"set"};
    Token tok{TokenType::STR, "\nset x 1", {0, 5, 5}, {1, 5, 11}};
    // Only 2 lines — not enough for recovery.
    auto vt = detect_missing_brace_at_command(tok, "", 0, known);
    CHECK(!vt.has_value());
}
