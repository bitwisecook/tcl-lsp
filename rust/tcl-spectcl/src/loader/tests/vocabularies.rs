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

//! The closed vocabularies: a native fold id installs the shipped folder, and every hook and value table covers its catalogue.

use super::*;
use crate::catalogue;
use crate::loader::value_parsers::{
    ANALYSER_HOOKS, BODY_KINDS, CODEGEN_HOOKS, CONNECTION_SIDES, FORM_KINDS, INLINE_CODEGEN_HOOKS,
    LOWERING_HOOKS, PRESENTATIONS, RETURN_TYPE_HOOKS, SIDE_EFFECT_TARGETS, STORAGE_TYPES,
    TCL_TYPES, VARIABLE_SCOPES,
};
use tcl_registry::ScriptLookupScope;

/// Native folder IDs select their actual command/subcommand callbacks.
/// Unknown or mismatched IDs retain a notice and abstaining placeholder.
#[test]
fn a_native_fold_id_installs_the_shipped_folder() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n\
             command string {\n\
                 subcommand range {\n\
                     arity 3\n\
                     const_fold -native string::range::const_fold\n\
                 }\n\
                 subcommand is {\n\
                     arity 2..\n\
                     const_fold_versioned -native string::is::const_fold_versioned\n\
                 }\n\
             }\n\
             command probe::short {\n\
                 arity 3\n\
                 const_fold -native string::range\n\
             }\n\
             command probe::unknown {\n\
                 arity 1\n\
                 const_fold -native probe::unknown::const_fold\n\
             }\n\
             }",
    );
    let string = pack.command("string").expect("declared");
    let sub = |name: &str| {
        string
            .spec
            .subcommands
            .iter()
            .find(|sub| sub.name == name)
            .expect("declared")
    };
    let range = sub("range").const_fold.expect("a folder");
    assert_eq!(range(&["abcdef", "1", "3"]).as_deref(), Some("bcd"));
    let is = sub("is").const_fold_versioned.expect("a folder");
    assert_eq!(
        is(&["integer", "42"], Some(tcl_dialect::TclVersion::V9_0)).as_deref(),
        Some("1")
    );
    for name in ["probe::short", "probe::unknown"] {
        let fold = pack
            .command(name)
            .expect("declared")
            .spec
            .const_fold
            .expect("the placeholder");
        assert_eq!(fold(&["abcdef", "1", "3"]), None, "{name}");
    }
    let notices: Vec<&str> = pack
        .notices
        .iter()
        .map(|notice| notice.message.as_str())
        .collect();
    assert!(
        notices.iter().any(|notice| notice.contains(
            "`const_fold -native string::range` is not this scope's id; spell it \
                 `probe::short::const_fold`"
        )),
        "{notices:#?}"
    );
    assert!(
        notices.iter().any(|notice| notice.contains(
            "`const_fold -native probe::unknown::const_fold` names nothing this build ships"
        )),
        "{notices:#?}"
    );
    assert_eq!(notices.len(), 2, "{notices:#?}");
}

/// The loader resolves a `-native ID` by matching the catalogue's own
/// spelling, so its value tables have to name every variant the catalogue
/// does. A registry that grows a hook and a studio catalogue that lists it
/// would otherwise leave the loader silently dropping the new id.
fn assert_table_covers(what: &str, mine: &[String], catalogue: &[catalogue::Variant]) {
    let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
    let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
    expected.sort_unstable();
    mine.sort_unstable();
    assert_eq!(mine, expected, "the {what}-hook table is out of step");
}

/// The five closed compiler-hook catalogues, by Rust variant name.
#[test]
fn compiler_hook_catalogues_cover_their_tables() {
    fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
        all.iter().map(catalogue::variant_name).collect()
    }
    for (what, mine, catalogue) in [
        ("lowering", names(LOWERING_HOOKS), catalogue::LOWERING_HOOKS),
        ("codegen", names(CODEGEN_HOOKS), catalogue::CODEGEN_HOOKS),
        (
            "inline codegen",
            names(INLINE_CODEGEN_HOOKS),
            catalogue::INLINE_CODEGEN_HOOKS,
        ),
        ("analyser", names(ANALYSER_HOOKS), catalogue::ANALYSER_HOOKS),
        (
            "return type",
            names(RETURN_TYPE_HOOKS),
            catalogue::RETURN_TYPE_HOOKS,
        ),
    ] {
        assert_table_covers(what, &mine, catalogue);
    }
}

