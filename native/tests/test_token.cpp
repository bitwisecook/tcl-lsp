#include "tcl_lsp/parsing/token.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("TokenType to_string", "[token]") {
    CHECK(to_string(TokenType::ESC) == "ESC");
    CHECK(to_string(TokenType::STR) == "STR");
    CHECK(to_string(TokenType::CMD) == "CMD");
    CHECK(to_string(TokenType::VAR) == "VAR");
    CHECK(to_string(TokenType::SEP) == "SEP");
    CHECK(to_string(TokenType::EOL) == "EOL");
    CHECK(to_string(TokenType::EOF_) == "EOF");
    CHECK(to_string(TokenType::COMMENT) == "COMMENT");
    CHECK(to_string(TokenType::EXPAND) == "EXPAND");
}

TEST_CASE("Token equality", "[token]") {
    auto a = Token{TokenType::ESC, "hello", {0, 0, 0}, {0, 4, 4}, false};
    auto b = Token{TokenType::ESC, "hello", {0, 0, 0}, {0, 4, 4}, false};
    auto c = Token{TokenType::VAR, "hello", {0, 0, 0}, {0, 4, 4}, false};
    CHECK(a == b);
    CHECK_FALSE(a == c);
}

TEST_CASE("Token to_string", "[token]") {
    auto t = Token{TokenType::VAR, "foo", {0, 0, 0}, {0, 2, 2}, false};
    CHECK(to_string(t) == "Token(VAR, \"foo\")");
}
