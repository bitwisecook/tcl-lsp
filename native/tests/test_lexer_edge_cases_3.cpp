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

// Group 13: TestExpandEdgeCases

TEST_CASE("Expand basic", "[lexer][edge-case]") {
    auto toks = lex("{*}list");
    auto has_expand = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::EXPAND; });
    CHECK(has_expand);
}

TEST_CASE("Expand with braces", "[lexer][edge-case]") {
    auto toks = lex("{*}{body}");
    auto has_expand = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::EXPAND; });
    auto has_str = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::STR; });
    CHECK(has_expand);
    CHECK(has_str);
}

TEST_CASE("Expand with var", "[lexer][edge-case]") {
    auto toks = lex("{*}$var");
    auto has_expand = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::EXPAND; });
    auto has_var = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::VAR; });
    CHECK(has_expand);
    CHECK(has_var);
}

TEST_CASE("Expand with cmd sub", "[lexer][edge-case]") {
    auto toks = lex("{*}[cmd]");
    auto has_expand = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::EXPAND; });
    auto has_cmd = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::CMD; });
    CHECK(has_expand);
    CHECK(has_cmd);
}

TEST_CASE("Expand with quoted string", "[lexer][edge-case]") {
    auto toks = lex(R"({*}"hello")");
    auto has_expand = std::any_of(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::EXPAND; });
    CHECK(has_expand);
}

TEST_CASE("Star brace not expand", "[lexer][edge-case]") {
    auto toks = lex("{**}");
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "**");
}

TEST_CASE("Expand at eof", "[lexer][edge-case]") {
    auto toks = lex("{*}");
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "*");
}

TEST_CASE("Expand before space", "[lexer][edge-case]") {
    auto toks = lex("{*} list");
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "*");
}

// Group 14: TestDiabolicalCombinations

TEST_CASE("Var in quoted string in cmd sub", "[lexer][edge-case]") {
    auto toks = lex(R"([puts "$var"])");
    CHECK(toks[0].type == TokenType::CMD);
    CHECK(toks[0].text.find("$var") != std::string::npos);
}

TEST_CASE("Cmd sub in array index in quoted string", "[lexer][edge-case]") {
    auto toks = lex(R"x("$arr([cmd])")x");
    std::vector<Token> vars;
    for (auto& t : toks) {
        if (t.type == TokenType::VAR)
            vars.push_back(t);
    }
    REQUIRE(vars.size() >= 1);
    CHECK(vars[0].text.find("arr([cmd])") != std::string::npos);
}

TEST_CASE("Nested quotes via escaping", "[lexer][edge-case]") {
    auto toks = lex(R"("hello \"world\" end")");
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text.find("\\\"world\\\"") != std::string::npos);
}

TEST_CASE("Backslash brace in braces", "[lexer][edge-case]") {
    auto toks = lex(R"({hello \} world})");
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "hello \\} world");
}

TEST_CASE("Backslash open brace in braces", "[lexer][edge-case]") {
    auto toks = lex(R"({hello \{ world})");
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text == "hello \\{ world");
}

TEST_CASE("String map with special chars", "[lexer][edge-case]") {
    auto toks = lex(R"x([string map {"\[" "(" "\]" ")"} $text])x");
    CHECK(toks[0].type == TokenType::CMD);
}

TEST_CASE("Switch with multiple bodies", "[lexer][edge-case]") {
    auto source = "switch -- $x {\n    a {puts A}\n    b {puts B}\n    default {puts C}\n}";
    auto toks = lex(source);
    CHECK(toks[0].text == "switch");
}

TEST_CASE("Proc body with nested procs", "[lexer][edge-case]") {
    auto source = "proc outer {} {\n    proc inner {} {\n        puts hello\n    }\n}";
    auto toks = lex(source);
    CHECK(toks[0].text == "proc");
    std::vector<Token> body_toks;
    for (auto& t : toks) {
        if (t.type == TokenType::STR && t.text.find("inner") != std::string::npos)
            body_toks.push_back(t);
    }
    CHECK(body_toks.size() >= 1);
}

TEST_CASE("Multiple semicolons", "[lexer][edge-case]") {
    auto toks = lex("set a 1 ;; set b 2");
    int set_count = 0;
    for (auto& t : toks) {
        if (t.type == TokenType::ESC && t.text == "set")
            set_count++;
    }
    CHECK(set_count == 2);
}

TEST_CASE("Escaped everything in one word", "[lexer][edge-case]") {
    auto source = R"(\$\[\]\{\}\"\;\#\\)";
    auto toks = lex(source);
    CHECK(toks.size() == 1);
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text.find("\\$") != std::string::npos);
    CHECK(toks[0].text.find("\\[") != std::string::npos);
}

