#include "tcl_lsp/registry/command_desc.hpp"
#include "tcl_lsp/registry/command_registry.hpp"

#include <catch2/catch_test_macros.hpp>

// Include sample commands (header-only constexpr data).
#include "../src/registry/sample_commands.cpp"

using namespace tcl_lsp;
using namespace tcl_lsp::sample_commands;
using enum ArgRole;

// ════════════════════════════════════════════════════════════════════
// Registry construction and lookup
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Registry construction and lookup", "[registry]") {
    CommandRegistry reg{all_sample_commands};

    SECTION("finds known command") {
        auto* desc = reg.find("proc");
        REQUIRE(desc != nullptr);
        CHECK(desc->name == "proc");
    }

    SECTION("returns nullptr for unknown command") {
        CHECK(reg.find("nonexistent") == nullptr);
    }

    SECTION("size matches input") {
        CHECK(reg.size() == std::size(all_sample_commands));
    }
}

// ════════════════════════════════════════════════════════════════════
// Dialect filtering
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Dialect filtering", "[registry][dialect]") {
    CommandRegistry reg{all_sample_commands};

    SECTION("ALL-dialect command visible everywhere") {
        CHECK(reg.find("proc", DialectFlags::TCL84) != nullptr);
        CHECK(reg.find("proc", DialectFlags::IRULES) != nullptr);
        CHECK(reg.find("proc", DialectFlags::TCL90) != nullptr);
    }

    SECTION("iRules-only command visible in iRules") {
        CHECK(reg.find("llookup", DialectFlags::IRULES) != nullptr);
        CHECK(reg.find("when", DialectFlags::IRULES) != nullptr);
        CHECK(reg.find("HTTP::uri", DialectFlags::IRULES) != nullptr);
    }

    SECTION("iRules-only command NOT visible in core Tcl") {
        CHECK(reg.find("llookup", DialectFlags::TCL86) == nullptr);
        CHECK(reg.find("when", DialectFlags::TCL90) == nullptr);
    }

    SECTION("tcl8.6+ command visible in tcl8.6 and tcl9.0") {
        CHECK(reg.find("oo::define", DialectFlags::TCL86) != nullptr);
        CHECK(reg.find("oo::define", DialectFlags::TCL90) != nullptr);
    }

    SECTION("tcl8.6+ command NOT visible in tcl8.4") {
        CHECK(reg.find("oo::define", DialectFlags::TCL84) == nullptr);
    }

    SECTION("EDA command visible in its dialect") {
        CHECK(reg.find("foreach_in_collection", DialectFlags::EDA_SYNOPSYS) != nullptr);
    }

    SECTION("EDA command NOT visible in core Tcl") {
        CHECK(reg.find("foreach_in_collection", DialectFlags::TCL86) == nullptr);
    }

    SECTION("all_for_dialect returns correct subset") {
        auto irules = reg.all_for_dialect(DialectFlags::IRULES);
        // Should include ALL-dialect commands + IRULES-specific
        bool found_proc = false, found_llookup = false, found_oo_define = false;
        for (auto* d : irules) {
            if (d->name == "proc")
                found_proc = true;
            if (d->name == "llookup")
                found_llookup = true;
            if (d->name == "oo::define")
                found_oo_define = true;
        }
        CHECK(found_proc);            // ALL-dialect
        CHECK(found_llookup);         // IRULES-specific
        CHECK_FALSE(found_oo_define); // tcl8.6+ only, not in iRules
    }
}

// ════════════════════════════════════════════════════════════════════
// Arity
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Arity constraints", "[registry][arity]") {
    SECTION("fixed arity (proc: exactly 3)") {
        CHECK(proc_cmd.arity.accepts(3));
        CHECK_FALSE(proc_cmd.arity.accepts(2));
        CHECK_FALSE(proc_cmd.arity.accepts(4));
    }

    SECTION("min-only arity (lassign: min 2)") {
        CHECK(lassign_cmd.arity.accepts(2));
        CHECK(lassign_cmd.arity.accepts(100));
        CHECK_FALSE(lassign_cmd.arity.accepts(1));
    }

    SECTION("modular arity (foreach: min 3, odd)") {
        CHECK(foreach_cmd.arity.accepts(3));       // varList list body
        CHECK(foreach_cmd.arity.accepts(5));       // vL1 l1 vL2 l2 body
        CHECK_FALSE(foreach_cmd.arity.accepts(4)); // even = invalid
        CHECK_FALSE(foreach_cmd.arity.accepts(2)); // too few
    }

    SECTION("range arity (set: 1-2)") {
        CHECK(set_cmd.arity.accepts(1));
        CHECK(set_cmd.arity.accepts(2));
        CHECK_FALSE(set_cmd.arity.accepts(0));
        CHECK_FALSE(set_cmd.arity.accepts(3));
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 1: Fixed positional (proc)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 1: Fixed positional — proc", "[registry][resolve]") {
    std::string_view args[] = {"myProc", "{a b c}", "{ puts $a }"};
    auto result = CommandRegistry::resolve_arg_roles(proc_cmd, args);
    REQUIRE(result.has_value());
    auto roles = result->roles();
    REQUIRE(roles.size() == 3);
    CHECK(roles[0] == NAME);
    CHECK(roles[1] == PARAM_LIST);
    CHECK(roles[2] == BODY);
}

