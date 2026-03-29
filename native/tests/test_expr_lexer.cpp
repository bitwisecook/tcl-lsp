#include <catch2/catch_test_macros.hpp>

#include "tcl_lsp/parsing/expr_lexer.hpp"

using namespace tcl_lsp;

TEST_CASE("expr lexer: simple number", "[expr_lexer]") {
    auto tokens = tokenise_expr("42");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::NUMBER);
    REQUIRE(tokens[0].text == "42");
}

TEST_CASE("expr lexer: float", "[expr_lexer]") {
    auto tokens = tokenise_expr("3.14");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::NUMBER);
    REQUIRE(tokens[0].text == "3.14");
}

TEST_CASE("expr lexer: hex number", "[expr_lexer]") {
    auto tokens = tokenise_expr("0xFF");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::NUMBER);
    REQUIRE(tokens[0].text == "0xFF");
}

TEST_CASE("expr lexer: scientific notation", "[expr_lexer]") {
    auto tokens = tokenise_expr("1e10");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::NUMBER);
    REQUIRE(tokens[0].text == "1e10");
}

TEST_CASE("expr lexer: variable", "[expr_lexer]") {
    auto tokens = tokenise_expr("$x");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::VARIABLE);
    REQUIRE(tokens[0].text == "$x");
}

TEST_CASE("expr lexer: braced variable", "[expr_lexer]") {
    auto tokens = tokenise_expr("${foo}");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::VARIABLE);
    REQUIRE(tokens[0].text == "${foo}");
}

TEST_CASE("expr lexer: array variable", "[expr_lexer]") {
    auto tokens = tokenise_expr("$arr(idx)");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::VARIABLE);
    REQUIRE(tokens[0].text == "$arr(idx)");
}

TEST_CASE("expr lexer: command substitution", "[expr_lexer]") {
    auto tokens = tokenise_expr("[llength $list]");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::COMMAND);
}

TEST_CASE("expr lexer: string", "[expr_lexer]") {
    auto tokens = tokenise_expr("\"hello\"");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::STRING);
}

TEST_CASE("expr lexer: operators", "[expr_lexer]") {
    auto tokens = tokenise_expr("1 + 2");
    // Filter out whitespace.
    std::vector<ExprToken> filtered;
    for (auto& t : tokens) {
        if (t.type != ExprTokenType::WHITESPACE) filtered.push_back(t);
    }
    REQUIRE(filtered.size() == 3);
    REQUIRE(filtered[0].type == ExprTokenType::NUMBER);
    REQUIRE(filtered[1].type == ExprTokenType::OPERATOR);
    REQUIRE(filtered[1].text == "+");
    REQUIRE(filtered[2].type == ExprTokenType::NUMBER);
}

TEST_CASE("expr lexer: multi-char operators", "[expr_lexer]") {
    auto tokens = tokenise_expr("$x == $y");
    std::vector<ExprToken> filtered;
    for (auto& t : tokens) {
        if (t.type != ExprTokenType::WHITESPACE) filtered.push_back(t);
    }
    REQUIRE(filtered.size() == 3);
    REQUIRE(filtered[1].type == ExprTokenType::OPERATOR);
    REQUIRE(filtered[1].text == "==");
}

TEST_CASE("expr lexer: word operators eq/ne", "[expr_lexer]") {
    auto tokens = tokenise_expr("$a eq $b");
    std::vector<ExprToken> filtered;
    for (auto& t : tokens) {
        if (t.type != ExprTokenType::WHITESPACE) filtered.push_back(t);
    }
    REQUIRE(filtered.size() == 3);
    REQUIRE(filtered[1].type == ExprTokenType::OPERATOR);
    REQUIRE(filtered[1].text == "eq");
}

TEST_CASE("expr lexer: parentheses", "[expr_lexer]") {
    auto tokens = tokenise_expr("(1 + 2)");
    REQUIRE(tokens[0].type == ExprTokenType::PAREN_OPEN);
    REQUIRE(tokens.back().type == ExprTokenType::PAREN_CLOSE);
}

TEST_CASE("expr lexer: math function", "[expr_lexer]") {
    auto tokens = tokenise_expr("sin(3.14)");
    REQUIRE(tokens[0].type == ExprTokenType::FUNCTION);
    REQUIRE(tokens[0].text == "sin");
    REQUIRE(tokens[1].type == ExprTokenType::PAREN_OPEN);
}

TEST_CASE("expr lexer: boolean literal", "[expr_lexer]") {
    auto tokens = tokenise_expr("true");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::BOOL);
    REQUIRE(tokens[0].text == "true");
}

TEST_CASE("expr lexer: ternary", "[expr_lexer]") {
    auto tokens = tokenise_expr("$x ? 1 : 0");
    std::vector<ExprToken> filtered;
    for (auto& t : tokens) {
        if (t.type != ExprTokenType::WHITESPACE) filtered.push_back(t);
    }
    REQUIRE(filtered[1].type == ExprTokenType::TERNARY_Q);
    REQUIRE(filtered[3].type == ExprTokenType::TERNARY_C);
}

TEST_CASE("expr lexer: comma", "[expr_lexer]") {
    auto tokens = tokenise_expr("max(1,2)");
    bool found_comma = false;
    for (auto& t : tokens) {
        if (t.type == ExprTokenType::COMMA) found_comma = true;
    }
    REQUIRE(found_comma);
}

TEST_CASE("expr lexer: braced expression", "[expr_lexer]") {
    auto tokens = tokenise_expr("{hello}");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == ExprTokenType::STRING);
    REQUIRE(tokens[0].text == "{hello}");
}

TEST_CASE("expr lexer: 1eq4 word boundary", "[expr_lexer]") {
    // "eq" should be an operator between "1" and "4", not part of "1e" or "q4".
    auto tokens = tokenise_expr("1eq4");
    std::vector<ExprToken> filtered;
    for (auto& t : tokens) {
        if (t.type != ExprTokenType::WHITESPACE) filtered.push_back(t);
    }
    REQUIRE(filtered.size() == 3);
    REQUIRE(filtered[0].type == ExprTokenType::NUMBER);
    REQUIRE(filtered[0].text == "1");
    REQUIRE(filtered[1].type == ExprTokenType::OPERATOR);
    REQUIRE(filtered[1].text == "eq");
    REQUIRE(filtered[2].type == ExprTokenType::NUMBER);
    REQUIRE(filtered[2].text == "4");
}

TEST_CASE("expr lexer: unknown identifier as function", "[expr_lexer]") {
    auto tokens = tokenise_expr("myFunc(1)");
    REQUIRE(tokens[0].type == ExprTokenType::FUNCTION);
    REQUIRE(tokens[0].text == "myFunc");
}