TEST_CASE("Var in cmd sub in var array index", "[lexer][edge-case]") {
    auto toks = lex("$outer([set inner $x])");
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text.find("outer(") != std::string::npos);
}

TEST_CASE("Deeply nested braces with backslash braces", "[lexer][edge-case]") {
    auto source = R"({level1 {level2 \{ \} level2} level1})";
    auto toks = lex(source);
    CHECK(toks[0].type == TokenType::STR);
    CHECK(toks[0].text.find("level2") != std::string::npos);
}

TEST_CASE("Unmatched open brace in quoted string", "[lexer][edge-case]") {
    auto toks = lex(R"("hello { world")");
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "hello { world");
}

TEST_CASE("Unmatched close brace in quoted string", "[lexer][edge-case]") {
    auto toks = lex(R"("hello } world")");
    CHECK(toks[0].type == TokenType::ESC);
    CHECK(toks[0].text == "hello } world");
}

// Group 15: TestPositionUnderStress

TEST_CASE("Braced var positions", "[lexer][edge-case]") {
    auto toks = lex("${a b}");
    CHECK(toks[0].start.offset == 0);
    CHECK(toks[0].start.line == 0);
}

TEST_CASE("Nested cmd sub positions", "[lexer][edge-case]") {
    auto toks = lex("set [expr {1}]");
    auto it = std::find_if(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::CMD; });
    REQUIRE(it != toks.end());
    CHECK(it->start.offset == 4);
}

TEST_CASE("Multiline brace end position", "[lexer][edge-case]") {
    auto source = "{line1\nline2\nline3}";
    auto toks = lex(source);
    CHECK(toks[0].end.line == 2);
}

TEST_CASE("Quoted string with escapes positions", "[lexer][edge-case]") {
    auto source = R"("hello \"world\"")";
    auto toks = lex(source);
    CHECK(toks[0].start.offset == 0);
    CHECK(toks[0].end.offset > 0);
}

TEST_CASE("Continuation position", "[lexer][edge-case]") {
    auto source = "set x \\\nhello";
    auto toks = lex(source);
    auto& last_tok = toks.back();
    if (last_tok.start.line == 1) {
        CHECK(last_tok.start.character >= 0);
    }
}

TEST_CASE("Comment continuation position", "[lexer][edge-case]") {
    auto source = "# comment \\\ncontinued\nset x 1";
    auto toks = lex(source);
    auto it = std::find_if(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::COMMENT; });
    REQUIRE(it != toks.end());
    CHECK(it->start.line == 0);
    CHECK(it->end.line == 1);
}

// Group 16: TestRealisticMeanPatterns

TEST_CASE("Regexp with brackets", "[lexer][edge-case]") {
    auto toks = lex("regexp {[a-z]+} $str match");
    CHECK(toks[0].text == "regexp");
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text == "[a-z]+");
}

TEST_CASE("Regsub with backref", "[lexer][edge-case]") {
    auto toks = lex(R"(regsub -all {(\w+)} $str {\1} result)");
    CHECK(toks[0].text == "regsub");
}

TEST_CASE("Format string", "[lexer][edge-case]") {
    auto toks = lex(R"(format "%s = %d" $name [expr {$x+1}])");
    CHECK(toks[0].text == "format");
    auto it = std::find_if(
        toks.begin(), toks.end(), [](const Token& t) { return t.type == TokenType::CMD; });
    REQUIRE(it != toks.end());
    CHECK(it->text.find("expr") != std::string::npos);
}

TEST_CASE("Double eval pattern", "[lexer][edge-case]") {
    auto toks = lex("eval [list set x [expr {1+2}]]");
    CHECK(toks[0].text == "eval");
    CHECK(toks[1].type == TokenType::CMD);
}

TEST_CASE("Namespace path var", "[lexer][edge-case]") {
    auto toks = lex("$::my::deeply::nested::var");
    CHECK(toks[0].type == TokenType::VAR);
    CHECK(toks[0].text == "::my::deeply::nested::var");
}

TEST_CASE("Multiline if", "[lexer][edge-case]") {
    auto source = "if {$x == 1} {\n"
                  "    puts one\n"
                  "} elseif {$x == 2} {\n"
                  "    puts two\n"
                  "} else {\n"
                  "    puts other\n"
                  "}";
    auto toks = lex(source);
    CHECK(toks[0].text == "if");
    std::vector<Token> strs;
    for (auto& t : toks) {
        if (t.type == TokenType::STR)
            strs.push_back(t);
    }
    CHECK(strs.size() >= 4);
}

