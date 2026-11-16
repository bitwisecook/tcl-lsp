// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Argument roles of a call: role queries, resolver capabilities, format and pattern arguments, option values and command prefixes.

use super::*;

mod command_prefixes;
mod format_and_pattern_arguments;
mod role_capabilities;

#[test]
fn representation_hint_requires_only_the_live_option_prefix() {
    use crate::InvocationWord::{Dynamic, Literal};
    let context = crate::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    for (arguments, subject) in [
        (vec![Literal("map"), Literal("a b"), Dynamic], 2),
        (
            vec![Literal("map"), Literal("-nocase"), Literal("a b"), Dynamic],
            3,
        ),
    ] {
        let words = crate::InvocationWords::structured(Literal("string"), &arguments).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            registry
                .arg_type_hint_words(words, subject)
                .unwrap()
                .expected,
            Some(crate::TclType::String)
        );
    }
    let arguments = [Literal("map"), Dynamic, Dynamic];
    let words = crate::InvocationWords::structured(Literal("string"), &arguments).with_dialect(
        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(registry.arg_type_hint_words(words, 2).is_none());
}

// The complement of `arg_indices_for_role`: what the *next* words would
// mean, which is the question a splice-a-trailing-argument quick fix asks.

#[test]
fn unfilled_trailing_roles_reports_the_optional_capture_variables() {
    let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
    // `catch {body}` leaves both `VarWrite` slots open.
    assert_eq!(
        reg.unfilled_trailing_roles("catch", &["{body}"]),
        vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
    );
    // One supplied leaves one.
    assert_eq!(
        reg.unfilled_trailing_roles("catch", &["{body}", "res"]),
        vec![(2, ArgRole::VarWrite)]
    );
    // Fully supplied leaves none.
    assert_eq!(
        reg.unfilled_trailing_roles("catch", &["{body}", "res", "opts"])
            .len(),
        0
    );
}

#[test]
fn prospective_trailing_layout_preserves_unknown_source_values() {
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    let ordinary = [InvocationWord::Literal("aa"), InvocationWord::Dynamic];
    let roles =
        reg.unfilled_trailing_roles_words("regexp", InvocationArguments::structured(&ordinary));
    assert_eq!(roles.first(), Some(&(2, ArgRole::VarWrite)));
    assert!(roles.iter().all(|(_, role)| *role == ArgRole::VarWrite));
    let replacement = [
        InvocationWord::Literal("-command"),
        InvocationWord::Literal("aa"),
        InvocationWord::Dynamic,
        InvocationWord::Dynamic,
    ];
    assert_eq!(
        reg.unfilled_trailing_roles_words("regsub", InvocationArguments::structured(&replacement),),
        vec![(4, ArgRole::VarWrite)]
    );
    for words in [
        vec![InvocationWord::Dynamic, InvocationWord::Dynamic],
        vec![
            InvocationWord::Literal("-inline"),
            InvocationWord::Literal("aa"),
            InvocationWord::Dynamic,
        ],
        vec![InvocationWord::Literal("aa"), InvocationWord::Expanded],
    ] {
        assert!(
            reg.unfilled_trailing_roles_words("regexp", InvocationArguments::structured(&words),)
                .is_empty(),
            "{words:?}"
        );
    }
}

#[test]
fn unfilled_trailing_roles_stops_at_the_arity_ceiling() {
    let reg = CommandRegistry::build_default();
    // Every position past `catch`'s maximum of three is absent, however
    // many are asked for.
    let roles = reg.unfilled_trailing_roles("catch", &["{body}"]);
    assert!(roles.iter().all(|(index, _)| *index < 3), "{roles:?}");
}

#[test]
fn unfilled_trailing_roles_is_empty_for_an_unknown_command() {
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.unfilled_trailing_roles("no_such_command", &[]).len(), 0);
}

#[test]
fn unfilled_trailing_roles_stops_at_an_undeclared_position() {
    let reg = CommandRegistry::build_default();
    // `puts` declares no trailing role a caller could fill with a
    // *known* meaning, so nothing is offered rather than an unbounded
    // run of `Value` slots.
    assert_eq!(reg.unfilled_trailing_roles("puts", &["hello"]).len(), 0);
}

