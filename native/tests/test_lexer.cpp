#include "tcl_lsp/parsing/lexer.hpp"

#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <string>
#include <vector>

using namespace tcl_lsp;

// Helper: tokenise, optionally filtering out SEP/EOL tokens.
static auto tokens(std::string_view source, bool include_sep = false) -> std::vector<Token> {
    TclLexer lexer(source);
    auto all = lexer.tokenise_all();
    if (include_sep)
        return all;
    std::vector<Token> result;
    for (auto& t : all) {
        if (t.type != TokenType::SEP && t.type != TokenType::EOL) {
            result.push_back(std::move(t));
        }
    }
    return result;
}

static auto texts(std::string_view source) -> std::vector<std::string> {
    auto toks = tokens(source);
    std::vector<std::string> result;
    for (auto& t : toks)
        result.push_back(t.text);
    return result;
}

static auto types(std::string_view source) -> std::vector<TokenType> {
    auto toks = tokens(source);
    std::vector<TokenType> result;
    for (auto& t : toks)
        result.push_back(t.type);
    return result;
}

// --- Basic Tokens ---

TEST_CASE("Simple command", "[lexer]") {
    CHECK(texts("puts hello") == std::vector<std::string>{"puts", "hello"});
}

TEST_CASE("Types simple", "[lexer]") {
    CHECK(types("puts hello") == std::vector<TokenType>{TokenType::ESC, TokenType::ESC});
}

TEST_CASE("Variable", "[lexer]") {
    auto toks = tokens("$foo");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "foo");
}

TEST_CASE("Bare dollar", "[lexer]") {
    auto toks = tokens("$");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "$");
}

TEST_CASE("Command substitution", "[lexer]") {
    auto toks = tokens("[+ 1 2]");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text == "+ 1 2");
}

TEST_CASE("Braces", "[lexer]") {
    auto toks = tokens("{hello world}");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "hello world");
}

TEST_CASE("Quoted string", "[lexer]") {
    auto toks = tokens("\"hello world\"");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "hello world");
}

TEST_CASE("Nested braces", "[lexer]") {
    auto toks = tokens("{a {b c} d}");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].text == "a {b c} d");
}

TEST_CASE("Nested brackets", "[lexer]") {
    auto toks = tokens("[+ 1 [+ 2 3]]");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text == "+ 1 [+ 2 3]");
}

TEST_CASE("Semicolon separator", "[lexer]") {
    auto toks = tokens("set a 1; set b 2", true);
    bool has_eol = false;
    for (auto& t : toks) {
        if (t.type == TokenType::EOL)
            has_eol = true;
    }
    CHECK(has_eol);
}

TEST_CASE("Comment", "[lexer]") {
    auto toks = tokens("# this is a comment\nputs hi");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::COMMENT);
    CHECK(toks[0].text.find("this is a comment") != std::string::npos);
}

TEST_CASE("Empty input", "[lexer]") {
    auto toks = tokens("");
    CHECK(toks.empty());
}

TEST_CASE("Multiple commands", "[lexer]") {
    CHECK(texts("set x 42") == std::vector<std::string>{"set", "x", "42"});
}

// --- Extended variable forms ---

TEST_CASE("Braced var", "[lexer]") {
    auto toks = tokens("${my var}");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "my var");
}

TEST_CASE("Namespace var", "[lexer]") {
    auto toks = tokens("$ns::var");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "ns::var");
}

TEST_CASE("Nested namespace var", "[lexer]") {
    auto toks = tokens("$a::b::c");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "a::b::c");
}

TEST_CASE("Array var", "[lexer]") {
    auto toks = tokens("$arr(idx)");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "arr(idx)");
}

TEST_CASE("Array with namespace", "[lexer]") {
    auto toks = tokens("$ns::arr(key)");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "ns::arr(key)");
}

// --- Positions ---

TEST_CASE("First token at origin", "[lexer]") {
    auto toks = tokens("puts hello");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].start.line == 0);
    CHECK(toks[0].start.character == 0);
    CHECK(toks[0].start.offset == 0);
}

