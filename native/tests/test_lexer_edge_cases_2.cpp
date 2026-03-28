#include "tcl_lsp/parsing/lexer.hpp"
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
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

static auto lex_texts(std::string_view source) -> std::vector<std::string> {
    auto toks = lex(source);
    std::vector<std::string> result;
    for (auto& t : toks) result.push_back(t.text);
    return result;
}

static auto lex_with_warnings(std::string_view source)
    -> std::pair<std::vector<Token>, std::vector<std::pair<SourcePosition, std::string>>> {
    TclLexer lexer(source);
    auto all = lexer.tokenise_all();
    std::vector<Token> filtered;
    for (auto& t : all) {
        if (t.type != TokenType::SEP && t.type != TokenType::EOL)
            filtered.push_back(std::move(t));
    }
    return {std::move(filtered), lexer.warnings()};
}

static auto lex_strict(std::string_view source) -> std::vector<Token> {
    LexerConfig cfg{.strict_quoting = true};
    TclLexer lexer(source, cfg);
    auto all = lexer.tokenise_all();
    std::vector<Token> result;
    for (auto& t : all) {
        if (t.type != TokenType::SEP && t.type != TokenType::EOL)
            result.push_back(std::move(t));
    }
    return result;
}

TEST_CASE("Array index: nested parens", "[lexer][edge-case]") {
    auto toks = lex("$arr((nested))");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr((nested))");
}

TEST_CASE("Array index: variable in index", "[lexer][edge-case]") {
    auto toks = lex("$arr($inner)");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr($inner)");
}

TEST_CASE("Array index: escaped brackets in index", "[lexer][edge-case]") {
    auto toks = lex(R"($arr(\[idx\]))");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text.find("arr(") != std::string::npos);
}

TEST_CASE("Array index: braces in index", "[lexer][edge-case]") {
    auto toks = lex("$arr({key})");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr({key})");
}

TEST_CASE("Array index: quotes in index", "[lexer][edge-case]") {
    auto toks = lex(R"($arr("key"))");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == R"(arr("key"))");
}

TEST_CASE("Array index: spaces in index", "[lexer][edge-case]") {
    auto toks = lex("$arr(a b)");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr(a b)");
}

TEST_CASE("Array index: deeply nested parens", "[lexer][edge-case]") {
    auto toks = lex("$arr(((deep)))");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr(((deep)))");
}

TEST_CASE("Array index: namespace array", "[lexer][edge-case]") {
    auto toks = lex("$ns::arr(key)");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "ns::arr(key)");
}

TEST_CASE("Array index: empty index", "[lexer][edge-case]") {
    auto toks = lex("$arr()");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr()");
}

TEST_CASE("Array index: semicolon in index", "[lexer][edge-case]") {
    auto toks = lex("$arr(a;b)");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr(a;b)");
}

TEST_CASE("Special names via braces: set var named dollar-a", "[lexer][edge-case]") {
    auto toks = lex("set {$a} 1");
    REQUIRE(toks.size() >= 3);
    CHECK(toks[0].text == "set");
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "$a");
}

TEST_CASE("Special names via braces: set var named bracket-cmd", "[lexer][edge-case]") {
    auto toks = lex("set {[cmd]} 1");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "[cmd]");
}

TEST_CASE("Special names via braces: set var named quote", "[lexer][edge-case]") {
    auto toks = lex(R"(set {"} 1)");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "\"");
}

TEST_CASE("Special names via braces: proc named escaped dollar", "[lexer][edge-case]") {
    auto toks = lex(R"(proc \$ {} {puts dollar})");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[0].text == "proc");
    CHECK(toks[1].type == TokenType::ESC);
    CHECK(toks[1].text == "\\$");
}

TEST_CASE("Special names via braces: proc named escaped bracket", "[lexer][edge-case]") {
    auto toks = lex(R"(proc \[ {} {puts bracket})");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[0].text == "proc");
    CHECK(toks[1].type == TokenType::ESC);
    CHECK(toks[1].text == "\\[");
}

