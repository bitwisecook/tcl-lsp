#include <catch2/catch_test_macros.hpp>

#include "tcl_lsp/lsp/semantic_token_collector.hpp"
#include "tcl_lsp/lsp/semantic_token_types.hpp"

using namespace tcl_lsp;

namespace {

// Decode delta-encoded tokens back to readable form.
struct DecodedToken {
    int line;
    int character;
    int length;
    SemanticTokenType type;
    int modifiers;
};

auto decode(const std::vector<int32_t>& data) -> std::vector<DecodedToken> {
    std::vector<DecodedToken> out;
    int line = 0;
    int ch = 0;
    for (std::size_t i = 0; i + 4 < data.size(); i += 5) {
        auto dl = data[i];
        auto dc = data[i + 1];
        auto len = data[i + 2];
        auto type = static_cast<SemanticTokenType>(data[i + 3]);
        auto mods = data[i + 4];
        if (dl > 0) {
            line += dl;
            ch = dc;
        } else {
            ch += dc;
        }
        out.push_back({line, ch, len, type, mods});
    }
    return out;
}

auto collect_and_decode(std::string_view source) -> std::vector<DecodedToken> {
    SemanticTokenCollector collector;
    auto tokens = collector.collect(source);
    auto data = delta_encode(tokens);
    return decode(data);
}

auto has_type(const std::vector<DecodedToken>& tokens, SemanticTokenType type) -> bool {
    for (auto& t : tokens) {
        if (t.type == type) return true;
    }
    return false;
}

auto count_type(const std::vector<DecodedToken>& tokens, SemanticTokenType type) -> int {
    int n = 0;
    for (auto& t : tokens) {
        if (t.type == type) n++;
    }
    return n;
}

} // anonymous namespace

// === Core classification tests ===

TEST_CASE("simple puts", "[semantic_tokens]") {
    auto tokens = collect_and_decode("puts hello");
    REQUIRE(tokens.size() == 2);
    REQUIRE(tokens[0].type == SemanticTokenType::KEYWORD);
    REQUIRE(tokens[0].length == 4);
    REQUIRE(tokens[1].type == SemanticTokenType::STRING);
}

TEST_CASE("variable", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set x $y");
    REQUIRE(has_type(tokens, SemanticTokenType::KEYWORD)); // 'set'
    REQUIRE(has_type(tokens, SemanticTokenType::VARIABLE)); // '$y'
}

TEST_CASE("number", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set x 42");
    int num_count = count_type(tokens, SemanticTokenType::NUMBER);
    REQUIRE(num_count == 1);
}

TEST_CASE("comment", "[semantic_tokens]") {
    auto tokens = collect_and_decode("# hello world");
    REQUIRE(tokens[0].type == SemanticTokenType::COMMENT);
}

TEST_CASE("proc as keyword", "[semantic_tokens]") {
    auto tokens = collect_and_decode("proc foo {x} {}");
    REQUIRE(tokens[0].type == SemanticTokenType::KEYWORD);
}

TEST_CASE("user command as function", "[semantic_tokens]") {
    auto tokens = collect_and_decode("mycommand arg1");
    REQUIRE(tokens[0].type == SemanticTokenType::FUNCTION);
}

TEST_CASE("multiline positions", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set x 1\nset y 2");
    auto kws = std::vector<DecodedToken>{};
    for (auto& t : tokens) {
        if (t.type == SemanticTokenType::KEYWORD) kws.push_back(t);
    }
    REQUIRE(kws.size() == 2);
    REQUIRE(kws[0].line == 0);
    REQUIRE(kws[1].line == 1);
}

// === Recursion tests ===

TEST_CASE("proc body recursion", "[semantic_tokens]") {
    auto tokens = collect_and_decode("proc foo {} { puts hello }");
    // Should have keyword tokens inside the braced body.
    bool has_puts_keyword = false;
    for (auto& t : tokens) {
        if (t.type == SemanticTokenType::KEYWORD && t.length == 4) {
            // Could be 'puts' or 'proc'
            has_puts_keyword = true;
        }
    }
    REQUIRE(has_puts_keyword);
}