// ════════════════════════════════════════════════════════════════════
// Shape 2: Unlimited tail (lassign)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 2: Unlimited tail — lassign", "[registry][resolve]") {
    SECTION("with 2 args") {
        std::string_view args[] = {"myList", "a"};
        auto result = CommandRegistry::resolve_arg_roles(lassign_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE);
        CHECK(roles[1] == VAR_NAME);
    }

    SECTION("with many args — no index-19 limit") {
        std::string_view args[22];
        args[0] = "myList";
        for (int i = 1; i < 22; ++i)
            args[i] = "v";
        auto result = CommandRegistry::resolve_arg_roles(lassign_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE);
        CHECK(roles[1] == VAR_NAME);
        CHECK(roles[20] == VAR_NAME); // past the old index-19 limit!
        CHECK(roles[21] == VAR_NAME);
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 3: Stride pattern (foreach)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 3: Stride pattern — foreach", "[registry][resolve]") {
    SECTION("single pair + body") {
        std::string_view args[] = {"x", "$list", "{ puts $x }"};
        auto result = CommandRegistry::resolve_arg_roles(foreach_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE); // varList at stride 0
        CHECK(roles[1] == VALUE); // list at stride 1
        CHECK(roles[2] == BODY);  // last = body
    }

    SECTION("two pairs + body") {
        std::string_view args[] = {"x", "$l1", "y", "$l2", "{ puts $x $y }"};
        auto result = CommandRegistry::resolve_arg_roles(foreach_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE); // varList
        CHECK(roles[1] == VALUE); // list
        CHECK(roles[2] == VALUE); // varList
        CHECK(roles[3] == VALUE); // list
        CHECK(roles[4] == BODY);  // body
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 4: Option value roles (tcltest — handled via OptionDesc)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 4: Option value roles — option desc", "[registry][resolve]") {
    // tcltest::test has -body option with value_role = BODY
    // Simulate: test myTest "desc" -body { expr {1+1} }
    std::string_view args[] = {"myTest", "desc", "-body", "{ expr {1+1} }"};
    auto result = CommandRegistry::resolve_arg_roles(tcltest_cmd, args);
    REQUIRE(result.has_value());
    auto roles = result->roles();
    CHECK(roles[0] == NAME);  // test name
    CHECK(roles[1] == VALUE); // description
    CHECK(roles[3] == BODY);  // -body value marked as BODY via OptionDesc
}

// ════════════════════════════════════════════════════════════════════
// Shape 5: Options with terminator (regexp)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 5: Options with terminator — regexp", "[registry][resolve]") {
    // regexp -nocase -- pattern string matchVar subVar
    std::string_view args[] = {"-nocase", "--", "pat", "str", "m", "s"};
    auto result = CommandRegistry::resolve_arg_roles(regexp_cmd, args);
    REQUIRE(result.has_value());
    auto roles = result->roles();
    // Static patterns assign from index 0, but real analysis would skip options.
    // The key test here is that TAIL from index 2 onward gives VAR_NAME.
    CHECK(roles[2] == VAR_NAME);
    CHECK(roles[3] == VAR_NAME);
    CHECK(roles[4] == VAR_NAME);
    CHECK(roles[5] == VAR_NAME);
}