/// Every `HookFamily` variant and the `semantics` / `evaluate` / `facts`
/// fields own a `SCOPE::FIELD`-keyed family in the installed surfaces' hook
/// tables (`docs/design/compiler/value-evaluation.md` § *`-native ID`, and
/// the per-family catalogues*). Each studio picker lists exactly the ids the
/// installed tables register, in install order; each id resolves through the
/// lookup the loader uses; and each carries a picker label, so a table that
/// gains a shipped entry without a label fails here.
#[test]
fn native_hook_catalogues_are_the_installed_registrations() {
    fn check<T: Copy>(
        what: &str,
        family: tcl_registry::HookFamilyRows<T>,
        catalogue: &[catalogue::Variant],
    ) {
        let installed = tcl_registry::installed();
        let keys: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
        assert_eq!(
            keys,
            installed.native_hook_ids(family),
            "the {what} picker is out of step with the hook tables"
        );
        for variant in catalogue {
            assert!(
                installed.native_hook(variant.key, family).is_some(),
                "{what}: {} does not resolve",
                variant.key
            );
            assert_ne!(
                variant.doc, variant.key,
                "{what}: {} has no picker label",
                variant.key
            );
        }
    }
    check(
        "arg_role_resolver",
        |table| table.arg_role_resolver,
        catalogue::arg_role_resolver_native(),
    );
    check(
        "command_prefix_resolver",
        |table| table.command_prefix_resolver,
        catalogue::command_prefix_resolver_native(),
    );
    check(
        "script_timing_resolver",
        |table| table.script_timing_resolver,
        catalogue::script_timing_resolver_native(),
    );
    check(
        "const_fold",
        |table| table.const_fold,
        catalogue::const_fold_native(),
    );
    check(
        "const_fold_versioned",
        |table| table.const_fold_versioned,
        catalogue::const_fold_versioned_native(),
    );
    check(
        "taint_sink_gate",
        |table| table.taint_sink_gate,
        catalogue::taint_sink_gate_native(),
    );
    check(
        "context_gate",
        |table| table.context_gate,
        catalogue::context_gate_native(),
    );
    check(
        "literal_argument_validator",
        |table| table.literal_argument_validator,
        catalogue::literal_argument_validator_native(),
    );
    check(
        "clause_shape_check",
        |table| table.clause_shape_check,
        catalogue::clause_shape_check_native(),
    );
    check(
        "option_arity",
        |table| table.option_arity,
        catalogue::option_arity_native(),
    );
    check(
        "constraints",
        |table| table.constraints,
        catalogue::constraints_native(),
    );
    check(
        "state_transition_resolver",
        |table| table.state_transition_resolver,
        catalogue::state_transition_resolver_native(),
    );
    check(
        "semantics",
        |table| table.semantics,
        catalogue::semantics_native(),
    );
    check(
        "evaluate",
        |table| table.evaluate,
        catalogue::evaluate_native(),
    );
    check("facts", |table| table.facts, catalogue::facts_native());
}

