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

//! Traits through declared stubs and resolved heads: document declarations, aliases, renames, shadowed built-ins and positioned bodies.

use super::*;

#[test]
fn declarations_shallow_surface_the_declared_body_role() {
    // `my_eval` isn't in the catalogue, but the document
    // declares its arg-0 as `body`.  An invocation
    // `my_eval $script` should therefore surface `Body` on
    // the `script` param.
    let declared = make_declarations(vec![stub_sig("my_eval", &[("script", ArgRole::Body)])]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["script"],
        "my_eval $script",
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    assert_trait(&traits, "script", ProcArgTrait::Body);
}

#[test]
fn declarations_shallow_surface_the_declared_var_write_role() {
    // A declared `VarWrite` on a bare `$param` substitution
    // surfaces the callee-local DynamicNameLocal refinement.
    let declared = make_declarations(vec![stub_sig(
        "with_var",
        &[("varName", ArgRole::VarWrite), ("value", ArgRole::Value)],
    )]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["v"],
        "with_var $v 42",
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    // A declared `VarWrite` role on a bare `$v` substitution is the same
    // callee-local dynamic-name shape as `set $v …` — refined to
    // DynamicNameLocal (+ VarRead), not a caller-frame VarWrite.
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "v", ProcArgTrait::VarRead);
}

#[test]
pub(super) fn declarations_deep_recurse_through_declared_body_args() {
    // The document declares `my_loop`'s arg-1 as a body.
    // Without the declaration the deep pass can't see that
    // `my_loop { uplevel 1 $body }` carries an Eval inside.
    // With it the pass descends into the brace and surfaces
    // the Eval.
    let declared = make_declarations(vec![stub_sig(
        "my_loop",
        &[("count", ArgRole::Value), ("body", ArgRole::Body)],
    )]);
    let registry = CommandRegistry::build_default();
    let body = "my_loop 5 { uplevel 1 $script }";
    // Sanity: without the declaration, the deep pass misses
    // the nested Eval because `my_loop` isn't recognised.
    let undeclared = infer_param_traits_deep(
        &["script"],
        body,
        env_for(&registry, LexerConfig::default()),
    );
    assert!(
        !undeclared
            .get("script")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "undeclared, my_loop's body shouldn't be recognised, got {undeclared:?}",
    );
    // With the declaration, the recursion fires.
    let with_declaration = infer_param_traits_deep(
        &["script"],
        body,
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    assert_trait(&with_declaration, "script", ProcArgTrait::Eval);
}

#[test]
fn declarations_do_not_disturb_catalogue_resolution() {
    // A declaration covering `my_thing` mustn't shadow any
    // catalogue command.  A shipped `foreach` invocation
    // still records its `LoopList` / `Body` traits through the
    // catalogue path even when an unrelated document
    // declaration is active.
    let declared = make_declarations(vec![stub_sig("my_thing", &[("a", ArgRole::Body)])]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["items", "body"],
        "foreach x $items $body",
        env_with_declarations(&registry, &declared),
    );
    assert_trait(&traits, "items", ProcArgTrait::LoopList);
    assert_trait(&traits, "body", ProcArgTrait::Body);
}

#[test]
fn no_declarations_matches_an_empty_declaration_set() {
    // An empty declaration set should produce the same result as
    // `None` — a document that declares nothing is exactly the
    // catalogue.
    let registry = CommandRegistry::build_default();
    let body = "foreach x {1 2 3} $body";
    let none = infer_param_traits(&["body"], body, env_for(&registry, LexerConfig::default()));
    let declared = DeclaredSurface::new();
    let empty = infer_param_traits(&["body"], body, env_with_declarations(&registry, &declared));
    assert_eq!(none, empty);
}

fn traits_under(prelude: &str, body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
    let registry = CommandRegistry::build_default();
    let (identities, body_span) = realm_for_body(prelude, body);
    infer_param_traits(
        &["body", "v"],
        body,
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, None),
            config: LexerConfig::default(),
            identities: &identities,
            executed_source: identities.executed_script_for_word(body_span),
            original_logical_body: None,
        },
    )
}

fn is_eval(traits: &HashMap<String, HashSet<ProcArgTrait>>) -> bool {
    traits
        .get("body")
        .is_some_and(|s| s.contains(&ProcArgTrait::Eval))
}

#[test]
fn param_traits_follow_an_aliased_head() {
    assert!(is_eval(&traits_under(
        "interp alias {} run {} eval\n",
        "run $body"
    )));
    // The `::`-qualified spelling of the alias classifies alike.
    assert!(is_eval(&traits_under(
        "interp alias {} run {} eval\n",
        "::run $body"
    )));
    // Guard: an unbound `run` evaluates nothing.
    assert!(!is_eval(&traits_under("set y 1\n", "run $body")));
}

