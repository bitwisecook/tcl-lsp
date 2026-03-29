#include <catch2/catch_test_macros.hpp>

#include "tcl_lsp/lsp/semantic_token_types.hpp"

using namespace tcl_lsp;

TEST_CASE("SemanticTokenType enum values", "[semantic_token_types]") {
    REQUIRE(static_cast<int>(SemanticTokenType::KEYWORD) == 0);
    REQUIRE(static_cast<int>(SemanticTokenType::FUNCTION) == 1);
    REQUIRE(static_cast<int>(SemanticTokenType::VARIABLE) == 2);
    REQUIRE(static_cast<int>(SemanticTokenType::STRING) == 3);
    REQUIRE(static_cast<int>(SemanticTokenType::COMMENT) == 4);
    REQUIRE(static_cast<int>(SemanticTokenType::NUMBER) == 5);
    REQUIRE(static_cast<int>(SemanticTokenType::OPERATOR) == 6);
    REQUIRE(static_cast<int>(SemanticTokenType::REGEXP) == 9);
    REQUIRE(static_cast<int>(SemanticTokenType::EVENT) == 10);
    REQUIRE(static_cast<int>(SemanticTokenType::DECORATOR) == 11);
    REQUIRE(static_cast<int>(SemanticTokenType::APL_VALIDATOR) == 52);
}

TEST_CASE("SemanticTokenType to_string", "[semantic_token_types]") {
    REQUIRE(to_string(SemanticTokenType::KEYWORD) == "keyword");
    REQUIRE(to_string(SemanticTokenType::FUNCTION) == "function");
    REQUIRE(to_string(SemanticTokenType::VARIABLE) == "variable");
    REQUIRE(to_string(SemanticTokenType::REGEXP) == "regexp");
    REQUIRE(to_string(SemanticTokenType::APL_VALIDATOR) == "aplValidator");
}

TEST_CASE("SemanticTokenModifier to_string", "[semantic_token_types]") {
    REQUIRE(to_string(SemanticTokenModifier::DECLARATION) == "declaration");
    REQUIRE(to_string(SemanticTokenModifier::DEFINITION) == "definition");
    REQUIRE(to_string(SemanticTokenModifier::READONLY) == "readonly");
    REQUIRE(to_string(SemanticTokenModifier::DEFAULT_LIBRARY) == "defaultLibrary");
}

TEST_CASE("modifier_bit", "[semantic_token_types]") {
    REQUIRE(modifier_bit(SemanticTokenModifier::DECLARATION) == 0x01);
    REQUIRE(modifier_bit(SemanticTokenModifier::DEFINITION) == 0x02);
    REQUIRE(modifier_bit(SemanticTokenModifier::READONLY) == 0x04);
    REQUIRE(modifier_bit(SemanticTokenModifier::DEFAULT_LIBRARY) == 0x08);
}

TEST_CASE("SEMANTIC_TOKEN_TYPE_NAMES count", "[semantic_token_types]") {
    REQUIRE(SEMANTIC_TOKEN_TYPE_NAMES.size() == SEMANTIC_TOKEN_TYPE_COUNT);
    REQUIRE(SEMANTIC_TOKEN_TYPE_COUNT == 53);
}

TEST_CASE("SEMANTIC_TOKEN_MODIFIER_NAMES count", "[semantic_token_types]") {
    REQUIRE(SEMANTIC_TOKEN_MODIFIER_NAMES.size() == SEMANTIC_TOKEN_MODIFIER_COUNT);
    REQUIRE(SEMANTIC_TOKEN_MODIFIER_COUNT == 4);
}

TEST_CASE("SemanticToken default construction", "[semantic_token_types]") {
    SemanticToken tok{};
    REQUIRE(tok.line == 0);
    REQUIRE(tok.character == 0);
    REQUIRE(tok.length == 0);
    REQUIRE(tok.modifiers == 0);
}

TEST_CASE("SemanticToken equality", "[semantic_token_types]") {
    SemanticToken a{0, 0, 3, SemanticTokenType::KEYWORD, 0};
    SemanticToken b{0, 0, 3, SemanticTokenType::KEYWORD, 0};
    SemanticToken c{0, 0, 3, SemanticTokenType::FUNCTION, 0};
    REQUIRE(a == b);
    REQUIRE_FALSE(a == c);
}
