#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("AnalysisResult default has global scope", "[analysis_result]") {
    AnalysisResult result;
    REQUIRE(result.global_scope().kind == ScopeKind::GLOBAL);
    REQUIRE(result.global_scope().name == "::");
    REQUIRE(result.global_scope().parent == nullptr);
}

TEST_CASE("AnalysisResult find_proc by qualified name", "[analysis_result]") {
    auto result = analyse("proc greet {name} { return $name }");
    REQUIRE(result.find_proc("greet") != nullptr);
    REQUIRE(result.find_proc("greet")->qualified_name == "::greet");
    REQUIRE(result.find_proc("::greet") != nullptr);
}

TEST_CASE("AnalysisResult find_proc returns nullptr for missing", "[analysis_result]") {
    AnalysisResult result;
    REQUIRE(result.find_proc("nonexistent") == nullptr);
}

TEST_CASE("AnalysisResult find_proc via bare name index", "[analysis_result]") {
    auto result = analyse("namespace eval math { proc add {a b} {} }");
    REQUIRE(result.find_proc("add") != nullptr);
    REQUIRE(result.find_proc("add")->qualified_name == "::math::add");
}

TEST_CASE("AnalysisResult active_package_names", "[analysis_result]") {
    auto result = analyse("package require Tk 8.6\npackage require http");
    auto names = result.active_package_names();
    REQUIRE(names.contains("Tk"));
    REQUIRE(names.contains("http"));
    REQUIRE(names.size() == 2);
}

TEST_CASE("AnalysisResult package_context classifies correctly", "[analysis_result]") {
    // Unconditional require
    auto result = analyse("package require Tk 8.6");
    auto ctx = result.package_context();
    REQUIRE(ctx.confidence("Tk") == Confidence::DEFINITE);
}

TEST_CASE("AnalysisResult package_context conditional", "[analysis_result]") {
    // catch wrapping makes it conditional
    auto result = analyse("catch { package require http }");
    auto ctx = result.package_context();
    REQUIRE(ctx.confidence("http") == Confidence::PROBABLE);
}

TEST_CASE("AnalysisResult package_context unconditional overrides conditional", "[analysis_result]") {
    auto result = analyse("catch { package require Tk }\npackage require Tk 8.6");
    auto ctx = result.package_context();
    REQUIRE(ctx.confidence("Tk") == Confidence::DEFINITE);
    REQUIRE_FALSE(ctx.probable.contains("Tk"));
}

TEST_CASE("AnalysisResult regex_position_set", "[analysis_result]") {
    TestCommandRegistry registry;
    registry.add_command("regexp", CommandSig{{1, 100}, {{0, ArgRole::PATTERN}}});

    auto result = Analyser(&registry).analyse("regexp {^foo$} test");
    auto& positions = result.regex_position_set();
    REQUIRE(positions.size() == 1);

    // Verify caching: same reference returned.
    auto& positions2 = result.regex_position_set();
    REQUIRE(&positions == &positions2);
}

TEST_CASE("AnalysisResult copy_for_snapshot creates independent copy", "[analysis_result]") {
    auto original = analyse(
        "proc greet {name} { set y 10 }\n"
        "package require Tk 8.6");

    // Create snapshot.
    auto snap = original.copy_for_snapshot();

    // Verify structure.
    REQUIRE(snap.global_scope().kind == ScopeKind::GLOBAL);
    REQUIRE(snap.global_scope().procs.contains("greet"));
    REQUIRE(snap.global_scope().procs.at("greet").qualified_name == "::greet");
    REQUIRE(snap.global_scope().children.size() == 1);
    REQUIRE(snap.global_scope().children[0]->kind == ScopeKind::PROC);
    REQUIRE(snap.global_scope().children[0]->parent == &snap.global_scope());

    // Verify flat indexes point into copied tree.
    REQUIRE(snap.find_proc("greet") != nullptr);
    REQUIRE(snap.find_proc("greet") != original.find_proc("greet")); // different pointers

    // Verify independence: mutate original scope, snap unchanged.
    original.global_scope().procs["greet"].doc = "CHANGED";
    REQUIRE(snap.global_scope().procs.at("greet").doc.empty());

    // Verify diagnostics independence.
    auto orig_diag_count = original.diagnostics().size();
    auto snap_diag_count = snap.diagnostics().size();
    REQUIRE(orig_diag_count == snap_diag_count);
}

TEST_CASE("AnalysisResult move construction", "[analysis_result]") {
    auto a = analyse("proc foo {} {}");
    REQUIRE(a.find_proc("foo") != nullptr);

    AnalysisResult b = std::move(a);
    REQUIRE(b.global_scope().procs.contains("foo"));
    REQUIRE(b.find_proc("foo") != nullptr);
}
