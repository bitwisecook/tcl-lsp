// Tests for the generated command registry (all 1840 commands).
// Validates correctness of the codegen output.

#include <catch2/catch_test_macros.hpp>

#include "tcl_lsp/registry/command_desc.hpp"
#include "tcl_lsp/registry/command_registry.hpp"

// Include generated commands.
#include "../src/registry/generated/all_commands.cpp"

using namespace tcl_lsp;
using namespace tcl_lsp::generated;
using enum ArgRole;

static CommandRegistry make_registry() {
    return CommandRegistry{all_commands, hover_table};
}

// ════════════════════════════════════════════════════════════════════
// Basic registry sanity
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Generated registry: size and basic lookup", "[generated]") {
    auto reg = make_registry();

    SECTION("has expected command count") {
        CHECK(reg.size() >= 1800);
    }

    SECTION("find core Tcl commands") {
        CHECK(reg.find("proc") != nullptr);
        CHECK(reg.find("set") != nullptr);
        CHECK(reg.find("if") != nullptr);
        CHECK(reg.find("foreach") != nullptr);
        CHECK(reg.find("string") != nullptr);
        CHECK(reg.find("append") != nullptr);
        CHECK(reg.find("expr") != nullptr);
        CHECK(reg.find("puts") != nullptr);
    }

    SECTION("find iRules commands") {
        CHECK(reg.find("HTTP::uri") != nullptr);
        CHECK(reg.find("HTTP::header") != nullptr);
        CHECK(reg.find("log") != nullptr);
        CHECK(reg.find("pool") != nullptr);
    }

    SECTION("find iApps commands") {
        CHECK(reg.find("tmsh::create") != nullptr);
        CHECK(reg.find("tmsh::modify") != nullptr);
    }

    SECTION("find Expect commands") {
        CHECK(reg.find("expect") != nullptr);
        CHECK(reg.find("spawn") != nullptr);
        CHECK(reg.find("send") != nullptr);
    }

    SECTION("find EDA commands") {
        CHECK(reg.find("analyze") != nullptr);
        CHECK(reg.find("elaborate") != nullptr);
    }

    SECTION("unknown command returns nullptr") {
        CHECK(reg.find("totally_nonexistent_cmd_xyz") == nullptr);
    }
}

// ════════════════════════════════════════════════════════════════════
// Descriptor invariants
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Generated registry: descriptor invariants", "[generated]") {
    auto reg = make_registry();

    SECTION("all commands have non-empty names") {
        for (const auto* desc : all_commands) {
            REQUIRE(desc != nullptr);
            CHECK_FALSE(desc->name.empty());
        }
    }

    SECTION("all commands have valid arity") {
        for (const auto* desc : all_commands) {
            CHECK(desc->arity.min >= 0);
            CHECK(desc->arity.max >= desc->arity.min);
        }
    }

    SECTION("all subcommands have non-empty names") {
        for (const auto* desc : all_commands) {
            for (const auto& sub : desc->subcommands) {
                CHECK_FALSE(sub.name.empty());
            }
        }
    }

    SECTION("all options start with -") {
        for (const auto* desc : all_commands) {
            for (const auto& opt : desc->options) {
                if (!opt.name.empty()) {
                    CHECK(opt.name[0] == '-');
                }
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Dialect filtering
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Generated registry: dialect filtering", "[generated][dialect]") {
    auto reg = make_registry();

    SECTION("iRules commands visible in IRULES dialect") {
        CHECK(reg.find("HTTP::uri", DialectFlags::IRULES) != nullptr);
        CHECK(reg.find("pool", DialectFlags::IRULES) != nullptr);
    }

    SECTION("iRules commands NOT visible in TCL86") {
        CHECK(reg.find("HTTP::uri", DialectFlags::TCL86) == nullptr);
    }

    SECTION("core Tcl commands visible in IRULES (inherits TCL84)") {
        // proc is ALL-dialect, should be visible everywhere
        CHECK(reg.find("proc", DialectFlags::IRULES) != nullptr);
        CHECK(reg.find("set", DialectFlags::IRULES) != nullptr);
    }

    SECTION("EDA commands visible in their dialect") {
        auto* analyze = reg.find("analyze");
        if (analyze != nullptr) {
            CHECK(reg.find("analyze", DialectFlags::EDA_SYNOPSYS) != nullptr);
        }
    }

    SECTION("EDA commands NOT visible in TCL86") {
        // analyze is EDA-specific
        auto* analyze = reg.find("analyze", DialectFlags::EDA_SYNOPSYS);
        if (analyze != nullptr) {
            CHECK(reg.find("analyze", DialectFlags::TCL86) == nullptr);
        }
    }

    SECTION("Expect commands visible in EXPECT") {
        CHECK(reg.find("expect", DialectFlags::EXPECT) != nullptr);
        CHECK(reg.find("spawn", DialectFlags::EXPECT) != nullptr);
    }

    SECTION("iApps commands visible in IAPPS") {
        CHECK(reg.find("tmsh::create", DialectFlags::IAPPS) != nullptr);
    }
}

// ════════════════════════════════════════════════════════════════════
// Hover text
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Generated registry: hover text", "[generated][hover]") {
    auto reg = make_registry();

    SECTION("most commands have hover text") {
        int with_hover = 0;
        int total = 0;
        for (const auto* desc : all_commands) {
            ++total;
            if (reg.hover_for(*desc) != nullptr) {
                ++with_hover;
            }
        }
        // At least 99% should have hover
        CHECK(with_hover > total * 99 / 100);
    }

    SECTION("hover text has non-empty summary") {
        auto* desc = reg.find("set");
        REQUIRE(desc != nullptr);
        auto* hover = reg.hover_for(*desc);
        REQUIRE(hover != nullptr);
        CHECK_FALSE(hover->summary.empty());
    }

    SECTION("hover text has synopsis") {
        auto* desc = reg.find("proc");
        REQUIRE(desc != nullptr);
        auto* hover = reg.hover_for(*desc);
        REQUIRE(hover != nullptr);
        CHECK_FALSE(hover->synopsis.empty());
    }

    SECTION("render_hover_markdown produces non-empty output") {
        auto* desc = reg.find("foreach");
        REQUIRE(desc != nullptr);
        auto* hover = reg.hover_for(*desc);
        REQUIRE(hover != nullptr);
        auto md = render_hover_markdown(*hover, "foreach");
        CHECK_FALSE(md.empty());
        CHECK(md.find("foreach") != std::string::npos);
    }

    SECTION("subcommand hover available") {
        auto* desc = reg.find("string");
        REQUIRE(desc != nullptr);
        bool found_sub_hover = false;
        for (const auto& sub : desc->subcommands) {
            if (sub.hover_index >= 0) {
                auto* hover = reg.hover_at(sub.hover_index);
                CHECK(hover != nullptr);
                if (hover != nullptr) {
                    found_sub_hover = true;
                }
            }
        }
        CHECK(found_sub_hover);
    }
}

// ════════════════════════════════════════════════════════════════════
// Specific command shapes (from C++ arg pattern overrides)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Generated registry: foreach has stride pattern", "[generated][resolve]") {
    auto reg = make_registry();
    auto* desc = reg.find("foreach");
    REQUIRE(desc != nullptr);

    SECTION("modular arity: accepts odd argc >= 3") {
        CHECK(desc->arity.accepts(3));
        CHECK(desc->arity.accepts(5));
        CHECK_FALSE(desc->arity.accepts(4));
        CHECK_FALSE(desc->arity.accepts(2));
    }

    SECTION("has stride arg patterns") {
        bool found_stride = false;
        for (const auto& pat : desc->arg_patterns) {
            if (pat.kind == ArgPatternKind::STRIDE) {
                found_stride = true;
                CHECK(pat.stride == 2);
            }
        }
        CHECK(found_stride);
    }

    SECTION("resolve gives body as last arg") {
        std::string_view args[] = {"x", "$list", "{ puts $x }"};
        auto result = CommandRegistry::resolve_arg_roles(*desc, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[2] == BODY);
    }
}

