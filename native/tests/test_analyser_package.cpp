// Ported from tests/test_analyser.py — TestPackageRequire + TestSourceTargets.

#include "tcl_lsp/analysis/analyser.hpp"

#include <catch2/catch_test_macros.hpp>

using namespace tcl_lsp;

// ---------------------------------------------------------------------------
// TestPackageRequire
// ---------------------------------------------------------------------------

TEST_CASE("package require: simple", "[analyser][package]") {
    auto result = analyse("package require http");
    REQUIRE(result.package_requires.size() == 1);
    CHECK(result.package_requires[0].name == "http");
    CHECK(result.package_requires[0].version.empty());
}

TEST_CASE("package require: with version", "[analyser][package]") {
    auto result = analyse("package require http 2.9");
    REQUIRE(result.package_requires.size() == 1);
    CHECK(result.package_requires[0].name == "http");
    CHECK(result.package_requires[0].version == "2.9");
}

TEST_CASE("package require: -exact", "[analyser][package]") {
    auto result = analyse("package require -exact http 2.9");
    REQUIRE(result.package_requires.size() == 1);
    CHECK(result.package_requires[0].name == "http");
    CHECK(result.package_requires[0].version == "2.9");
}

TEST_CASE("package require: multiple", "[analyser][package]") {
    auto result = analyse("package require http\npackage require tls");
    REQUIRE(result.package_requires.size() == 2);
    std::unordered_set<std::string> names;
    for (const auto& pr : result.package_requires) names.insert(pr.name);
    CHECK(names.count("http") == 1);
    CHECK(names.count("tls") == 1);
}

TEST_CASE("package provide: not recorded as require", "[analyser][package]") {
    auto result = analyse("package provide mylib 1.0");
    CHECK(result.package_requires.empty());
    REQUIRE(result.package_provides.size() == 1);
    CHECK(result.package_provides[0].name == "mylib");
    CHECK(result.package_provides[0].version == "1.0");
}

TEST_CASE("no package commands", "[analyser][package]") {
    auto result = analyse("set x 42");
    CHECK(result.package_requires.empty());
}

TEST_CASE("package require: conditional inside catch", "[analyser][package]") {
    auto result = analyse("catch { package require optional_pkg }");
    REQUIRE(result.package_requires.size() == 1);
    CHECK(result.package_requires[0].conditional == true);
}

// ---------------------------------------------------------------------------
// TestSourceTargets
// ---------------------------------------------------------------------------

TEST_CASE("source: literal path", "[analyser][source]") {
    auto result = analyse("source lib/utils.tcl");
    REQUIRE(result.source_targets.size() == 1);
    CHECK(result.source_targets[0].raw_path == "lib/utils.tcl");
    CHECK(result.source_targets[0].is_literal == true);
}

TEST_CASE("source: variable path is not literal", "[analyser][source]") {
    auto result = analyse("source $dir/utils.tcl");
    REQUIRE(result.source_targets.size() == 1);
    CHECK(result.source_targets[0].is_literal == false);
}

TEST_CASE("source: cmd subst path is not literal", "[analyser][source]") {
    auto result = analyse("source [file join [file dirname [info script]] helper.tcl]");
    REQUIRE(result.source_targets.size() == 1);
    CHECK(result.source_targets[0].is_literal == false);
}

TEST_CASE("source: with -encoding", "[analyser][source]") {
    auto result = analyse("source -encoding utf-8 myfile.tcl");
    REQUIRE(result.source_targets.size() == 1);
    CHECK(result.source_targets[0].raw_path == "myfile.tcl");
    CHECK(result.source_targets[0].is_literal == true);
}

TEST_CASE("source: multiple", "[analyser][source]") {
    auto result = analyse("source a.tcl\nsource b.tcl\nsource $c");
    REQUIRE(result.source_targets.size() == 3);
    CHECK(result.source_targets[0].is_literal == true);
    CHECK(result.source_targets[1].is_literal == true);
    CHECK(result.source_targets[2].is_literal == false);
}