#[test]
fn arg_roles_for_static_command() {
    let reg = CommandRegistry::build_default();
    let bodies = reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Body);
    assert!(bodies.contains(&0)); // init
    assert!(bodies.contains(&2)); // next
    assert!(bodies.contains(&3)); // body
    assert!(!bodies.contains(&1)); // cond is Expr, not Body
}

#[test]
fn arg_roles_for_expr() {
    let reg = CommandRegistry::build_default();
    let exprs = reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Expr);
    assert_eq!(exprs, vec![1]); // only the condition
}

#[test]
fn uplevel_body_arg_role_skips_optional_level() {
    // `uplevel ?level? {body}` — the body word's index depends
    // on whether a leading `level` word is present. The registry resolver
    // is the single source of truth every body consumer (semantic tokens,
    // green-tree descent, SSA) queries.
    let reg = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    // Literal relative level → body at 1.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["1", "{set x 1}"], ArgRole::Body),
        vec![1]
    );
    // Absolute `#0` level → body at 1.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["#0", "{set x 1}"], ArgRole::Body),
        vec![1]
    );
    // No level → body at 0.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["{set x 1}"], ArgRole::Body),
        vec![0]
    );
    // The literal bytes `$lvl` are a script word, not a supplied level.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["$lvl", "{set x 1}"], ArgRole::Body),
        vec![0]
    );
    // A lone dynamic word is the body itself (implicit level 1) → body at 0.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["$body"], ArgRole::Body),
        vec![0]
    );
    // Bodyless `uplevel 1` (a wrong-#args error) exposes no body word — the
    // literal level must not be mis-tagged as a script.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["1"], ArgRole::Body)
            .len(),
        0
    );
}

#[test]
fn if_marks_structural_keywords() {
    let reg = CommandRegistry::build_default();
    let args = [
        "1", "then", "{a}", "elseif", "2", "then", "{b}", "else", "{c}",
    ];
    let kw = reg.arg_indices_for_role("if", &args, ArgRole::Keyword);
    // then@1, elseif@3, then@5, else@7
    assert_eq!(kw, vec![1, 3, 5, 7], "{kw:?}");
    // The bodies and exprs still resolve too.
    let bodies = reg.arg_indices_for_role("if", &args, ArgRole::Body);
    assert!(bodies.contains(&2) && bodies.contains(&6) && bodies.contains(&8));
}

#[test]
fn try_marks_structural_keywords() {
    let reg = CommandRegistry::build_default();
    let args = ["{body}", "on", "error", "{e}", "{h}", "finally", "{f}"];
    let kw = reg.arg_indices_for_role("try", &args, ArgRole::Keyword);
    // on@1, finally@5
    assert_eq!(kw, vec![1, 5], "{kw:?}");
}

