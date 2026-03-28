// Ported from tests/test_analyser.py — TestW123UnresolvedCommand.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>
#include <algorithm>
#include <string>
#include <vector>

using namespace tcl_lsp;

// Build a minimal Tcl-like registry for W123 tests.
static auto make_w123_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("if", CommandSig{Arity{2, 100}, {}});
    reg.add_command("while", CommandSig{Arity{2, 2}, {}});
    reg.add_command("for", CommandSig{Arity{4, 4}, {}});
    reg.add_command("foreach", CommandSig{Arity{3, 100}, {}});
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    reg.add_command("break", CommandSig{Arity{0, 0}, {}});
    reg.add_command("continue", CommandSig{Arity{0, 0}, {}});
    reg.add_command("switch", CommandSig{Arity{2, 100}, {}});
    reg.add_command("catch", CommandSig{Arity{1, 3}, {}});
    reg.add_command("try", CommandSig{Arity{1, 100}, {}});
    reg.add_command("package", CommandSig{Arity{1, 100}, {}});
    reg.add_command("source", CommandSig{Arity{1, 3}, {}});
    reg.add_command("interp", CommandSig{Arity{1, 100}, {}});
    reg.add_command("load", CommandSig{Arity{1, 3}, {}});
    reg.add_command("rename", CommandSig{Arity{2, 2}, {}});
    reg.add_command("namespace", CommandSig{Arity{1, 100}, {}});
    reg.add_command("lappend", CommandSig{Arity{1, 100}, {}});
    reg.add_command("incr", CommandSig{Arity{1, 2}, {}});
    reg.add_command("global", CommandSig{Arity{1, 100}, {}});
    reg.add_command("variable", CommandSig{Arity{1, 100}, {}});
    reg.add_command("dict", CommandSig{Arity{2, 100}, {}});
    reg.add_command("exec", CommandSig{Arity{1, 100}, {}});
    reg.add_command("auto_load", CommandSig{Arity{1, 1}, {}});
    reg.add_command("string", CommandSig{Arity{2, 100}, {}});
    return reg;
}

// Helper: extract W123 diagnostics.
static auto w123_diags(const AnalysisResult& result) -> std::vector<Diagnostic> {
    std::vector<Diagnostic> out;
    for (const auto& d : result.diagnostics()) {
        if (d.code == "W123") out.push_back(d);
    }
    return out;
}

// ---------------------------------------------------------------------------
// Basic W123 detection
// ---------------------------------------------------------------------------

TEST_CASE("W123: unknown command emits warning", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("mycommand a b c", &reg);
    auto diags = w123_diags(result);
    REQUIRE(diags.size() == 1);
    CHECK(diags[0].message.find("mycommand") != std::string::npos);
}

TEST_CASE("W123: known builtin no warning", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("set x 1", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: user proc no warning", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("proc mycommand {x} { puts $x }\nmycommand hello", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: forward-defined proc no warning", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("mycommand hello\nproc mycommand {x} { puts $x }", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: namespace-qualified skipped", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("::myns::cmd arg1", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: variable command skipped", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("set cmd puts\n$cmd hello", &reg);
    auto diags = w123_diags(result);
    for (const auto& d : diags) {
        CHECK(d.message.find("$cmd") == std::string::npos);
    }
}

TEST_CASE("W123: dynamic providers suppress", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("load mylib.so\nmycommand arg1", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: stub command no warning", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse(R"(# tcl-lsp: stubs-begin
# stub mycommand {arg1 arg2}
# tcl-lsp: stubs-end
mycommand x y)", &reg);
    CHECK(w123_diags(result).empty());
}

// ---------------------------------------------------------------------------
// Suppression patterns
// ---------------------------------------------------------------------------

TEST_CASE("W123: package require suppresses", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("package require Tk\nbutton .b -text hi", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: rename suppresses", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("rename puts myputs\nmyputs hello", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: namespace import suppresses", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("namespace import ::foo::*\nbar arg", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: lappend auto_path suppresses", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("lappend auto_path /opt/mylib\nmycmd arg", &reg);
    CHECK(w123_diags(result).empty());
}

// ---------------------------------------------------------------------------
// Did you mean?
// ---------------------------------------------------------------------------

TEST_CASE("W123: did you mean suggestion", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("putz hello", &reg);
    auto diags = w123_diags(result);
    REQUIRE(diags.size() == 1);
    CHECK(diags[0].message.find("puts") != std::string::npos);
    REQUIRE_FALSE(diags[0].fixes.empty());
    CHECK(diags[0].fixes[0].new_text == "puts");
}

TEST_CASE("W123: no suggestion for distant name", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("xyzzyplugh arg1", &reg);
    auto diags = w123_diags(result);
    REQUIRE(diags.size() == 1);
    CHECK(diags[0].message.find("did you mean") == std::string::npos);
}

// ---------------------------------------------------------------------------
// Unknown proc info
// ---------------------------------------------------------------------------

TEST_CASE("W123: unknown proc empty stub still warns", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("proc unknown {args} {}\nmycommand arg1", &reg);
    auto upi = result.unknown_proc_info();
    REQUIRE(upi.has_value());
    CHECK(upi->empty_stub == true);
    // Empty stub should not suppress W123.
    CHECK(w123_diags(result).size() == 1);
}

TEST_CASE("W123: unknown proc info populated", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse(R"(proc unknown {cmd args} {
    switch $cmd {
        foo { puts foo }
        bar { puts bar }
    }
})", &reg);
    auto upi = result.unknown_proc_info();
    REQUIRE(upi.has_value());
    CHECK_FALSE(upi->empty_stub);
}

TEST_CASE("W123: no unknown proc info by default", "[analyser][w123]") {
    auto result = analyse("set x 1");
    CHECK_FALSE(result.unknown_proc_info().has_value());
}

// ---------------------------------------------------------------------------
// Alias integration with W123
// ---------------------------------------------------------------------------

TEST_CASE("W123: alias command no warning", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("interp alias {} = {} expr\n= {1 + 2}", &reg);
    CHECK(w123_diags(result).empty());
}

TEST_CASE("W123: alias in did you mean", "[analyser][w123]") {
    auto reg = make_w123_registry();
    auto result = analyse("interp alias {} myput {} puts stdout\nmyptu hello", &reg);
    auto diags = w123_diags(result);
    REQUIRE(diags.size() == 1);
    CHECK(diags[0].message.find("myput") != std::string::npos);
}
