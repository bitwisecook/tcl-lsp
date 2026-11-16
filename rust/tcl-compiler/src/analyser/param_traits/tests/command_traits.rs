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

//! The trait each command gives a parameter: evaluation, loops, `upvar`, dynamic-name locals, command substitutions, reference components and value copies.

use super::*;

fn lexical_reference_values<'p>(
    source_word: &str,
    parameters: &[&'p str],
    config: LexerConfig,
) -> ArgumentValues<'p> {
    let registry = CommandRegistry::build_default();
    let source = format!("command {source_word}");
    let commands = segment_commands_with_offset_and_config(&source, 0, config);
    assert_eq!(commands.len(), 1);
    let parameters = parameters.iter().copied().collect();
    ArgumentValues::lexical(
        &source,
        &commands[0],
        &parameters,
        &Aliases::default(),
        config,
        &registry,
    )
}

#[test]
fn extract_var_name_simple() {
    let parameters = ["foo", "1abc"];
    let whole =
        |source| lexical_reference_values(source, &parameters, LexerConfig::default()).argument(0);
    assert_eq!(whole("$foo"), Some("foo"));
    assert_eq!(whole("${foo}"), Some("foo"));
    assert_eq!(whole("foo"), None);
    assert_eq!(whole("$"), None);
    assert_eq!(whole("$1abc"), Some("1abc"));
}

#[test]
fn eval_param_records_eval_trait() {
    let traits = infer(&["body"], "eval $body");
    assert_trait(&traits, "body", ProcArgTrait::Eval);
}

/// The *deep* pass
/// (`infer_param_traits_deep`, which alone recurses into braced body
/// arguments) must reach real commands *inside* an `apply` lambda body,
/// not misread the whole `{argList} {body}` blob as one script (which
/// would treat the parameter word as a command name and never find the
/// `eval` at all) — mirrors `declarations_deep_recurse_through_declared_body_args`,
/// swapping the registry-known `apply`/`LambdaLiteral` shape in for a
/// stub-declared `Body` shape. The lambda's own param is `x`, bound to
/// the literal `1` — an enclosing `body` param is neither the lambda's
/// param nor forwarded into the call, so it must record nothing: a lambda
/// body runs in a fresh frame, and a same-named enclosing param is not
/// implicitly in scope there.
#[test]
fn eval_inside_apply_lambda_body_does_not_leak_to_unrelated_enclosing_param() {
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits_deep(
        &["body"],
        "apply {x {eval $body}} 1",
        env_for(&registry, LexerConfig::default()),
    );
    assert!(
        !traits
            .get("body")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "an apply lambda's fresh frame must not leak a same-named \
             enclosing param's trait when that param is never forwarded \
             into the call; got {traits:?}"
    );
}

/// The real forwarding case: the lambda's own param `x` is `eval`'d
/// inside its body, and the enclosing `body` param is passed as the
/// actual argument that binds to `x` — so `body`'s value genuinely does
/// flow into an `eval`, and the trait must propagate back to it.
#[test]
fn eval_param_forwarded_into_apply_lambda_records_eval_trait() {
    let traits = infer_owned(&["body"], "apply {x {eval $x}} $body", true);
    assert_trait(&traits, "body", ProcArgTrait::Eval);
}

#[test]
fn trait_inference_is_dialect_aware_via_expand_syntax() {
    // `extract_commands` / `scan_deep` re-segment under the document
    // dialect, so the body's `{*}` is the expansion operator on 8.5+
    // but a literal brace word on 8.4.  `eval {*}$p` is therefore
    // `eval $p` (→ `p` is Eval) on 9.0 but `eval` of the single
    // composite word `{*}$p` (no clean `$p`, no trait) on 8.4.
    let registry = CommandRegistry::build_default();
    let on = infer(&["p"], "eval {*}$p");
    assert!(
        on.get("p").is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "9.0 expands `{{*}}` → eval $p → Eval: {on:?}",
    );
    let off = infer_param_traits(
        &["p"],
        "eval {*}$p",
        env_for(&registry, LexerConfig::for_dialect("tcl8.4")),
    );
    assert!(
        off.get("p")
            .is_none_or(|s| !s.contains(&ProcArgTrait::Eval)),
        "8.4 treats `{{*}}` as literal → no clean $p → no Eval: {off:?}",
    );
}