/// The repeated argument tails a fixed index table cannot
/// express answer through the ordinary role query.
#[test]
fn repeated_layouts_answer_through_arg_indices_for_role() {
    let reg = CommandRegistry::build_default();
    let vars = |name: &str, args: &[&str]| reg.arg_indices_for_role(name, args, ArgRole::VarWrite);
    // TP — every argument of `global`.
    assert_eq!(vars("global", &["a", "b", "c"]), vec![0, 1, 2]);
    // TP — every *even* argument of `variable` (names, not values).
    assert_eq!(vars("variable", &["x", "1", "y", "2"]), vec![0, 2]);
    // TP — the local of each `namespace upvar` pair, past the namespace.
    assert_eq!(
        vars("namespace", &["upvar", "::ns", "o1", "l1", "o2", "l2"]),
        vec![3, 5]
    );
    // TP — each `dict update` varName, with the trailing body excluded.
    // Index 1 is the dictionary variable itself, which the subcommand
    // already declared (it is read on entry and written back after the
    // body).  The pair locals answer as `LoopVarList` — bound once
    // before the body, and only when the key is present — NOT as
    // `VarWrite`: an unconditional SSA def both defeated the key-aware
    // read-before-set suppression and would hide the genuine
    // absent-key warning.
    assert_eq!(
        vars("dict", &["update", "d", "k1", "v1", "k2", "v2", "{body}"]),
        vec![1]
    );
    assert_eq!(
        reg.arg_indices_for_role(
            "dict",
            &["update", "d", "k1", "v1", "k2", "v2", "{body}"],
            ArgRole::LoopVarList
        ),
        vec![3, 5]
    );
    // TP — `foreach` / `lmap` variable specs, body excluded.
    for name in ["foreach", "lmap"] {
        assert_eq!(
            reg.arg_indices_for_role(
                name,
                &["{a b}", "$l1", "c", "$l2", "{body}"],
                ArgRole::LoopVarList
            ),
            vec![0, 2],
            "{name}"
        );
    }
    // FN guard — the explicitly global spellings resolve identically.
    assert_eq!(vars("::global", &["a", "b"]), vars("global", &["a", "b"]));
    // TN — a command with no repeated tail is unaffected.
    assert_eq!(vars("puts", &["a", "b"]), Vec::<usize>::new());
    // FP guard — a short call names nothing it should not.
    assert_eq!(vars("global", &[]), Vec::<usize>::new());
    assert_eq!(
        reg.arg_indices_for_role("foreach", &["{body}"], ArgRole::LoopVarList),
        Vec::<usize>::new()
    );
}

#[test]
fn option_value_arity_shifts_registry_argument_roles() {
    let reg = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    assert_eq!(
        reg.arg_indices_for_role(
            "regexp",
            &["-start", "2", "pattern", "$text", "match"],
            ArgRole::VarWrite,
        ),
        vec![4]
    );
    assert_eq!(
        reg.arg_indices_for_role(
            "regsub",
            &["-start", "2", "pattern", "$text", "replacement", "result"],
            ArgRole::VarWrite,
        ),
        vec![5]
    );
    assert_eq!(
        reg.arg_indices_for_role("upvar", &["0", "remote", "local"], ArgRole::VarWrite,),
        vec![2]
    );
}

/// Each `otherVar` of `upvar` and `namespace upvar` names the cell the
/// pair links (#2259), wherever the optional level puts it, beside the
/// `myVar` local the link writes.
#[test]
fn upvar_other_vars_carry_the_cell_role() {
    let reg = CommandRegistry::build_default();
    for (args, cells, locals) in [
        (&["a", "b"][..], vec![0], vec![1]),
        (&["1", "a", "b"][..], vec![1], vec![2]),
        (&["a", "b", "c", "d"][..], vec![0, 2], vec![1, 3]),
        (&["#0", "a", "b", "c", "d"][..], vec![1, 3], vec![2, 4]),
    ] {
        assert_eq!(
            reg.arg_indices_for_role("upvar", args, ArgRole::VarCell),
            cells,
            "{args:?}"
        );
        assert_eq!(
            reg.arg_indices_for_role("upvar", args, ArgRole::VarWrite),
            locals,
            "{args:?}"
        );
    }
    let args = ["upvar", "::ns", "a", "b", "c", "d"];
    assert_eq!(
        reg.arg_indices_for_role("namespace", &args, ArgRole::VarCell),
        vec![2, 4]
    );
    assert_eq!(
        reg.arg_indices_for_role("namespace", &args, ArgRole::VarWrite),
        vec![3, 5]
    );
}