// ════════════════════════════════════════════════════════════════════
// Shape 6: Subcommands (string)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 6: Subcommands — string", "[registry][subcommand]") {
    SECTION("string has subcommands") {
        CHECK_FALSE(string_cmd.subcommands.empty());
    }

    SECTION("find compare subcommand") {
        const SubCmdDesc* cmp = nullptr;
        for (const auto& sub : string_cmd.subcommands) {
            if (sub.name == "compare") {
                cmp = &sub;
                break;
            }
        }
        REQUIRE(cmp != nullptr);
        CHECK(cmp->pure);
        CHECK(cmp->return_type == TclType::INT);
        CHECK_FALSE(cmp->options.empty());
    }

    SECTION("string length has type hint") {
        const SubCmdDesc* len = nullptr;
        for (const auto& sub : string_cmd.subcommands) {
            if (sub.name == "length") {
                len = &sub;
                break;
            }
        }
        REQUIRE(len != nullptr);
        REQUIRE_FALSE(len->arg_types.empty());
        CHECK(len->arg_types[0].expected == TclType::STRING);
    }

    SECTION("string is has -failindex with VAR_NAME role") {
        const SubCmdDesc* is = nullptr;
        for (const auto& sub : string_cmd.subcommands) {
            if (sub.name == "is") {
                is = &sub;
                break;
            }
        }
        REQUIRE(is != nullptr);
        bool found = false;
        for (const auto& opt : is->options) {
            if (opt.name == "-failindex") {
                CHECK(opt.takes_value);
                CHECK(opt.value_role == VAR_NAME);
                found = true;
            }
        }
        CHECK(found);
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 7: Getter/Setter forms (HTTP::uri)
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 7: Getter/Setter — HTTP::uri", "[registry][forms]") {
    REQUIRE(http_uri_cmd.forms.size() == 2);

    const auto& getter = http_uri_cmd.forms[0];
    CHECK(getter.kind == FormKind::GETTER);
    CHECK(getter.arity.accepts(0));
    CHECK_FALSE(getter.arity.accepts(1));
    CHECK(getter.pure);

    const auto& setter = http_uri_cmd.forms[1];
    CHECK(setter.kind == FormKind::SETTER);
    CHECK(setter.arity.accepts(1));
    CHECK(setter.mutator);
}

