// Ported from tests/test_analyser.py — TestDiagnostics + TestUnusedProcParameters.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>
#include <string>
#include <vector>

using namespace tcl_lsp;

// Build a minimal registry for arity checking tests.
static auto make_diag_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("break", CommandSig{Arity{0, 0}, {}});
    reg.add_command("continue", CommandSig{Arity{0, 0}, {}});
    reg.add_command("while", CommandSig{Arity{2, 2}, {}});
    reg.add_command("for", CommandSig{Arity{4, 4}, {}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("if", CommandSig{Arity{2, 100}, {}});
    reg.add_command("incr", CommandSig{Arity{1, 2}, {}});
    reg.add_command("interp", CommandSig{Arity{1, 100}, {}});
    reg.add_command("foreach", CommandSig{Arity{3, 100}, {}});
    reg.add_command("catch", CommandSig{Arity{1, 3}, {}});
    reg.add_command("try", CommandSig{Arity{1, 100}, {}});
    reg.add_command("switch", CommandSig{Arity{2, 100}, {}});
    reg.add_command("namespace", CommandSig{Arity{1, 100}, {}});
    reg.add_command("lsearch", CommandSig{Arity{2, 100}, {}});
    reg.add_command("lreplace", CommandSig{Arity{3, 100}, {}});
    reg.add_command("eval", CommandSig{Arity{1, 100}, {{0, ArgRole::BODY}}});

    // string as subcommand sig.
    SubcommandSig string_sig;
    string_sig.subcommands["length"] = CommandSig{Arity{1, 1}, {}};
    string_sig.subcommands["range"] = CommandSig{Arity{3, 3}, {}};
    string_sig.subcommands["match"] = CommandSig{Arity{2, 3}, {}};
    string_sig.allow_unknown = false;
    reg.add_subcommand("string", string_sig);

    return reg;
}

// Helpers.
static auto errors(const AnalysisResult& result) -> std::vector<Diagnostic> {
    std::vector<Diagnostic> out;
    for (const auto& d : result.diagnostics()) {
        if (d.severity == Severity::ERROR) out.push_back(d);
    }
    return out;
}

static auto by_code(const AnalysisResult& result, DiagCode code)
    -> std::vector<Diagnostic> {
    std::vector<Diagnostic> out;
    for (const auto& d : result.diagnostics()) {
        if (d.code == code) out.push_back(d);
    }
    return out;
}

// ---------------------------------------------------------------------------
// Proc call arity (E002/E003) — no registry needed
// ---------------------------------------------------------------------------

TEST_CASE("proc call: too few args (E002)", "[analyser][diagnostics]") {
    auto result = analyse("proc add {a b} { return [expr {$a + $b}] }\nadd 1");
    auto e002 = by_code(result, DiagCode::E002);
    REQUIRE(e002.size() >= 1);
    CHECK(e002[0].message.find("::add") != std::string::npos);
}

TEST_CASE("proc call: too many args (E003)", "[analyser][diagnostics]") {
    auto result = analyse("proc add {a b} { return [expr {$a + $b}] }\nadd 1 2 3");
    auto e003 = by_code(result, DiagCode::E003);
    REQUIRE(e003.size() >= 1);
    CHECK(e003[0].message.find("::add") != std::string::npos);
}

TEST_CASE("proc call: with default arg no error", "[analyser][diagnostics]") {
    auto result = analyse(
        "proc greet {name {title Mr}} { return \"$title $name\" }\ngreet Bob");
    auto e = by_code(result, DiagCode::E002);
    auto e3 = by_code(result, DiagCode::E003);
    CHECK(e.empty());
    CHECK(e3.empty());
}

TEST_CASE("proc call: variadic args no error", "[analyser][diagnostics]") {
    auto result = analyse("proc log {level args} { return $level }\nlog info a b c");
    CHECK(by_code(result, DiagCode::E002).empty());
    CHECK(by_code(result, DiagCode::E003).empty());
}

TEST_CASE("proc call: namespace proc arity", "[analyser][diagnostics]") {
    auto result = analyse(R"(namespace eval math {
    proc add {a b} { return [expr {$a + $b}] }
    add 1
})");
    auto e002 = by_code(result, DiagCode::E002);
    REQUIRE(e002.size() >= 1);
    CHECK(e002[0].message.find("::math::add") != std::string::npos);
}

TEST_CASE("proc call: unknown commands no arity error", "[analyser][diagnostics]") {
    auto result = analyse("mycommand a b c d e");
    CHECK(errors(result).empty());
}

TEST_CASE("diagnostics: multiline error on correct line", "[analyser][diagnostics]") {
    auto reg = make_diag_registry();
    auto result = analyse("set x 1\nbreak extra\nset y 2", &reg);
    auto errs = errors(result);
    REQUIRE(errs.size() >= 1);
    CHECK(errs[0].range.start.line == 1);
}

// ---------------------------------------------------------------------------
// W214 — unused proc parameters
// ---------------------------------------------------------------------------

TEST_CASE("W214: unused param detected", "[analyser][w214]") {
    auto result = analyse("proc foo {x y} { puts $x }");
    auto w214 = by_code(result, DiagCode::W214);
    REQUIRE(w214.size() == 1);
    CHECK(w214[0].message.find("y") != std::string::npos);
    CHECK(w214[0].severity == Severity::HINT);
}

TEST_CASE("W214: all params used no warning", "[analyser][w214]") {
    auto result = analyse("proc foo {a b} { puts $a; puts $b }");
    CHECK(by_code(result, DiagCode::W214).empty());
}

TEST_CASE("W214: args param not flagged", "[analyser][w214]") {
    auto result = analyse("proc foo {args} { puts hello }");
    CHECK(by_code(result, DiagCode::W214).empty());
}

TEST_CASE("W214: underscore prefix not flagged", "[analyser][w214]") {
    auto result = analyse("proc foo {_unused x} { puts $x }");
    CHECK(by_code(result, DiagCode::W214).empty());
}

TEST_CASE("W214: multiple unused params", "[analyser][w214]") {
    auto result = analyse("proc foo {a b c} { puts hello }");
    CHECK(by_code(result, DiagCode::W214).size() == 3);
}

TEST_CASE("W214: param used in return", "[analyser][w214]") {
    auto result = analyse("proc foo {x} { return $x }");
    CHECK(by_code(result, DiagCode::W214).empty());
}

TEST_CASE("W214: param used in branch condition", "[analyser][w214]") {
    auto result = analyse("proc foo {x} { if {$x > 0} { puts yes } }");
    CHECK(by_code(result, DiagCode::W214).empty());
}

TEST_CASE("W214: truly unused still warns with return", "[analyser][w214]") {
    auto result = analyse("proc foo {x y} { return [expr {$x + 1}] }");
    auto w214 = by_code(result, DiagCode::W214);
    REQUIRE(w214.size() == 1);
    CHECK(w214[0].message.find("y") != std::string::npos);
}

// ---------------------------------------------------------------------------
// return [expr/alias] — issue #42
// ---------------------------------------------------------------------------

TEST_CASE("return: expr braced no W214", "[analyser][w214]") {
    auto result = analyse("proc foo {x} { return [expr {$x + 1}] }");
    CHECK(by_code(result, DiagCode::W214).empty());
}

TEST_CASE("return: cmd subst no W214", "[analyser][w214]") {
    auto reg = make_diag_registry();
    auto result = analyse("proc foo {x} { return [string length $x] }", &reg);
    CHECK(by_code(result, DiagCode::W214).empty());
}