#[test]
fn uplevel_records_eval_on_last_arg_only() {
    let traits = infer(&["lvl", "script"], "uplevel $lvl $script");
    assert_trait(&traits, "script", ProcArgTrait::Eval);
    assert!(
        !traits
            .get("lvl")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval))
    );
}

/// Eval follows the registry's evaluation traits: every word of a script
/// that concatenates its arguments (`uplevel 1 $a $b` runs `$a $b`), a
/// script word of any evaluating command, and a `::`-rooted spelling.
#[test]
fn eval_reads_the_registry_evaluation_traits() {
    let traits = infer(&["a", "b"], "uplevel 1 $a $b");
    assert_trait(&traits, "a", ProcArgTrait::Eval);
    assert_trait(&traits, "b", ProcArgTrait::Eval);
    assert_trait(
        &infer(&["s"], "namespace eval ::ns $s"),
        "s",
        ProcArgTrait::Eval,
    );
    assert_trait(&infer(&["s"], "::eval $s"), "s", ProcArgTrait::Eval);
}

/// Both trackers read the `VarWrite` role: any write of an `upvar` alias
/// writes the caller's variable, a read through it does not, and any write
/// of a local ends the parameter value it carried.
#[test]
fn the_copy_trackers_read_the_var_write_role() {
    assert_trait(
        &infer(&["v"], "upvar 1 $v arr\narray set arr {k 1}"),
        "v",
        ProcArgTrait::VarWrite,
    );
    let traits = infer(&["v"], "upvar 1 $v local\nreturn [set local]");
    assert!(!traits["v"].contains(&ProcArgTrait::VarWrite), "{traits:?}");
    assert!(!infer(&["v"], "set n $v\nlassign {x} n\nset $n 1").contains_key("v"));
}

#[test]
fn foreach_records_loop_list_and_body() {
    let traits = infer(&["items", "body"], "foreach x $items $body");
    assert_trait(&traits, "items", ProcArgTrait::LoopList);
    assert_trait(&traits, "body", ProcArgTrait::Body);
}

#[test]
fn while_records_expr_and_body() {
    let traits = infer(&["cond", "body"], "while $cond $body");
    assert_trait(&traits, "cond", ProcArgTrait::Expr);
    assert_trait(&traits, "body", ProcArgTrait::Body);
}

#[test]
fn for_records_init_cond_next_body() {
    let traits = infer(&["i", "c", "n", "b"], "for $i $c $n $b");
    assert_trait(&traits, "i", ProcArgTrait::Body);
    assert_trait(&traits, "c", ProcArgTrait::Expr);
    assert_trait(&traits, "n", ProcArgTrait::Body);
    assert_trait(&traits, "b", ProcArgTrait::Body);
}

#[test]
fn upvar_records_var_read_and_aliases_writes() {
    let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
    assert_trait(&traits, "var", ProcArgTrait::VarRead);
    // Write through the alias upgrades to VarWrite.
    assert_trait(&traits, "var", ProcArgTrait::VarWrite);
}

#[test]
fn upvar_my_var_slot_param_is_callee_local_not_var_write() {
    // `upvar 1 caller $local` — the param in the `myVar` slot names the
    // *callee-local* alias, not a caller variable, so it is a
    // `DynamicNameLocal` use, never a caller-frame `VarWrite`.
    let traits = infer(&["local"], "upvar 1 caller $local");
    assert_trait(&traits, "local", ProcArgTrait::DynamicNameLocal);
    assert!(
        !traits
            .get("local")
            .unwrap()
            .contains(&ProcArgTrait::VarWrite),
        "myVar-slot param must not be a caller VarWrite; got {:?}",
        traits.get("local"),
    );
}

