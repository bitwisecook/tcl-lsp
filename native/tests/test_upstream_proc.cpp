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

TEST_CASE("upstream proc: param in proc scope", "[upstream][proc]") {
    auto r = analyse("proc double {x} { expr {$x * 2} }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& proc_scope = *root.children[0];
    CHECK(proc_scope.kind == ScopeKind::PROC);
    CHECK(proc_scope.variables.contains("x"));
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

TEST_CASE("upstream proc: qualified name field", "[upstream][proc]") {
    auto r = analyse("proc greet {name} { puts $name }");
    auto* p = r.find_proc("greet");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::greet");
}

TEST_CASE("upstream proc: namespace eval proc in all_procs", "[upstream][proc]") {
    auto r = analyse("namespace eval ns {\n    proc foo {} { return 1 }\n}");
    CHECK(r.all_procs().contains("::ns::foo"));
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

TEST_CASE("upstream proc: variadic still needs required args", "[upstream][proc]") {
    auto r = analyse("proc mylog {msg args} {}\nmylog");
    CHECK(has_code(r, "E002"));
}

// proc-4.*: proc documentation from comments

TEST_CASE("upstream proc: doc from preceding comment", "[upstream][proc]") {
    auto r = analyse("# Calculate the sum\nproc add {a b} { expr {$a+$b} }");
    auto* p = r.find_proc("add");
    REQUIRE(p != nullptr);
    CHECK(p->doc.find("sum") != std::string::npos);
}

TEST_CASE("upstream proc: no doc without comment", "[upstream][proc]") {
    auto r = analyse("proc foo {} { return }");
    auto* p = r.find_proc("foo");
    REQUIRE(p != nullptr);
    CHECK(p->doc.empty());
}

// proc-5.*: proc shadows built-in (W113)
// Requires a populated command registry; tested in test_analyser_w123.cpp.

// proc-6.*: scope isolation

TEST_CASE("upstream proc: local not visible globally", "[upstream][proc]") {
    auto r = analyse("proc foo {} { set local_var 42 }\nset x $local_var");
    const auto& root = r.global_scope();
    CHECK_FALSE(root.variables.contains("local_var"));
}

TEST_CASE("upstream proc: global not visible in proc", "[upstream][proc]") {
    auto r = analyse("set gvar 99\nproc bar {} { puts $gvar }");
    const auto& root = r.global_scope();
    // Find the proc scope
    const Scope* proc_scope = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC) {
            proc_scope = child.get();
            break;
        }
    }
    REQUIRE(proc_scope != nullptr);
    // The proc scope should not have 'gvar' unless an explicit
    // 'global gvar' declaration is present.
    CHECK_FALSE(proc_scope->variables.contains("gvar"));
}

TEST_CASE("upstream proc: multiple procs separate scopes", "[upstream][proc]") {
    auto r = analyse("proc alpha {} { set a_var 1 }\nproc beta {} { set b_var 2 }");
    const auto& root = r.global_scope();
    // Collect proc scopes
    std::vector<const Scope*> proc_scopes;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC)
            proc_scopes.push_back(child.get());
    }
    REQUIRE(proc_scopes.size() == 2);

    // Identify scopes by name
    const Scope* alpha_scope = nullptr;
    const Scope* beta_scope = nullptr;
    for (const auto* s : proc_scopes) {
        if (s->name == "alpha")
            alpha_scope = s;
        if (s->name == "beta")
            beta_scope = s;
    }
    REQUIRE(alpha_scope != nullptr);
    REQUIRE(beta_scope != nullptr);

    // Each scope contains only its own variable
    CHECK(alpha_scope->variables.contains("a_var"));
    CHECK_FALSE(alpha_scope->variables.contains("b_var"));
    CHECK(beta_scope->variables.contains("b_var"));
    CHECK_FALSE(beta_scope->variables.contains("a_var"));
}

// proc-7.*: builtin arity checks
// These require a populated command registry and are covered in
// test_analyser_diagnostics.cpp (E001 tests with native_registry).