/// `for`'s three script arguments share the semantic
/// [`ArgRole::Body`], and the *presentation* fact is what separates the
/// trailing body (block-expanded) from `start` / `next` (inline).
#[test]
fn for_declares_inline_presentation_for_start_and_next() {
    use crate::presentation::ArgPresentation;
    let reg = CommandRegistry::build_default();
    let args = ["{set i 0}", "{$i < 3}", "{incr i}", "{puts $i}"];
    // Semantics unchanged: all three scripts are still bodies.
    assert_eq!(
        reg.arg_indices_for_role("for", &args, ArgRole::Body),
        vec![0, 2, 3]
    );
    // Presentation splits them.
    assert_eq!(
        reg.arg_presentation("for", &args, 0),
        ArgPresentation::InlineScript
    );
    assert_eq!(
        reg.arg_presentation("for", &args, 2),
        ArgPresentation::InlineScript
    );
    assert_eq!(
        reg.arg_presentation("for", &args, 3),
        ArgPresentation::BlockScript
    );
    assert!(reg.arg_presentation("for", &args, 3).is_block());
    // The absolute global spelling resolves to the same spec.
    assert_eq!(
        reg.arg_presentation("::for", &args, 0),
        ArgPresentation::InlineScript
    );
    // A command with nothing to say answers the block default, and so
    // does one the registry does not know at all.
    assert_eq!(
        reg.arg_presentation("while", &["{$x}", "{body}"], 1),
        ArgPresentation::BlockScript
    );
    assert_eq!(
        reg.arg_presentation("no::such::command", &["{a}"], 0),
        ArgPresentation::BlockScript
    );
}

#[test]
fn option_value_role_emits_body_index() {
    use crate::hover::OptionValue;
    let options = [
        opt("-command", OptionValue::script()),
        opt("-flag", OptionValue::flag()),
    ];
    let args = ["-command", "{puts hi}", "-flag", "x"];
    // The `-command` value is a Body; the flag consumes nothing.
    assert_eq!(indices(&options, &args, ArgRole::Body), vec![1]);
    // It is not a generic Value role.
    assert_eq!(indices(&options, &args, ArgRole::Value).len(), 0);
}

#[test]
fn option_value_fixed_arity_emits_n_indices() {
    use crate::hover::OptionValue;
    let options = [opt("-rect", OptionValue::fixed(4, ArgRole::Value, "coord"))];
    let args = ["-rect", "1", "2", "3", "4", "tail"];
    assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3, 4]);
}

/// An authored hook owns its own termination policy.
fn rest_value(args: &[&str], start: usize) -> crate::hover::OptionValueOutcome {
    crate::hover::OptionValueOutcome {
        words: args[start..]
            .iter()
            .position(|word| *word == "--")
            .unwrap_or(args.len() - start),
        invalid: None,
    }
}

#[test]
fn option_value_rest_arity_stops_at_terminator() {
    let rest = crate::hover::OptionSpec {
        name: "-rest",
        value: crate::hover::OptionValue::Takes(crate::hover::OptionArg {
            arity: crate::hover::OptionArity::Hook(rest_value),
            ..crate::hover::OptionArg::DEFAULT
        }),
        ..crate::hover::OptionSpec::DEFAULT
    };
    let options = [rest];
    let args = ["-rest", "a", "b", "c", "--", "d"];
    // Consumes a, b, c up to `--`, not d.
    assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3]);
}

#[test]
fn option_value_terminator_stops_scan() {
    use crate::hover::OptionValue;
    let options = [opt("-command", OptionValue::script())];
    let args = ["--", "-command", "{x}"];
    assert_eq!(indices(&options, &args, ArgRole::Body).len(), 0);
}

#[test]
fn option_value_two_way_var_name_emits_for_both_roles() {
    use crate::hover::OptionValue;
    let options = [opt("-textvariable", OptionValue::var_name())];
    let args = ["-textvariable", "myvar"];
    assert_eq!(indices(&options, &args, ArgRole::VarWrite), vec![1]);
    assert_eq!(indices(&options, &args, ArgRole::VarRead), vec![1]);
}

#[test]
fn option_value_alias_matches_and_dynamic_flag_skipped() {
    use crate::hover::OptionValue;
    let options = [opt_with_alias("-command", &["-cmd"], OptionValue::script())];
    // Alias resolves to the same value role.
    assert_eq!(indices(&options, &["-cmd", "{x}"], ArgRole::Body), vec![1]);
    // A `$var` in flag position isn't an option name → treated as a
    // positional, so the real `-command` after it still resolves.
    assert_eq!(
        indices(&options, &["$opt", "v", "-command", "{x}"], ArgRole::Body),
        vec![3]
    );
}