TEST_CASE("Second token offset", "[lexer]") {
    auto toks = tokens("puts hello");
    REQUIRE(toks.size() >= 2);
    CHECK(toks[1].start.offset == 5);
    CHECK(toks[1].start.character == 5);
}

TEST_CASE("Multiline positions", "[lexer]") {
    auto toks = tokens("set x 1\nset y 2");
    std::vector<Token> sets;
    for (auto& t : toks) {
        if (t.text == "set")
            sets.push_back(t);
    }
    REQUIRE(sets.size() == 2);
    CHECK(sets[1].start.line == 1);
    CHECK(sets[1].start.character == 0);
}

TEST_CASE("Var position", "[lexer]") {
    auto toks = tokens("set x $y");
    std::vector<Token> vars;
    for (auto& t : toks) {
        if (t.type == TokenType::VAR)
            vars.push_back(t);
    }
    REQUIRE(vars.size() == 1);
    CHECK(vars[0].start.offset == 6);
    CHECK(vars[0].text == "y");
}

TEST_CASE("Cmd position", "[lexer]") {
    auto toks = tokens("set x [+ 1 2]");
    std::vector<Token> cmds;
    for (auto& t : toks) {
        if (t.type == TokenType::CMD)
            cmds.push_back(t);
    }
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].text == "+ 1 2");
}

TEST_CASE("Comment position", "[lexer]") {
    auto toks = tokens("# comment\nputs hi");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::COMMENT);
    CHECK(toks[0].start.line == 0);
}

TEST_CASE("End position", "[lexer]") {
    auto toks = tokens("puts");
    REQUIRE(toks.size() == 1);
    CHECK(toks[0].end.character == 3);
    CHECK(toks[0].end.offset == 3);
}

TEST_CASE("Multiline braced string", "[lexer]") {
    auto toks = tokens("{line1\nline2}");
    REQUIRE(toks.size() >= 1);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "line1\nline2");
    CHECK(toks[0].end.line == 1);
}

// --- Tcl constructs ---

TEST_CASE("If else", "[lexer]") {
    auto t = texts("if {== 1 1} {puts yes} else {puts no}");
    CHECK(t[0] == "if");
    CHECK(std::find(t.begin(), t.end(), "else") != t.end());
}

TEST_CASE("While loop", "[lexer]") {
    auto toks = tokens("while {< $i 10} {\n  set i [+ $i 1]\n}");
    CHECK(toks[0].text == "while");
}

TEST_CASE("Proc definition", "[lexer]") {
    auto t = texts("proc double {x} {return [* $x 2]}");
    CHECK(t[0] == "proc");
    CHECK(t[1] == "double");
}

TEST_CASE("String interpolation", "[lexer]") {
    auto toks = tokens("\"hello $name, result is [+ 1 2]!\"");
    bool has_var = false, has_cmd = false;
    for (auto& t : toks) {
        if (t.type == TokenType::VAR)
            has_var = true;
        if (t.type == TokenType::CMD)
            has_cmd = true;
    }
    CHECK(has_var);
    CHECK(has_cmd);
}

