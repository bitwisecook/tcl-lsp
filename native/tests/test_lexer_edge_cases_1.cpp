#include "tcl_lsp/parsing/lexer.hpp"

#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <string>
#include <vector>

using namespace tcl_lsp;

static auto lex(std::string_view source) -> std::vector<Token> {
    TclLexer lexer(source);
    auto all = lexer.tokenise_all();
    std::vector<Token> result;
    for (auto& t : all) {
        if (t.type != TokenType::SEP && t.type != TokenType::EOL)
            result.push_back(std::move(t));
    }
    return result;
}

static auto lex_types(std::string_view source) -> std::vector<TokenType> {
    auto toks = lex(source);
    std::vector<TokenType> result;
    for (auto& t : toks)
        result.push_back(t.type);
    return result;
}

// Helper: lex including warnings (non-strict mode).
static auto lex_with_warnings(std::string_view source)
    -> std::pair<std::vector<Token>, std::vector<std::pair<SourcePosition, std::string>>> {
    TclLexer lexer(source);
    std::vector<Token> toks;
    while (true) {
        auto t = lexer.get_token();
        if (!t.has_value())
            break;
        if (t->type != TokenType::SEP && t->type != TokenType::EOL)
            toks.push_back(std::move(*t));
    }
    return {std::move(toks), lexer.warnings()};
}

// Group 1: Escaped special characters

TEST_CASE("Escaped dollar is literal ESC", "[lexer][edge-case]") {
    auto toks = lex("\\$");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "\\$");
}

TEST_CASE("Escaped dollar before name is literal ESC", "[lexer][edge-case]") {
    auto toks = lex("\\$a");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "\\$a");
}

TEST_CASE("Escaped open bracket is literal ESC", "[lexer][edge-case]") {
    auto toks = lex("\\[cmd\\]");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "\\[cmd\\]");
}

TEST_CASE("Escaped open brace is literal ESC", "[lexer][edge-case]") {
    auto toks = lex("\\{body\\}");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "\\{body\\}");
}

TEST_CASE("Escaped double quote is literal ESC", "[lexer][edge-case]") {
    auto toks = lex(R"(\"hello\")");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == R"(\"hello\")");
}

TEST_CASE("Escaped semicolon keeps word together", "[lexer][edge-case]") {
    auto toks = lex("a\\;b");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "a\\;b");
}

TEST_CASE("Escaped hash is not a comment", "[lexer][edge-case]") {
    auto toks = lex("\\#notacomment");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
}

TEST_CASE("Escaped space joins words", "[lexer][edge-case]") {
    auto toks = lex("hello\\ world");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "hello\\ world");
}

TEST_CASE("Escaped newline continues word", "[lexer][edge-case]") {
    auto toks = lex("foo\\\nbar");
    std::vector<std::string> texts;
    for (auto& t : toks) {
        if (t.type == TokenType::ESC)
            texts.push_back(t.text);
    }
    CHECK(texts.size() >= 1);
}

// Group 2: Braced variable special characters

TEST_CASE("Braced var with double quote name", "[lexer][edge-case]") {
    auto toks = lex(R"_x_(${")_x_");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "\"");
}

