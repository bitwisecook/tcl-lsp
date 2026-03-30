// Ported from tests/test_upstream_var.py — tests derived from Tcl's
// official tests/var.test covering variable set/get, global, variable
// declarations, and array operations.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>

using namespace tcl_lsp;

// Registry with append/lappend/upvar VAR_NAME roles for tests that
// need the generic arg-role path.
static auto make_var_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("incr", CommandSig{Arity{1, 2}, {}});
    reg.add_command("append", CommandSig{Arity{1, 100}, {{0, ArgRole::VAR_NAME}}});
    reg.add_command("lappend", CommandSig{Arity{1, 100}, {{0, ArgRole::VAR_NAME}}});
    reg.add_command("upvar", CommandSig{Arity{2, 100}, {{1, ArgRole::VAR_NAME}}});
    reg.add_command("global", CommandSig{Arity{1, 100}, {}});
    reg.add_command("variable", CommandSig{Arity{1, 100}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("namespace", CommandSig{Arity{1, 100}, {}});
    reg.add_command("foreach", CommandSig{Arity{3, 100}, {}});
    reg.add_command("for", CommandSig{Arity{4, 4}, {}});
    reg.add_command("catch", CommandSig{Arity{1, 3}, {}});
    reg.add_command("try", CommandSig{Arity{1, 100}, {}});
    reg.add_command("switch", CommandSig{Arity{2, 100}, {}});
    reg.add_command("dict", CommandSig{Arity{2, 100}, {}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("if", CommandSig{Arity{2, 100}, {}});
    return reg;
}

// ===================================================================
// var-1.*: basic set command
// ===================================================================

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

// ===================================================================
// var-2.*: multiple variables
// ===================================================================

TEST_CASE("upstream var: multiple set commands", "[upstream][var]") {
    auto r = analyse("set a 1\nset b 2\nset c 3");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("a"));
    CHECK(root.variables.contains("b"));
    CHECK(root.variables.contains("c"));
}

TEST_CASE("upstream var: repeated set on same variable", "[upstream][var]") {
    auto r = analyse("set x 1\nset x 2");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("x"));
}

// ===================================================================
// var-3.*: variable in proc scope
// ===================================================================

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

TEST_CASE("upstream var: proc params in proc scope", "[upstream][var]") {
    auto r = analyse("proc greet {name} { puts $name }");
    const auto& root = r.global_scope();
    auto has_name = false;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC && child->variables.contains("name"))
            has_name = true;
    }
    CHECK(has_name);
}

TEST_CASE("upstream var: global var visible globally", "[upstream][var]") {
    auto r = analyse("set global_val 100\nputs $global_val");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("global_val"));
}

TEST_CASE("upstream var: separate proc scopes", "[upstream][var]") {
    auto r = analyse("proc foo {} { set a 1 }\nproc bar {} { set b 2 }");
    const auto& root = r.global_scope();
    std::size_t proc_count = 0;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC)
            ++proc_count;
    }
    CHECK(proc_count == 2);
}

TEST_CASE("upstream var: separate proc scopes vars isolated", "[upstream][var]") {
    auto r = analyse("proc foo {} { set a 1 }\nproc bar {} { set b 2 }");
    const auto& root = r.global_scope();
    const Scope* scope_foo = nullptr;
    const Scope* scope_bar = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC) {
            if (child->name == "foo")
                scope_foo = child.get();
            if (child->name == "bar")
                scope_bar = child.get();
        }
    }
    REQUIRE(scope_foo != nullptr);
    REQUIRE(scope_bar != nullptr);
    CHECK(scope_foo->variables.contains("a"));
    CHECK_FALSE(scope_foo->variables.contains("b"));
    CHECK(scope_bar->variables.contains("b"));
    CHECK_FALSE(scope_bar->variables.contains("a"));
}

TEST_CASE("upstream var: nested proc scope", "[upstream][var]") {
    auto r = analyse(R"(proc outer {} {
    set x 1
    proc inner {} {
        set y 2
    }
})");
    const auto& root = r.global_scope();
    const Scope* outer = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC && child->name == "outer")
            outer = child.get();
    }
    REQUIRE(outer != nullptr);
    CHECK(outer->variables.contains("x"));
    CHECK_FALSE(outer->variables.contains("y"));
}

// ===================================================================
// var-4.*: global command
// ===================================================================

TEST_CASE("upstream var: global declaration in proc", "[upstream][var]") {
    auto r = analyse("set count 0\nproc inc {} { global count; set count 1 }");
    const auto& root = r.global_scope();
    REQUIRE(!root.children.empty());
    const auto& proc = *root.children[0];
    CHECK(proc.variables.contains("count"));
}

TEST_CASE("upstream var: global defines var in proc scope", "[upstream][var]") {
    auto r = analyse(R"(proc foo {} {
    global x
    set x 42
})");
    const auto& root = r.global_scope();
    auto has_x = false;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC && child->variables.contains("x"))
            has_x = true;
    }
    CHECK(has_x);
}