#[test]
fn param_traits_follow_a_renamed_head() {
    assert!(is_eval(&traits_under("rename eval run\n", "run $body")));
    assert!(
        !is_eval(&traits_under("rename eval run\n", "eval $body")),
        "a renamed-away `eval` must not keep the built-in's grammar"
    );
}

#[test]
fn param_traits_abstain_for_a_builtin_shadowed_by_a_user_proc() {
    assert!(
        !is_eval(&traits_under("proc eval {s} { return $s }\n", "eval $body")),
        "a user `proc eval` takes the name over; its argument is not a script"
    );
    // Guard: the unshadowed built-in still records the trait.
    assert!(is_eval(&traits_under("set y 1\n", "eval $body")));
}

#[test]
fn param_traits_abstain_for_a_dynamic_binding() {
    assert!(
        !is_eval(&traits_under("rename $old run\n", "run $body")),
        "a dynamic rename must not make `run` an evaluator"
    );
    assert!(
        !is_eval(&traits_under("rename $old run\n", "eval $body")),
        "the unknown source may be eval, so its later evaluator identity is unresolved"
    );
    assert!(
        is_eval(&traits_under("rename puts output\n", "eval $body")),
        "a proved unrelated rename preserves the evaluator binding"
    );
}

#[test]
fn positioned_own_body_tracks_local_symbolic_copy_without_a_caller() {
    let source = "proc p {body} {set local $body; eval $local}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let identities = crate::realm::document_realm_bindings(
        source,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        registry,
    );
    let command = segment_commands_with_offset_and_config(source, 0, LexerConfig::default())
        .pop()
        .unwrap();
    let script = identities
        .executed_script_for_word(command.argv.last().unwrap().span)
        .unwrap();
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, None),
        config: LexerConfig::default(),
        identities: &identities,
        executed_source: Some(script),
        original_logical_body: None,
    };
    let traits = infer_param_traits(&["body"], script.try_text().unwrap(), env);
    assert_trait(&traits, "body", ProcArgTrait::Eval);
    assert!(caller_frame_upvar_params(&["body"], script.try_text().unwrap(), env).is_empty());
    assert!(caller_frame_literal_targets(script.try_text().unwrap(), env).is_empty());
    let bare = infer_param_traits(
        &["body"],
        script.try_text().unwrap(),
        env_for(registry, LexerConfig::default()),
    );
    assert!(
        !bare
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn declaration_retains_symbolic_caller_name_without_caller_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let declaration = "proc p {v} {upvar 1 $v alias; set alias 1}";
    for (called, source) in [
        (false, declaration.to_owned()),
        (true, format!("{declaration}; p target")),
    ] {
        let identities = crate::realm::document_realm_bindings(
            &source,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            registry,
        );
        let commands = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default());
        let script = identities
            .executed_script_for_word(commands[0].argv.last().unwrap().span)
            .unwrap();
        let env = TraitScanEnv {
            surface: DocumentCommandSurface::new(registry, None),
            config: LexerConfig::default(),
            identities: &identities,
            executed_source: Some(script),
            original_logical_body: None,
        };
        assert!(caller_frame_upvar_params(&["v"], script.try_text().unwrap(), env).contains("v"));
        assert_eq!(env.has_actual_procedure_entry(), called);
        assert!(caller_frame_literal_targets(script.try_text().unwrap(), env).is_empty());
    }
}

#[test]
fn conditional_own_body_traits_reach_whole_and_per_item_proc_consumers() {
    let source = "proc p {body} {set local $body; eval $local}";
    let mut analyser = crate::analyser::Analyser::new();
    let whole = analyser.analyse(source, "tcl8.6");
    let mut analyser = crate::analyser::Analyser::new();
    let per_item = analyser.analyse_per_item(source, "tcl8.6");
    for result in [whole, per_item] {
        let procedure = result.all_procs.get("::p").expect("original declaration");
        assert_trait(&procedure.param_traits, "body", ProcArgTrait::Eval);
        assert!(procedure.caller_frame_params.is_empty());
        assert!(procedure.caller_frame_literals.is_empty());
    }
}

#[test]
fn declared_roles_cannot_restore_a_replaced_or_opaque_slot() {
    let declared = make_declarations(vec![stub_sig("my_eval", &[("script", ArgRole::Body)])]);
    let registry = CommandRegistry::build_default();
    for prelude in [
        "proc my_eval args {return ordinary};",
        "rename $old output;",
    ] {
        let body = "my_eval $body";
        let (identities, span) = realm_for_body(prelude, body);
        let traits = infer_param_traits(
            &["body"],
            body,
            TraitScanEnv {
                surface: DocumentCommandSurface::new(&registry, Some(&declared)),
                config: LexerConfig::default(),
                identities: &identities,
                executed_source: identities.executed_script_for_word(span),
                original_logical_body: None,
            },
        );
        assert!(
            !traits
                .get("body")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Body)),
            "{prelude}"
        );
    }
}