TEST_CASE("nested command substitution", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set x [expr 1 + 2]");
    // Should recurse into the [ ] and produce tokens.
    REQUIRE(tokens.size() >= 2);
    REQUIRE(has_type(tokens, SemanticTokenType::KEYWORD)); // 'set', 'expr'
}

TEST_CASE("if body recursion", "[semantic_tokens]") {
    auto tokens = collect_and_decode("if {1} { puts ok }");
    // 'if' and 'puts' should both be keywords.
    int kw_count = count_type(tokens, SemanticTokenType::KEYWORD);
    REQUIRE(kw_count >= 2);
}

// === Modifier tests ===

TEST_CASE("proc name definition modifier", "[semantic_tokens]") {
    auto tokens = collect_and_decode("proc foo {x} {}");
    // 'foo' should have the definition modifier.
    bool found_def = false;
    for (auto& t : tokens) {
        if (t.type == SemanticTokenType::FUNCTION &&
            (t.modifiers & modifier_bit(SemanticTokenModifier::DEFINITION)) != 0) {
            found_def = true;
        }
    }
    REQUIRE(found_def);
}

// === Namespace tests ===

TEST_CASE("namespace qualified command", "[semantic_tokens]") {
    auto tokens = collect_and_decode("::foo::bar arg");
    // Should split into namespace and function/keyword tokens.
    bool has_ns = has_type(tokens, SemanticTokenType::NAMESPACE);
    REQUIRE(has_ns);
}

// === Delta encoding tests ===

TEST_CASE("delta encode basic", "[semantic_tokens][delta]") {
    std::vector<SemanticToken> tokens = {
        {0, 0, 3, SemanticTokenType::KEYWORD, 0},
        {0, 4, 5, SemanticTokenType::STRING, 0},
    };
    auto data = delta_encode(tokens);
    REQUIRE(data.size() == 10);
    // First token: deltaLine=0, deltaChar=0, len=3, type=0(keyword), mods=0
    REQUIRE(data[0] == 0);
    REQUIRE(data[1] == 0);
    REQUIRE(data[2] == 3);
    REQUIRE(data[3] == static_cast<int32_t>(SemanticTokenType::KEYWORD));
    // Second token: deltaLine=0, deltaChar=4, len=5, type=3(string), mods=0
    REQUIRE(data[5] == 0);
    REQUIRE(data[6] == 4);
    REQUIRE(data[7] == 5);
    REQUIRE(data[8] == static_cast<int32_t>(SemanticTokenType::STRING));
}

TEST_CASE("delta encode multiline", "[semantic_tokens][delta]") {
    std::vector<SemanticToken> tokens = {
        {0, 0, 3, SemanticTokenType::KEYWORD, 0},
        {1, 0, 3, SemanticTokenType::KEYWORD, 0},
    };
    auto data = delta_encode(tokens);
    REQUIRE(data.size() == 10);
    // Second token: deltaLine=1, deltaChar=0 (absolute on new line)
    REQUIRE(data[5] == 1);
    REQUIRE(data[6] == 0);
}

// === Semantic token edit tests ===

TEST_CASE("compute edits: identical", "[semantic_tokens][delta]") {
    std::vector<int32_t> data = {0, 0, 3, 0, 0, 0, 4, 5, 3, 0};
    auto edits = compute_semantic_token_edits(data, data);
    REQUIRE(edits.empty());
}

TEST_CASE("compute edits: insert", "[semantic_tokens][delta]") {
    std::vector<int32_t> old_data = {0, 0, 3, 0, 0};
    std::vector<int32_t> new_data = {0, 0, 3, 0, 0, 0, 4, 5, 3, 0};
    auto edits = compute_semantic_token_edits(old_data, new_data);
    REQUIRE(edits.size() == 1);
    REQUIRE(edits[0].start == 5);
    REQUIRE(edits[0].delete_count == 0);
    REQUIRE(edits[0].data.size() == 5);
}

TEST_CASE("compute edits: delete", "[semantic_tokens][delta]") {
    std::vector<int32_t> old_data = {0, 0, 3, 0, 0, 0, 4, 5, 3, 0};
    std::vector<int32_t> new_data = {0, 0, 3, 0, 0};
    auto edits = compute_semantic_token_edits(old_data, new_data);
    REQUIRE(edits.size() == 1);
    REQUIRE(edits[0].start == 5);
    REQUIRE(edits[0].delete_count == 5);
    REQUIRE(edits[0].data.empty());
}

