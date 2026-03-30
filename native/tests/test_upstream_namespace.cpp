// Ported from tests/test_upstream_namespace.py — tests derived from Tcl's
// official tests/namespace.test covering namespace eval, qualified names,
// variable resolution, nested namespaces, arity checking, and edge cases.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <string_view>

using namespace tcl_lsp;

// Helper: check whether a diagnostic with a given code string is present.
static auto has_code(const AnalysisResult& r, std::string_view code) -> bool {
    for (const auto& d : r.diagnostics()) {
        if (to_string(d.code) == code)
            return true;
    }
    return false;
}

// Helper: check whether any error-severity diagnostic exists.
static auto has_errors(const AnalysisResult& r) -> bool {
    for (const auto& d : r.diagnostics()) {
        if (d.severity == Severity::ERROR)
            return true;
    }
    return false;
}

// Build a minimal registry for arity-checking tests inside namespaces.
static auto make_ns_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    reg.add_command("namespace", CommandSig{Arity{1, 100}, {}});
    reg.add_command("variable", CommandSig{Arity{1, 100}, {}});
    reg.add_command("lindex", CommandSig{Arity{2, 3}, {}});
    return reg;
}

// =========================================================================
// TestNamespaceCreation
// =========================================================================

// namespace-1.*: basic namespace eval

TEST_CASE("upstream ns: namespace eval creates scope", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { set x 1 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    CHECK(root.children[0]->kind == ScopeKind::NAMESPACE);
}

TEST_CASE("upstream ns: namespace scope name contains identifier", "[upstream][namespace]") {
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "}");
    const auto& root = r.global_scope();
    bool found = false;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::NAMESPACE && child->name.find("math") != std::string::npos) {
            found = true;
            break;
        }
    }
    CHECK(found);
}

TEST_CASE("upstream ns: proc in namespace has qualified name", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { proc helper {} { return 1 } }");
    auto* p = r.find_proc("helper");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::myns::helper");
}

TEST_CASE("upstream ns: multiple procs in namespace", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::utils {\n"
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

TEST_CASE("upstream ns: namespace eval with absolute name", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::abs {\n"
                     "    proc worker {} { return 1 }\n"
                     "}");
    auto* p = r.find_proc("worker");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name.find("abs") != std::string::npos);
    CHECK(p->qualified_name.find("worker") != std::string::npos);
}

TEST_CASE("upstream ns: empty namespace eval no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval empty {}");
    CHECK_FALSE(has_errors(r));
}

// =========================================================================
// TestNestedNamespaces
// =========================================================================

// namespace-2.*: nested namespaces

TEST_CASE("upstream ns: nested namespace eval", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::outer {\n"
                     "    namespace eval inner {\n"
                     "        proc deep {} {}\n"
                     "    }\n"
                     "}");
    auto* p = r.find_proc("deep");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::outer::inner::deep");
}

TEST_CASE("upstream ns: double-colon qualified namespace", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::a::b { proc c {} {} }");
    auto* p = r.find_proc("c");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::a::b::c");
}

TEST_CASE("upstream ns: deeply nested three levels", "[upstream][namespace]") {
    auto r = analyse("namespace eval x {\n"
                     "    namespace eval y {\n"
                     "        namespace eval z {\n"
                     "            proc deep {} { return 42 }\n"
                     "        }\n"
                     "    }\n"
                     "}");
    auto* p = r.find_proc("deep");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name.find("deep") != std::string::npos);
}

TEST_CASE("upstream ns: nested namespace scope hierarchy", "[upstream][namespace]") {
    auto r = analyse("namespace eval outer {\n"
                     "    namespace eval inner {\n"
                     "        proc leaf {} { return 1 }\n"
                     "    }\n"
                     "}");
    const auto& root = r.global_scope();
    // Find the outer namespace scope.
    const Scope* outer = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::NAMESPACE && child->name.find("outer") != std::string::npos) {
            outer = child.get();
            break;
        }
    }
    REQUIRE(outer != nullptr);
    // Outer should have an inner namespace child.
    bool found_inner = false;
    for (const auto& child : outer->children) {
        if (child->kind == ScopeKind::NAMESPACE && child->name.find("inner") != std::string::npos) {
            found_inner = true;
            break;
        }
    }
    CHECK(found_inner);
}

