// Ported from tests/test_analyser.py — TestInterpAlias.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>
#include <string>
#include <tuple>
#include <vector>

using namespace tcl_lsp;

// Build a registry that knows about expr, eval, puts, foreach for alias resolution tests.
static auto make_alias_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("eval", CommandSig{Arity{1, 100}, {{0, ArgRole::BODY}}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("interp", CommandSig{Arity{1, 100}, {}});
    reg.add_command("foreach", CommandSig{
        Arity{3, 100},
        {{0, ArgRole::VAR_NAME}, {2, ArgRole::BODY}}
    });
    reg.add_command("file", CommandSig{Arity{2, 100}, {}});
    reg.add_command("lsearch", CommandSig{Arity{2, 100}, {}});
    reg.add_command("lreplace", CommandSig{Arity{3, 100}, {}});
    reg.add_command("if", CommandSig{Arity{2, 100}, {}});
    return reg;
}

// Helper: check if a vector of tuples matches.
using AliasPair = std::pair<std::string, std::vector<std::string>>;

// ---------------------------------------------------------------------------
// Basic alias recording
// ---------------------------------------------------------------------------

TEST_CASE("alias: recorded", "[analyser][alias]") {
    auto result = analyse("interp alias {} = {} expr");
    REQUIRE(result.command_aliases().contains("::="));
    auto& [target, args] = result.command_aliases().at("::=");
    CHECK(target == "expr");
    CHECK(args.empty());
}

TEST_CASE("alias: with prepended args", "[analyser][alias]") {
    auto result = analyse("interp alias {} myput {} puts stdout");
    REQUIRE(result.command_aliases().contains("::myput"));
    auto& [target, args] = result.command_aliases().at("::myput");
    CHECK(target == "puts");
    REQUIRE(args.size() == 1);
    CHECK(args[0] == "stdout");
}

TEST_CASE("alias: non-current interp not recorded", "[analyser][alias]") {
    auto result = analyse("interp alias child = {} expr");
    CHECK_FALSE(result.command_aliases().contains("::="));
}

TEST_CASE("alias: qualified name", "[analyser][alias]") {
    auto result = analyse("interp alias {} ::myexpr {} expr");
    CHECK(result.command_aliases().contains("::myexpr"));
}

TEST_CASE("alias: chain not resolved transitively", "[analyser][alias]") {
    auto result = analyse("interp alias {} a {} b\ninterp alias {} b {} expr");
    CHECK(result.command_aliases().at("::a").first == "b");
    CHECK(result.command_aliases().at("::b").first == "expr");
}

TEST_CASE("alias: redefinition overwrites", "[analyser][alias]") {
    auto result = analyse("interp alias {} myop {} expr\ninterp alias {} myop {} puts");
    auto& [target, args] = result.command_aliases().at("::myop");
    CHECK(target == "puts");
}

TEST_CASE("alias: too few args no crash", "[analyser][alias]") {
    auto result = analyse("interp alias {} = {}");
    CHECK_FALSE(result.command_aliases().contains("::="));
}

TEST_CASE("alias: target in child interp not tracked", "[analyser][alias]") {
    auto result = analyse("interp alias {} myexpr child expr");
    CHECK_FALSE(result.command_aliases().contains("::myexpr"));
}

TEST_CASE("alias: query form no crash", "[analyser][alias]") {
    auto result = analyse("interp alias {} myexpr");
    CHECK_FALSE(result.command_aliases().contains("::myexpr"));
}

// ---------------------------------------------------------------------------
// Alias analysis (body/expr roles resolved through alias)
// ---------------------------------------------------------------------------

TEST_CASE("alias: expr analysis works", "[analyser][alias]") {
    auto reg = make_alias_registry();
    auto result = analyse("interp alias {} = {} expr\n= {1 + 2}", &reg);
    auto errors = std::vector<Diagnostic>{};
    for (const auto& d : result.diagnostics()) {
        if (d.severity == Severity::ERROR) errors.push_back(d);
    }
    CHECK(errors.empty());
}

TEST_CASE("alias: body analysis works via eval alias", "[analyser][alias]") {
    auto reg = make_alias_registry();
    auto result = analyse(R"(interp alias {} myeval {} eval
proc foo {x} {
    myeval { set y 1 }
    return $x
})", &reg);
    // The body should be analysed — y should appear in proc scope
    auto& children = result.global_scope().children;
    REQUIRE(children.size() == 1);
}

// ---------------------------------------------------------------------------
// Real-world patterns
// ---------------------------------------------------------------------------

TEST_CASE("alias: file shortcuts", "[analyser][alias]") {
    auto result = analyse(R"(interp alias {} cp {} file copy -force
interp alias {} mkdir {} file mkdir
interp alias {} rm {} file delete -force)");

    auto& cp = result.command_aliases().at("::cp");
    CHECK(cp.first == "file");
    REQUIRE(cp.second.size() == 2);
    CHECK(cp.second[0] == "copy");
    CHECK(cp.second[1] == "-force");

    auto& mkd = result.command_aliases().at("::mkdir");
    CHECK(mkd.first == "file");
    REQUIRE(mkd.second.size() == 1);
    CHECK(mkd.second[0] == "mkdir");

    auto& rm = result.command_aliases().at("::rm");
    CHECK(rm.first == "file");
    REQUIRE(rm.second.size() == 2);
    CHECK(rm.second[0] == "delete");
    CHECK(rm.second[1] == "-force");
}

TEST_CASE("alias: deletion form preserves original", "[analyser][alias]") {
    auto result = analyse(R"(interp alias {} myalias {} list
interp alias {} myalias {})");
    // The deletion form (4 args) should not be detected as a new alias.
    // The original alias should still be present.
    REQUIRE(result.command_aliases().contains("::myalias"));
    CHECK(result.command_aliases().at("::myalias").first == "list");
}

TEST_CASE("alias: safe interp not tracked", "[analyser][alias]") {
    auto result = analyse(R"(set i [interp create -safe]
interp alias $i add {} ::api::add)");
    CHECK_FALSE(result.command_aliases().contains("::add"));
}

// ---------------------------------------------------------------------------
// W214 — unused proc parameters (with alias resolution)
// ---------------------------------------------------------------------------

TEST_CASE("alias: expr alias suppresses W214", "[analyser][alias][w214]") {
    auto reg = make_alias_registry();
    auto result = analyse(R"(interp alias {} = {} expr
proc foo {x y} {
    set result [= {$x + $y}]
    return $result
})", &reg);
    std::vector<Diagnostic> w214;
    for (const auto& d : result.diagnostics()) {
        if (d.code == DiagCode::W214) w214.push_back(d);
    }
    CHECK(w214.empty());
}

TEST_CASE("alias: namespace-scoped alias resolves", "[analyser][alias][w214]") {
    auto reg = make_alias_registry();
    auto result = analyse(R"(interp alias {} ::math::= {} expr
namespace eval math {
    proc calc {x y} {
        set result [= {$x + $y}]
        return $result
    }
})", &reg);
    std::vector<Diagnostic> w214;
    for (const auto& d : result.diagnostics()) {
        if (d.code == DiagCode::W214) w214.push_back(d);
    }
    CHECK(w214.empty());
}

TEST_CASE("alias: global fallback resolves", "[analyser][alias][w214]") {
    auto reg = make_alias_registry();
    auto result = analyse(R"(interp alias {} = {} expr
namespace eval utils {
    proc calc {x y} {
        set result [= {$x + $y}]
        return $result
    }
})", &reg);
    std::vector<Diagnostic> w214;
    for (const auto& d : result.diagnostics()) {
        if (d.code == DiagCode::W214) w214.push_back(d);
    }
    CHECK(w214.empty());
}