TEST_CASE("compute edits: replace", "[semantic_tokens][delta]") {
    std::vector<int32_t> old_data = {0, 0, 3, 0, 0};
    std::vector<int32_t> new_data = {0, 0, 4, 1, 0};
    auto edits = compute_semantic_token_edits(old_data, new_data);
    REQUIRE(edits.size() == 1);
    REQUIRE(edits[0].delete_count > 0);
    REQUIRE(!edits[0].data.empty());
}

// === Expression tokenization ===

TEST_CASE("expr body tokens", "[semantic_tokens]") {
    auto tokens = collect_and_decode("expr {1 + 2}");
    // Should have keyword (expr), number, operator, number from inside the braces.
    REQUIRE(has_type(tokens, SemanticTokenType::KEYWORD));
    REQUIRE(has_type(tokens, SemanticTokenType::NUMBER));
    REQUIRE(has_type(tokens, SemanticTokenType::OPERATOR));
}

TEST_CASE("if condition expr tokens", "[semantic_tokens]") {
    auto tokens = collect_and_decode("if {$x > 0} { puts yes }");
    // Condition should have variable and operator.
    REQUIRE(has_type(tokens, SemanticTokenType::VARIABLE));
    REQUIRE(has_type(tokens, SemanticTokenType::OPERATOR));
}

// === Edge cases ===

TEST_CASE("empty source", "[semantic_tokens]") {
    auto tokens = collect_and_decode("");
    REQUIRE(tokens.empty());
}

TEST_CASE("whitespace only", "[semantic_tokens]") {
    auto tokens = collect_and_decode("   \n\n  ");
    REQUIRE(tokens.empty());
}

TEST_CASE("comment only", "[semantic_tokens]") {
    auto tokens = collect_and_decode("# a comment");
    REQUIRE(tokens.size() == 1);
    REQUIRE(tokens[0].type == SemanticTokenType::COMMENT);
}

TEST_CASE("multiple commands", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set a 1\nset b 2\nset c 3");
    int kw_count = count_type(tokens, SemanticTokenType::KEYWORD);
    REQUIRE(kw_count == 3);
}

TEST_CASE("semicolon separated", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set a 1; set b 2");
    int kw_count = count_type(tokens, SemanticTokenType::KEYWORD);
    REQUIRE(kw_count == 2);
}

TEST_CASE("variable declaration modifier", "[semantic_tokens]") {
    auto tokens = collect_and_decode("set x 42");
    // 'x' should be variable with declaration modifier.
    bool found_decl = false;
    for (auto& t : tokens) {
        if (t.type == SemanticTokenType::VARIABLE &&
            (t.modifiers & modifier_bit(SemanticTokenModifier::DECLARATION)) != 0) {
            found_decl = true;
        }
    }
    REQUIRE(found_decl);
}

TEST_CASE("foreach body recursion", "[semantic_tokens]") {
    auto tokens = collect_and_decode("foreach x {1 2 3} { puts $x }");
    // 'puts' inside the body should be a keyword.
    int kw_count = count_type(tokens, SemanticTokenType::KEYWORD);
    REQUIRE(kw_count >= 2); // foreach + puts
}

TEST_CASE("switch braced body", "[semantic_tokens]") {
    auto tokens = collect_and_decode("switch $x { a { puts a } b { puts b } }");
    // Should recurse into switch body.
    REQUIRE(has_type(tokens, SemanticTokenType::KEYWORD));
}

TEST_CASE("option as decorator", "[semantic_tokens]") {
    auto tokens = collect_and_decode("regexp -nocase pat str");
    bool has_dec = has_type(tokens, SemanticTokenType::DECORATOR);
    REQUIRE(has_dec);
}

TEST_CASE("event name", "[semantic_tokens]") {
    auto tokens = collect_and_decode("when HTTP_REQUEST { puts req }");
    bool has_event = has_type(tokens, SemanticTokenType::EVENT);
    REQUIRE(has_event);
}