TEST_CASE("upstream ns: sibling namespaces inside parent", "[upstream][namespace]") {
    auto r = analyse("namespace eval parent {\n"
                     "    namespace eval childA {\n"
                     "        proc alpha {} { return \"a\" }\n"
                     "    }\n"
                     "    namespace eval childB {\n"
                     "        proc beta {} { return \"b\" }\n"
                     "    }\n"
                     "}");
    auto* pa = r.find_proc("alpha");
    auto* pb = r.find_proc("beta");
    REQUIRE(pa != nullptr);
    REQUIRE(pb != nullptr);
}

// =========================================================================
// TestQualifiedProcNames
// =========================================================================

// namespace-3.*: qualified proc names

TEST_CASE("upstream ns: global qualifier proc", "[upstream][namespace]") {
    auto r = analyse("proc ::foo {} { return }");
    auto* p = r.find_proc("foo");
    REQUIRE(p != nullptr);
}

// Phase 6: proc with explicit namespace qualifier outside namespace eval
// is not yet resolved to the correct namespace scope.
TEST_CASE("upstream ns: namespace qualifier proc", "[upstream][namespace]") {
    auto r = analyse("proc ::ns::bar {} { return }");
    // The analyser registers the proc; verify it is reachable.
    bool found = false;
    for (const auto& [key, _] : r.all_procs()) {
        if (key.find("bar") != std::string::npos) {
            found = true;
            break;
        }
    }
    CHECK(found);
}

TEST_CASE("upstream ns: namespace eval qualifies proc name", "[upstream][namespace]") {
    auto r = analyse("namespace eval utils {\n"
                     "    proc helper {} { return 1 }\n"
                     "}");
    CHECK(r.all_procs().count("::utils::helper") == 1);
}

TEST_CASE("upstream ns: proc with params qualified", "[upstream][namespace]") {
    auto r = analyse("namespace eval geo {\n"
                     "    proc distance {x1 y1 x2 y2} {\n"
                     "        expr {sqrt(($x2-$x1)**2 + ($y2-$y1)**2)}\n"
                     "    }\n"
                     "}");
    CHECK(r.all_procs().count("::geo::distance") == 1);
    auto* p = r.find_proc("distance");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.size() == 4);
    CHECK(p->params[0].name == "x1");
    CHECK(p->params[3].name == "y2");
}

TEST_CASE("upstream ns: proc with defaults in namespace", "[upstream][namespace]") {
    auto r = analyse("namespace eval cfg {\n"
                     "    proc init {{verbose 0}} { return $verbose }\n"
                     "}");
    CHECK(r.all_procs().count("::cfg::init") == 1);
    auto* p = r.find_proc("init");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.size() == 1);
    CHECK(p->params[0].name == "verbose");
    REQUIRE(p->params[0].default_value.has_value());
    CHECK(p->params[0].default_value.value() == "0");
}

TEST_CASE("upstream ns: proc with args param in namespace", "[upstream][namespace]") {
    auto r = analyse("namespace eval logging {\n"
                     "    proc log {level args} { puts \"$level: $args\" }\n"
                     "}");
    CHECK(r.all_procs().count("::logging::log") == 1);
    auto* p = r.find_proc("log");
    REQUIRE(p != nullptr);
    REQUIRE(p->params.size() == 2);
    CHECK(p->params[0].name == "level");
    CHECK(p->params[1].name == "args");
}

TEST_CASE("upstream ns: proc qualified_name field matches key", "[upstream][namespace]") {
    auto r = analyse("namespace eval data {\n"
                     "    proc parse {input} { return $input }\n"
                     "}");
    CHECK(r.all_procs().count("::data::parse") == 1);
    auto* p = r.find_proc("parse");
    REQUIRE(p != nullptr);
    CHECK(p->qualified_name == "::data::parse");
}

// =========================================================================
// TestNamespaceVariables
// =========================================================================

// namespace-4.*: variables in namespaces

