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

//! The `SpecTcl` pack: the `.tclspec` DSL described in the registry it
//! configures.
//!
//! registry-metadata: every assertion here reads registry data or the frozen
//! syntax memo (`docs/design/spec-dsl-examples/README.md`), not C-Tcl
//! behaviour — `SpecTcl` is our own DSL, so the memo *is* its oracle.
use tcl_registry::ArgRole;
use tcl_registry::definer::{DefinerFamily, SPECTCL_GRAMMARS};
use tcl_registry::model::ingress::static_context_for;
use tcl_dialect::model::{SpecSurface};

/// The pack loads by profile identity, through exactly the same
/// `base_layers` path every other dialect pack takes — so the registry sweep
/// and every other `registry_for_dialect` consumer picks it up with no
/// per-pack wiring at all.
#[test]
fn the_pack_loads_for_the_spectcl_profile_and_nowhere_else() {
    let spectcl = static_context_for("spectcl").commands();
    assert!(
        spectcl.get("speclib").is_some(),
        "the spectcl registry carries the pack"
    );
    // The base Tcl surface stays underneath it: a pack is an ordinary Tcl
    // script, and its hook bodies are real Tcl.
    for core in ["set", "if", "string", "lindex"] {
        assert!(
            spectcl.get(core).is_some(),
            "{core} must still resolve inside a pack"
        );
    }
    // …and the statement words reach no other dialect. `speclib` is the one
    // word that can only ever be SpecTcl's, so it is the honest probe.
    for other in ["tcl8.6", "tcl9.0", "f5-irules", "expect", "bpf"] {
        assert!(
            static_context_for(other)
                .commands()
                .get("speclib")
                .is_none(),
            "{other} must not see SpecTcl's statement words"
        );
    }
}

/// Extremely generic statement words (`command`, `option`, `arg`) resolve to
/// the `SpecTcl` spec under the `SpecTcl` profile even where another pack already
/// claims the name — Tk's `option` is the live collision — and never leak the
/// other way.
#[test]
fn generic_statement_words_do_not_collide_across_dialects() {
    let spectcl = static_context_for("spectcl").commands();
    let mask = Some(
        tcl_registry::model::ingress::resolve_environment("spectcl")
            .analyser_profile()
            .surface_query(),
    );
    for word in ["command", "option", "arg", "hover", "values", "hook"] {
        let spec = spectcl
            .get_for_surface(word, mask)
            .unwrap_or_else(|| panic!("{word} resolves under spectcl"));
        assert_eq!(
            spec.surface,
            Some(SpecSurface::SPECTCL),
            "{word} must resolve to the SpecTcl spec, not another pack's"
        );
    }
    // Tk keeps its own `option` everywhere else.
    let tcl = static_context_for("tcl9.0").commands();
    let tk_option = tcl.get("option").expect("Tk `option` still exists");
    assert_ne!(tk_option.surface, Some(SpecSurface::SPECTCL));
}

/// Every `SpecTcl` statement word is gated to the `SpecTcl` bit alone. A word
/// that leaked `ALL_TCL` would offer `arity` / `traits` / `detail` as
/// completions in every ordinary Tcl document.
#[test]
fn every_statement_word_is_gated_to_spectcl() {
    let specs = tcl_registry::commands::spectcl::spectcl_command_specs();
    assert!(specs.len() > 100, "the pack covers the whole vocabulary");
    let mut names: Vec<&str> = specs.iter().map(|s| s.name).collect();
    for spec in &specs {
        assert_eq!(
            spec.surface,
            Some(SpecSurface::SPECTCL),
            "{}: statement words are SpecTcl-only",
            spec.name
        );
    }
    names.sort_unstable();
    let unique = names.len();
    names.dedup();
    assert_eq!(unique, names.len(), "a statement word is declared twice");
}

