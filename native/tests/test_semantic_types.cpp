#include "tcl_lsp/analysis/auxiliary_types.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

// --- ProcArgTrait ---

TEST_CASE("ProcArgTrait bitmask operations", "[semantic_types]") {
    ProcArgTraits traits = ProcArgTrait::EVAL | ProcArgTrait::BODY;
    REQUIRE(has_trait(traits, ProcArgTrait::EVAL));
    REQUIRE(has_trait(traits, ProcArgTrait::BODY));
    REQUIRE_FALSE(has_trait(traits, ProcArgTrait::VAR_WRITE));

    traits = traits | ProcArgTrait::EXPR;
    REQUIRE(has_trait(traits, ProcArgTrait::EXPR));
}

TEST_CASE("ProcArgTrait to_string", "[semantic_types]") {
    REQUIRE(to_string(ProcArgTrait::EVAL) == "EVAL");
    REQUIRE(to_string(ProcArgTrait::BODY) == "BODY");
    REQUIRE(to_string(ProcArgTrait::VAR_WRITE) == "VAR_WRITE");
    REQUIRE(to_string(ProcArgTrait::VAR_READ) == "VAR_READ");
    REQUIRE(to_string(ProcArgTrait::EXPR) == "EXPR");
    REQUIRE(to_string(ProcArgTrait::LOOP_LIST) == "LOOP_LIST");
}

// --- ParamDef ---

TEST_CASE("ParamDef default construction", "[semantic_types]") {
    ParamDef p;
    p.name = "x";
    REQUIRE(p.name == "x");
    REQUIRE(p.has_default == false);
    REQUIRE(p.default_value.empty());
}

TEST_CASE("ParamDef with default value", "[semantic_types]") {
    ParamDef p{"name", true, "World"};
    REQUIRE(p.name == "name");
    REQUIRE(p.has_default == true);
    REQUIRE(p.default_value == "World");
}

TEST_CASE("ParamDef equality", "[semantic_types]") {
    ParamDef a{"x", false, ""};
    ParamDef b{"x", false, ""};
    ParamDef c{"y", false, ""};
    REQUIRE(a == b);
    REQUIRE_FALSE(a == c);
}

// --- VarDef ---

TEST_CASE("VarDef construction", "[semantic_types]") {
    VarDef v;
    v.name = "counter";
    v.definition_range = Range::zero();
    v.warn_if_unused = true;
    REQUIRE(v.name == "counter");
    REQUIRE(v.references.empty());
    REQUIRE(v.warn_if_unused == true);
}

TEST_CASE("VarDef references grow during analysis", "[semantic_types]") {
    VarDef v;
    v.name = "x";
    v.references.push_back({{1, 5, 10}, {1, 6, 11}});
    v.references.push_back({{3, 2, 40}, {3, 3, 41}});
    REQUIRE(v.references.size() == 2);
}

// --- ProcDef ---

TEST_CASE("ProcDef construction", "[semantic_types]") {
    ProcDef p;
    p.name = "greet";
    p.qualified_name = "::greet";
    p.params = {{"name", false, ""}};
    REQUIRE(p.name == "greet");
    REQUIRE(p.qualified_name == "::greet");
    REQUIRE(p.params.size() == 1);
    REQUIRE(p.params[0].name == "name");
    REQUIRE(p.doc.empty());
}

TEST_CASE("ProcDef with param traits", "[semantic_types]") {
    ProcDef p;
    p.name = "eval_body";
    p.qualified_name = "::eval_body";
    p.params = {{"script", false, ""}};
    p.param_traits["script"] = static_cast<ProcArgTraits>(ProcArgTrait::EVAL);
    REQUIRE(has_trait(p.param_traits["script"], ProcArgTrait::EVAL));
}

// --- ScopeKind ---

TEST_CASE("ScopeKind to_string", "[semantic_types]") {
    REQUIRE(to_string(ScopeKind::GLOBAL) == "global");
    REQUIRE(to_string(ScopeKind::NAMESPACE) == "namespace");
    REQUIRE(to_string(ScopeKind::PROC) == "proc");
}

// --- Scope ---

TEST_CASE("Scope default construction is global", "[semantic_types]") {
    Scope s;
    REQUIRE(s.kind == ScopeKind::GLOBAL);
    REQUIRE(s.parent == nullptr);
    REQUIRE(s.variables.empty());
    REQUIRE(s.procs.empty());
    REQUIRE(s.children.empty());
}

TEST_CASE("Scope tree with parent pointers", "[semantic_types]") {
    Scope global;
    global.kind = ScopeKind::GLOBAL;
    global.name = "::";

    auto child = std::make_unique<Scope>();
    child->kind = ScopeKind::PROC;
    child->name = "foo";
    child->parent = &global;

    global.children.push_back(child.release());

    REQUIRE(global.children.size() == 1);
    REQUIRE(global.children[0]->name == "foo");
    REQUIRE(global.children[0]->parent == &global);
    // Scope destructor deletes children automatically.
}

