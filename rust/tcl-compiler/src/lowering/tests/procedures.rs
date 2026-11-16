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

//! `proc` declarations: defining namespaces, formal parameters, qualified names and dynamic names and bodies.

use super::*;

#[test]
fn original_name_operand_publishes_a_body_without_borrowing_the_head_lookup() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    let source = r#"proc f {p} {set b [set {$p}]; set a 1; set a 2; return "$a$b"}"#;
    let module = lower_to_ir_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(profile),
    );
    let procedures = module.procedures.iter().collect::<Vec<_>>();
    assert_eq!(procedures.len(), 1);
    let (id, procedure) = procedures[0];
    assert!(module.procedure_ids.owns(*id));
    assert_eq!(procedure.qualified_name, "::f");
    let tokens = module
        .top_level
        .statements
        .iter()
        .find_map(Statement::tokens)
        .expect("the actual original declaration command");
    let binding = tokens
        .source_binding
        .as_ref()
        .expect("original source binding");
    let input = binding
        .original_written_name_input(tokens, 1)
        .expect("actual original name operand");
    assert!(
        binding.original_command_lookup(tokens, &input).is_none(),
        "an operand cannot replace the original command-head owner"
    );
    assert!(module.source_entry.native_entry.is_none());
    assert_eq!(
        procedure.declaration_site.as_ref(),
        binding.invocation_site()
    );
    let head = binding
        .original_written_name_input(tokens, 0)
        .expect("actual original declaration head");
    assert_eq!(head.bytes(), b"proc");
    assert_eq!(input.bytes(), b"f");
    assert_ne!(
        head.original_word_key().unwrap().original_word(),
        input.original_word_key().unwrap().original_word()
    );
    let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key) = &input else {
        panic!("original literal name word");
    };
    assert_eq!(
        key.policy().authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    if let Some(scope) = binding.original_operand_naming_scope(&input) {
        assert_eq!(
            procedure.publication.as_ref(),
            crate::signature_scan::scope::SignatureSourceCommand::procedure_from_key(&scope, key)
                .as_ref()
        );
    } else {
        assert!(procedure.publication.is_none());
    }
}

#[test]
fn proc_body_namespace_peels_the_names_own_qualifier() {
    // Unit-level: the helper is the inverse of `qualify_proc_name`.
    assert_eq!(proc_body_namespace("::a::outer", "::"), "::a");
    assert_eq!(proc_body_namespace("::b::c::o2", "::"), "::b::c");
    assert_eq!(proc_body_namespace("::plain", "::ignored"), "::");
    // A bare (unrooted) key can't come out of `qualify_proc_name`, but the
    // fallback must not manufacture an empty namespace if it ever did.
    assert_eq!(proc_body_namespace("plain", "::lex"), "::lex");
}

#[test]
fn nested_proc_homes_to_the_outer_procs_defining_namespace() {
    // TP. On tclsh 9.0.4 and 8.6.16, identically:
    //   namespace eval ::a {}
    //   proc a::outer {} { proc helper {x} {return $x} }
    //   a::outer ; info commands ::a::helper   -> ::a::helper
    //              info commands ::helper      -> {}
    let m = lower_to_ir(
        "namespace eval ::a {}\nproc a::outer {} { proc helper {x} { return $x } }\n",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::a::helper").is_some(),
        "nested proc must home to ::a, got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::helper").is_none(),
        "nested proc must NOT home lexically to ::",
    );
}

