// Ported from tests/test_analyser.py — TestRegexPatterns + TestRegexVariablePropagation.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"

#include <catch2/catch_test_macros.hpp>
#include <algorithm>
#include <string>

using namespace tcl_lsp;

// Build a minimal registry that knows about regexp/regsub with PATTERN roles.
static auto make_regex_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;

    // regexp ?options? pattern string ?matchVar? ?subMatchVar ...?
    // Simplified: pattern is at arg index 0 for the basic form.
    reg.add_command("regexp", CommandSig{
        Arity{2, 100},
        {{0, ArgRole::PATTERN}}
    });

    // regsub ?options? pattern string subSpec ?resultVar?
    reg.add_command("regsub", CommandSig{
        Arity{3, 100},
        {{0, ArgRole::PATTERN}}
    });

    // Other common commands.
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("if", CommandSig{Arity{2, 100}, {}});
    reg.add_command("switch", CommandSig{Arity{2, 100}, {}});

    return reg;
}

// ---------------------------------------------------------------------------
// TestRegexPatterns
// ---------------------------------------------------------------------------

TEST_CASE("regexp: simple pattern", "[analyser][regex]") {
    auto reg = make_regex_registry();
    auto result = analyse("regexp {^[a-z]+$} $str", &reg);
    REQUIRE(result.regex_patterns.size() == 1);
    CHECK(result.regex_patterns[0].pattern == "^[a-z]+$");
    CHECK(result.regex_patterns[0].command == "regexp");
}

TEST_CASE("regsub: simple pattern", "[analyser][regex]") {
    auto reg = make_regex_registry();
    auto result = analyse("regsub {\\d+} $str replacement result", &reg);
    REQUIRE(result.regex_patterns.size() == 1);
    CHECK(result.regex_patterns[0].pattern == "\\d+");
    CHECK(result.regex_patterns[0].command == "regsub");
}

TEST_CASE("switch -regexp braced body", "[analyser][regex]") {
    // switch -regexp is handled directly, no registry needed.
    auto result = analyse(R"(switch -regexp $x {
        {^a} {puts a}
        {^b} {puts b}
    })");
    REQUIRE(result.regex_patterns.size() == 2);
    auto patterns = std::vector<std::string>{};
    for (const auto& rp : result.regex_patterns) {
        patterns.push_back(rp.pattern);
    }
    std::sort(patterns.begin(), patterns.end());
    CHECK(patterns[0] == "^a");
    CHECK(patterns[1] == "^b");
    CHECK(result.regex_patterns[0].command == "switch");
}

TEST_CASE("switch -regexp inline pairs", "[analyser][regex]") {
    auto result = analyse("switch -regexp $x {^a} {puts a} {^b} {puts b}");
    REQUIRE(result.regex_patterns.size() == 2);
    auto patterns = std::vector<std::string>{};
    for (const auto& rp : result.regex_patterns) {
        patterns.push_back(rp.pattern);
    }
    std::sort(patterns.begin(), patterns.end());
    CHECK(patterns[0] == "^a");
    CHECK(patterns[1] == "^b");
}

TEST_CASE("switch -regexp default excluded", "[analyser][regex]") {
    auto result = analyse(R"(switch -regexp $x {
        {^a} {puts a}
        default {puts other}
    })");
    REQUIRE(result.regex_patterns.size() == 1);
    CHECK(result.regex_patterns[0].pattern == "^a");
}

TEST_CASE("switch -glob: no regex patterns", "[analyser][regex]") {
    auto result = analyse(R"(switch -glob $x { a* {puts a} b* {puts b} })");
    CHECK(result.regex_patterns.empty());
}

TEST_CASE("switch -exact: no regex patterns", "[analyser][regex]") {
    auto result = analyse(R"(switch -exact $x { hello {puts hi} })");
    CHECK(result.regex_patterns.empty());
}

TEST_CASE("switch default (glob): no regex patterns", "[analyser][regex]") {
    auto result = analyse(R"(switch $x { hello {puts hi} })");
    CHECK(result.regex_patterns.empty());
}

TEST_CASE("no regex in other commands", "[analyser][regex]") {
    auto result = analyse("set x 1\nputs hello\nif {$x > 0} { puts yes }");
    CHECK(result.regex_patterns.empty());
}

TEST_CASE("multiple regexp in same file", "[analyser][regex]") {
    auto reg = make_regex_registry();
    auto result = analyse("regexp {^a} $x\nregsub {^b} $y c result", &reg);
    REQUIRE(result.regex_patterns.size() == 2);
    std::unordered_set<std::string> commands;
    for (const auto& rp : result.regex_patterns) {
        commands.insert(rp.command);
    }
    CHECK(commands.count("regexp") == 1);
    CHECK(commands.count("regsub") == 1);
}

TEST_CASE("regexp inside proc", "[analyser][regex]") {
    auto reg = make_regex_registry();
    auto result = analyse("proc check {s} { regexp {^\\d+$} $s }", &reg);
    REQUIRE(result.regex_patterns.size() == 1);
    CHECK(result.regex_patterns[0].command == "regexp");
}

// ---------------------------------------------------------------------------
// TestRegexVariablePropagation
// ---------------------------------------------------------------------------

TEST_CASE("regex var propagation: set then regexp", "[analyser][regex][propagation]") {
    auto reg = make_regex_registry();
    auto result = analyse("set pat {^\\d+$}\nregexp $pat $str", &reg);
    // One for the $pat usage in regexp, one for the set definition site.
    REQUIRE(result.regex_patterns.size() == 2);
    for (const auto& rp : result.regex_patterns) {
        CHECK(rp.pattern == "^\\d+$");
    }
}

TEST_CASE("regex var propagation: set then regsub", "[analyser][regex][propagation]") {
    auto reg = make_regex_registry();
    auto result = analyse("set pat {foo}\nregsub $pat $str bar result", &reg);
    REQUIRE(result.regex_patterns.size() == 2);
    for (const auto& rp : result.regex_patterns) {
        CHECK(rp.pattern == "foo");
    }
}

TEST_CASE("regex var propagation: variable not constant", "[analyser][regex][propagation]") {
    auto reg = make_regex_registry();
    auto result = analyse("set pat $dynamic_value\nregexp $pat $str", &reg);
    CHECK(result.regex_patterns.empty());
}

TEST_CASE("regex var propagation: variable reassigned dynamic", "[analyser][regex][propagation]") {
    auto reg = make_regex_registry();
    auto result = analyse("set pat {^\\d+}\nset pat $other\nregexp $pat $str", &reg);
    CHECK(result.regex_patterns.empty());
}

TEST_CASE("regex var propagation: variable in proc scope", "[analyser][regex][propagation]") {
    auto reg = make_regex_registry();
    auto result = analyse("proc check {s} { set pat {^\\w+$}; regexp $pat $s }", &reg);
    REQUIRE(result.regex_patterns.size() == 2);
    for (const auto& rp : result.regex_patterns) {
        CHECK(rp.pattern == "^\\w+$");
    }
}

TEST_CASE("regex var propagation: literal and variable mixed", "[analyser][regex][propagation]") {
    auto reg = make_regex_registry();
    auto result = analyse("set pat {^a}\nregexp $pat $str\nregexp {^b} $str2", &reg);
    auto patterns = std::vector<std::string>{};
    for (const auto& rp : result.regex_patterns) {
        patterns.push_back(rp.pattern);
    }
    std::sort(patterns.begin(), patterns.end());
    CHECK(std::count(patterns.begin(), patterns.end(), "^a") >= 1);
    CHECK(std::count(patterns.begin(), patterns.end(), "^b") >= 1);
}