#[test]
fn lassign_records_dynamic_name_local() {
    // `lassign {1 2} $a $b` names CALLEE-LOCAL output vars via the
    // params' values — DynamicNameLocal (+ VarRead), NOT VarWrite
    // (which would imply a caller-frame alias).
    let traits = infer(&["a", "b"], "lassign {1 2} $a $b");
    for p in ["a", "b"] {
        assert_trait(&traits, p, ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, p, ProcArgTrait::VarRead);
        assert!(
            !traits.get(p).unwrap().contains(&ProcArgTrait::VarWrite),
            "{p}: callee-local lassign target must not be VarWrite, got {:?}",
            traits.get(p),
        );
    }
}

#[test]
fn command_substitution_is_scanned() {
    // A param used inside a `[...]` substitution is a first-order use — the
    // shallow scan must descend into it.  `return [set $v 1]` names a
    // callee-local variable via `$v` → DynamicNameLocal (+ VarRead).
    let traits = infer(&["v"], "return [set $v 1]");
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "v", ProcArgTrait::VarRead);
    // The read form (single-arg `set`) is scanned identically.
    let traits = infer(&["v"], "return [set $v]");
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
}

#[test]
fn command_substitution_plain_value_is_not_a_var_name() {
    // `[string length $x]` reads `$x` as a value, not a variable name, so
    // no role is recorded — the descent must not over-mark ordinary reads.
    let traits = infer(&["x"], "return [string length $x]");
    assert!(
        traits.get("x").is_none_or(HashSet::is_empty),
        "plain value read must record no trait, got {:?}",
        traits.get("x"),
    );
}

#[test]
fn command_substitution_geometry_keeps_quoted_compound_and_index_bodies() {
    for body in [
        "return \"[set $v]\"",
        "return before[set $v]after",
        "return $array([set $v])",
        "return [list [set $v]]",
    ] {
        let traits = infer(&["v"], body);
        assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "v", ProcArgTrait::VarRead);
    }
    for body in ["return {[set $v]}", "return \\[set\\ $v\\]"] {
        let traits = infer(&["v"], body);
        assert!(traits.get("v").is_none_or(HashSet::is_empty), "{body}");
    }
}

#[test]
fn command_substitution_geometry_preserves_index_and_root_alias_order() {
    let traits = infer(&["v"], "return \"$array([set alias $v])[set $alias]\"");
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "v", ProcArgTrait::VarRead);
    let reversed = infer(&["v"], "return \"[set $alias]$array([set alias $v])\"");
    assert!(reversed.get("v").is_none_or(HashSet::is_empty));
}

#[test]
fn braced_var_name_has_no_substitution() {
    // `set {arr($k)} 1` — the braces make `arr($k)` a *literal* variable
    // name, so `$k` is not a substitution and must not mark `k` (the
    // segmenter drops the braces, so the guard is by token kind).
    assert!(!infer(&["k"], "set {arr($k)} 1").contains_key("k"));
    assert!(!infer(&["p"], "set {$p} 1").contains_key("p"));
    // The unbraced form *does* substitute, so it still marks the component.
    assert_trait(
        &infer(&["k"], "set arr($k) 1"),
        "k",
        ProcArgTrait::DynamicNameLocal,
    );
}

#[test]
fn compound_dynamic_array_name_marks_components() {
    // `set ${v}($k) 1` — both the array-name param `v` and the key param
    // `k` flow into a dynamic variable name, so both are DynamicNameLocal
    // (+ VarRead).  This is the shape behind `return [set ${v}($k)]`.
    for body in ["set ${v}($k) 1", "set $v($k) 1", "return [set ${v}($k)]"] {
        let traits = infer(&["v", "k"], body);
        for p in ["v", "k"] {
            assert_trait(&traits, p, ProcArgTrait::DynamicNameLocal);
            assert_trait(&traits, p, ProcArgTrait::VarRead);
        }
    }
    // A literal array name with a `$k` key marks only the key.
    let traits = infer(&["k"], "set arr($k) 1");
    assert_trait(&traits, "k", ProcArgTrait::DynamicNameLocal);
}