/// The nesting chain the whole design rests on: `speclib`'s body is a
/// definition body whose `command` member opens *another* definition body
/// with a different vocabulary, and so on down. Each level is a `CommandSpec`
/// carrying the next grammar, which is what lets the shared walker switch
/// vocabularies with no walker changes.
#[test]
fn block_statements_carry_the_grammar_of_their_own_body() {
    let reg = static_context_for("spectcl").commands();
    let chain = [
        ("speclib", "command"),
        ("command", "arity"),
        ("subcommand", "detail"),
        ("hover", "summary"),
        ("values", "value"),
        ("clause_grammar", "head"),
        ("case_list", "subject_args"),
        ("event_requires", "client_side"),
        ("world_effects", "composition"),
        ("state_transitions", "widen"),
        ("definition_body", "member"),
        ("body_scope", "command"),
        ("object_class", "method"),
    ];
    for (statement, inner) in chain {
        let grammar = reg
            .get(statement)
            .unwrap_or_else(|| panic!("{statement} is a registered statement"))
            .definition_body
            .unwrap_or_else(|| panic!("{statement} carries its body's grammar"));
        assert_eq!(grammar.family, DefinerFamily::SpecTcl, "{statement}");
        assert!(
            grammar.is_member(inner),
            "{statement}'s grammar must know `{inner}`"
        );
    }
}

/// A hook property statement carries **no** grammar: its body is ordinary
/// Tcl (the sandbox whitelist), so the walker must drop back out of
/// definition context for it exactly as it does for a `method` body.
#[test]
fn hook_bodies_are_ordinary_tcl_not_a_declaration_block() {
    let reg = static_context_for("spectcl").commands();
    for hook in [
        "const_fold",
        "const_fold_versioned",
        "arg_role_resolver",
        "command_prefix_resolver",
        "taint_sink_gate",
        "context_gate",
        "literal_argument_validator",
        "clause_shape_check",
    ] {
        let spec = reg
            .get(hook)
            .unwrap_or_else(|| panic!("{hook} is a statement"));
        assert!(
            spec.definition_body.is_none(),
            "{hook}'s body is Tcl, not a declaration block"
        );
        assert_eq!(
            spec.arg_role_at(1),
            Some(ArgRole::Body),
            "{hook}'s script word is a body"
        );
        assert_eq!(
            spec.arg_role_at(0),
            Some(ArgRole::ParamList),
            "{hook}'s signature word is a parameter list"
        );
    }
}

/// `command NAME ?-override? { … }` — the block moves when the optional flag
/// is present, and the pack grammar's member layout agrees with the
/// command spec's own resolver. Both readings are exercised because the
/// folding walk uses the *member* layout inside a `speclib` body and the
/// spec's resolver everywhere else.
#[test]
fn the_override_flag_shifts_the_command_block() {
    let reg = static_context_for("spectcl").commands();
    let plain = ["mylib::sort", "{ arity 1.. }"];
    let overriding = ["lsort", "-override", "{ arity 1.. }"];
    assert_eq!(
        reg.arg_indices_for_role("command", &plain, ArgRole::Body),
        vec![1]
    );
    assert_eq!(
        reg.arg_indices_for_role("command", &overriding, ArgRole::Body),
        vec![2],
        "-override shifts the block one place right"
    );

    let member = tcl_registry::definer::SPECTCL_PACK_GRAMMAR
        .member("command")
        .expect("`command` is a pack-level member");
    assert_eq!(
        member
            .indices_for_call(&plain, ArgRole::Body)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        member
            .indices_for_call(&overriding, ArgRole::Body)
            .collect::<Vec<_>>(),
        vec![2],
        "the member layout must agree with the spec's resolver"
    );
}