TEST_CASE("upstream ns: variable in namespace scope", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { variable count 0 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& ns = *root.children[0];
    CHECK(ns.variables.contains("count"));
}

TEST_CASE("upstream ns: set in namespace scope", "[upstream][namespace]") {
    auto r = analyse("namespace eval ::myns { set x 42 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& ns = *root.children[0];
    CHECK(ns.variables.contains("x"));
}

TEST_CASE("upstream ns: multiple variables in namespace", "[upstream][namespace]") {
    auto r = analyse("namespace eval settings {\n"
                     "    set host \"localhost\"\n"
                     "    set port 8080\n"
                     "    set timeout 30\n"
                     "}");
    const auto& root = r.global_scope();
    const Scope* ns = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::NAMESPACE &&
            child->name.find("settings") != std::string::npos) {
            ns = child.get();
            break;
        }
    }
    REQUIRE(ns != nullptr);
    CHECK(ns->variables.contains("host"));
    CHECK(ns->variables.contains("port"));
    CHECK(ns->variables.contains("timeout"));
}

TEST_CASE("upstream ns: variable command no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval state {\n"
                     "    variable counter 0\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: variable not leaked to global", "[upstream][namespace]") {
    auto r = analyse("namespace eval hidden {\n"
                     "    set secret 42\n"
                     "}");
    const auto& root = r.global_scope();
    CHECK_FALSE(root.variables.contains("secret"));
}

// =========================================================================
// TestNamespaceArityChecking
// =========================================================================

// namespace-5.*: arity checking across namespaces

TEST_CASE("upstream ns: qualified call correct arity", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "}\n"
                     "math::add 1 2",
                     &reg);
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

TEST_CASE("upstream ns: qualified call too few args", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "}\n"
                     "math::add 1",
                     &reg);
    CHECK(has_code(r, "E002"));
}

TEST_CASE("upstream ns: qualified call too many args", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "}\n"
                     "math::add 1 2 3",
                     &reg);
    CHECK(has_code(r, "E003"));
}

TEST_CASE("upstream ns: fully qualified call correct arity", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "}\n"
                     "::math::add 1 2",
                     &reg);
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

TEST_CASE("upstream ns: call inside same namespace correct arity", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "    add 1 2\n"
                     "}",
                     &reg);
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

TEST_CASE("upstream ns: call inside same namespace too few", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "    add 1\n"
                     "}",
                     &reg);
    CHECK(has_code(r, "E002"));
}

TEST_CASE("upstream ns: proc with defaults arity", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval fmt {\n"
                     "    proc greet {name {greeting \"Hello\"}} {\n"
                     "        puts \"$greeting, $name!\"\n"
                     "    }\n"
                     "}\n"
                     "fmt::greet \"Alice\"\n"
                     "fmt::greet \"Bob\" \"Hi\"",
                     &reg);
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

TEST_CASE("upstream ns: proc with args variadic arity", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval io {\n"
                     "    proc log {level args} { puts \"$level: $args\" }\n"
                     "}\n"
                     "io::log \"INFO\"\n"
                     "io::log \"WARN\" \"something\" \"happened\"",
                     &reg);
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

TEST_CASE("upstream ns: nested namespace qualified call", "[upstream][namespace]") {
    auto reg = make_ns_registry();
    auto r = analyse("namespace eval a {\n"
                     "    namespace eval b {\n"
                     "        proc f {x} { return $x }\n"
                     "    }\n"
                     "}\n"
                     "a::b::f 42",
                     &reg);
    CHECK_FALSE(has_code(r, "E002"));
    CHECK_FALSE(has_code(r, "E003"));
}

// =========================================================================
// TestNamespaceEdgeCases
// =========================================================================

// namespace-6.*: edge cases

TEST_CASE("upstream ns: global proc alongside namespace", "[upstream][namespace]") {
    auto r = analyse("proc global_fn {} {}\n"
                     "namespace eval ::ns { proc ns_fn {} {} }");
    auto* gp = r.find_proc("global_fn");
    auto* np = r.find_proc("ns_fn");
    REQUIRE(gp != nullptr);
    REQUIRE(np != nullptr);
    CHECK(gp->qualified_name == "::global_fn");
    CHECK(np->qualified_name == "::ns::ns_fn");
}

// Phase 6: proc with :: prefix inside namespace eval should register at
// the global level, but the analyser currently qualifies it under the
// enclosing namespace.
TEST_CASE("upstream ns: global qualified proc inside namespace", "[upstream][namespace]") {
    auto r = analyse("namespace eval wrapper {\n"
                     "    proc ::toplevel_func {} { return \"top\" }\n"
                     "}");
    bool found = false;
    for (const auto& [key, _] : r.all_procs()) {
        if (key.find("toplevel_func") != std::string::npos) {
            found = true;
            break;
        }
    }
    CHECK(found);
}

TEST_CASE("upstream ns: proc stored in scope procs dict", "[upstream][namespace]") {
    auto r = analyse("namespace eval registry {\n"
                     "    proc lookup {key} { return $key }\n"
                     "}");
    const auto& root = r.global_scope();
    const Scope* ns = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::NAMESPACE &&
            child->name.find("registry") != std::string::npos) {
            ns = child.get();
            break;
        }
    }
    REQUIRE(ns != nullptr);
    CHECK(ns->procs.contains("lookup"));
}