TEST_CASE("Generated registry: proc has fixed args", "[generated][resolve]") {
    auto reg = make_registry();
    auto* desc = reg.find("proc");
    REQUIRE(desc != nullptr);

    CHECK(desc->arity.accepts(3));
    CHECK_FALSE(desc->arity.accepts(2));
    CHECK(desc->defines_procedure);
    CHECK(desc->is_scope_barrier);

    std::string_view args[] = {"myProc", "{a b}", "{ puts $a }"};
    auto result = CommandRegistry::resolve_arg_roles(*desc, args);
    REQUIRE(result.has_value());
    auto roles = result->roles();
    CHECK(roles[0] == NAME);
    CHECK(roles[1] == PARAM_LIST);
    CHECK(roles[2] == BODY);
}

TEST_CASE("Generated registry: lassign has tail pattern", "[generated][resolve]") {
    auto reg = make_registry();
    auto* desc = reg.find("lassign");
    REQUIRE(desc != nullptr);

    SECTION("resolves tail VAR_NAME past index 19") {
        std::string_view args[22];
        args[0] = "myList";
        for (int i = 1; i < 22; ++i) args[i] = "v";
        auto result = CommandRegistry::resolve_arg_roles(*desc, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE);
        CHECK(roles[1] == VAR_NAME);
        CHECK(roles[20] == VAR_NAME);
    }
}

TEST_CASE("Generated registry: HTTP::uri forms", "[generated][irules]") {
    auto reg = make_registry();
    auto* desc = reg.find("HTTP::uri");
    REQUIRE(desc != nullptr);

    CHECK(desc->dialects == DialectFlags::IRULES);
    CHECK_FALSE(desc->forms.empty());

    // Should have getter and setter forms
    bool found_getter = false, found_setter = false;
    for (const auto& f : desc->forms) {
        if (f.kind == FormKind::GETTER) found_getter = true;
        if (f.kind == FormKind::SETTER) found_setter = true;
    }
    CHECK(found_getter);
    CHECK(found_setter);
}

TEST_CASE("Generated registry: string subcommands", "[generated][subcommand]") {
    auto reg = make_registry();
    auto* desc = reg.find("string");
    REQUIRE(desc != nullptr);
    CHECK_FALSE(desc->subcommands.empty());

    // Find length subcommand
    const SubCmdDesc* len = nullptr;
    for (const auto& sub : desc->subcommands) {
        if (sub.name == "length") { len = &sub; break; }
    }
    REQUIRE(len != nullptr);
    CHECK(len->pure);
}

TEST_CASE("Generated registry: behavioral traits", "[generated][traits]") {
    auto reg = make_registry();

    SECTION("eval is taint sink and creates dynamic barrier") {
        auto* desc = reg.find("eval");
        REQUIRE(desc != nullptr);
        CHECK(desc->taint_sink);
        CHECK(desc->creates_dynamic_barrier);
    }

    SECTION("if is control flow") {
        auto* desc = reg.find("if");
        REQUIRE(desc != nullptr);
        CHECK(desc->is_control_flow);
        CHECK(desc->layout_resolver == LayoutResolver::IF);
    }

    SECTION("set assigns variable at 0") {
        auto* desc = reg.find("set");
        REQUIRE(desc != nullptr);
        CHECK(desc->assigns_variable_at == 0);
    }
}
