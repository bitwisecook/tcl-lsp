#include "tcl_lsp/analysis/stub_parser.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

// ---------------------------------------------------------------------------
// is_stubs_begin / is_stubs_end
// ---------------------------------------------------------------------------

TEST_CASE("is_stubs_begin detects marker", "[stub_parser]") {
    REQUIRE(is_stubs_begin("# tcl-lsp: stubs-begin"));
    REQUIRE(is_stubs_begin("# TCL-LSP: stubs-begin"));
    REQUIRE(is_stubs_begin("# tcl-lsp: STUBS-BEGIN"));
    REQUIRE_FALSE(is_stubs_begin("# just a comment"));
    REQUIRE_FALSE(is_stubs_begin("# stubs-begin")); // missing tcl-lsp:
}

TEST_CASE("is_stubs_end detects marker", "[stub_parser]") {
    REQUIRE(is_stubs_end("# tcl-lsp: stubs-end"));
    REQUIRE(is_stubs_end("# TCL-LSP: STUBS-END"));
    REQUIRE_FALSE(is_stubs_end("# stubs-end"));
}

// ---------------------------------------------------------------------------
// parse_stub_line
// ---------------------------------------------------------------------------

TEST_CASE("parse_stub_line: simple command stub", "[stub_parser]") {
    auto result = parse_stub_line("# stub myCmd {arg1 arg2:body}", Range::zero());
    REQUIRE(result.has_value());
    REQUIRE(result->name == "myCmd");
    REQUIRE(result->args.size() == 2);
    REQUIRE(result->args[0].name == "arg1");
    REQUIRE(result->args[0].role == "value");
    REQUIRE(result->args[1].name == "arg2");
    REQUIRE(result->args[1].role == "body");
}

TEST_CASE("parse_stub_line: optional arguments", "[stub_parser]") {
    auto result = parse_stub_line("# stub myCmd {?opt:expr?}", Range::zero());
    REQUIRE(result.has_value());
    REQUIRE(result->args.size() == 1);
    REQUIRE(result->args[0].name == "opt");
    REQUIRE(result->args[0].role == "expr");
    REQUIRE(result->args[0].optional == true);
}

TEST_CASE("parse_stub_line: flags", "[stub_parser]") {
    auto result = parse_stub_line("# stub myCmd {body:body} -barrier -loop", Range::zero());
    REQUIRE(result.has_value());
    REQUIRE(result->barrier == true);
    REQUIRE(result->loop == true);
    REQUIRE(result->pure == false);
}

TEST_CASE("parse_stub_line: empty args", "[stub_parser]") {
    auto result = parse_stub_line("# stub myCmd {}", Range::zero());
    REQUIRE(result.has_value());
    REQUIRE(result->args.empty());
}

TEST_CASE("parse_stub_line: malformed returns error", "[stub_parser]") {
    auto r1 = parse_stub_line("# not a stub", Range::zero());
    REQUIRE_FALSE(r1.has_value());
    CHECK(r1.error() == StubParseError::NOT_A_STUB);

    auto r2 = parse_stub_line("# stub", Range::zero());
    REQUIRE_FALSE(r2.has_value());
    CHECK(r2.error() == StubParseError::NOT_A_STUB);
}

TEST_CASE("parse_stub_line: invalid role returns error", "[stub_parser]") {
    auto r = parse_stub_line("# stub cmd {arg:bogus}", Range::zero());
    REQUIRE_FALSE(r.has_value());
    CHECK(r.error() == StubParseError::INVALID_ARG_SYNTAX);
}

// ---------------------------------------------------------------------------
// parse_expr_stub_line
// ---------------------------------------------------------------------------

TEST_CASE("parse_expr_stub_line: function", "[stub_parser]") {
    auto result = parse_expr_stub_line("# stub expr-func mysin 1", Range::zero());
    REQUIRE(result.has_value());
    REQUIRE(result->name == "mysin");
    REQUIRE(result->kind == "function");
    REQUIRE(result->arity == 1);
}

TEST_CASE("parse_expr_stub_line: operator", "[stub_parser]") {
    auto result = parse_expr_stub_line("# stub expr-op ~~ 2", Range::zero());
    REQUIRE(result.has_value());
    REQUIRE(result->name == "~~");
    REQUIRE(result->kind == "operator");
    REQUIRE(result->arity == 2);
}

TEST_CASE("parse_expr_stub_line: default arity", "[stub_parser]") {
    auto func = parse_expr_stub_line("# stub expr-func myfn", Range::zero());
    REQUIRE(func.has_value());
    REQUIRE(func->arity == 1); // default for function

    auto op = parse_expr_stub_line("# stub expr-op myop", Range::zero());
    REQUIRE(op.has_value());
    REQUIRE(op->arity == 2); // default for operator
}

// ---------------------------------------------------------------------------
// scan_source_for_stubs
// ---------------------------------------------------------------------------

TEST_CASE("scan_source_for_stubs: full block", "[stub_parser]") {
    auto source = R"(
# tcl-lsp: stubs-begin
# stub myLoop {body:body} -loop
# stub expr-func mysin 1
# tcl-lsp: stubs-end
proc hello {} {}
)";
    auto result = scan_source_for_stubs(source);
    REQUIRE(result.commands.size() == 1);
    REQUIRE(result.commands[0].name == "myLoop");
    REQUIRE(result.commands[0].loop == true);
    REQUIRE(result.expressions.size() == 1);
    REQUIRE(result.expressions[0].name == "mysin");
}

TEST_CASE("scan_source_for_stubs: stubs outside block are ignored", "[stub_parser]") {
    auto source = "# stub myCmd {}\nproc hello {} {}\n";
    auto result = scan_source_for_stubs(source);
    REQUIRE(result.commands.empty());
}

TEST_CASE("scan_source_for_stubs: empty source", "[stub_parser]") {
    auto result = scan_source_for_stubs("");
    REQUIRE(result.commands.empty());
    REQUIRE(result.expressions.empty());
}