TEST_CASE("Backslash in string", "[lexer]") {
    auto toks = tokens("\"line1\\nline2\"");
    CHECK(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
}

TEST_CASE("Expr body", "[lexer]") {
    auto toks = tokens("expr {$x + $y * 2}");
    CHECK(toks[0].text == "expr");
    CHECK(toks[1].type == TokenType::STR);
}

TEST_CASE("Namespace eval", "[lexer]") {
    auto t = texts("namespace eval myns { proc foo {} {} }");
    CHECK(t[0] == "namespace");
}

TEST_CASE("Foreach", "[lexer]") {
    auto t = texts("foreach item $list { puts $item }");
    CHECK(t[0] == "foreach");
}

TEST_CASE("Switch", "[lexer]") {
    auto t = texts("switch $x { a {puts A} b {puts B} }");
    CHECK(t[0] == "switch");
}

// --- Trailing whitespace ---

TEST_CASE("Sep does not consume newline", "[lexer]") {
    auto toks = tokens("set a {body}    \nset b val", true);
    bool has_eol = false;
    for (auto& t : toks) {
        if (t.type == TokenType::EOL)
            has_eol = true;
    }
    CHECK(has_eol);
    int set_count = 0;
    for (auto& t : toks) {
        if (t.type == TokenType::ESC && t.text == "set")
            set_count++;
    }
    CHECK(set_count == 2);
}

TEST_CASE("Command after brace with trailing spaces", "[lexer]") {
    auto toks = tokens("switch -- $foo {\n    default {}\n}    \ntable set foo1 bar1");
    bool has_table = false;
    for (auto& t : toks) {
        if (t.text == "table") {
            has_table = true;
            CHECK(t.type == TokenType::ESC);
        }
    }
    CHECK(has_table);
}

// --- Strict quoting mode ---

TEST_CASE("Strict mode raises on missing close-bracket", "[lexer]") {
    LexerConfig cfg{.strict_quoting = true};
    TclLexer lexer("[hello", cfg);
    CHECK_THROWS_AS(lexer.tokenise_all(), TclParseError);
}

TEST_CASE("Strict mode raises on missing close-brace", "[lexer]") {
    LexerConfig cfg{.strict_quoting = true};
    TclLexer lexer("{hello", cfg);
    CHECK_THROWS_AS(lexer.tokenise_all(), TclParseError);
}

TEST_CASE("Strict mode raises on missing close-quote", "[lexer]") {
    LexerConfig cfg{.strict_quoting = true};
    TclLexer lexer("\"hello", cfg);
    CHECK_THROWS_AS(lexer.tokenise_all(), TclParseError);
}

TEST_CASE("Non-strict mode collects warnings", "[lexer]") {
    TclLexer lexer("[hello");
    auto toks = lexer.tokenise_all();
    CHECK(!lexer.warnings().empty());
    CHECK(lexer.warnings()[0].second == "missing close-bracket");
}

// --- Expand syntax ---

TEST_CASE("Expand syntax recognized", "[lexer]") {
    auto toks = tokens("{*}$list");
    bool has_expand = false;
    for (auto& t : toks) {
        if (t.type == TokenType::EXPAND)
            has_expand = true;
    }
    CHECK(has_expand);
}

TEST_CASE("Expand syntax disabled", "[lexer]") {
    LexerConfig cfg{.expand_syntax = false};
    TclLexer lexer("{*}$list", cfg);
    auto toks = lexer.tokenise_all();
    // Should be parsed as a braced string {*} followed by $list.
    bool has_str = false;
    for (auto& t : toks) {
        if (t.type == TokenType::STR && t.text == "*")
            has_str = true;
    }
    CHECK(has_str);
}

// --- iRules brace separator ---

TEST_CASE("iRules brace separator injects SEP", "[lexer]") {
    LexerConfig cfg{.irules_brace_separator = true};
    TclLexer lexer("if {cond}{body}", cfg);
    auto toks = lexer.tokenise_all();
    bool has_sep = false;
    for (auto& t : toks) {
        if (t.type == TokenType::SEP)
            has_sep = true;
    }
    CHECK(has_sep);
}

// --- Base offset support ---

TEST_CASE("Base offset shifts positions", "[lexer]") {
    TclLexer lexer("puts", {}, /*base_offset=*/100, /*base_line=*/5, /*base_col=*/10);
    auto toks = lexer.tokenise_all();
    // First content token should have base offset applied.
    Token* puts_tok = nullptr;
    for (auto& t : toks) {
        if (t.text == "puts") {
            puts_tok = &t;
            break;
        }
    }
    REQUIRE(puts_tok != nullptr);
    CHECK(puts_tok->start.offset == 100);
    CHECK(puts_tok->start.line == 5);
    CHECK(puts_tok->start.character == 10);
}

// --- Virtual insertions ---

TEST_CASE("Virtual insertion closes bracket", "[lexer]") {
    std::unordered_map<int32_t, char> vi = {{6, ']'}};
    TclLexer lexer("[hello", {}, 0, 0, 0, vi);
    auto toks = lexer.tokenise_all();
    bool has_cmd = false;
    for (auto& t : toks) {
        if (t.type == TokenType::CMD)
            has_cmd = true;
    }
    CHECK(has_cmd);
    CHECK(lexer.warnings().empty());
}