TEST_CASE("Special names via braces: proc with spaces in name", "[lexer][edge-case]") {
    auto toks = lex("proc {weird name} {} {}");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "weird name");
}

TEST_CASE("Special names via braces: proc name all special chars", "[lexer][edge-case]") {
    auto toks = lex("proc {$[]{}} {} {}");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "$[]{}");
}

TEST_CASE("Empty structures: empty quoted string", "[lexer][edge-case]") {
    auto toks = lex(R"("")");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "");
}

TEST_CASE("Empty structures: empty braces", "[lexer][edge-case]") {
    auto toks = lex("{}");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "");
}

TEST_CASE("Empty structures: empty cmd sub", "[lexer][edge-case]") {
    auto toks = lex("[]");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text == "");
}

TEST_CASE("Empty structures: whitespace cmd sub", "[lexer][edge-case]") {
    auto toks = lex("[  ]");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::CMD);
}

TEST_CASE("Empty structures: single char tokens", "[lexer][edge-case]") {
    for (char ch : std::string("abcxyz019_")) {
        auto toks = lex(std::string(1, ch));
        REQUIRE(toks.size() == 1);
        CHECK(toks[0].text == std::string(1, ch));
    }
}

TEST_CASE("Bare delimiters: bare close bracket", "[lexer][edge-case]") {
    auto toks = lex("]");
    REQUIRE(!toks.empty());
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "]");
}

TEST_CASE("Bare delimiters: bare close bracket as command", "[lexer][edge-case]") {
    auto toks = lex("] foo");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[0].text == "]");
    CHECK(toks[1].text == "foo");
}

TEST_CASE("Bare delimiters: bare close brace is text", "[lexer][edge-case]") {
    auto toks = lex("} foo");
    auto texts = lex_texts("} foo");
    bool has_brace = texts[0].find("}") != std::string::npos;
    bool has_foo = std::find(texts.begin(), texts.end(), "foo") != texts.end();
    CHECK((has_brace || has_foo));
}

TEST_CASE("Backslash-newline: continuation in bare word", "[lexer][edge-case]") {
    auto texts = lex_texts("set x \\\nhello");
    CHECK(std::find(texts.begin(), texts.end(), "set") != texts.end());
    CHECK(std::find(texts.begin(), texts.end(), "x") != texts.end());
}

TEST_CASE("Backslash-newline: continuation in quoted string", "[lexer][edge-case]") {
    auto toks = lex("\"hello \\\n  world\"");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text.find("hello") != std::string::npos);
    CHECK(toks[0].text.find("world") != std::string::npos);
}

TEST_CASE("Backslash-newline: continuation in comment", "[lexer][edge-case]") {
    auto toks = lex("# comment \\\ncontinued");
    std::vector<Token> comments;
    for (auto& t : toks) {
        if (t.type == TokenType::COMMENT) comments.push_back(t);
    }
    REQUIRE(comments.size() == 1);
    CHECK(comments[0].text.find("continued") != std::string::npos);
}

TEST_CASE("Backslash-newline: backslash at EOF in word", "[lexer][edge-case]") {
    auto toks = lex("set x \\");
    CHECK(toks.size() >= 2);
}

TEST_CASE("Error cases: unclosed quote warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("\"hello");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("missing \"") != std::string::npos;
    }));
}

TEST_CASE("Error cases: unclosed quote strict raises", "[lexer][edge-case]") {
    CHECK_THROWS_AS(lex_strict("\"hello"), TclParseError);
    CHECK_THROWS_WITH(lex_strict("\"hello"), Catch::Matchers::ContainsSubstring("missing \""));
}