/// Same obligation for the vocabularies a row flag resolves.
/// [`value_transfer_tables_cover_their_catalogues`] does the same for
/// the `value_transfer` vocabulary.
#[test]
fn value_tables_cover_their_catalogues() {
    fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
        all.iter().map(catalogue::variant_name).collect()
    }
    for (what, mine, catalogue) in [
        ("argument role", names(ArgRole::ALL), catalogue::ARG_ROLES),
        (
            "source deprecation advice",
            names(&tcl_registry::SourceDeprecationAdvice::ALL),
            catalogue::SOURCE_DEPRECATION_ADVICE,
        ),
        ("Tcl type", names(TCL_TYPES), catalogue::TCL_TYPES),
        ("body kind", names(BODY_KINDS), catalogue::BODY_KINDS),
        (
            "executable lookup frame",
            names(&ScriptLookupScope::ALL),
            catalogue::SCRIPT_LOOKUP_SCOPES,
        ),
        (
            "variable scope",
            names(VARIABLE_SCOPES),
            catalogue::VARIABLE_SCOPES,
        ),
        (
            "argument presentation",
            names(PRESENTATIONS),
            catalogue::ARG_PRESENTATIONS,
        ),
        (
            "side-effect target",
            names(SIDE_EFFECT_TARGETS),
            catalogue::SIDE_EFFECT_TARGETS.as_slice(),
        ),
        (
            "connection side",
            names(CONNECTION_SIDES),
            catalogue::CONNECTION_SIDES,
        ),
        ("form kind", names(FORM_KINDS), catalogue::FORM_KINDS),
        (
            "storage type",
            names(STORAGE_TYPES),
            catalogue::STORAGE_TYPES,
        ),
    ] {
        let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
        let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
        expected.sort_unstable();
        mine.sort_unstable();
        assert_eq!(mine, expected, "the {what} table is out of step");
    }
}

/// [`value_tables_cover_their_catalogues`], split for clippy's
/// function-length lint: the `value_transfer` vocabulary's own closed
/// word lists. Most are matched by their DSL spelling (`as_str()`:
/// `write_or_preserve`, `tcl.expr`, `bounded_tcl`, …), not the Rust
/// variant name `names` reads — only `evaluate -direct ID`
/// (`NativeEvalId`) keeps the older, Rust-spelled convention, since it
/// predates `-native`.
#[test]
fn value_transfer_tables_cover_their_catalogues() {
    fn names<T: Copy + fmt::Debug>(all: &[T]) -> Vec<String> {
        all.iter().map(catalogue::variant_name).collect()
    }
    for (what, mine, catalogue) in [
        (
            "native evaluator id",
            names(tcl_registry::value_transfer::NativeEvalId::ALL),
            catalogue::NATIVE_EVAL_IDS,
        ),
        (
            "language profile",
            tcl_registry::value_transfer::LanguageProfileId::ALL
                .iter()
                .map(|profile| profile.as_str().to_owned())
                .collect(),
            catalogue::LANGUAGE_PROFILES,
        ),
        (
            "host kind",
            tcl_registry::value_transfer::HostKind::ALL
                .iter()
                .map(|host| host.as_str().to_owned())
                .collect(),
            catalogue::HOST_KINDS,
        ),
        (
            "exactness",
            tcl_registry::value_transfer::Exactness::ALL
                .iter()
                .map(|exactness| exactness.as_str().to_owned())
                .collect(),
            catalogue::EXACTNESS,
        ),
        (
            "context dependency",
            tcl_registry::value_transfer::ContextDependency::WORDS
                .iter()
                .map(|dependency| dependency.as_str().to_owned())
                .collect(),
            catalogue::CONTEXT_DEPENDENCIES,
        ),
        (
            "outcome kind",
            tcl_registry::value_transfer::OutcomeKind::ALL
                .iter()
                .map(|outcome| outcome.as_str().to_owned())
                .collect(),
            catalogue::OUTCOME_KINDS,
        ),
        (
            "declared effect",
            tcl_registry::value_transfer::DeclaredEffect::ALL
                .iter()
                .map(|effect| effect.as_str().to_owned())
                .collect(),
            catalogue::DECLARED_EFFECTS,
        ),
        (
            "evaluate reason",
            tcl_registry::value_transfer::OptionEvaluation::REASONS
                .iter()
                .map(|(word, _)| (*word).to_owned())
                .collect(),
            catalogue::EVALUATE_REASONS,
        ),
    ] {
        let mut expected: Vec<&str> = catalogue.iter().map(|variant| variant.key).collect();
        let mut mine: Vec<&str> = mine.iter().map(String::as_str).collect();
        expected.sort_unstable();
        mine.sort_unstable();
        assert_eq!(mine, expected, "the {what} table is out of step");
    }
}
