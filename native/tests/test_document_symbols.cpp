#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/lsp/document_symbols.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

namespace {

auto analyse_source(std::string_view source) -> AnalysisResult {
    Analyser analyser;
    return analyser.analyse(source);
}

auto find_by_name(const std::vector<DocumentSymbolInfo>& syms,
                  const std::string& name) -> const DocumentSymbolInfo* {
    for (auto& s : syms) {
        if (s.name == name)
            return &s;
    }
    return nullptr;
}

} // anonymous namespace

TEST_CASE("document symbols: empty source", "[document_symbols]") {
    auto analysis = analyse_source("");
    auto syms = get_document_symbols(analysis);
    REQUIRE(syms.empty());
}

TEST_CASE("document symbols: single proc", "[document_symbols]") {
    auto analysis = analyse_source("proc greet {name} { puts $name }");
    auto syms = get_document_symbols(analysis);
    auto* proc = find_by_name(syms, "greet");
    REQUIRE(proc != nullptr);
    REQUIRE(proc->kind == SymbolKind::FUNCTION);
    REQUIRE(proc->detail == "(name)");
}

TEST_CASE("document symbols: proc with default", "[document_symbols]") {
    auto analysis = analyse_source("proc greet {name {greeting Hello}} { puts $greeting }");
    auto syms = get_document_symbols(analysis);
    auto* proc = find_by_name(syms, "greet");
    REQUIRE(proc != nullptr);
    REQUIRE(proc->detail == "(name {greeting Hello})");
}

TEST_CASE("document symbols: multiple procs", "[document_symbols]") {
    auto analysis = analyse_source("proc foo {} {}\nproc bar {x} {}");
    auto syms = get_document_symbols(analysis);
    REQUIRE(find_by_name(syms, "foo") != nullptr);
    REQUIRE(find_by_name(syms, "bar") != nullptr);
}

TEST_CASE("document symbols: namespace with procs", "[document_symbols]") {
    auto analysis = analyse_source("namespace eval math {\n"
                                   "    proc add {a b} { expr {$a + $b} }\n"
                                   "    proc sub {a b} { expr {$a - $b} }\n"
                                   "}");
    auto syms = get_document_symbols(analysis);
    // Should have a namespace symbol with children.
    auto* ns = find_by_name(syms, "math");
    REQUIRE(ns != nullptr);
    REQUIRE(ns->kind == SymbolKind::NAMESPACE);
    // The procs should be in the namespace or at top level.
    // (Implementation detail: depends on how scope tree is structured.)
}

TEST_CASE("document symbols: global variable", "[document_symbols]") {
    auto analysis = analyse_source("set myvar 42");
    auto syms = get_document_symbols(analysis);
    // Should find the variable.
    auto* var = find_by_name(syms, "myvar");
    REQUIRE(var != nullptr);
    REQUIRE(var->kind == SymbolKind::VARIABLE);
}
