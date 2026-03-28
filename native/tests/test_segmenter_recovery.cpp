// Tests for segmenter error recovery (ported from tests/test_command_segmenter.py
// TestErrorRecovery, TestHasSuspiciousToken, TestFindRecoveryOffset).

#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <unordered_set>

using namespace tcl_lsp;

namespace {

auto make_unclosed_source(const std::string& valid_cmd = "set x 1") -> std::string {
    return "proc broken {} {\n"
           "    set inner 1\n"
           "    set inner2 2\n"
           "    # missing close brace\n" +
           valid_cmd +
           "\n"
           "set after_recovery 42";
}

const std::unordered_set<std::string> known = {"proc", "set", "puts", "return", "if"};

} // namespace

TEST_CASE("recovery finds known command", "[segmenter][recovery]") {
    auto source = make_unclosed_source();
    auto cmds = segment_commands(source, nullptr, &known);
    auto partial_count = 0;
    auto valid_count = 0;
    for (const auto& c : cmds) {
        if (c.is_partial) {
            ++partial_count;
        } else {
            ++valid_count;
        }
    }
    CHECK(partial_count >= 1);
    CHECK(valid_count >= 1);
}

TEST_CASE("partial command marked", "[segmenter][recovery]") {
    auto source = make_unclosed_source();
    auto cmds = segment_commands(source, nullptr, &known);
    bool found_partial = false;
    for (const auto& c : cmds) {
        if (c.is_partial) {
            found_partial = true;
            break;
        }
    }
    CHECK(found_partial);
}

TEST_CASE("no recovery for valid source", "[segmenter][recovery]") {
    std::string source = "proc foo {} {\n    set a 1\n    return $a\n}\nset b 2";
    auto cmds = segment_commands(source, nullptr, &known);
    for (const auto& c : cmds) {
        CHECK(!c.is_partial);
    }
}

TEST_CASE("no recovery for body token", "[segmenter][recovery]") {
    std::unordered_set<std::string> kc = {"set", "puts"};
    std::string source = "set a 1\nset b 2\nset c 3\nset d 4";
    Token body{TokenType::STR, source, {0, 0, 0}, {3, 7, static_cast<int32_t>(source.size())}};
    auto cmds = segment_commands(source, &body, &kc);
    for (const auto& c : cmds) {
        CHECK(!c.is_partial);
    }
}

TEST_CASE("recovered commands have correct names", "[segmenter][recovery]") {
    auto source = make_unclosed_source("set x 1");
    auto cmds = segment_commands(source, nullptr, &known);
    bool found_set = false;
    for (const auto& c : cmds) {
        if (!c.is_partial && c.name() == "set") {
            found_set = true;
        }
    }
    CHECK(found_set);
}

TEST_CASE("partial delimiter brace", "[segmenter][recovery]") {
    auto source = make_unclosed_source("set x 1");
    auto cmds = segment_commands(source, nullptr, &known);
    for (const auto& c : cmds) {
        if (c.is_partial) {
            REQUIRE(c.partial_delimiter.has_value());
            CHECK(c.partial_delimiter.value() == UnclosedDelimiter::BRACE);
            break;
        }
    }
}

TEST_CASE("recovery from unclosed bracket", "[segmenter][recovery]") {
    std::unordered_set<std::string> kc = {"set", "puts"};
    std::string source = "set x [foo\nset y 2\nset z 3\nputs hello";
    auto cmds = segment_commands(source, nullptr, &kc);
    bool found_partial = false;
    bool found_valid = false;
    for (const auto& c : cmds) {
        if (c.is_partial) {
            found_partial = true;
            REQUIRE(c.partial_delimiter.has_value());
            CHECK(c.partial_delimiter.value() == UnclosedDelimiter::BRACKET);
        } else {
            found_valid = true;
        }
    }
    CHECK(found_partial);
    CHECK(found_valid);
}

TEST_CASE("recovery from unclosed quote", "[segmenter][recovery]") {
    std::unordered_set<std::string> kc = {"set", "puts"};
    std::string source = "set x \"hello\nset y 2\nset z 3\nputs hello";
    auto cmds = segment_commands(source, nullptr, &kc);
    bool found_partial = false;
    bool found_valid = false;
    for (const auto& c : cmds) {
        if (c.is_partial) {
            found_partial = true;
            REQUIRE(c.partial_delimiter.has_value());
            CHECK(c.partial_delimiter.value() == UnclosedDelimiter::QUOTE);
        } else {
            found_valid = true;
        }
    }
    CHECK(found_partial);
    CHECK(found_valid);
}

// TestHasSuspiciousToken

TEST_CASE("not suspicious for short str", "[segmenter][suspicious]") {
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {Token{TokenType::STR, "short", {0, 0, 0}, {0, 5, 5}}};
    CHECK(!has_suspicious_token(cmd, 100).has_value());
}

TEST_CASE("suspicious when spans lines and reaches eof", "[segmenter][suspicious]") {
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {Token{TokenType::STR, "line1\nline2\nline3\nline4", {0, 0, 0}, {3, 5, 22}}};
    auto result = has_suspicious_token(cmd, 23);
    CHECK(result.has_value());
}

TEST_CASE("not suspicious when not reaching eof", "[segmenter][suspicious]") {
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {Token{TokenType::STR, "line1\nline2\nline3\nline4", {0, 0, 0}, {3, 5, 22}}};
    CHECK(!has_suspicious_token(cmd, 200).has_value());
}

TEST_CASE("suspicious single line cmd token", "[segmenter][suspicious]") {
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {Token{TokenType::CMD, "foo bar", {0, 4, 4}, {0, 10, 10}}};
    auto result = has_suspicious_token(cmd, 11);
    REQUIRE(result.has_value());
    CHECK(result->second == UnclosedDelimiter::BRACKET);
}

TEST_CASE("suspicious cmd token spanning lines", "[segmenter][suspicious]") {
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {Token{TokenType::CMD, "foo\nbar\nbaz\nqux", {0, 0, 0}, {3, 3, 15}}};
    auto result = has_suspicious_token(cmd, 16);
    REQUIRE(result.has_value());
    CHECK(result->second == UnclosedDelimiter::BRACKET);
}

TEST_CASE("suspicious esc token", "[segmenter][suspicious]") {
    SegmentedCommand cmd;
    cmd.range = Range::zero();
    cmd.all_tokens = {Token{TokenType::ESC, "hello\nworld\nfoo\nbar", {0, 0, 0}, {3, 3, 19}}};
    auto result = has_suspicious_token(cmd, 20);
    REQUIRE(result.has_value());
    CHECK(result->second == UnclosedDelimiter::QUOTE);
}

// TestFindRecoveryOffset

TEST_CASE("finds known command on later line", "[segmenter][recovery_offset]") {
    std::string_view text = "    set inner 1\n    set inner2 2\nset x 1";
    std::unordered_set<std::string> kc = {"set", "puts"};
    auto offset = find_recovery_offset(text, 10, kc);
    CHECK(offset.has_value());
}

TEST_CASE("returns none when no known command", "[segmenter][recovery_offset]") {
    std::string_view text = "    foo bar\n    baz quux";
    std::unordered_set<std::string> kc = {"set", "puts"};
    auto offset = find_recovery_offset(text, 0, kc);
    CHECK(!offset.has_value());
}

TEST_CASE("skips first line", "[segmenter][recovery_offset]") {
    std::string_view text = "set x 1\nset y 2";
    std::unordered_set<std::string> kc = {"set"};
    auto offset = find_recovery_offset(text, 0, kc);
    REQUIRE(offset.has_value());
    CHECK(offset.value() > 7); // past "set x 1"
}
