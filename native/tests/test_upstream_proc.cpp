// Ported from tests/test_upstream_proc.py — tests derived from Tcl's
// official tests/proc.test covering proc definitions, parameter handling,
// arity checking, and scope creation.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <string_view>

using namespace tcl_lsp;

static auto has_code(const AnalysisResult& r, std::string_view code) -> bool {
    for (const auto& d : r.diagnostics()) {
        if (to_string(d.code) == code)
            return true;
    }
    return false;
}

// proc-1.*: valid proc definitions

TEST_CASE("upstream proc: simple single parameter", "[upstream][proc]") {
    auto r = analyse("proc greet {name} { puts $name }");
    auto* p = r.find_proc("greet");
    REQUIRE(p != nullptr);
    CHECK(p->params.size() == 1);
    CHECK(p->params[0].name == "name");
}

TEST_CASE("upstream proc: multi-parameter", "[upstream][proc]") {
    auto r = analyse("proc add {a b} { expr {$a + $b} }");
    auto* p = r.find_proc("add");
    REQUIRE(p != nullptr);
    CHECK(p->params.size() == 2);
    CHECK(p->params[0].name == "a");
    CHECK(p->params[1].name == "b");
}

TEST_CASE("upstream proc: default parameter values", "[upstream][proc]") {
    auto r = analyse(R"(proc greet {name {greeting Hello}} { puts "$greeting $name" })");
    auto* p = r.find_proc("greet");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.size() == 2);
    CHECK(p->params[0].name == "name");
    CHECK_FALSE(p->params[0].default_value.has_value());
    CHECK(p->params[1].name == "greeting");
    REQUIRE(p->params[1].default_value.has_value());
    CHECK(p->params[1].default_value.value() == "Hello");
}

TEST_CASE("upstream proc: variadic args parameter", "[upstream][proc]") {
    auto r = analyse("proc mylog {msg args} { puts $msg }");
    auto* p = r.find_proc("mylog");
    REQUIRE(p != nullptr);
    CHECK(p->params.size() == 2);
    CHECK(p->params.back().name == "args");
}

TEST_CASE("upstream proc: no parameters", "[upstream][proc]") {
    auto r = analyse("proc noop {} { return }");
    auto* p = r.find_proc("noop");
    REQUIRE(p != nullptr);
    CHECK(p->params.empty());
}

TEST_CASE("upstream proc: creates child scope", "[upstream][proc]") {
    auto r = analyse("proc foo {x} { set y 1 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& child = *root.children[0];
    CHECK(child.kind == ScopeKind::PROC);
    CHECK(child.name == "foo");
}

// proc-2.*: namespace-qualified procs

TEST_CASE("upstream proc: namespace-qualified name", "[upstream][proc]") {
    auto r = analyse("namespace eval ::foo { proc bar {} { return 1 } }");
    auto* p = r.find_proc("bar");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::foo::bar");
}

TEST_CASE("upstream proc: nested namespace", "[upstream][proc]") {
    auto r = analyse("namespace eval ::a { namespace eval b { proc c {} {} } }");
    auto* p = r.find_proc("c");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::a::b::c");
}

// proc-3.*: arity diagnostics

TEST_CASE("upstream proc: too few args produces E002", "[upstream][proc]") {
    auto r = analyse("proc greet {name greeting} {}\ngreet Alice");
    CHECK(has_code(r, "E002"));
}

TEST_CASE("upstream proc: too many args produces E003", "[upstream][proc]") {
    auto r = analyse("proc greet {name} {}\ngreet Alice Bob");
    CHECK(has_code(r, "E003"));
}

TEST_CASE("upstream proc: correct arity no diagnostic", "[upstream][proc]") {
    auto r = analyse("proc greet {name} {}\ngreet Alice");
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

TEST_CASE("upstream proc: default param allows fewer args", "[upstream][proc]") {
    auto r = analyse("proc greet {{name World}} {}\ngreet");
    CHECK_FALSE(has_code(r, "E002"));
}

TEST_CASE("upstream proc: args param allows extra args", "[upstream][proc]") {
    auto r = analyse("proc mylog {msg args} {}\nmylog Hello World Extra");
    CHECK_FALSE(has_code(r, "E003"));
}

// proc-4.*: proc documentation from comments

TEST_CASE("upstream proc: doc from preceding comment", "[upstream][proc]") {
    auto r = analyse("# Calculate the sum\nproc add {a b} { expr {$a+$b} }");
    auto* p = r.find_proc("add");
    REQUIRE(p != nullptr);
    CHECK(p->doc.find("sum") != std::string::npos);
}

// proc-5.*: proc shadows built-in (W113)
// Requires a populated command registry; tested in test_analyser_w123.cpp.
