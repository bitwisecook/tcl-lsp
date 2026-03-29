// Ported from tests/test_upstream_namespace.py — tests derived from Tcl's
// official tests/namespace.test covering namespace eval, qualified names,
// variable resolution, and nested namespaces.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>
#include <string>

using namespace tcl_lsp;

// namespace-1.*: basic namespace eval

TEST_CASE("upstream ns: namespace eval creates scope",
          "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { set x 1 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    CHECK(root.children[0]->kind == ScopeKind::NAMESPACE);
}

TEST_CASE("upstream ns: proc in namespace has qualified name",
          "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { proc helper {} { return 1 } }");
    auto* p = r.find_proc("helper");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::myns::helper");
}

// namespace-2.*: nested namespaces

TEST_CASE("upstream ns: nested namespace eval",
          "[upstream][namespace]") {
    auto r = analyse(
        "namespace eval ::outer {\n"
        "    namespace eval inner {\n"
        "        proc deep {} {}\n"
        "    }\n"
        "}");
    auto* p = r.find_proc("deep");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::outer::inner::deep");
}

TEST_CASE("upstream ns: double-colon qualified namespace",
          "[upstream][namespace]") {
    auto r = analyse("namespace eval ::a::b { proc c {} {} }");
    auto* p = r.find_proc("c");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::a::b::c");
}

// namespace-3.*: variables in namespaces

TEST_CASE("upstream ns: variable in namespace scope",
          "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { variable count 0 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& ns = *root.children[0];
    CHECK(ns.variables.contains("count"));
}

TEST_CASE("upstream ns: set in namespace scope",
          "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { set x 42 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& ns = *root.children[0];
    CHECK(ns.variables.contains("x"));
}

// namespace-4.*: proc calling across namespaces
// Cross-namespace resolution (::lib::greet) requires a populated command
// registry; tested in test_analyser_w123.cpp.

// namespace-5.*: multiple procs in same namespace

TEST_CASE("upstream ns: multiple procs in namespace",
          "[upstream][namespace]") {
    auto r = analyse(
        "namespace eval ::utils {\n"
        "    proc a {} {}\n"
        "    proc b {x} { return $x }\n"
        "}");
    auto* pa = r.find_proc("a");
    auto* pb = r.find_proc("b");
    REQUIRE(pa != nullptr);
    REQUIRE(pb != nullptr);
    CHECK(pa->qualified_name == "::utils::a");
    CHECK(pb->qualified_name == "::utils::b");
}

// namespace-6.*: global scope still works

TEST_CASE("upstream ns: global proc alongside namespace",
          "[upstream][namespace]") {
    auto r = analyse(
        "proc global_fn {} {}\n"
        "namespace eval ::ns { proc ns_fn {} {} }");
    auto* gp = r.find_proc("global_fn");
    auto* np = r.find_proc("ns_fn");
    REQUIRE(gp != nullptr);
    REQUIRE(np != nullptr);
    CHECK(gp->qualified_name == "::global_fn");
    CHECK(np->qualified_name == "::ns::ns_fn");
}