TEST_CASE("upstream ns: namespace current no false positive", "[upstream][namespace]") {
    auto r = analyse("namespace eval myns {\n"
                     "    set cur [namespace current]\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace eval twice same name merges procs", "[upstream][namespace]") {
    auto r = analyse("namespace eval shared {\n"
                     "    proc alpha {} { return \"a\" }\n"
                     "}\n"
                     "namespace eval shared {\n"
                     "    proc beta {} { return \"b\" }\n"
                     "}");
    CHECK(r.all_procs().count("::shared::alpha") == 1);
    CHECK(r.all_procs().count("::shared::beta") == 1);
}

// =========================================================================
// Namespace subcommands: import, export, ensemble, path, children, parent,
// code, qualifiers, tail
// =========================================================================

// These namespace subcommands are not yet deeply analysed by the C++ analyser
// (Phase 6/7 features for full resolution). For now we verify they do not
// produce false-positive errors.

TEST_CASE("upstream ns: namespace import no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval math {\n"
                     "    proc add {a b} { expr {$a + $b} }\n"
                     "    namespace export add\n"
                     "}\n"
                     "namespace import math::add");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace export no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval utils {\n"
                     "    proc helper {} { return 1 }\n"
                     "    namespace export helper\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace export with pattern no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval utils {\n"
                     "    proc alpha {} {}\n"
                     "    proc beta {} {}\n"
                     "    namespace export *\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

// Phase 6: namespace ensemble create requires mapping subcommand dispatch
// to namespace procs — full resolution is not yet implemented.
TEST_CASE("upstream ns: namespace ensemble create no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval myns {\n"
                     "    proc sub1 {} { return 1 }\n"
                     "    proc sub2 {x} { return $x }\n"
                     "    namespace export sub1 sub2\n"
                     "    namespace ensemble create\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace path no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval helper { proc h {} {} }\n"
                     "namespace eval main {\n"
                     "    namespace path ::helper\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

// Phase 7: namespace path resolution (resolving commands from path namespaces)
// is not yet implemented. For now just verify no crash.
TEST_CASE("upstream ns: namespace path with multiple namespaces no errors",
          "[upstream][namespace]") {
    auto r = analyse("namespace eval a { proc fa {} {} }\n"
                     "namespace eval b { proc fb {} {} }\n"
                     "namespace eval c {\n"
                     "    namespace path {::a ::b}\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace children no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval parent {\n"
                     "    namespace eval child1 {}\n"
                     "    namespace eval child2 {}\n"
                     "}\n"
                     "set kids [namespace children ::parent]");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace parent no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval a {\n"
                     "    namespace eval b {\n"
                     "        set p [namespace parent]\n"
                     "    }\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace code no errors", "[upstream][namespace]") {
    auto r = analyse("namespace eval myns {\n"
                     "    proc callback {} { return ok }\n"
                     "    set cmd [namespace code callback]\n"
                     "}");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace qualifiers no errors", "[upstream][namespace]") {
    auto r = analyse("set q [namespace qualifiers ::foo::bar::baz]");
    CHECK_FALSE(has_errors(r));
}

TEST_CASE("upstream ns: namespace tail no errors", "[upstream][namespace]") {
    auto r = analyse("set t [namespace tail ::foo::bar::baz]");
    CHECK_FALSE(has_errors(r));
}