#[test]
fn var_substitutions_finds_compound_components() {
    let parameters = ["v", "k", "escaped", "1backref"];
    let components = |source| {
        lexical_reference_values(source, &parameters, LexerConfig::default())
            .components
            .remove(1)
    };
    assert_eq!(components("$v"), vec!["v"]);
    assert_eq!(components("${v}($k)"), vec!["v", "k"]);
    assert_eq!(components("$v($k)"), vec!["v", "k"]);
    assert_eq!(components("arr($k)"), vec!["k"]);
    assert_eq!(components("literal"), Vec::<&str>::new());
    assert_eq!(components("\\$escaped"), Vec::<&str>::new());
    assert_eq!(components("$1backref"), vec!["1backref"]);
}

#[test]
fn original_parameter_reference_parts_keep_configured_names_and_literal_indices() {
    let parameters = [
        "1abc", "", "odd name", "caf", "café", "a{b", "a{b}c", "v", "k", "arr",
    ];
    for (dialect, unicode, nested) in [
        ("tcl8.6", false, false),
        ("tcl9.1", false, true),
        ("jim", true, false),
    ] {
        let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
            .expect("selected original reference grammar environment")
            .unit_profile();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let values = |word| lexical_reference_values(word, &parameters, config);
        for (word, expected) in [
            ("$1abc", Some("1abc")),
            ("${}", Some("")),
            ("${odd name}", Some("odd name")),
            ("\"$1abc\"", Some("1abc")),
            ("{$1abc}", None),
            ("\\$1abc", None),
        ] {
            assert_eq!(values(word).argument(0), expected, "{dialect}/{word}");
        }
        assert_eq!(values("$café").argument(0), unicode.then_some("café"));
        assert_eq!(
            values("$café").argument_components(0),
            if unicode {
                &["café"][..]
            } else {
                &["caf"][..]
            }
        );
        assert_eq!(values("${a{b}c}").argument(0), nested.then_some("a{b}c"));
        assert_eq!(
            values("${a{b}c}").argument_components(0),
            if nested { &["a{b}c"][..] } else { &["a{b"][..] }
        );
        assert_eq!(values("$arr($k)").argument_components(0), &["arr", "k"]);
        assert_eq!(values("${arr($k)}").argument_components(0), [] as [&str; 0]);
        assert_eq!(
            values("[set $café]").argument_components(0),
            if unicode {
                &["café"][..]
            } else {
                &["caf"][..]
            },
        );
        assert_eq!(
            values("[set ${a{b}c}]").argument_components(0),
            if nested { &["a{b}c"][..] } else { &["a{b"][..] },
        );
        assert!(values("${arr($k)}").argument(0).is_none());
        assert_eq!(values("{$arr($k)}").argument_components(0), [] as [&str; 0]);
    }
}

#[test]
fn original_parameter_reference_components_keep_lexical_scalar_names() {
    let parameters = ["p", "p(key)", "p($k)", "k", "after"];
    for dialect in ["tcl8.6", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
            .unwrap()
            .unit_profile();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        for (source, expected) in [
            ("${p(key)}", vec!["p(key)"]),
            ("${p($k)}", vec!["p($k)"]),
            ("$p(key)", vec!["p"]),
            ("$p($k)", vec!["p", "k"]),
            ("prefix-${p(key)}", vec!["p(key)"]),
            ("[set ${p(key)}]", vec!["p(key)"]),
            ("[set ${p($k)}]", vec!["p($k)"]),
            ("[set $p($k)]", vec!["p", "k"]),
            ("$p([set ${p(key)}])", vec!["p", "p(key)"]),
            ("[set ${p(key)}]$after", vec!["p(key)", "after"]),
            ("[set [set [set ${p(key)}]]]", vec!["p(key)"]),
            ("[set {${p(key)}}]", vec![]),
            ("{${p(key)}}", vec![]),
            ("\\${p(key)}", vec![]),
        ] {
            let values = lexical_reference_values(source, &parameters, config);
            assert_eq!(
                values.argument_components(0),
                expected,
                "{dialect}/{source}"
            );
        }
        assert!(
            lexical_reference_values("${p(key)}", &parameters, config)
                .argument(0)
                .is_none()
        );
    }
}

