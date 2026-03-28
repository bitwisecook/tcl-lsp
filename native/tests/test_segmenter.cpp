// Tests for core command segmentation (ported from tests/test_command_segmenter.py
// TestSegmentCommands).

#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("single command", "[segmenter]") {
    auto cmds = segment_commands("set a 1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
    auto args = cmds[0].args();
    REQUIRE(args.size() == 2);
    CHECK(args[0] == "a");
    CHECK(args[1] == "1");
}

TEST_CASE("two commands separated by newline", "[segmenter]") {
    auto cmds = segment_commands("set a 1\nset b 2");
    REQUIRE(cmds.size() == 2);
    CHECK(cmds[0].name() == "set");
    CHECK(cmds[1].name() == "set");
    auto args = cmds[1].args();
    REQUIRE(args.size() == 2);
    CHECK(args[0] == "b");
    CHECK(args[1] == "2");
}

TEST_CASE("semicolon separator", "[segmenter]") {
    auto cmds = segment_commands("set a 1; set b 2");
    REQUIRE(cmds.size() == 2);
    CHECK(cmds[0].name() == "set");
    CHECK(cmds[1].name() == "set");
}

TEST_CASE("empty source", "[segmenter]") {
    auto cmds = segment_commands("");
    CHECK(cmds.empty());
}

TEST_CASE("comment only", "[segmenter]") {
    auto cmds = segment_commands("# just a comment");
    CHECK(cmds.empty());
}

TEST_CASE("preceding comment attached", "[segmenter]") {
    auto cmds = segment_commands("# note\nset a 1");
    REQUIRE(cmds.size() == 1);
    REQUIRE(cmds[0].preceding_comment.has_value());
    CHECK(cmds[0].preceding_comment.value() == "note");
}

TEST_CASE("multiline preceding comment", "[segmenter]") {
    auto cmds = segment_commands("# line one\n# line two\nset a 1");
    REQUIRE(cmds.size() == 1);
    REQUIRE(cmds[0].preceding_comment.has_value());
    CHECK(cmds[0].preceding_comment.value() == "line one\nline two");
}

TEST_CASE("blank line breaks comment accumulation", "[segmenter]") {
    auto cmds = segment_commands("# orphan\n\nset a 1");
    REQUIRE(cmds.size() == 1);
    CHECK(!cmds[0].preceding_comment.has_value());
}

TEST_CASE("body token segmentation", "[segmenter]") {
    Token body{TokenType::STR, "set a 1\nset b 2", {0, 1, 1}, {1, 5, 16}};
    auto cmds = segment_commands(body.text, &body);
    REQUIRE(cmds.size() == 2);
}

TEST_CASE("variable word piece", "[segmenter]") {
    auto cmds = segment_commands("puts $x");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 1);
    CHECK(args[0] == "${x}");
}

TEST_CASE("command substitution word piece", "[segmenter]") {
    auto cmds = segment_commands("puts [clock seconds]");
    REQUIRE(cmds.size() == 1);
    auto args = cmds[0].args();
    REQUIRE(args.size() == 1);
    CHECK(args[0] == "[clock seconds]");
}

TEST_CASE("multi token word", "[segmenter]") {
    auto cmds = segment_commands("puts \"hello $name\"");
    REQUIRE(cmds.size() == 1);
    CHECK(!cmds[0].single_token_word.back());
}

TEST_CASE("is_partial false for normal commands", "[segmenter]") {
    auto cmds = segment_commands("set a 1\nset b 2");
    for (const auto& cmd : cmds) {
        CHECK(!cmd.is_partial);
    }
}

TEST_CASE("trailing command without final newline", "[segmenter]") {
    auto cmds = segment_commands("set a 1");
    REQUIRE(cmds.size() == 1);
    CHECK(cmds[0].name() == "set");
}

TEST_CASE("expand word flags", "[segmenter]") {
    auto cmds = segment_commands("cmd {*}$args");
    REQUIRE(cmds.size() == 1);
    REQUIRE(cmds[0].expand_word.has_value());
    auto& ew = cmds[0].expand_word.value();
    REQUIRE(ew.size() == 2);
    CHECK(!ew[0]); // cmd itself not expanded
    CHECK(ew[1]);  // $args is expanded
}