#[test]
fn nested_proc_homing_composes_through_three_levels() {
    // TP. On tclsh, `proc g::o7 {} { proc o7b {} { proc h7 {u} … } }` →
    // running `g::o7` then `::g::o7b` leaves `::g::h7` (never `::h7`).
    let m = lower_to_ir(
        "namespace eval ::g {}\nproc g::o7 {} { proc o7b {} { proc h7 {u} { return $u } } }\n",
        &reg(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::g::o7b").is_some());
    assert!(
        fixture_id_if_present(&m.procedures, "::g::h7").is_some(),
        "third level must stay in ::g, got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::h7").is_none());
}

#[test]
fn nested_proc_name_qualifier_beats_the_lexical_namespace_eval() {
    // TP. On tclsh, inside `namespace eval ::d`, `proc ::e::o3` still homes
    // its body to `::e` — `::e::h3` exists, `::d::h3` and `::h3` do not.
    let m = lower_to_ir(
        "namespace eval ::e {}\nnamespace eval ::d {\n    proc ::e::o3 {} { proc h3 {z} { return $z } }\n}\n",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::e::h3").is_some(),
        "got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::d::h3").is_none());
    assert!(fixture_id_if_present(&m.procedures, "::h3").is_none());
}

#[test]
fn unqualified_outer_proc_leaves_nested_homing_unchanged() {
    // TN. The common shape: lexical and defining namespace agree, so
    // homing changes nothing. Both at top level…
    let m = lower_to_ir("proc outer {} { proc inner {} { return 1 } }\n", &reg());
    assert!(fixture_id_if_present(&m.procedures, "::inner").is_some());
    // …and inside a `namespace eval`, where both are `::ns`.
    let m = lower_to_ir(
        "namespace eval ::ns {\n    proc outer {} { proc inner {} { return 1 } }\n}\n",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::ns::inner").is_some(),
        "got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn absolutely_named_nested_proc_is_unaffected() {
    // TN. On tclsh, `proc a::outer4 {} { proc ::abs {q} … }` creates `::abs`
    // — an absolute inner name ignores the enclosing frame entirely.
    let m = lower_to_ir(
        "namespace eval ::a {}\nproc a::outer4 {} { proc ::abs {q} { return $q } }\n",
        &reg(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::abs").is_some());
    assert!(fixture_id_if_present(&m.procedures, "::a::abs").is_none());
}

#[test]
fn original_formal_name_headers_use_selected_grammar_and_keep_activation_separate() {
    // naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    let tcl = Some(tcl_dialect::ParameterGrammar::Tcl);
    let jim = Some(tcl_dialect::ParameterGrammar::Jim);
    assert_eq!(parse_formal_param_names("{a::b}", FOLD_RULES, tcl), None);
    assert_eq!(
        parse_formal_param_names("{a::b}", LITERAL_RULES, jim),
        Some(vec!["a::b".to_owned()])
    );
    assert_eq!(parse_formal_param_names("{a(1)}", FOLD_RULES, tcl), None);
    assert_eq!(
        parse_formal_param_names("{a(1)}", LITERAL_RULES, jim),
        Some(vec!["a(1)".to_owned()])
    );
    // Name-only headers retain original link spelling. They cannot claim
    // the ordinary local storage name or manufacture a caller-link frame.
    assert_eq!(
        parse_formal_param_names("&a {&b default}", LITERAL_RULES, jim),
        Some(vec!["&a".to_owned(), "&b".to_owned()])
    );
    assert_eq!(
        parse_formal_param_names("args args", LITERAL_RULES, jim),
        None
    );
    assert_eq!(
        parse_formal_param_names("args args", FOLD_RULES, tcl),
        Some(vec!["args".to_owned(), "args".to_owned()])
    );
    assert_eq!(parse_formal_param_names("a b", FOLD_RULES, None), None);
}

#[test]
fn formal_param_names_take_the_specifier_name_only() {
    assert_eq!(
        parse_formal_param_names(
            "a b c",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["a", "b", "c"],
    );
    assert_eq!(
        parse_formal_param_names(
            "{x default} y",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["x", "y"]
    );
    assert_eq!(
        parse_formal_param_names("", FOLD_RULES, Some(tcl_dialect::ParameterGrammar::Tcl)).unwrap(),
        [] as [std::string::String; 0]
    );
    // A wrapped parameter list collapses its continuation before splitting.
    assert_eq!(
        parse_formal_param_names(
            "a b\\\n    c",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn formal_param_names_reject_what_tcl_rejects() {
    // Tcl refuses to create these procedures, so lowering declines too.
    assert_eq!(
        parse_formal_param_names(
            "{a b c}",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        ),
        None
    );
    assert_eq!(
        parse_formal_param_names(
            "{a::b}",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        ),
        None
    );
    assert_eq!(
        parse_formal_param_names(
            "{a(1)}",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        ),
        None
    );
    assert_eq!(
        parse_formal_param_names("{a", FOLD_RULES, Some(tcl_dialect::ParameterGrammar::Tcl)),
        None
    );
}

#[test]
fn var_list_names_keep_grouped_elements_whole() {
    assert_eq!(
        parse_var_list_names("a b", FOLD_RULES).unwrap(),
        vec!["a", "b"]
    );
    // tclsh 9.0: both spellings bind the one variable named `a b`.
    assert_eq!(
        parse_var_list_names("{a b}", FOLD_RULES).unwrap(),
        vec!["a b"]
    );
    assert_eq!(
        parse_var_list_names("a\\ b", FOLD_RULES).unwrap(),
        vec!["a b"]
    );
    assert_eq!(
        parse_var_list_names("", FOLD_RULES).unwrap(),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        parse_var_list_names("a b\\\n    c", FOLD_RULES).unwrap(),
        vec!["a", "b", "c"]
    );
    // Unbalanced — no binding can be named, so the caller barriers.
    assert_eq!(parse_var_list_names("{a", FOLD_RULES), None);
}

// A formal-parameter list has two list levels, so a grouped or escaped
// specifier is one parameter, not one name per whitespace run:
// `proc p {a\ b} {}` has arity one.

/// Parameter names recorded for `::p` when `src` is lowered.
fn proc_params(src: &str) -> Vec<String> {
    let m = lower_to_ir(src, &reg());
    fixture_id_if_present(&m.procedures, "::p")
        .and_then(|id| m.procedures.get(&id))
        .unwrap_or_else(|| panic!("::p registered for {src:?}"))
        .params
        .clone()
}

#[test]
fn escaped_space_proc_param_is_one_parameter() {
    // tclsh 9.0: `proc p {a\ b} {}` → `info args p` is `a`, with
    // `info default p a` yielding `b`. Only the name reaches the IR, so
    // the arity is one, not two (`a\` and `b`).
    assert_eq!(proc_params("proc p {a\\ b} {}"), vec!["a"]);
    // The grouped spelling decodes identically.
    assert_eq!(proc_params("proc p {{a b}} {}"), vec!["a"]);
}

#[test]
fn proc_params_keep_defaults_and_args_as_names() {
    // FP-guard: `{x {y 2} args}` is still the three parameters `x`, `y`,
    // and the variadic `args`.
    assert_eq!(
        proc_params("proc p {x {y 2} args} {}"),
        vec!["x", "y", "args"]
    );
    assert_eq!(proc_params("proc p {} {}"), [] as [std::string::String; 0]);
}

#[test]
fn proc_params_tcl_rejects_fall_through_to_barrier() {
    // tclsh: `proc p {{a b c}} {}` → "too many fields in argument
    // specifier". No procedure is created, so lowering registers none and
    // leaves the runtime `proc` to report it.
    for src in [
        "proc p {{a b c}} {}",
        "proc p {a::b} {}",
        "proc p {a(1)} {}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            fixture_id_if_present(&m.procedures, "::p").is_none(),
            "no ::p for {src:?}: {:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
        assert!(
            matches!(&m.top_level.statements[0], Statement::Barrier { reason, .. } if reason == "malformed proc params"),
            "expected a malformed-params barrier for {src:?}, got {:?}",
            m.top_level.statements[0],
        );
    }
}

#[test]
fn qualify_proc_name_global() {
    assert_eq!(qualify_proc_name("::", "foo"), "::foo");
}

#[test]
fn qualify_proc_name_nested() {
    assert_eq!(qualify_proc_name("::ns", "bar"), "::ns::bar");
}

#[test]
fn qualify_proc_name_already_qualified() {
    assert_eq!(qualify_proc_name("::ns", "::abs"), "::abs");
}

#[test]
fn qualify_proc_name_preserves_colon_named_namespace_keys() {
    assert_eq!(qualify_proc_name(":::", ":"), "::::::");
    assert_eq!(qualify_proc_name("::a", "a:::b"), "::a::a::b");
    assert_eq!(tcl_syntax::naming::key_holder_and_tail("::::::").0, ":::");
}

#[test]
fn proc_dynamic_name_resolved_via_const_map() {
    // ``set name {Verbose}; proc \$name {} { ... }``
    // inside a proc — ``name`` is in the const-map, so the
    // inner proc registers as ``::Verbose``.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n proc $name {} { puts hi } }",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::Verbose").is_some(),
        "expected ::Verbose in {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn proc_dynamic_name_no_binding_stays_barrier() {
    // ``proc \$name`` with no const-map entry — bail to barrier.
    let m = lower_to_ir("proc factory {} { proc $name {} { puts hi } }", &reg());
    let factory = fixture_id_if_present(&m.procedures, "::factory")
        .and_then(|id| m.procedures.get(&id))
        .expect("factory registered");
    let last = factory.body.statements.last().expect("body");
    assert!(
        matches!(last, Statement::Barrier { .. }),
        "expected Barrier, got {last:?}"
    );
}

#[test]
fn proc_with_trailing_colons_routes_through_barrier() {
    // Tcl 9 ``proc ::ns:: {args} {body}`` registers a command
    // named ``""`` inside ``::ns``.  Static normalisation would
    // strip the trailing ``::`` and register the proc under the
    // namespace name itself — wrong cmd.  Route through
    // Statement::Barrier so the runtime ``proc`` builtin handles
    // the registration.
    let m = lower_to_ir("proc ::ns:: {a b} { puts $a$b }", &reg());
    // No procedure entry should have been AOT-registered under
    // ``::ns`` (which is what the bug would produce) or any
    // empty-key variant.
    assert!(
        fixture_id_if_present(&m.procedures, "::ns").is_none()
            && fixture_id_if_present(&m.procedures, "::ns::").is_none()
            && fixture_id_if_present(&m.procedures, "").is_none(),
        "trailing-:: proc should not be AOT-registered; got {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    // The top-level statement should be a Barrier with the
    // empty-simple-name reason.
    let top = m
        .top_level
        .statements
        .last()
        .expect("at least one top-level statement");
    match top {
        Statement::Barrier {
            reason, command, ..
        } => {
            assert_eq!(command, "proc");
            assert!(
                reason.contains("empty-simple-name"),
                "unexpected barrier reason: {reason}"
            );
        }
        other => panic!("expected Statement::Barrier, got {other:?}"),
    }
}

#[test]
fn proc_with_command_substitution_in_name_stays_barrier() {
    // ``proc \$name[suffix]`` — multi-token name with a command
    // substitution. The const-map gate only covers single-VAR
    // tokens; this stays on the runtime path.
    let m = lower_to_ir(
        "proc factory {} { set name {x}\n proc $name[suffix] {} { puts hi } }",
        &reg(),
    );
    // ``::factory`` registered; the inner proc must NOT be
    // registered under any literal name (``::x...``).
    assert!(
        !m.procedures
            .values()
            .any(|body| body.qualified_name.contains("suffix"))
    );
}

#[test]
fn proc_subst_nocommands_body_materialised() {
    // ``proc \$name {x} [subst -nocommands {return \$default}]``
    // with both ``name`` and ``default`` const-tracked materialises
    // the body to ``return 0`` and lowers it as a real script.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nocommands {return $default}] }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Verbose")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Verbose registered");
    // Body should contain a Return statement (or at least be
    // non-empty — the materialised body lowers to a real
    // statement, not a Barrier).
    assert!(
        !inner.body.statements.is_empty(),
        "expected lowered body, got empty"
    );
}

#[test]
fn proc_var_body_const_map_materialised() {
    // A bare `$body` body word whose value
    // is in the const-map materialises into the Procedure body
    // (matching the `proc $name {x} $body` shape).
    let m = lower_to_ir(
        "proc factory {} { set name {Greeter}\n set body {return hi}\n proc $name {} $body }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Greeter")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Greeter registered");
    // The body was const-resolved to "return hi"; lowering it
    // produces a Return statement.
    assert!(
        !inner.body.statements.is_empty(),
        "expected lowered body, got empty"
    );
}

#[test]
fn proc_var_body_unresolved_keeps_empty_body() {
    // `$body` has no const-map binding but `$name` does — the
    // proc name resolves, but the body stays empty (the runtime
    // proc call below carries the dynamic body text).
    let m = lower_to_ir(
        "proc factory {} { set name {Greeter}\n proc $name {} $body }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Greeter")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Greeter registered");
    assert_eq!(
        inner.body.statements.len(),
        0,
        "expected empty body for unresolved dynamic body, got {:?}",
        inner.body.statements
    );
}

#[test]
fn proc_dynamic_params_routes_through_barrier() {
    // A dynamic params word — there's no static arg-list to
    // build a real Procedure from, so the whole proc shape
    // bails to a Barrier.  Even when the name resolves via the
    // const-map.
    let m = lower_to_ir(
        "proc factory {} { set name {x}\n proc $name $params { puts hi } }",
        &reg(),
    );
    // No inner procedure should be registered — params couldn't
    // be parsed statically.
    assert!(
        fixture_id_if_present(&m.procedures, "::x").is_none(),
        "dynamic-params shape must not AOT-register: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn proc_multi_token_body_word_is_dynamic() {
    // `proc foo {} "pre${body}post"` — the body is a quoted
    // multi-token word.  Even though the first token is Str,
    // single_token_word is false so the whole word is dynamic
    // and we route through Barrier (the multi-token guard).
    let m = lower_to_ir(
        r#"proc factory {} { set body {return hi}; proc inner {} "pre${body}post" }"#,
        &reg(),
    );
    // `::inner` would only be registered if the body was
    // materialised — which can't happen for multi-token words.
    // We allow either no entry or an empty-body entry; the key
    // assertion is that the literal source text isn't compiled
    // as a script.
    if let Some(inner) =
        fixture_id_if_present(&m.procedures, "::inner").and_then(|id| m.procedures.get(&id))
    {
        assert_eq!(
            inner.body.statements.len(),
            0,
            "multi-token body must not be statically lowered, got {:?}",
            inner.body.statements
        );
    }
}

#[test]
fn proc_subst_nocommands_missing_var_skips_materialisation() {
    // ``\$default`` is not in the const-map — the materialiser
    // refuses, leaving the body to fall back to runtime
    // dispatch.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n proc $name {x} [subst -nocommands {return $default}] }",
        &reg(),
    );
    // Verbose not registered (because \$default missing means
    // we keep the dynamic body which routes via Barrier — but
    // the proc name itself was substituted). Actually
    // the proc IS registered with whatever the body lowering
    // produces; the assertion is that it's NOT the materialised
    // form.
    let inner =
        fixture_id_if_present(&m.procedures, "::Verbose").and_then(|id| m.procedures.get(&id));
    if let Some(p) = inner {
        // The body should not contain a Return whose value is
        // the literal "0" — that would be the materialised
        // form. It can be empty / contain a Barrier.
        // Conservative check: just verify the helper refused.
        // (The body might still lower the original CMD-token
        // text as a runtime call.)
        assert!(
            p.body.statements.is_empty()
                || matches!(
                    p.body.statements[0],
                    Statement::Call { .. } | Statement::Barrier { .. }
                )
        );
    }
}

/// Tcl 9.1's positive family names the same effect set `-nocommands`
/// leaves — variables and backslashes on, commands off — so the body
/// materialises the same way.
///
/// tclsh 9.1b0, with `name` set to `world`:
/// `subst -variables -backslashes {hello $name\n[format X]}` → `hello
/// world`, a newline, then the untouched `[format X]`.
#[test]
fn proc_subst_tcl91_positive_switches_materialised() {
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -variables -backslashes {return $default}] }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Verbose")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Verbose registered");
    assert!(
        !inner.body.statements.is_empty(),
        "expected lowered body, got empty"
    );
    assert!(
        !matches!(inner.body.statements[0], Statement::Barrier { .. }),
        "expected a materialised body, got {:?}",
        inner.body.statements
    );
}

/// A computed switch word leaves the call unreadable, so the registry
/// answers every kind and the materialiser refuses.
#[test]
fn proc_subst_computed_switch_word_refused() {
    let m = lower_to_ir(
        "proc factory {opt} { set name {Verbose}\n set default {0}\n proc $name {x} [subst $opt {return $default}] }",
        &reg(),
    );
    if let Some(p) =
        fixture_id_if_present(&m.procedures, "::Verbose").and_then(|id| m.procedures.get(&id))
    {
        assert!(
            p.body.statements.is_empty()
                || matches!(
                    p.body.statements[0],
                    Statement::Call { .. } | Statement::Barrier { .. }
                ),
            "expected no materialised body, got {:?}",
            p.body.statements
        );
    }
}

#[test]
fn proc_subst_nocommands_nobackslashes_refused() {
    // ``-nobackslashes`` flag — semantics differ from our
    // evaluator's default. Refuse and fall through.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nobackslashes -nocommands {return $default}] }",
        &reg(),
    );
    let inner =
        fixture_id_if_present(&m.procedures, "::Verbose").and_then(|id| m.procedures.get(&id));
    if let Some(p) = inner {
        // Body did not materialise — should be empty or have
        // a fallback shape.
        assert!(
            p.body.statements.is_empty()
                || matches!(
                    p.body.statements[0],
                    Statement::Call { .. } | Statement::Barrier { .. }
                )
        );
    }
}

#[test]
fn proc_dynamic_name_picks_latest_set() {
    // Multiple ``set`` calls — the most recent literal wins.
    let m = lower_to_ir(
        "proc factory {} { set name {First}\n set name {Second}\n proc $name {} { puts hi } }",
        &reg(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::Second").is_some());
    assert!(fixture_id_if_present(&m.procedures, "::First").is_none());
}

// `proc`, `namespace eval`, `foreach`, `lmap`, `dict` dispatch
// through registry-driven hook-ID, with shape preconditions
// enforced inside each match arm.

#[test]
fn registry_dispatch_proc_lowers_to_proc_call_and_registers_procedure() {
    let m = lower_to_ir("proc greet {name} { puts hi }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "proc",
            "expected `proc` call via registry hook, got command={command}",
        ),
        other => panic!("expected Call via registry hook, got {other:?}"),
    }
    assert!(
        fixture_id_if_present(&m.procedures, "::greet").is_some(),
        "expected ::greet to be registered in module.procedures, got keys={:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn registry_dispatch_proc_wrong_arity_falls_through_to_default() {
    // `proc` with two args — fails the `args.len() == 3`
    // precondition.  The registry-driven path returns None,
    // falls through to the residual string match's
    // `_ => lower_default(...)` arm, which produces a
    // runtime `Call` rather than registering a procedure.
    let m = lower_to_ir("proc greet onlytwo", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Call { .. }),
        "expected Call via lower_default fallback, got {stmt:?}",
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::greet").is_none(),
        "expected ::greet NOT to be registered when arity is wrong",
    );
}