#[test]
fn original_parameter_scalar_components_do_not_borrow_an_array_root_trait() {
    for deep in [false, true] {
        for body in [
            "set ${p(key)} 1",
            "set target [set ${p(key)}]",
            "set target [set $p([set ${p(key)}])]",
        ] {
            let traits = infer_owned(&["p", "p(key)"], body, deep);
            assert_trait(&traits, "p(key)", ProcArgTrait::DynamicNameLocal);
            if !body.contains("$p(") {
                assert!(!traits.contains_key("p"), "{deep}/{body}: {traits:?}");
            }
        }
        let traits = infer_owned(&["p", "k"], "set $p($k) 1", deep);
        for parameter in ["p", "k"] {
            assert_trait(&traits, parameter, ProcArgTrait::DynamicNameLocal);
        }
    }
}

#[test]
fn original_parameter_reference_parts_refuse_changed_segment_presentations() {
    let config = LexerConfig::default();
    let registry = CommandRegistry::build_default();
    let original = "command $old";
    let commands = segment_commands_with_offset_and_config(original, 0, config);
    let parameters = HashSet::from(["old", "new"]);
    let aliases = Aliases::default();
    let valid = ArgumentValues::lexical(
        original,
        &commands[0],
        &parameters,
        &aliases,
        config,
        &registry,
    );
    assert_eq!(valid.argument(0), Some("old"));
    let changed = ArgumentValues::lexical(
        "command $new",
        &commands[0],
        &parameters,
        &aliases,
        config,
        &registry,
    );
    assert!(changed.argument(0).is_none());
    assert_eq!(changed.argument_components(0), [] as [&str; 0]);
    let literal = lexical_reference_values("{$old}", &["old"], config);
    assert!(literal.argument(0).is_none());
    assert_eq!(literal.argument_components(0), [] as [&str; 0]);
}

#[test]
fn value_copy_carries_param_into_name_and_command_positions() {
    // `set n $p` copies the param's value into local `n`; a later `$n` in a
    // name / command position then resolves back to the param.
    assert_trait(
        &infer(&["v"], "set n $v\nset $n 1"),
        "v",
        ProcArgTrait::DynamicNameLocal,
    );
    assert_trait(
        &infer(&["cmd"], "set c $cmd\n$c arg"),
        "cmd",
        ProcArgTrait::Command,
    );
    // In a command substitution too.
    assert_trait(
        &infer(&["v"], "set n $v\nreturn [set $n]"),
        "v",
        ProcArgTrait::DynamicNameLocal,
    );
    // Transitive copies chain.
    assert_trait(
        &infer(&["cmd"], "set c $cmd\nset d $c\n$d arg"),
        "cmd",
        ProcArgTrait::Command,
    );
}

#[test]
fn value_copy_is_invalidated_and_not_over_eager() {
    // Reassigning the local to a non-param drops the copy, so a later `$n`
    // no longer resolves to the original param.
    assert!(!infer(&["v"], "set n $v\nset n other\nset $n 1").contains_key("v"));
    // Merely passing the value through (never used as a name) is not a role.
    assert!(!infer(&["v"], "set n $v\nreturn $n").contains_key("v"));
}

#[test]
fn accepted_copies_feed_eval_without_surviving_unknown_or_mutated_values() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    assert_trait(
        &infer(&["body"], "set local $body; eval $local"),
        "body",
        ProcArgTrait::Eval,
    );
    for body in [
        "set local $body; set local ordinary; eval $local",
        "set local $body; set ::auto_index(unknown_callback) {source lib.tcl}; unknown_callback; eval $local",
        "set local $body; eval {$local}",
    ] {
        assert!(
            !infer(&["body"], body)
                .get("body")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval)),
            "{body}"
        );
    }
}