// ════════════════════════════════════════════════════════════════════
// Shape 8: Variable-layout — if
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 8: Variable-layout — if", "[registry][resolve][if]") {
    SECTION("simple if expr body") {
        std::string_view args[] = {"$x > 0", "{ puts yes }"};
        auto result = CommandRegistry::resolve_arg_roles(if_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == EXPR);
        CHECK(roles[1] == BODY);
    }

    SECTION("if expr then body else body") {
        std::string_view args[] = {"$x > 0", "then", "{ puts yes }", "else", "{ puts no }"};
        auto result = CommandRegistry::resolve_arg_roles(if_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == EXPR);
        CHECK(roles[1] == VALUE); // "then" keyword
        CHECK(roles[2] == BODY);
        CHECK(roles[3] == VALUE); // "else" keyword
        CHECK(roles[4] == BODY);
    }

    SECTION("if expr body elseif expr body else body") {
        std::string_view args[] = {
            "$x > 0", "{ puts a }", "elseif", "$x < 0", "{ puts b }", "else", "{ puts c }"};
        auto result = CommandRegistry::resolve_arg_roles(if_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == EXPR);
        CHECK(roles[1] == BODY);
        CHECK(roles[2] == VALUE); // "elseif" keyword
        CHECK(roles[3] == EXPR);
        CHECK(roles[4] == BODY);
        CHECK(roles[5] == VALUE); // "else" keyword
        CHECK(roles[6] == BODY);
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 8b: Variable-layout — switch
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 8b: Variable-layout — switch", "[registry][resolve][switch]") {
    SECTION("switch with pattern-body pairs") {
        std::string_view args[] = {"$val", "a", "{ puts a }", "b", "{ puts b }"};
        auto result = CommandRegistry::resolve_arg_roles(switch_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE); // string to match
        CHECK(roles[1] == PATTERN);
        CHECK(roles[2] == BODY);
        CHECK(roles[3] == PATTERN);
        CHECK(roles[4] == BODY);
    }

    SECTION("switch with single body block") {
        std::string_view args[] = {"$val", "{ a {puts a} b {puts b} }"};
        auto result = CommandRegistry::resolve_arg_roles(switch_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == VALUE);
        CHECK(roles[1] == BODY);
    }

    SECTION("switch with options") {
        std::string_view args[] = {"-exact", "--", "$val", "a", "{ puts a }"};
        auto result = CommandRegistry::resolve_arg_roles(switch_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == OPTION);
        CHECK(roles[1] == OPTION); // "--"
        CHECK(roles[2] == VALUE);  // string
        CHECK(roles[3] == PATTERN);
        CHECK(roles[4] == BODY);
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 8c: Variable-layout — try
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 8c: Variable-layout — try", "[registry][resolve][try]") {
    SECTION("try body on ok result body") {
        std::string_view args[] = {"{ risky }", "on", "ok", "result", "{ puts $result }"};
        auto result = CommandRegistry::resolve_arg_roles(try_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == BODY);  // try body
        CHECK(roles[1] == VALUE); // "on"
        CHECK(roles[2] == VALUE); // code
        CHECK(roles[3] == VALUE); // varList
        CHECK(roles[4] == BODY);  // handler body
    }

    SECTION("try body finally body") {
        std::string_view args[] = {"{ risky }", "finally", "{ cleanup }"};
        auto result = CommandRegistry::resolve_arg_roles(try_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == BODY);
        CHECK(roles[1] == VALUE); // "finally"
        CHECK(roles[2] == BODY);
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 9: TclOO — oo::define subcommands
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 9: TclOO — oo::define", "[registry][oo]") {
    SECTION("has subcommands") {
        CHECK_FALSE(oo_define_cmd.subcommands.empty());
    }

    SECTION("method subcommand has visibility options") {
        const SubCmdDesc* method = nullptr;
        for (const auto& sub : oo_define_cmd.subcommands) {
            if (sub.name == "method") {
                method = &sub;
                break;
            }
        }
        REQUIRE(method != nullptr);
        CHECK(method->defines_procedure);
        bool found_private = false;
        for (const auto& opt : method->options) {
            if (opt.name == "-private")
                found_private = true;
        }
        CHECK(found_private);
    }

    SECTION("slot commands have slot operations") {
        const SubCmdDesc* var = nullptr;
        for (const auto& sub : oo_define_cmd.subcommands) {
            if (sub.name == "variable") {
                var = &sub;
                break;
            }
        }
        REQUIRE(var != nullptr);
        CHECK(var->is_slot_command);
        bool found_clear = false, found_appendifnew = false;
        for (const auto& opt : var->options) {
            if (opt.name == "-clear")
                found_clear = true;
            if (opt.name == "-appendifnew")
                found_appendifnew = true;
        }
        CHECK(found_clear);
        CHECK(found_appendifnew);
    }

    SECTION("constructor has body pattern") {
        const SubCmdDesc* ctor = nullptr;
        for (const auto& sub : oo_define_cmd.subcommands) {
            if (sub.name == "constructor") {
                ctor = &sub;
                break;
            }
        }
        REQUIRE(ctor != nullptr);
        CHECK(ctor->defines_procedure);
        REQUIRE(ctor->arg_patterns.size() == 2);
        CHECK(ctor->arg_patterns[0].role == PARAM_LIST);
        CHECK(ctor->arg_patterns[1].role == BODY);
    }
}

// ════════════════════════════════════════════════════════════════════
// Shape 10: Simple iRules — llookup
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Shape 10: Simple iRules — llookup", "[registry][irules]") {
    CHECK(llookup_cmd.arity.accepts(2));
    CHECK_FALSE(llookup_cmd.arity.accepts(1));
    CHECK_FALSE(llookup_cmd.arity.accepts(3));
    CHECK(llookup_cmd.pure);
    CHECK(llookup_cmd.return_type == TclType::LIST);
    REQUIRE_FALSE(llookup_cmd.arg_types.empty());
    CHECK(llookup_cmd.arg_types[0].expected == TclType::LIST);
    CHECK(llookup_cmd.arg_types[0].shimmers);
}

// ════════════════════════════════════════════════════════════════════
// when — top_level_only + body-is-last
// ════════════════════════════════════════════════════════════════════

TEST_CASE("when — layout resolver", "[registry][resolve][when]") {
    CHECK(when_cmd.top_level_only);
    CHECK(when_cmd.is_scope_barrier);

    SECTION("simple: when EVENT body") {
        std::string_view args[] = {"HTTP_REQUEST", "{ puts hello }"};
        auto result = CommandRegistry::resolve_arg_roles(when_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == NAME); // event
        CHECK(roles[1] == BODY);
    }

    SECTION("with priority: when EVENT priority 100 body") {
        std::string_view args[] = {"HTTP_REQUEST", "priority", "100", "{ puts hello }"};
        auto result = CommandRegistry::resolve_arg_roles(when_cmd, args);
        REQUIRE(result.has_value());
        auto roles = result->roles();
        CHECK(roles[0] == NAME);
        CHECK(roles[3] == BODY); // last arg
    }
}

// ════════════════════════════════════════════════════════════════════
// binary scan — tail VAR_NAME from index 2
// ════════════════════════════════════════════════════════════════════

TEST_CASE("binary scan — tail VAR_NAME fixes index-19 limit", "[registry][resolve]") {
    // binary scan subcommand: scan string formatString ?varName ...?
    auto result = CommandRegistry::resolve_arg_roles(
        CommandDesc{.arity = {2}, .arg_patterns = binary_scan_args},
        std::array<std::string_view, 24>{"data", "a4a4a4", "v1",  "v2",  "v3",  "v4",
                                         "v5",   "v6",     "v7",  "v8",  "v9",  "v10",
                                         "v11",  "v12",    "v13", "v14", "v15", "v16",
                                         "v17",  "v18",    "v19", "v20", "v21", "v22"});
    REQUIRE(result.has_value());
    auto roles = result->roles();
    CHECK(roles[0] == VALUE);
    CHECK(roles[1] == FORMAT_STRING);
    CHECK(roles[2] == VAR_NAME);
    CHECK(roles[21] == VAR_NAME); // past old index-19 limit
    CHECK(roles[23] == VAR_NAME);
}

// ════════════════════════════════════════════════════════════════════
// Command behavioral traits
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Behavioral traits", "[registry][traits]") {
    CHECK(proc_cmd.defines_procedure);
    CHECK(proc_cmd.is_scope_barrier);
    CHECK(eval_cmd.creates_dynamic_barrier);
    CHECK(eval_cmd.unsafe);
    CHECK(eval_cmd.taint_sink);
    CHECK(foreach_cmd.has_loop_body);
    CHECK(foreach_cmd.loop_list_header);
    CHECK(foreach_cmd.is_control_flow);
    CHECK(if_cmd.is_control_flow);
    CHECK(if_cmd.never_inline_body);
    CHECK(set_cmd.assigns_variable_at == 0);
    CHECK(foreach_in_collection_cmd.has_loop_body);
    CHECK(foreach_in_collection_cmd.loop_list_header);
}