/// `object_class NAME ?-superclass {…}? ?-allow-unknown? { … }` — the block
/// is the last word however many flags intervene.
#[test]
fn object_class_finds_its_block_past_any_flags() {
    let reg = static_context_for("spectcl").commands();
    for call in [
        vec!["Resistor", "{ method value {} }"],
        vec!["Resistor", "-allow-unknown", "{ method value {} }"],
        vec![
            "Resistor",
            "-superclass",
            "{Device}",
            "-allow-unknown",
            "{ method value {} }",
        ],
    ] {
        let last = call.len() - 1;
        assert_eq!(
            reg.arg_indices_for_role("object_class", &call, ArgRole::Body),
            vec![last],
            "block index for {call:?}"
        );
        assert_eq!(
            reg.arg_indices_for_role("object_class", &call, ArgRole::Name),
            vec![0]
        );
    }
    // A bare reference form (`object_class NAME`) claims no block at all.
    assert!(
        reg.arg_indices_for_role("object_class", &["Resistor"], ArgRole::Body)
            .is_empty()
    );
    // …and it is deliberately not a *member row* of the command grammar: a
    // member layout cannot express two optional flags of different widths, so
    // it would have painted `-superclass`'s braced class list as a block. The
    // word still reads as a keyword (`LANGUAGE_KEYWORD`) and still switches
    // grammars (`definition_body`), which is everything member-hood bought it.
    assert!(
        !tcl_registry::definer::SPECTCL_COMMAND_GRAMMAR.is_member("object_class"),
        "the resolver, not a member layout, owns `object_class`'s block index"
    );
    let spec = reg.get("object_class").expect("registered statement");
    assert!(spec.traits.contains(tcl_registry::Traits::LANGUAGE_KEYWORD));
    assert!(spec.definition_body.is_some());
}

/// `SpecTcl` is a *declaration* family, not a class system: it manufactures
/// nothing and dispatches nothing, so every field a consumer would read to
/// create an instance is empty. Consumers keyed on those fields therefore
/// see nothing without having to know the family exists.
#[test]
fn the_spectcl_family_manufactures_nothing() {
    for grammar in SPECTCL_GRAMMARS {
        assert_eq!(grammar.family, DefinerFamily::SpecTcl);
        assert!(grammar.manufacturers.is_empty());
        assert!(grammar.builtin_object_methods.is_empty());
        assert!(grammar.builtin_type_methods.is_empty());
        assert!(grammar.member_body_commands.is_empty());
        assert!(grammar.implicit_vars.is_empty());
        assert!(!grammar.bare_word_construction);
        assert!(!grammar.dynamic_method_dispatch);
        assert!(grammar.unknown_dispatch_method.is_none());
        assert!(!grammar.members.is_empty(), "a grammar with no members");
    }
}

/// The syntax memo's coverage matrix promises one DSL word per schema key.
/// Spot-check the keys whose spelling the memo *rules on*, so a future
/// rename of one of them fails here rather than silently in a pack.
#[test]
fn the_ruled_spellings_are_the_ones_the_memo_settled() {
    let grammar = &tcl_registry::definer::SPECTCL_COMMAND_GRAMMAR;
    // Singular row statements, never plural blocks.
    for singular in [
        "option",
        "subcommand",
        "form",
        "side_effect",
        "setter_constraint",
        "repeat",
        "manufacturer",
        "option_conflict",
        "oo_context_fact",
        "sub_subcommand",
        "versioned_arg_value",
    ] {
        assert!(grammar.is_member(singular), "missing row word `{singular}`");
    }
    for plural in [
        "options",
        "subcommands",
        "forms",
        "side_effects",
        "setter_constraints",
        "repeated_args",
        "manufacturer_methods",
        "option_relations",
        "oo_context_facts",
        "sub_subcommands",
        "versioned_arg_values",
    ] {
        assert!(
            !grammar.is_member(plural),
            "`{plural}` is the schema key, not the DSL word"
        );
    }
    // The three hover keys the memo deliberately renames.
    let hover = &tcl_registry::definer::SPECTCL_HOVER_GRAMMAR;
    for renamed in ["description", "example", "returns"] {
        assert!(hover.is_member(renamed), "missing hover key `{renamed}`");
    }
    for rust_name in ["snippet", "examples", "return_value"] {
        assert!(
            !hover.is_member(rust_name),
            "`{rust_name}` is the Rust field name, not the DSL word"
        );
    }
    // Excluded fields have no word at all — writing syntax for one is the
    // single thing the memo says a draft must not do.
    for excluded in [
        "completion",
        "dispatch_dependencies",
        "command_forms",
        "subcommand_forms",
    ] {
        assert!(
            !grammar.is_member(excluded),
            "`{excluded}` is excluded and must have no DSL word"
        );
    }
}