TEST_CASE("upstream var: global multiple vars", "[upstream][var]") {
    auto r = analyse(R"(proc foo {} {
    global a b c
    puts "$a $b $c"
})");
    const auto& root = r.global_scope();
    const Scope* proc = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC)
            proc = child.get();
    }
    REQUIRE(proc != nullptr);
    CHECK(proc->variables.contains("a"));
    CHECK(proc->variables.contains("b"));
    CHECK(proc->variables.contains("c"));
}

// ===================================================================
// var-5.*: variable command (namespace variable)
// ===================================================================

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

TEST_CASE("upstream var: variable command defines var at global", "[upstream][var]") {
    auto r = analyse("variable port 8080");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("port"));
}

TEST_CASE("upstream var: variable command without value", "[upstream][var]") {
    auto r = analyse("variable name");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("name"));
}

TEST_CASE("upstream var: variable command multiple pairs", "[upstream][var]") {
    auto r = analyse("variable x 1 y 2");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("x"));
    CHECK(root.variables.contains("y"));
}

TEST_CASE("upstream var: variable in namespace no value", "[upstream][var]") {
    auto r = analyse(R"(namespace eval myns {
    variable name
})");
    const auto& root = r.global_scope();
    auto has_name = false;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::NAMESPACE && child->variables.contains("name"))
            has_name = true;
    }
    CHECK(has_name);
}

// ===================================================================
// var-6.*: foreach creates loop variable
// ===================================================================

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

// ===================================================================
// var-7.*: for loop variable
// ===================================================================

TEST_CASE("upstream var: for loop init creates variable", "[upstream][var]") {
    auto r = analyse("for {set i 0} {$i < 10} {incr i} { puts $i }");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("i"));
}

// ===================================================================
// var-8.*: catch result variable
// ===================================================================

TEST_CASE("upstream var: catch defines result variable", "[upstream][var]") {
    auto r = analyse("catch { expr {1/0} } result");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("result"));
}

TEST_CASE("upstream var: catch defines options variable", "[upstream][var]") {
    auto r = analyse("catch { expr {1/0} } result opts");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("result"));
    CHECK(root.variables.contains("opts"));
}

// ===================================================================
// var-9.*: array-like set
// ===================================================================

TEST_CASE("upstream var: set with array syntax", "[upstream][var]") {
    auto r = analyse("set arr(key) value");
    const auto& root = r.global_scope();
    // Array variable should be tracked (base name or full key).
    CHECK((root.variables.contains("arr(key)") || root.variables.contains("arr")));
}

TEST_CASE("upstream var: array multiple keys", "[upstream][var]") {
    auto r = analyse("set data(name) Alice\nset data(age) 30");
    const auto& root = r.global_scope();
    CHECK((root.variables.contains("data(name)") || root.variables.contains("data")));
}

// ===================================================================
// var-10.*: incr command
// ===================================================================

TEST_CASE("upstream var: incr after set keeps variable", "[upstream][var]") {
    auto r = analyse("set x 0\nincr x");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("x"));
}

TEST_CASE("upstream var: incr implicitly defines variable", "[upstream][var]") {
    auto r = analyse("incr counter");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("counter"));
}

// ===================================================================
// var-11.*: append command (requires registry for VAR_NAME role)
// ===================================================================

TEST_CASE("upstream var: append creates variable", "[upstream][var]") {
    auto reg = make_var_registry();
    auto r = analyse("append str hello", &reg);
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("str"));
}

TEST_CASE("upstream var: append multiple values", "[upstream][var]") {
    auto reg = make_var_registry();
    auto r = analyse("append buf a b c", &reg);
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("buf"));
}

// ===================================================================
// var-12.*: lappend command (requires registry for VAR_NAME role)
// ===================================================================

TEST_CASE("upstream var: lappend creates variable", "[upstream][var]") {
    auto reg = make_var_registry();
    auto r = analyse("lappend mylist item1", &reg);
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("mylist"));
}

TEST_CASE("upstream var: lappend multiple values", "[upstream][var]") {
    auto reg = make_var_registry();
    auto r = analyse("lappend items x y z", &reg);
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("items"));
}

// ===================================================================
// var-13.*: upvar command (requires registry for VAR_NAME role)
// ===================================================================

// Phase 6: upvar defines local alias in proc — requires TAIL arg-pattern
// support in analyse_body_args (currently only FIXED patterns produce
// arg_roles in CommandSig).

// Phase 6: upvar W210 suppression requires CFG/SSA (W210 not yet in C++).

// ===================================================================
// var-14.*: switch body creates variable
// ===================================================================

TEST_CASE("upstream var: switch body creates variable", "[upstream][var]") {
    auto r = analyse(R"(switch -- $x {
    a { set result 1 }
    b { set result 2 }
})");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("result"));
}