// ════════════════════════════════════════════════════════════════════
// Hover rendering
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Hover rendering", "[registry][hover]") {
    constexpr std::string_view syn[] = {"llookup MMAP KEY"};
    HoverText h{
        .summary = "Returns values matching a key.",
        .synopsis = syn,
        .source = "F5 docs",
        .return_value = "A Tcl list.",
    };

    SECTION("markdown includes all sections") {
        auto md = render_hover_markdown(h, "llookup");
        CHECK(md.find("```tcl") != std::string::npos);
        CHECK(md.find("llookup MMAP KEY") != std::string::npos);
        CHECK(md.find("Returns values matching") != std::string::npos);
        CHECK(md.find("**Returns**") != std::string::npos);
        CHECK(md.find("F5 docs") != std::string::npos);
    }

    SECTION("lean shows synopsis + summary only") {
        auto lean = render_hover_lean(h, "llookup");
        CHECK(lean.find("llookup MMAP KEY") != std::string::npos);
        CHECK(lean.find("Returns values matching") != std::string::npos);
        CHECK(lean.find("**Returns**") == std::string::npos);
    }

    SECTION("empty hover renders symbol fallback") {
        HoverText empty{};
        auto md = render_hover_markdown(empty, "myCmd");
        CHECK(md.find("myCmd") != std::string::npos);
    }
}

// ════════════════════════════════════════════════════════════════════
// {*} expansion flag
// ════════════════════════════════════════════════════════════════════

TEST_CASE("Expansion flag propagation", "[registry][resolve][expand]") {
    std::string_view args[] = {"myList", "a", "b"};
    bool expand[] = {true, false, false};
    auto result = CommandRegistry::resolve_arg_roles(lassign_cmd, args, expand);
    REQUIRE(result.has_value());
    CHECK(result->has_expansion);
}

// ════════════════════════════════════════════════════════════════════
// Dialect visibility helper
// ════════════════════════════════════════════════════════════════════

TEST_CASE("dialect_visibility expansion", "[registry][dialect]") {
    using enum DialectFlags;
    auto irules_vis = dialect_visibility(IRULES);
    CHECK((irules_vis & TCL84) != NONE);
    CHECK((irules_vis & IRULES) != NONE);
    CHECK((irules_vis & TCL86) == NONE); // iRules doesn't see tcl8.6

    auto tmsh_vis = dialect_visibility(TMSH);
    CHECK((tmsh_vis & TCL84) != NONE);
    CHECK((tmsh_vis & TCL86) != NONE);
    CHECK((tmsh_vis & TMSH) != NONE);
    CHECK((tmsh_vis & IRULES) == NONE);
}