#[test]
fn options_do_not_leak_into_positional_role_queries() {
    // A real command with value-taking options but no role annotations
    // must not surface any option value under Body/VarWrite (inert until
    // annotated).
    let reg = CommandRegistry::build_default();
    let b = reg.arg_indices_for_role("entry", &[".e", "-width", "10"], ArgRole::Body);
    assert!(b.is_empty(), "{b:?}");
}

#[test]
fn namespace_name_option_carries_name_role() {
    // `interp invokehidden path -namespace ns cmd` — the `-namespace` value is a
    // symbolic (namespace) name: captured declaratively for a
    // Name query, never for Body/VarWrite (not recursed, not a var def).
    let reg = CommandRegistry::build_default();
    let args = ["invokehidden", "child", "-namespace", "ns", "cmd"];
    assert_eq!(
        reg.arg_indices_for_role("interp", &args, ArgRole::Name),
        vec![3],
        "the -namespace value should carry the Name role",
    );
    assert!(
        reg.arg_indices_for_role("interp", &args, ArgRole::Body)
            .is_empty()
            && reg
                .arg_indices_for_role("interp", &args, ArgRole::VarWrite)
                .is_empty(),
        "a name value must not be recursed or treated as a variable",
    );
}

#[test]
fn dialect_filter() {
    let reg = CommandRegistry::build_default();
    let spec = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(spec.is_some());
    // dict is tcl8.5+ so should NOT be available in 8.4
    let spec84 = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
    assert!(spec84.is_none());
}

#[test]
fn subcommand_arg_roles() {
    let reg = CommandRegistry::build_default();
    let bodies = reg.arg_indices_for_role("dict", &["for", "{k v}", "$d", "body"], ArgRole::Body);
    // dict for {varList} dictExpr body → body is at index 3 (subcmd=0, args 1-based +1)
    assert!(bodies.contains(&3));
}

#[test]
fn variable_write_commands() {
    let reg = CommandRegistry::build_default();
    let set_vars = reg.arg_indices_for_role("set", &["x", "1"], ArgRole::VarWrite);
    assert_eq!(set_vars, vec![0]);
}

#[test]
fn role_queries_share_selected_set_forms_with_owned_facts() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    for (args, role, expected) in [
        (vec!["x"], ArgRole::VarRead, vec![0]),
        (vec!["x"], ArgRole::VarWrite, vec![]),
        (vec!["x", "value"], ArgRole::VarRead, vec![]),
        (vec!["x", "value"], ArgRole::VarWrite, vec![0]),
    ] {
        let facts = registry
            .resolve_invocation("set", &args, registry.own_surface_query())
            .expect("native set")
            .facts();
        let positions: Vec<_> = facts
            .arg_roles
            .iter()
            .filter(|(_, found)| *found == role)
            .map(|(index, _)| facts.argument_offset + usize::from(*index))
            .collect();
        assert_eq!(positions, expected);
        assert_eq!(
            registry.arg_indices_for_role("::set", &args, role),
            expected
        );
    }
}

#[test]
fn source_role_queries_select_forms_without_requiring_operand_values() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    let read = [InvocationWord::Dynamic];
    let write = [InvocationWord::Dynamic, InvocationWord::Dynamic];
    assert_eq!(
        registry.arg_indices_for_role_words(
            "set",
            InvocationArguments::structured(&read),
            ArgRole::VarRead
        ),
        Some(vec![0])
    );
    assert_eq!(
        registry.arg_indices_for_role_words(
            "set",
            InvocationArguments::structured(&write),
            ArgRole::VarWrite
        ),
        Some(vec![0])
    );
    let expanded = [InvocationWord::Expanded];
    assert_eq!(
        registry.arg_indices_for_role_words(
            "set",
            InvocationArguments::structured(&expanded),
            ArgRole::VarWrite
        ),
        None
    );
}

