// Ported from tests/test_upstream_var.py — tests derived from Tcl's
// official tests/var.test covering variable set/get, global, variable
// declarations, and array operations.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>

using namespace tcl_lsp;

// var-1.*: basic set command

TEST_CASE("upstream var: set creates variable in scope", "[upstream][var]") {
    auto r = analyse("set x 42");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("x"));
}

TEST_CASE("upstream var: set with value defines variable", "[upstream][var]") {
    auto r = analyse("set greeting hello");
    const auto& root = r.global_scope();
    REQUIRE(root.variables.contains("greeting"));
    // set with 2 args is a definition (write).
    CHECK(root.variables.at("greeting").definition_range != Range::zero());
}

TEST_CASE("upstream var: set with one arg reads variable", "[upstream][var]") {
    auto r = analyse("set x 1\nset x");
    const auto& root = r.global_scope();
    REQUIRE(root.variables.contains("x"));
    // Variable should have references (the second 'set x' reads it).
    CHECK(!root.variables.at("x").references.empty());
}

// var-2.*: multiple variables

TEST_CASE("upstream var: multiple set commands", "[upstream][var]") {
    auto r = analyse("set a 1\nset b 2\nset c 3");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("a"));
    CHECK(root.variables.contains("b"));
    CHECK(root.variables.contains("c"));
}

// var-3.*: variable in proc scope

TEST_CASE("upstream var: set in proc creates local", "[upstream][var]") {
    auto r = analyse("proc foo {} { set local_var 99 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& proc = *root.children[0];
    CHECK(proc.variables.contains("local_var"));
    // Global scope should NOT have this variable.
    CHECK_FALSE(root.variables.contains("local_var"));
}

TEST_CASE("upstream var: proc param is a variable", "[upstream][var]") {
    auto r = analyse("proc foo {x y} { return $x }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& proc = *root.children[0];
    CHECK(proc.variables.contains("x"));
    CHECK(proc.variables.contains("y"));
}

// var-4.*: global command

TEST_CASE("upstream var: global declaration in proc", "[upstream][var]") {
    auto r = analyse("set count 0\nproc inc {} { global count; set count 1 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& proc = *root.children[0];
    CHECK(proc.variables.contains("count"));
}

// var-5.*: variable command (namespace variable)

TEST_CASE("upstream var: variable declaration in namespace", "[upstream][var]") {
    auto r = analyse("namespace eval ::ns { variable counter 0 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& ns = *root.children[0];
    CHECK(ns.variables.contains("counter"));
}

TEST_CASE("upstream var: variable with initialiser", "[upstream][var]") {
    auto r = analyse("namespace eval ::ns { variable x 10 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& ns = *root.children[0];
    REQUIRE(ns.variables.contains("x"));
}

// var-6.*: foreach creates loop variable

TEST_CASE("upstream var: foreach defines loop variable", "[upstream][var]") {
    auto r = analyse("foreach item {a b c} { puts $item }");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("item"));
}

TEST_CASE("upstream var: foreach with multiple vars", "[upstream][var]") {
    auto r = analyse("foreach {k v} {a 1 b 2} { puts $k }");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("k"));
    CHECK(root.variables.contains("v"));
}

// var-7.*: for loop variable

TEST_CASE("upstream var: for loop init creates variable", "[upstream][var]") {
    auto r = analyse("for {set i 0} {$i < 10} {incr i} { puts $i }");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("i"));
}

// var-8.*: catch result variable

TEST_CASE("upstream var: catch defines result variable", "[upstream][var]") {
    auto r = analyse("catch { expr {1/0} } result");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("result"));
}

// var-9.*: array-like set

TEST_CASE("upstream var: set with array syntax", "[upstream][var]") {
    auto r = analyse("set arr(key) value");
    const auto& root = r.global_scope();
    // Array variable should be tracked (base name or full key).
    CHECK((root.variables.contains("arr(key)") || root.variables.contains("arr")));
}
