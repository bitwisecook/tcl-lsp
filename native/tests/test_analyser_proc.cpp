// Ported from tests/test_analyser.py — TestProcAnalysis + TestNamespaceAnalysis.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>
#include <string_view>

using namespace tcl_lsp;

// ---------------------------------------------------------------------------
// TestProcAnalysis
// ---------------------------------------------------------------------------

TEST_CASE("proc: simple definition", "[analyser][proc]") {
    auto result = analyse("proc greet {name} { return \"Hello $name\" }");
    auto* p = result.find_proc("greet");
    REQUIRE(p != nullptr);
    REQUIRE(p->name == "greet");
    REQUIRE(p->qualified_name == "::greet");
    REQUIRE(p->params.size() == 1);
    REQUIRE(p->params[0].name == "name");
}

TEST_CASE("proc: multiple parameters", "[analyser][proc]") {
    auto result = analyse("proc add {a b} { expr {$a + $b} }");
    auto* p = result.find_proc("add");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.size() == 2);
    REQUIRE(p->params[0].name == "a");
    REQUIRE(p->params[1].name == "b");
}

TEST_CASE("proc: default parameter", "[analyser][proc]") {
    auto result = analyse("proc greet {{name World}} { puts $name }");
    auto* p = result.find_proc("greet");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.size() == 1);
    REQUIRE(p->params[0].name == "name");
    REQUIRE(p->params[0].default_value.has_value());
    REQUIRE(p->params[0].default_value.value() == "World");
}

TEST_CASE("proc: no parameters", "[analyser][proc]") {
    auto result = analyse("proc noop {} {}");
    auto* p = result.find_proc("noop");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.empty());
}

TEST_CASE("proc: in all_procs with qualified name", "[analyser][proc]") {
    auto result = analyse("proc greet {name} { return $name }");
    REQUIRE(result.all_procs().contains("::greet"));
}

TEST_CASE("proc: doc from preceding comment", "[analyser][proc]") {
    auto result = analyse("# This proc greets someone\nproc greet {name} { return $name }");
    auto* p = result.find_proc("greet");
    REQUIRE(p != nullptr);
    REQUIRE(p->doc.find("greets") != std::string::npos);
}

TEST_CASE("proc: multiple procs", "[analyser][proc]") {
    auto result = analyse("proc a {} {}\nproc b {} {}");
    REQUIRE(result.find_proc("a") != nullptr);
    REQUIRE(result.find_proc("b") != nullptr);
}

TEST_CASE("proc: creates child scope", "[analyser][proc]") {
    auto result = analyse("proc test {x} { set y 10 }");
    auto& global = result.global_scope();
    REQUIRE(global.children.size() == 1);
    auto* proc_scope = global.children[0].get();
    REQUIRE(proc_scope->kind == ScopeKind::PROC);
    REQUIRE(proc_scope->name == "test");
    // x is a parameter defined in proc scope
    REQUIRE(proc_scope->variables.contains("x"));
    // y is defined in proc scope by set
    REQUIRE(proc_scope->variables.contains("y"));
    // Neither should be in global scope
    REQUIRE_FALSE(global.variables.contains("x"));
    REQUIRE_FALSE(global.variables.contains("y"));
}

// ---------------------------------------------------------------------------
// TestNamespaceAnalysis
// ---------------------------------------------------------------------------

TEST_CASE("namespace eval: creates scope", "[analyser][namespace]") {
    auto result = analyse("namespace eval math { proc add {a b} { expr {$a+$b} } }");
    auto& global = result.global_scope();
    REQUIRE(global.children.size() == 1);
    auto* ns_scope = global.children[0].get();
    REQUIRE(ns_scope->kind == ScopeKind::NAMESPACE);
    REQUIRE(ns_scope->name == "math");
    REQUIRE(ns_scope->procs.contains("add"));
}

TEST_CASE("namespace eval: qualified proc name", "[analyser][namespace]") {
    auto result = analyse("namespace eval math { proc add {a b} {} }");
    auto* p = result.find_proc("add");
    REQUIRE(p != nullptr);
    REQUIRE(p->qualified_name == "::math::add");
}