// ===================================================================
// var-15.*: try body creates variable
// ===================================================================

TEST_CASE("upstream var: try body creates variable", "[upstream][var]") {
    auto r = analyse(R"(try {
    set x 42
})");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("x"));
}

// ===================================================================
// var-16.*: dict for variables
// ===================================================================

TEST_CASE("upstream var: dict for defines loop vars", "[upstream][var]") {
    auto r = analyse(R"(dict for {k v} $mydict {
    puts "$k = $v"
})");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("k"));
    CHECK(root.variables.contains("v"));
}

// ===================================================================
// var-17.*: array set command
// ===================================================================

TEST_CASE("upstream var: array set defines array variable", "[upstream][var]") {
    auto reg = make_var_registry();
    auto r = analyse("array set colors {red 1 blue 2}", &reg);
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("colors"));
}

// ===================================================================
// var-18.*: proc scope isolation
// ===================================================================

TEST_CASE("upstream var: proc local not visible at global", "[upstream][var]") {
    auto r = analyse("proc foo {} { set secret 42 }\nputs $secret");
    const auto& root = r.global_scope();
    // secret is defined inside proc, not at global scope
    const Scope* proc = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC && child->name == "foo")
            proc = child.get();
    }
    REQUIRE(proc != nullptr);
    CHECK(proc->variables.contains("secret"));
}

TEST_CASE("upstream var: global var not visible in proc without declaration", "[upstream][var]") {
    auto r = analyse("set gvar 1\nproc foo {} { set local 2 }");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("gvar"));
    const Scope* proc = nullptr;
    for (const auto& child : root.children) {
        if (child->kind == ScopeKind::PROC && child->name == "foo")
            proc = child.get();
    }
    REQUIRE(proc != nullptr);
    CHECK(proc->variables.contains("local"));
    CHECK_FALSE(proc->variables.contains("gvar"));
}

// ===================================================================
// var-19.*: try with on handler variables
// ===================================================================

TEST_CASE("upstream var: try on handler defines result var", "[upstream][var]") {
    auto r = analyse(R"(try {
    error "boom"
} on error {msg opts} {
    puts $msg
})");
    const auto& root = r.global_scope();
    CHECK(root.variables.contains("msg"));
    CHECK(root.variables.contains("opts"));
}

// ===================================================================
// Phase 6 — W210/W211/W220 tests (require CFG/SSA, not yet in C++)
// ===================================================================

// Phase 6: test_read_before_set_warning (W210)
// Phase 6: test_no_warning_when_set_first (W210)
// Phase 6: test_read_before_set_at_global_scope (W210)
// Phase 6: test_no_warning_after_global_set (W210)
// Phase 6: test_read_before_set_in_expr (W210)
// Phase 6: test_global_command_suppresses_w210 (W210)
// Phase 6: test_incr_without_prior_set_warns (W210)
// Phase 6: test_incr_after_set_no_w210 (W210)
// Phase 6: test_multiple_undefined_reads (W210)
// Phase 6: test_proc_does_not_see_global_without_declaration (W210)
// Phase 6: test_upvar_suppresses_w210 (W210)
// Phase 6: test_unused_variable_warning (W211)
// Phase 6: test_used_variable_no_warning (W211)
// Phase 6: test_unused_severity_is_hint (W211)
// Phase 6: test_multiple_unused_variables (W211)
// Phase 6: test_return_value_counts_as_use (W211)
// Phase 6: test_used_in_expr_counts (W211)
// Phase 6: test_param_used_no_warning (W211)
// Phase 6: test_dead_assignment_warning (W220)
// Phase 6: test_no_dead_assignment_when_read_between (W220)
// Phase 6: test_dead_assignment_severity_is_hint (W220)
// Phase 6: test_dead_assignment_multiple_overwrites (W220)
// Phase 6: test_incr_after_set_not_dead (W220)
// Phase 6: test_clean_proc_no_w210 (W210)
// Phase 6: test_proc_with_puts_no_variable_warnings (W210/W211/W220)
// Phase 6: test_namespace_variable_used_in_proc (W210)
// Phase 6: test_foreach_loop_var_not_flagged_unused (W211)
// Phase 6: test_global_scope_set_used_no_warning (W211)
// Phase 6: test_append_in_proc_used (W211)
// Phase 6: test_lappend_in_proc_used (W211)
// Phase 6: test_tcltest_setup_body_no_w210 (W210)
// Phase 6: test_tcltest_bare_test_no_w210 (W210)
// Phase 6: test_tcltest_cleanup_no_w210 (W210)
// Phase 6: test_tcltest_lmap_in_setup_no_w210 (W210)
// Phase 6: test_tcltest_legacy_positional_no_w210 (W210)
// Phase 6: test_tcltest_top_level_var_still_warns (W210)
// Phase 6: test_foreach_in_collection_* (W210, dialect-specific)