#[test]
fn referenced_upvar_contents_are_not_the_parameter_value_that_named_them() {
    let traits = infer(
        &["name"],
        "upvar 1 $name contents; set contents SCRIPT; set copy $contents; eval $copy",
    );
    assert_trait(&traits, "name", ProcArgTrait::VarWrite);
    assert!(
        !traits["name"].contains(&ProcArgTrait::Eval),
        "the caller cell's contents are distinct from its parameter-supplied name"
    );
}

#[test]
fn param_used_as_command_head_is_command() {
    // `$cmd arg1 arg2` — the param's value is the command word, so it names
    // a command.  Works at the top level and inside a `[...]` substitution.
    assert_trait(&infer(&["cmd"], "$cmd a b"), "cmd", ProcArgTrait::Command);
    assert_trait(
        &infer(&["cmd"], "return [$cmd x]"),
        "cmd",
        ProcArgTrait::Command,
    );
    // A braced `{$cmd}` head is a *literal* command name — no substitution.
    assert!(!infer(&["cmd"], "{$cmd} arg").contains_key("cmd"));
    // A param merely read as a value is not a command.
    assert!(!infer(&["x"], "puts $x").contains_key("x"));
}

#[test]
fn stub_command_prefix_role_is_command() {
    // A stub-declared `command_prefix` argument names a command, so a
    // `$param` there is inferred `Command`.
    let declared = make_declarations(vec![stub_sig(
        "on_event",
        &[
            ("event", ArgRole::Value),
            ("handler", ArgRole::CommandPrefix),
        ],
    )]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["h"],
        "on_event click $h",
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    assert_trait(&traits, "h", ProcArgTrait::Command);
}

#[test]
fn namespace_upvar_dynamic_name_reads_other_var() {
    // `namespace upvar ns $token arr` — the token/state-array idiom, with a
    // dynamic `$token` naming the namespace variable aliased into `arr`.
    // The pairs start after the `namespace` word (index 2 of the
    // sub-command args), so `$token` is the *other* var → VarRead, and a
    // later write through the `arr` alias upgrades it to VarWrite.
    let traits = infer(&["token"], "namespace upvar ns $token arr");
    assert_trait(&traits, "token", ProcArgTrait::VarRead);
    let traits = infer(&["token"], "namespace upvar ns $token arr\nset arr 1");
    assert_trait(&traits, "token", ProcArgTrait::VarWrite);
}

#[test]
fn real_world_by_reference_patterns_are_inferred() {
    // The dominant variable-name-by-reference shapes mined from the tcllib /
    // Tcl-library corpus (`http`, `smtp`, `ftp`, … token idiom).  Each names
    // a variable via a parameter, so each must record a read/write trait.
    let has_var = |body: &str, p: &str| {
        let t = infer(&[p], body);
        t.get(p).is_some_and(|s| {
            s.contains(&ProcArgTrait::VarRead) || s.contains(&ProcArgTrait::VarWrite)
        })
    };
    for (body, p) in [
        ("upvar 1 $varname data", "varname"),
        ("upvar 0 $token state", "token"),
        ("upvar #0 $token state", "token"),
        ("upvar $v local", "v"),
        ("variable $name", "name"),
        ("array set $a {x 1}", "a"),
        ("array get $a", "a"),
        ("array names $a", "a"),
        ("unset $v", "v"),
        ("dict set $d k 1", "d"),
        ("incr $counter", "counter"),
        ("append $resultVar x", "resultVar"),
        ("lappend $pages_var p", "pages_var"),
        ("set ${tok}(state) connected", "tok"),
        ("set $gas(idx) 1", "gas"),
        ("namespace upvar ns $v local", "v"),
        ("foreach k [array names $a] { puts $k }", "a"),
        ("return [set ${token}($field)]", "token"),
    ] {
        assert!(
            has_var(body, p),
            "expected a by-reference var trait for `{p}` in `{body}`; got {:?}",
            infer(&[p], body).get(p),
        );
    }
}

#[test]
fn after_records_eval_skipping_cancel_info() {
    let traits = infer(&["body"], "after 100 $body");
    assert_trait(&traits, "body", ProcArgTrait::Eval);
    let traits = infer(&["x"], "after cancel $x");
    // ``after cancel`` doesn't take a script, so $x is not eval.
    assert!(
        traits
            .get("x")
            .is_none_or(|s| !s.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn regsub_records_dynamic_name_local() {
    // `regsub`'s output var is CALLEE-LOCAL — DynamicNameLocal
    // (+ VarRead), not VarWrite.
    let traits = infer(&["line", "out"], "regsub -all foo $line bar $out");
    assert_trait(&traits, "out", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "out", ProcArgTrait::VarRead);
    assert!(
        !traits.get("out").unwrap().contains(&ProcArgTrait::VarWrite),
        "callee-local regsub target must not be VarWrite, got {:?}",
        traits.get("out"),
    );
}

#[test]
fn regexp_capture_vars_after_switches_record_dynamic_name_local() {
    // `regexp -nocase -- $pat $s $m` — the registry `regexp` spec's
    // arg-role resolver performs the switch skip (`--` terminator
    // included), so the capture var `$m` names a callee-local output
    // variable.  The pattern / subject args are plain value reads and
    // must record no trait.
    let traits = infer(&["pat", "s", "m"], "regexp -nocase -- $pat $s $m");
    assert_trait(&traits, "m", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "m", ProcArgTrait::VarRead);
    assert!(!traits.contains_key("pat"), "{traits:?}");
    assert!(!traits.contains_key("s"), "{traits:?}");
}

#[test]
fn regexp_start_switch_value_is_skipped() {
    // `-start` consumes its value word (spec-declared), so the capture
    // var still resolves after the pattern + string positionals.
    let traits = infer(&["s", "m"], "regexp -start 2 {x+} $s $m");
    assert_trait(&traits, "m", ProcArgTrait::DynamicNameLocal);
}

#[test]
fn regsub_command_switch_is_skipped_to_output_var() {
    // Tcl 9 `regsub -command exp string cmdPrefix varName` — `-command`
    // is a spec-declared flag (one a hardcoded switch list misses), so
    // the output var must still resolve at exp + 3.
    let traits = infer(&["s", "out"], "regsub -command {x+} $s myCb $out");
    assert_trait(&traits, "out", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "out", ProcArgTrait::VarRead);
}

#[test]
fn set_dynamic_name_records_dynamic_name_local_not_var_write() {
    // `set $p 1` — the param's VALUE names a callee-local variable
    // (registry `VarWrite` role on the `$p` substitution).  The
    // refined trait is DynamicNameLocal (+ VarRead), NOT VarWrite:
    // a caller passing a literal name (`f x`) does not have its `x`
    // consumed by this callee.
    let traits = infer(&["p"], "set $p 1");
    assert_trait(&traits, "p", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "p", ProcArgTrait::VarRead);
    assert!(
        !traits.get("p").unwrap().contains(&ProcArgTrait::VarWrite),
        "callee-local `set $p` must not be VarWrite, got {:?}",
        traits.get("p"),
    );
}

#[test]
fn genuine_upvar_write_stays_var_write_not_dynamic_name_local() {
    // A real caller-frame write-back through an `upvar` alias is a
    // genuine `VarWrite` — the DynamicNameLocal refinement must NOT
    // leak onto it (otherwise caller-side suppression of a genuine
    // call-by-name write would regress).
    let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
    assert_trait(&traits, "var", ProcArgTrait::VarWrite);
    assert!(
        !traits
            .get("var")
            .unwrap()
            .contains(&ProcArgTrait::DynamicNameLocal),
        "upvar write-back must not be DynamicNameLocal, got {:?}",
        traits.get("var"),
    );
}

#[test]
fn empty_body_returns_empty_map() {
    let traits = infer(&["a"], "");
    assert!(traits.is_empty());
}