#[test]
fn source_procedure_roles_do_not_require_native_definition_acceptance() {
    // naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    let registry = CommandRegistry::build_default();
    let values = [
        InvocationWord::Dynamic,
        InvocationWord::KnownBytes(b"\xff"),
        InvocationWord::Dynamic,
    ];
    let arguments = InvocationArguments::structured(&values);
    let selected = registry
        .resolve_structured_invocation(
            InvocationWords::from_arguments(InvocationWord::Literal("proc"), arguments),
            None,
        )
        .resolved()
        .unwrap();
    assert!(!selected.facts().arg_roles_complete);
    assert_eq!(selected.facts().arg_roles.len(), 0);
    assert_eq!(
        registry.arg_role_assignments_words(
            "proc",
            arguments,
            &[ArgRole::Name, ArgRole::ParamList, ArgRole::Body],
        ),
        Some(vec![
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body)
        ])
    );
    assert_eq!(
        registry.arg_indices_for_role("proc", &["p", "a b", "return"], ArgRole::Body,),
        vec![2]
    );
    let expanded = [InvocationWord::Expanded];
    assert_eq!(
        registry.arg_role_assignments_words(
            "proc",
            InvocationArguments::structured(&expanded),
            &[ArgRole::Body],
        ),
        None
    );
    let opaque = [
        InvocationWord::Dynamic,
        InvocationWord::Opaque,
        InvocationWord::Dynamic,
    ];
    assert_eq!(
        registry.arg_role_assignments_words(
            "proc",
            InvocationArguments::structured(&opaque),
            &[ArgRole::Body],
        ),
        None
    );
}

#[test]
fn source_role_queries_retain_explicit_optional_frame_grammar() {
    let registry = CommandRegistry::build_default();
    let args = [
        InvocationWord::Literal("+0"),
        InvocationWord::Literal("remote"),
        InvocationWord::Literal("local"),
    ];
    let legacy = InvocationArguments::structured(&args).with_dialect(
        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
    );
    let modern = InvocationArguments::structured(&args).with_dialect(
        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(
        registry.arg_indices_for_role_words("upvar", legacy, ArgRole::VarWrite),
        Some(vec![])
    );
    assert_eq!(
        registry.arg_indices_for_role_words("upvar", modern, ArgRole::VarWrite),
        Some(vec![2])
    );
}

/// `unset x y z` names *every* argument as a variable, not
/// just the first, so all of them highlight as variables.
#[test]
fn unset_marks_every_name() {
    let reg = CommandRegistry::build_default();
    let vars = reg.arg_indices_for_role("unset", &["x", "y", "z"], ArgRole::VarWrite);
    assert_eq!(vars, vec![0, 1, 2]);
}

/// `arg_indices_evaluated_in_frame` is the call-site half of
/// `ArgRole::braced_word_evaluated_in_frame`: it names the positions whose
/// brace-quoted word the callee still substitutes against the *caller's*
/// variables. tclsh 9.0.4: `set x 5; expr {$x} + 1` → `6`, while
/// `lindex {$x} 0` → the two characters `$x` and
/// `apply {{} {puts $x}}` errors with `can't read "x"` (a fresh frame).
#[test]
fn arg_indices_evaluated_in_frame_names_only_caller_frame_code_words() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_evaluated_in_frame("expr", &["$x", "+", "1"]),
        vec![0],
    );
    assert_eq!(
        reg.arg_indices_evaluated_in_frame("lindex", &["$x", "0"])
            .len(),
        0
    );
    assert_eq!(
        reg.arg_indices_evaluated_in_frame("apply", &["{{} {puts $x}}"])
            .len(),
        0
    );
}

/// `unset -nocomplain -- a b` skips the leading options and names only the
/// real variables (`a`, `b`), mirroring `lower_unset`.
#[test]
fn unset_skips_leading_options() {
    let reg = CommandRegistry::build_default();
    let vars =
        reg.arg_indices_for_role("unset", &["-nocomplain", "--", "a", "b"], ArgRole::VarWrite);
    assert_eq!(vars, vec![2, 3]);
}

/// `unset` recognises only `-nocomplain` / `--` as options — a dash-prefixed
/// word like `-foo` is a real variable name (verified against tclsh), so it
/// keeps its `VarWrite` role.
#[test]
fn unset_dash_name_is_a_variable() {
    let reg = CommandRegistry::build_default();
    // `unset -foo bar` — both are variables (no `--` needed to reach them).
    assert_eq!(
        reg.arg_indices_for_role("unset", &["-foo", "bar"], ArgRole::VarWrite),
        vec![0, 1]
    );
    // `unset -nocomplain -foo` — `-nocomplain` is skipped, `-foo` is a name.
    assert_eq!(
        reg.arg_indices_for_role("unset", &["-nocomplain", "-foo"], ArgRole::VarWrite),
        vec![1]
    );
}