TEST_CASE("Braced var with open bracket name", "[lexer][edge-case]") {
    auto toks = lex("${[}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "[");
}

TEST_CASE("Braced var with close bracket name", "[lexer][edge-case]") {
    auto toks = lex("${]}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "]");
}

TEST_CASE("Braced var with dollar name", "[lexer][edge-case]") {
    auto toks = lex("${$}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "$");
}

TEST_CASE("Braced var with backslash name", "[lexer][edge-case]") {
    auto toks = lex("${\\}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "\\");
}

TEST_CASE("Braced var with space name", "[lexer][edge-case]") {
    auto toks = lex("${ }");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == " ");
}

TEST_CASE("Braced var with spaces in name", "[lexer][edge-case]") {
    auto toks = lex("${a b c}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "a b c");
}

TEST_CASE("Braced var with empty name", "[lexer][edge-case]") {
    auto toks = lex("${}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "");
}

TEST_CASE("Braced var with semicolons in name", "[lexer][edge-case]") {
    auto toks = lex("${a;b}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "a;b");
}

TEST_CASE("Braced var with newlines in name", "[lexer][edge-case]") {
    auto toks = lex("${a\nb}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "a\nb");
}

TEST_CASE("Braced var with open brace unclosed", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("${{}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "{");
}

TEST_CASE("Braced var with nested braces", "[lexer][edge-case]") {
    auto toks = lex("${a{b}c}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "a{b");
}

TEST_CASE("Braced var with equals and colons", "[lexer][edge-case]") {
    auto toks = lex("${foo::bar=baz}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "foo::bar=baz");
}

// Group 3: Dollar edge cases

TEST_CASE("Bare dollar at EOF is literal", "[lexer][edge-case]") {
    auto toks = lex("$");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
}

TEST_CASE("Bare dollar before space is literal", "[lexer][edge-case]") {
    auto toks = lex("$ x");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
}

TEST_CASE("Bare dollar before semicolon is literal", "[lexer][edge-case]") {
    auto toks = lex("$;");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
}

TEST_CASE("Double dollar - first bare, second starts var", "[lexer][edge-case]") {
    auto toks = lex("$$foo");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
    CHECK(toks[1].type == TokenType::VAR);
    CHECK(toks[1].text == "foo");
}

TEST_CASE("Triple dollar - first two bare, third starts var", "[lexer][edge-case]") {
    auto toks = lex("$$$x");
    REQUIRE(toks.size() >= 3);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "$");
    CHECK(toks[2].type == TokenType::VAR);
    CHECK(toks[2].text == "x");
}

TEST_CASE("Dollar before command substitution", "[lexer][edge-case]") {
    auto toks = lex("$[set x 1]");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
    CHECK(toks[1].type == TokenType::CMD);
    CHECK(toks[1].text == "set x 1");
}

TEST_CASE("Dollar paren is scalar variable ref", "[lexer][edge-case]") {
    auto toks = lex("$(foo)");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "(foo)");
}

TEST_CASE("Dollar namespace global variable", "[lexer][edge-case]") {
    auto toks = lex("$::foo");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "::foo");
}

TEST_CASE("Dollar double colon alone is valid var ref", "[lexer][edge-case]") {
    auto toks = lex("$::");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "::");
}

TEST_CASE("Dollar single colon is bare dollar", "[lexer][edge-case]") {
    auto toks = lex("$:");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
}

// Group 4: Deep nesting

TEST_CASE("5-deep command substitution", "[lexer][edge-case]") {
    auto toks = lex("[[[[[set x 1]]]]]");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text == "[[[[set x 1]]]]");
}

TEST_CASE("5-deep braces", "[lexer][edge-case]") {
    auto toks = lex("{{{{{body}}}}}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "{{{{body}}}}");
}

TEST_CASE("Realistic deep nesting with set and expr", "[lexer][edge-case]") {
    auto toks = lex("[set x [set y [set z [expr {1+2}]]]]");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text.find("set x [set y") != std::string::npos);
    CHECK(toks[0].text.find("expr {1+2}") != std::string::npos);
}

TEST_CASE("10-deep command substitution", "[lexer][edge-case]") {
    std::string inner = "set x 1";
    for (int i = 0; i < 9; ++i) {
        inner = "[" + inner + "]";
    }
    std::string source = "[" + inner + "]";
    auto toks = lex(source);
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text.find("set x 1") != std::string::npos);
}

TEST_CASE("10-deep braces", "[lexer][edge-case]") {
    std::string inner = "body";
    for (int i = 0; i < 10; ++i) {
        inner = "{" + inner + "}";
    }
    auto toks = lex(inner);
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
}

TEST_CASE("Alternating braces and brackets", "[lexer][edge-case]") {
    auto toks = lex("[if {1} {[if {2} {[expr {3}]}]}]");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text.find("expr {3}") != std::string::npos);
}

// Group 5: Mixed nesting

TEST_CASE("Cmd sub containing braces with brackets", "[lexer][edge-case]") {
    auto toks = lex("[set x {[puts hello]}]");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text == "set x {[puts hello]}");
}

TEST_CASE("Quoted string with cmd sub with braces", "[lexer][edge-case]") {
    auto toks = lex(R"("[set x {hello world}]")");
    auto types = lex_types(R"("[set x {hello world}]")");
    CHECK(std::any_of(types.begin(), types.end(), [](TokenType t) { return t == TokenType::CMD; }));
    auto it = std::find_if(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::CMD; });
    REQUIRE(it != toks.end());
    CHECK(it->text == "set x {hello world}");
}

TEST_CASE("Cmd sub with quoted brackets", "[lexer][edge-case]") {
    auto toks = lex(R"([string map {"[" "("} $x])");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text.find(R"("[")") != std::string::npos);
}

TEST_CASE("Braces protect everything", "[lexer][edge-case]") {
    auto toks = lex(R"({$var [cmd] "quotes" \escape})");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == R"($var [cmd] "quotes" \escape)");
}

TEST_CASE("Quotes inside braces are literal", "[lexer][edge-case]") {
    auto toks = lex(R"({a"b"c})");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == R"(a"b"c)");
}

TEST_CASE("Brackets inside braces are literal", "[lexer][edge-case]") {
    auto toks = lex("{a[b]c}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "a[b]c");
}

TEST_CASE("Dollar inside braces is literal", "[lexer][edge-case]") {
    auto toks = lex("{a$bc}");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "a$bc");
}

TEST_CASE("Braces inside quotes are literal", "[lexer][edge-case]") {
    auto toks = lex(R"("a{b}c")");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "a{b}c");
}

TEST_CASE("Hash after semicolon is comment", "[lexer][edge-case]") {
    auto toks = lex("set x 1 ;# trailing comment");
    auto types = lex_types("set x 1 ;# trailing comment");
    CHECK(std::any_of(
        types.begin(), types.end(), [](TokenType t) { return t == TokenType::COMMENT; }));
    CHECK(toks.back().type == TokenType::COMMENT);
}

TEST_CASE("Hash in argument position is not comment", "[lexer][edge-case]") {
    auto toks = lex("set x #notacomment");
    auto types = lex_types("set x #notacomment");
    CHECK(!std::any_of(
        types.begin(), types.end(), [](TokenType t) { return t == TokenType::COMMENT; }));
    CHECK(toks.back().type == TokenType::ESC);
    CHECK(toks.back().text == "#notacomment");
}

TEST_CASE("Hash in quotes is not comment", "[lexer][edge-case]") {
    auto types = lex_types(R"(set x "#text")");
    CHECK(!std::any_of(
        types.begin(), types.end(), [](TokenType t) { return t == TokenType::COMMENT; }));
}

TEST_CASE("Hash in braces is not comment", "[lexer][edge-case]") {
    auto types = lex_types("set x {#text}");
    CHECK(!std::any_of(
        types.begin(), types.end(), [](TokenType t) { return t == TokenType::COMMENT; }));
}

// Group 6: Interleaved substitutions

TEST_CASE("Consecutive vars in quotes", "[lexer][edge-case]") {
    auto toks = lex(R"("$a$b$c")");
    std::vector<Token> vars;
    for (auto& t : toks) {
        if (t.type == TokenType::VAR)
            vars.push_back(t);
    }
    REQUIRE(vars.size() == 3);
    CHECK(vars[0].text == "a");
    CHECK(vars[1].text == "b");
    CHECK(vars[2].text == "c");
}

TEST_CASE("Var cmd var in quotes", "[lexer][edge-case]") {
    auto toks = lex(R"("$a[cmd]$b")");
    std::vector<TokenType> types_no_esc;
    for (auto& t : toks) {
        if (t.type != TokenType::ESC)
            types_no_esc.push_back(t.type);
    }
    CHECK(types_no_esc == std::vector<TokenType>{TokenType::VAR, TokenType::CMD, TokenType::VAR});
}

TEST_CASE("Consecutive cmd subs in quotes", "[lexer][edge-case]") {
    auto toks = lex(R"("[a][b][c]")");
    std::vector<Token> cmds;
    for (auto& t : toks) {
        if (t.type == TokenType::CMD)
            cmds.push_back(t);
    }
    REQUIRE(cmds.size() == 3);
    CHECK(cmds[0].text == "a");
    CHECK(cmds[1].text == "b");
    CHECK(cmds[2].text == "c");
}

TEST_CASE("Escaped dollar in quotes is not var", "[lexer][edge-case]") {
    auto types = lex_types(R"("\$notavar")");
    CHECK(
        !std::any_of(types.begin(), types.end(), [](TokenType t) { return t == TokenType::VAR; }));
}

TEST_CASE("Escaped bracket in quotes is not cmd", "[lexer][edge-case]") {
    auto types = lex_types(R"("\[notacmd]")");
    CHECK(
        !std::any_of(types.begin(), types.end(), [](TokenType t) { return t == TokenType::CMD; }));
}

TEST_CASE("Text var text cmd text in quotes", "[lexer][edge-case]") {
    auto toks = lex(R"("hello $name, result=[calc]!")");
    std::vector<TokenType> types_no_esc;
    for (auto& t : toks) {
        if (t.type != TokenType::ESC)
            types_no_esc.push_back(t.type);
    }
    CHECK(types_no_esc == std::vector<TokenType>{TokenType::VAR, TokenType::CMD});
    auto var_it = std::find_if(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::VAR; });
    REQUIRE(var_it != toks.end());
    CHECK(var_it->text == "name");
    auto cmd_it = std::find_if(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::CMD; });
    REQUIRE(cmd_it != toks.end());
    CHECK(cmd_it->text == "calc");
}
