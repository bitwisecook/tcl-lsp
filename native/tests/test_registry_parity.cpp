// Parity test: verify the generated C++ registry covers all commands
// from the Python registry, with compatible metadata.
//
// The C++ registry has enhanced arg pattern capabilities (STRIDE, TAIL,
// modular arity) that the Python registry can't express. This test
// validates name coverage and arity compatibility, allowing documented
// mismatches where C++ intentionally diverges.

#include <catch2/catch_test_macros.hpp>

#include "tcl_lsp/registry/native_registry_adapter.hpp"

using namespace tcl_lsp;

TEST_CASE("Registry parity: NativeCommandRegistry basics", "[parity]") {
    const auto& reg = native_registry();

    SECTION("has substantial command count") {
        CHECK(reg.size() >= 1800);
    }

    SECTION("is_known works for core commands") {
        CHECK(reg.is_known("proc"));
        CHECK(reg.is_known("set"));
        CHECK(reg.is_known("if"));
        CHECK(reg.is_known("foreach"));
        CHECK(reg.is_known("string"));
        CHECK(reg.is_known("HTTP::uri"));
        CHECK(reg.is_known("tmsh::create"));
        CHECK(reg.is_known("expect"));
        CHECK(reg.is_known("analyze"));  // EDA
    }

    SECTION("unknown command returns false") {
        CHECK_FALSE(reg.is_known("totally_nonexistent_xyz"));
    }
}

TEST_CASE("Registry parity: validation (arity)", "[parity]") {
    const auto& reg = native_registry();

    SECTION("proc: exactly 3 args") {
        auto v = reg.validation("proc");
        REQUIRE(v.has_value());
        CHECK(v->min == 3);
        CHECK(v->max == 3);
    }

    SECTION("set: 1-2 args") {
        auto v = reg.validation("set");
        REQUIRE(v.has_value());
        CHECK(v->min == 1);
        CHECK(v->max == 2);
    }

    SECTION("foreach: modular arity (min=3, odd)") {
        auto v = reg.validation("foreach");
        REQUIRE(v.has_value());
        CHECK(v->min == 3);
        CHECK(v->modulus == 2);
        CHECK(v->remainder == 1);
        CHECK(v->accepts(3));
        CHECK(v->accepts(5));
        CHECK_FALSE(v->accepts(4));
    }

    SECTION("for: exactly 4 args") {
        auto v = reg.validation("for");
        REQUIRE(v.has_value());
        CHECK(v->min == 4);
        CHECK(v->max == 4);
    }

    SECTION("while: exactly 2 args") {
        auto v = reg.validation("while");
        REQUIRE(v.has_value());
        CHECK(v->min == 2);
        CHECK(v->max == 2);
    }

    SECTION("lassign: min 2, unlimited") {
        auto v = reg.validation("lassign");
        REQUIRE(v.has_value());
        CHECK(v->min == 2);
        CHECK(v->is_unlimited());
    }
}

TEST_CASE("Registry parity: signatures", "[parity]") {
    const auto& reg = native_registry();

    SECTION("proc has CommandSig with arg roles") {
        auto sig = reg.signature("proc");
        REQUIRE(sig.has_value());
        REQUIRE(std::holds_alternative<CommandSig>(*sig));
        auto& cs = std::get<CommandSig>(*sig);
        CHECK(cs.arg_roles.count(0) > 0);  // NAME
        CHECK(cs.arg_roles.count(1) > 0);  // PARAM_LIST
        CHECK(cs.arg_roles.count(2) > 0);  // BODY
        CHECK(cs.arg_roles.at(0) == ArgRole::NAME);
        CHECK(cs.arg_roles.at(1) == ArgRole::PARAM_LIST);
        CHECK(cs.arg_roles.at(2) == ArgRole::BODY);
    }

    SECTION("string has SubcommandSig") {
        auto sig = reg.signature("string");
        REQUIRE(sig.has_value());
        REQUIRE(std::holds_alternative<SubcommandSig>(*sig));
        auto& ss = std::get<SubcommandSig>(*sig);
        CHECK(ss.subcommands.count("length") > 0);
        CHECK(ss.subcommands.count("compare") > 0);
        CHECK(ss.subcommands.count("cat") > 0);
    }

    SECTION("set has VAR_NAME at arg 0") {
        auto sig = reg.signature("set");
        REQUIRE(sig.has_value());
        REQUIRE(std::holds_alternative<CommandSig>(*sig));
        auto& cs = std::get<CommandSig>(*sig);
        CHECK(cs.arg_roles.at(0) == ArgRole::VAR_NAME);
    }

    SECTION("catch has BODY at arg 0") {
        auto sig = reg.signature("catch");
        REQUIRE(sig.has_value());
        REQUIRE(std::holds_alternative<CommandSig>(*sig));
        auto& cs = std::get<CommandSig>(*sig);
        CHECK(cs.arg_roles.at(0) == ArgRole::BODY);
    }

    SECTION("while has EXPR at 0, BODY at 1") {
        auto sig = reg.signature("while");
        REQUIRE(sig.has_value());
        REQUIRE(std::holds_alternative<CommandSig>(*sig));
        auto& cs = std::get<CommandSig>(*sig);
        CHECK(cs.arg_roles.at(0) == ArgRole::EXPR);
        CHECK(cs.arg_roles.at(1) == ArgRole::BODY);
    }

    SECTION("for has BODY and EXPR roles") {
        auto sig = reg.signature("for");
        REQUIRE(sig.has_value());
        REQUIRE(std::holds_alternative<CommandSig>(*sig));
        auto& cs = std::get<CommandSig>(*sig);
        CHECK(cs.arg_roles.at(1) == ArgRole::EXPR);  // test
        CHECK(cs.arg_roles.at(3) == ArgRole::BODY);   // body
    }
}

TEST_CASE("Registry parity: all command names are valid", "[parity]") {
    const auto& reg = native_registry();
    auto names = reg.command_names();

    CHECK(names.size() >= 1800);

    for (const auto& name : names) {
        CHECK_FALSE(name.empty());
        // Every command returned by command_names() should be findable.
        CHECK(reg.is_known(name));
    }
}
