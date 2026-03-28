#include "tcl_lsp/analysis/analysis_result.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

TEST_CASE("AnalysisResult default has global scope", "[analysis_result]") {
    AnalysisResult result;
    REQUIRE(result.global_scope().kind == ScopeKind::GLOBAL);
    REQUIRE(result.global_scope().name == "::");
    REQUIRE(result.global_scope().parent == nullptr);
}

TEST_CASE("AnalysisResult find_proc by qualified name", "[analysis_result]") {
    AnalysisResult result;
    auto& scope = result.global_scope();
    scope.procs["greet"] = ProcDef{"greet", "::greet", {{"name", std::nullopt}},
                                   Range::zero(), Range::zero(), "", {}};
    result.all_procs["::greet"] = &scope.procs["greet"];

    REQUIRE(result.find_proc("greet") != nullptr);
    REQUIRE(result.find_proc("greet")->qualified_name == "::greet");
    REQUIRE(result.find_proc("::greet") != nullptr);
}

TEST_CASE("AnalysisResult find_proc returns nullptr for missing", "[analysis_result]") {
    AnalysisResult result;
    REQUIRE(result.find_proc("nonexistent") == nullptr);
}

TEST_CASE("AnalysisResult find_proc via bare name index", "[analysis_result]") {
    AnalysisResult result;
    auto& scope = result.global_scope();

    // Create a proc with a namespace-qualified key that doesn't match a bare lookup.
    scope.procs["add"] = ProcDef{"add", "::math::add", {}, Range::zero(), Range::zero(), "", {}};
    result.all_procs["::math::add"] = &scope.procs["add"];

    // Lookup by bare name "add" should find it via the bare-name index.
    REQUIRE(result.find_proc("add") != nullptr);
    REQUIRE(result.find_proc("add")->qualified_name == "::math::add");
}

TEST_CASE("AnalysisResult active_package_names", "[analysis_result]") {
    AnalysisResult result;
    result.package_requires.push_back({"Tk", "8.6", Range::zero(), false});
    result.package_requires.push_back({"http", "", Range::zero(), true});

    auto names = result.active_package_names();
    REQUIRE(names.contains("Tk"));
    REQUIRE(names.contains("http"));
    REQUIRE(names.size() == 2);
}

TEST_CASE("AnalysisResult package_context classifies correctly", "[analysis_result]") {
    AnalysisResult result;
    result.package_requires.push_back({"Tk", "8.6", Range::zero(), false});
    result.package_requires.push_back({"http", "", Range::zero(), true});
    result.package_provides.push_back({"mylib", "1.0", Range::zero()});
    result.has_dynamic_providers = true;

    auto ctx = result.package_context();
    REQUIRE(ctx.confidence("Tk") == Confidence::DEFINITE);
    REQUIRE(ctx.confidence("http") == Confidence::PROBABLE);
    REQUIRE(ctx.provided.contains("mylib"));
    REQUIRE(ctx.unknown_providers == true);
}

TEST_CASE("AnalysisResult package_context unconditional overrides conditional", "[analysis_result]") {
    AnalysisResult result;
    // Same package both conditional and unconditional.
    result.package_requires.push_back({"Tk", "", Range::zero(), true});
    result.package_requires.push_back({"Tk", "8.6", Range::zero(), false});

    auto ctx = result.package_context();
    REQUIRE(ctx.confidence("Tk") == Confidence::DEFINITE);
    REQUIRE_FALSE(ctx.probable.contains("Tk"));
}

TEST_CASE("AnalysisResult regex_position_set", "[analysis_result]") {
    AnalysisResult result;
    result.regex_patterns.push_back({{{2, 5, 30}, {2, 15, 40}}, "^foo$", "regexp"});
    result.regex_patterns.push_back({{{5, 0, 80}, {5, 10, 90}}, "\\d+", "regsub"});

    auto& positions = result.regex_position_set();
    REQUIRE(positions.size() == 2);
    REQUIRE(positions.contains({2, 5}));
    REQUIRE(positions.contains({5, 0}));

    // Verify caching: same reference returned.
    auto& positions2 = result.regex_position_set();
    REQUIRE(&positions == &positions2);
}

TEST_CASE("AnalysisResult copy_for_snapshot creates independent copy", "[analysis_result]") {
    AnalysisResult original;
    auto& scope = original.global_scope();

    // Add a proc with params.
    scope.procs["greet"] = ProcDef{"greet", "::greet", {{"name", std::nullopt}},
                                   Range::zero(), Range::zero(), "Says hello", {}};
    original.all_procs["::greet"] = &scope.procs["greet"];

    // Add a variable.
    scope.variables["x"] = VarDef{"x", Range::zero(), {}, false};
    original.all_variables["x"] = &scope.variables["x"];

    // Add a child scope.
    auto child = std::make_unique<Scope>();
    child->kind = ScopeKind::PROC;
    child->name = "greet";
    child->parent = &scope;
    child->variables["name"] = VarDef{"name", Range::zero(), {}, true};
    scope.children.push_back(std::move(child));

    // Add other data.
    original.diagnostics.push_back({Range::zero(), Severity::WARNING, "W100", "test", {}});
    original.package_requires.push_back({"Tk", "8.6", Range::zero(), false});
    original.regex_patterns.push_back({Range::zero(), "^foo$", "regexp"});

    // Create snapshot.
    auto snap = original.copy_for_snapshot();

    // Verify structure.
    REQUIRE(snap.global_scope().kind == ScopeKind::GLOBAL);
    REQUIRE(snap.global_scope().procs.contains("greet"));
    REQUIRE(snap.global_scope().procs["greet"].qualified_name == "::greet");
    REQUIRE(snap.global_scope().variables.contains("x"));
    REQUIRE(snap.global_scope().children.size() == 1);
    REQUIRE(snap.global_scope().children[0]->kind == ScopeKind::PROC);
    REQUIRE(snap.global_scope().children[0]->parent == &snap.global_scope());
    REQUIRE(snap.global_scope().children[0]->variables.contains("name"));

    // Verify flat indexes point into copied tree.
    REQUIRE(snap.find_proc("greet") != nullptr);
    REQUIRE(snap.find_proc("greet") != original.find_proc("greet")); // different pointers

    // Verify independence: mutate original, snap unchanged.
    original.global_scope().procs["greet"].doc = "CHANGED";
    REQUIRE(snap.global_scope().procs["greet"].doc == "Says hello");

    original.diagnostics.clear();
    REQUIRE(snap.diagnostics.size() == 1);

    // No manual cleanup needed — unique_ptr handles it.
}

TEST_CASE("AnalysisResult move construction", "[analysis_result]") {
    AnalysisResult a;
    a.global_scope().procs["foo"] = ProcDef{"foo", "::foo", {}, Range::zero(), Range::zero(), "", {}};
    a.all_procs["::foo"] = &a.global_scope().procs["foo"];

    AnalysisResult b = std::move(a);
    REQUIRE(b.global_scope().procs.contains("foo"));
    REQUIRE(b.find_proc("foo") != nullptr);
}