TEST_CASE("Scope with variables and procs", "[semantic_types]") {
    Scope s;
    s.kind = ScopeKind::PROC;
    s.name = "test_proc";

    s.variables["x"] = VarDef{"x", Range::zero(), {}, false};
    s.procs["nested"] = ProcDef{"nested", "::nested", {}, Range::zero(), Range::zero(), "", {}};

    REQUIRE(s.variables.contains("x"));
    REQUIRE(s.procs.contains("nested"));
    REQUIRE(s.procs["nested"].qualified_name == "::nested");
}

// --- Auxiliary types ---

TEST_CASE("RegexPattern equality", "[auxiliary_types]") {
    RegexPattern a{Range::zero(), "^foo$", "regexp"};
    RegexPattern b{Range::zero(), "^foo$", "regexp"};
    RegexPattern c{Range::zero(), "^bar$", "regexp"};
    REQUIRE(a == b);
    REQUIRE_FALSE(a == c);
}

TEST_CASE("CommandInvocation construction", "[auxiliary_types]") {
    CommandInvocation inv{"puts", Range::zero(), "::puts"};
    REQUIRE(inv.name == "puts");
    REQUIRE(inv.resolved_qualified_name == "::puts");
}

TEST_CASE("PackageRequire construction", "[auxiliary_types]") {
    PackageRequire pr{"Tk", "8.6", Range::zero(), false};
    REQUIRE(pr.name == "Tk");
    REQUIRE(pr.version == "8.6");
    REQUIRE(pr.conditional == false);
}

TEST_CASE("PackageProvide construction", "[auxiliary_types]") {
    PackageProvide pp{"mylib", "1.0", Range::zero()};
    REQUIRE(pp.name == "mylib");
    REQUIRE(pp.version == "1.0");
}

TEST_CASE("SourceTarget literal vs non-literal", "[auxiliary_types]") {
    SourceTarget lit{"utils.tcl", Range::zero(), true};
    SourceTarget dyn{"$dir/utils.tcl", Range::zero(), false};
    REQUIRE(lit.is_literal == true);
    REQUIRE(dyn.is_literal == false);
}

TEST_CASE("StubArgDef construction", "[auxiliary_types]") {
    StubArgDef arg{"body", "body", false};
    REQUIRE(arg.name == "body");
    REQUIRE(arg.role == "body");
    REQUIRE(arg.optional == false);
}

TEST_CASE("StubCommandDef with flags", "[auxiliary_types]") {
    StubCommandDef stub;
    stub.name = "myloop";
    stub.args = {{"body", "body", false}};
    stub.loop = true;
    stub.pure = false;
    REQUIRE(stub.name == "myloop");
    REQUIRE(stub.loop == true);
    REQUIRE(stub.barrier == false);
}

TEST_CASE("StubExprDef construction", "[auxiliary_types]") {
    StubExprDef expr{"mysin", "function", 1, true, Range::zero()};
    REQUIRE(expr.name == "mysin");
    REQUIRE(expr.kind == "function");
    REQUIRE(expr.arity == 1);
    REQUIRE(expr.pure == true);
}

TEST_CASE("UnknownProcInfo construction", "[auxiliary_types]") {
    UnknownProcInfo info;
    info.dispatch_targets = {"foo", "bar"};
    info.chains_original = true;
    REQUIRE(info.dispatch_targets.contains("foo"));
    REQUIRE(info.dispatch_targets.contains("bar"));
    REQUIRE(info.chains_original == true);
    REQUIRE(info.empty_stub == false);
}

TEST_CASE("Confidence enum values", "[auxiliary_types]") {
    REQUIRE(Confidence::DEFINITE != Confidence::PROBABLE);
    REQUIRE(Confidence::PROBABLE != Confidence::UNKNOWN);
}

TEST_CASE("PackageContext confidence classification", "[auxiliary_types]") {
    PackageContext ctx;
    ctx.definite = {"Tk"};
    ctx.probable = {"http"};
    ctx.provided = {"mylib"};
    ctx.unknown_providers = false;

    REQUIRE(ctx.confidence("Tk") == Confidence::DEFINITE);
    REQUIRE(ctx.confidence("http") == Confidence::PROBABLE);
    REQUIRE(ctx.confidence("nonexistent") == Confidence::UNKNOWN);
}

TEST_CASE("PackageContext all_required merges definite and probable", "[auxiliary_types]") {
    PackageContext ctx;
    ctx.definite = {"Tk"};
    ctx.probable = {"http"};

    auto all = ctx.all_required();
    REQUIRE(all.contains("Tk"));
    REQUIRE(all.contains("http"));
    REQUIRE(all.size() == 2);
}