TEST_CASE("Error cases: unclosed brace warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("{hello");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("missing close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: unclosed brace strict raises", "[lexer][edge-case]") {
    CHECK_THROWS_AS(lex_strict("{hello"), TclParseError);
    CHECK_THROWS_WITH(lex_strict("{hello"), Catch::Matchers::ContainsSubstring("missing close-brace"));
}

TEST_CASE("Error cases: unclosed bracket warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("[hello");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("missing close-bracket") != std::string::npos;
    }));
}

TEST_CASE("Error cases: unclosed bracket strict raises", "[lexer][edge-case]") {
    CHECK_THROWS_AS(lex_strict("[hello"), TclParseError);
    CHECK_THROWS_WITH(lex_strict("[hello"), Catch::Matchers::ContainsSubstring("missing close-bracket"));
}

TEST_CASE("Error cases: extra chars after quote warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("\"hello\"world");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-quote") != std::string::npos;
    }));
}

TEST_CASE("Error cases: extra chars after quote strict raises", "[lexer][edge-case]") {
    CHECK_THROWS_AS(lex_strict("\"hello\"world"), TclParseError);
    CHECK_THROWS_WITH(lex_strict("\"hello\"world"), Catch::Matchers::ContainsSubstring("extra characters after close-quote"));
}

TEST_CASE("Error cases: extra chars after brace warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("{hello}world");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: extra chars after brace strict raises", "[lexer][edge-case]") {
    CHECK_THROWS_AS(lex_strict("{hello}world"), TclParseError);
    CHECK_THROWS_WITH(lex_strict("{hello}world"), Catch::Matchers::ContainsSubstring("extra characters after close-brace"));
}

TEST_CASE("Error cases: backslash-newline after close-brace no warning", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("{hello}\\\n world");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash-newline after close-brace strict no raise", "[lexer][edge-case]") {
    auto result = lex_strict("{hello}\\\n world");
    CHECK(!result.empty());
}

TEST_CASE("Error cases: backslash non-newline after close-brace warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("{hello}\\world");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: spicegentcl testtemplate pattern", "[lexer][edge-case]") {
    std::string source =
        "testTemplate testResistorClass-1 {} "
        "{Resistor new 1 netp netm -r 1e3 -tc1 1 -ac 1e6 -temp 25}\\\n"
        "        {r1 netp netm 1e3 tc1=1 ac=1e6 temp=25}";
    auto [toks, warnings] = lex_with_warnings(source);
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: multiple backslash continuations after braces", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("cmd {arg1}\\\n        {arg2}\\\n        {arg3}");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: empty braces backslash continuation", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("cmd {}\\\n        {arg}");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash CRLF after close-brace no warning", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("{hello}\\\r\n world");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash CR after close-brace no warning", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("{hello}\\\r world");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-brace") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash-newline after close-quote no warning", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("\"hello\"\\\n world");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-quote") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash CRLF after close-quote no warning", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("\"hello\"\\\r\n world");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-quote") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash CR after close-quote no warning", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("\"hello\"\\\r world");
    CHECK(!std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-quote") != std::string::npos;
    }));
}

TEST_CASE("Error cases: backslash non-newline after close-quote warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("\"hello\"\\world");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("extra characters after close-quote") != std::string::npos;
    }));
}

TEST_CASE("Error cases: unclosed braced var warns", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("${name");
    CHECK(std::any_of(warnings.begin(), warnings.end(), [](auto& w) {
        return w.second.find("missing close-brace for variable name") != std::string::npos;
    }));
}

TEST_CASE("Error cases: unclosed braced var strict raises", "[lexer][edge-case]") {
    CHECK_THROWS_AS(lex_strict("${name"), TclParseError);
    CHECK_THROWS_WITH(lex_strict("${name"), Catch::Matchers::ContainsSubstring("missing close-brace for variable name"));
}

TEST_CASE("Error cases: unclosed brace in cmd sub", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("[a {b c]");
    CHECK(toks.size() >= 1);
}

TEST_CASE("Error cases: unclosed quote in cmd sub", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings(R"([a "b c])");
    CHECK(toks.size() >= 1);
}