TEST_CASE("Catch with result var", "[lexer][edge-case]") {
    auto toks = lex("catch {expr {1/0}} result opts");
    CHECK(toks[0].text == "catch");
    CHECK(toks[1].type == TokenType::STR);
    CHECK(toks[1].text.find("expr {1/0}") != std::string::npos);
}

TEST_CASE("Dict with special keys", "[lexer][edge-case]") {
    auto toks = lex("dict set d {key with spaces} {value with $dollar}");
    std::vector<Token> strs;
    for (auto& t : toks) {
        if (t.type == TokenType::STR)
            strs.push_back(t);
    }
    CHECK(std::any_of(strs.begin(), strs.end(), [](const Token& t) {
        return t.text.find("key with spaces") != std::string::npos;
    }));
    CHECK(std::any_of(strs.begin(), strs.end(), [](const Token& t) {
        return t.text.find("value with $dollar") != std::string::npos;
    }));
}

TEST_CASE("Lmap with nested cmd", "[lexer][edge-case]") {
    auto toks = lex("lmap x $list {string toupper [string index $x 0]}");
    CHECK(toks[0].text == "lmap");
    auto it = std::find_if(toks.begin(), toks.end(), [](const Token& t) {
        return t.type == TokenType::STR && t.text.find("toupper") != std::string::npos;
    });
    REQUIRE(it != toks.end());
    CHECK(it->text.find("[string index $x 0]") != std::string::npos);
}

TEST_CASE("Apply lambda", "[lexer][edge-case]") {
    auto toks = lex("apply {{x y} {expr {$x + $y}}} 3 4");
    CHECK(toks[0].text == "apply");
    CHECK(toks[1].type == TokenType::STR);
}

// Group 17: TestIRulesBraceSeparator (ported as lexer-level tests)

TEST_CASE("Brace separator produces separate words", "[lexer][edge-case]") {
    LexerConfig cfg{.irules_brace_separator = true};
    TclLexer lexer("if {$a}{puts a}", cfg);
    auto all = lexer.tokenise_all();
    std::vector<Token> toks;
    for (auto& t : all) {
        if (t.type != TokenType::SEP && t.type != TokenType::EOL)
            toks.push_back(std::move(t));
    }
    std::vector<Token> strs;
    for (auto& t : toks) {
        if (t.type == TokenType::STR)
            strs.push_back(t);
    }
    REQUIRE(strs.size() >= 2);
    CHECK(strs[0].text == "$a");
    CHECK(strs[1].text == "puts a");
}

TEST_CASE("Brace separator no warning", "[lexer][edge-case]") {
    LexerConfig cfg{.irules_brace_separator = true};
    TclLexer lexer("if {$a}{puts a}", cfg);
    auto all = lexer.tokenise_all();
    CHECK(lexer.warnings().empty());
}

TEST_CASE("Standard tcl warns on brace separator", "[lexer][edge-case]") {
    auto [toks, warnings] = lex_with_warnings("if {$a}{puts a}");
    CHECK(std::any_of(
        warnings.begin(), warnings.end(), [](const std::pair<SourcePosition, std::string>& w) {
            return w.second.find("extra characters after close-brace") != std::string::npos;
        }));
}

TEST_CASE("Standard tcl warns on brace concatenation", "[lexer][edge-case]") {
    // In standard Tcl, {a}{b} warns about extra chars after close-brace
    // (the segmenter concatenates into one word, but the lexer warns)
    auto [toks, warnings] = lex_with_warnings("cmd {a}{b}");
    CHECK(std::any_of(
        warnings.begin(), warnings.end(), [](const std::pair<SourcePosition, std::string>& w) {
            return w.second.find("extra characters after close-brace") != std::string::npos;
        }));
}

TEST_CASE("Triple brace separator", "[lexer][edge-case]") {
    LexerConfig cfg{.irules_brace_separator = true};
    TclLexer lexer("if {cond}{body1}{body2}", cfg);
    auto all = lexer.tokenise_all();
    std::vector<Token> toks;
    for (auto& t : all) {
        if (t.type != TokenType::SEP && t.type != TokenType::EOL)
            toks.push_back(std::move(t));
    }
    std::vector<Token> strs;
    for (auto& t : toks) {
        if (t.type == TokenType::STR)
            strs.push_back(t);
    }
    REQUIRE(strs.size() >= 3);
    CHECK(strs[0].text == "cond");
    CHECK(strs[1].text == "body1");
    CHECK(strs[2].text == "body2");
}