#[test]
fn arg_indices_for_role_trace_add_variable_var_write() {
    let reg = CommandRegistry::build_default();
    let writes = reg.arg_indices_for_role(
        "trace",
        &["add", "variable", "x", "write", "body"],
        ArgRole::VarWrite,
    );
    // +1 for subcommand offset.  arg "x" is at idx 2 in the
    // full args list (sub args[1] + 1).
    assert!(writes.contains(&2), "VarWrite writes={writes:?}");
}

/// `trace add execution` does NOT declare `VarWrite`
/// (the second arg is a command name, not a variable).
#[test]
fn arg_indices_for_role_trace_add_execution_no_var_write() {
    let reg = CommandRegistry::build_default();
    let writes = reg.arg_indices_for_role(
        "trace",
        &["add", "execution", "foo", "enter", "body"],
        ArgRole::VarWrite,
    );
    assert!(writes.is_empty(), "VarWrite writes={writes:?}");
}

/// `trace add command`/`execution` trace a command *by name*, so the name
/// argument is a `CommandName` reference (navigation follows it), while a
/// `variable` trace's name is not.
#[test]
fn arg_indices_for_role_trace_add_command_name_reference() {
    let reg = CommandRegistry::build_default();
    for kind in ["command", "execution"] {
        let names = reg.arg_indices_for_role(
            "trace",
            &["add", kind, "foo", "enter", "body"],
            ArgRole::CommandName,
        );
        // +1 subcommand offset: `foo` is at full-args index 2.
        assert!(names.contains(&2), "{kind}: CommandName names={names:?}");
    }
    let var = reg.arg_indices_for_role(
        "trace",
        &["add", "variable", "x", "write", "body"],
        ArgRole::CommandName,
    );
    assert!(var.is_empty(), "a variable trace names no command: {var:?}");
}

/// `trace remove variable` behaves like `trace add variable`:
/// both alias spellings flow through the same `VarWrite` query.
#[test]
fn arg_indices_for_role_trace_remove_variable_var_write() {
    let reg = CommandRegistry::build_default();
    let writes = reg.arg_indices_for_role(
        "trace",
        &["remove", "variable", "y", "write", "body"],
        ArgRole::VarWrite,
    );
    assert!(writes.contains(&2), "VarWrite writes={writes:?}");
}

#[test]
fn arg_indices_for_role_dict_with_multirole() {
    let reg = CommandRegistry::build_default();
    let reads = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarRead);
    let writes = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarWrite);
    assert!(reads.contains(&1), "VarRead reads={reads:?}");
    assert!(writes.contains(&1), "VarWrite writes={writes:?}");
}

#[test]
fn arg_indices_for_role_dict_update_multirole() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let reg = CommandRegistry::build_default();
    let reads = reg.arg_indices_for_role(
        "dict",
        &["update", "$var", "k", "vname", "body"],
        ArgRole::VarRead,
    );
    let writes = reg.arg_indices_for_role(
        "dict",
        &["update", "$var", "k", "vname", "body"],
        ArgRole::VarWrite,
    );
    assert!(reads.contains(&1), "VarRead reads={reads:?}");
    assert!(writes.contains(&1), "VarWrite writes={writes:?}");
}

#[test]
fn dynamic_arg_role_resolution() {
    let reg = CommandRegistry::build_default();
    // if expr body elseif expr body else body
    let roles = reg.arg_indices_for_role(
        "if",
        &[
            "$x", "then", "body1", "elseif", "$y", "body2", "else", "body3",
        ],
        ArgRole::Body,
    );
    // Bodies are at positions 2, 5, 7
    assert!(roles.contains(&2));
    assert!(roles.contains(&5));
    assert!(roles.contains(&7));
}
