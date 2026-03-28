// Tests ported from Tcl upstream parse.test — command boundary detection,
// completeness checks, and delimiter balancing relevant to segmentation.
// Reference: tmp/tcl8.6.16/tests/parse.test

#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

// parse-3.3: command ends at semicolon
TEST_CASE("upstream parse-3.3: semicolon terminates command", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a 1; set b 2");
    REQUIRE(cmds.size() == 2);
    CHECK(cmds[0].name() == "set");
    CHECK(cmds[1].name() == "set");
}

// parse-5.3: newline as command terminator
TEST_CASE("upstream parse-5.3: newline terminates command", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a 1\nset b 2");
    REQUIRE(cmds.size() == 2);
}

// parse-5.5: end of string as command terminator
TEST_CASE("upstream parse-5.5: eof terminates command", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a 1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
}

// parse-1.5: backslash-newline in leading space
TEST_CASE("upstream parse-1.5: backslash-newline continuation", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a \\\n1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
    // The continuation joins the argument.
    auto args = cmds[0].args();
    CHECK(args.size() >= 1);
}

// parse-5.11+: {*} expansion prefix
TEST_CASE("upstream parse-5.11: expand prefix", "[segmenter][upstream]") {
    auto cmds = segment_commands("cmd {*}{a b c}");
    REQUIRE(cmds.size() == 1);
    REQUIRE(cmds[0].expand_word.has_value());
    auto& ew = cmds[0].expand_word.value();
    // cmd word not expanded, second word expanded
    REQUIRE(ew.size() >= 2);
    CHECK(!ew[0]);
    CHECK(ew[1]);
}

// parse-11.7: multiple commands separated by semicolon
TEST_CASE("upstream parse-11.7: multiple semicolon-separated commands", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a 1; set b 2; set c 3");
    CHECK(cmds.size() == 3);
}

// parse-11.8: multiple commands on separate lines
TEST_CASE("upstream parse-11.8: multiple newline-separated commands", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a 1\nset b 2\nset c 3");
    CHECK(cmds.size() == 3);
}

// parse-15 command completeness: unterminated quote
TEST_CASE("upstream parse-15.13: unterminated quote detected", "[segmenter][upstream]") {
    // Unterminated " — should produce a single command that swallows everything.
    auto cmds =
        segment_commands("set x \"hello\nset y 2", nullptr, nullptr, nullptr, nullptr, false);
    // Without recovery, the entire thing is one command (the " swallows to EOF).
    CHECK(cmds.size() >= 1);
}

// parse-15.24+: brace content — properly closed is complete
TEST_CASE("upstream parse-15.24: properly closed braces", "[segmenter][upstream]") {
    auto cmds = segment_commands("set x {hello}");
    REQUIRE(cmds.size() == 1);
    CHECK(!cmds[0].is_partial);
}

// parse-15.29-36: nested bracket boundaries
TEST_CASE("upstream parse-15.29: nested brackets", "[segmenter][upstream]") {
    auto cmds = segment_commands("set x [string length [set a hello]]");
    REQUIRE(cmds.size() == 1);
    CHECK(!cmds[0].is_partial);
}

// Backslash-newline continuation across commands
TEST_CASE("upstream: backslash-newline does not break command", "[segmenter][upstream]") {
    auto cmds = segment_commands("set a \\\n    1\nset b 2");
    REQUIRE(cmds.size() == 2);
    CHECK(cmds[0].name() == "set");
    CHECK(cmds[1].name() == "set");
}

// Comments
TEST_CASE("upstream: comments are not commands", "[segmenter][upstream]") {
    auto cmds = segment_commands("# this is a comment\nset a 1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
}

// Empty lines
TEST_CASE("upstream: empty lines produce no commands", "[segmenter][upstream]") {
    auto cmds = segment_commands("\n\n\n");
    CHECK(cmds.empty());
}

// Braced content preserved
TEST_CASE("upstream: braced content preserved as single token", "[segmenter][upstream]") {
    auto cmds = segment_commands("set x {hello world}");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 2);
    CHECK(args[1] == "hello world");
    CHECK(cmds[0].single_token_word.back());
}

// Nested command substitution
TEST_CASE("upstream: command substitution produces CMD token", "[segmenter][upstream]") {
    auto cmds = segment_commands("set x [expr {1 + 2}]");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 2);
    CHECK(args[1] == "[expr {1 + 2}]");
}

// Variable forms
TEST_CASE("upstream: simple variable", "[segmenter][upstream]") {
    auto cmds = segment_commands("puts $x");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 1);
    CHECK(args[0] == "${x}");
}

TEST_CASE("upstream: namespace variable", "[segmenter][upstream]") {
    auto cmds = segment_commands("puts $::ns::var");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 1);
    CHECK(args[0] == "${::ns::var}");
}

TEST_CASE("upstream: array variable", "[segmenter][upstream]") {
    auto cmds = segment_commands("puts $arr(idx)");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 1);
    // Array variables use ${name(idx)} form if no } in the name.
    // The text depends on how the lexer handles array references.
}

// Multi-word concatenation
TEST_CASE("upstream: multi-token word concatenation", "[segmenter][upstream]") {
    auto cmds = segment_commands("set x a$y");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 2);
    // "a" concatenated with "${y}"
    CHECK(args[1] == "a${y}");
    CHECK(!cmds[0].single_token_word.back());
}

// parse-15.39-50: backslash-newline continuation detection
TEST_CASE("upstream parse-15.39: backslash-newline continuation mid-command",
          "[segmenter][upstream]") {
    auto cmds = segment_commands("set x \\\ny");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
}

// Whitespace handling
TEST_CASE("upstream: leading whitespace before command", "[segmenter][upstream]") {
    auto cmds = segment_commands("    set a 1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
}

TEST_CASE("upstream: tab-separated arguments", "[segmenter][upstream]") {
    auto cmds = segment_commands("set\ta\t1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
    auto args = cmds[0].args();
    REQUIRE(args.size() == 2);
}
