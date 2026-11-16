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
/// fields each own a `SCOPE::FIELD`-keyed native table in
/// `tcl_registry::pack_hooks`
/// (`docs/design/compiler/value-evaluation.md` § *`-native ID`, and the
/// per-family catalogues*); this is the same obligation as
/// [`compiler_hook_catalogues_cover_their_tables`], read from the table's
/// own id rather than a Rust variant name.
#[test]
fn native_hook_tables_cover_their_catalogues() {
    fn ids<T>(table: &[(&str, T)]) -> Vec<String> {
        table.iter().map(|(id, _)| (*id).to_owned()).collect()
    }
    for (what, mine, catalogue) in [
        (
            "arg-role-resolver native",
            ids(tcl_registry::pack_hooks::ARG_ROLE_RESOLVER_NATIVE),
            catalogue::ARG_ROLE_RESOLVER_NATIVE,
        ),
        (
            "command-prefix-resolver native",
            ids(tcl_registry::pack_hooks::COMMAND_PREFIX_RESOLVER_NATIVE),
            catalogue::COMMAND_PREFIX_RESOLVER_NATIVE,
        ),
        (
            "script-timing-resolver native",
            ids(tcl_registry::pack_hooks::SCRIPT_TIMING_RESOLVER_NATIVE),
            catalogue::SCRIPT_TIMING_RESOLVER_NATIVE,
        ),
        (
            "const-fold native",
            ids(tcl_registry::pack_hooks::CONST_FOLD_NATIVE),
            catalogue::CONST_FOLD_NATIVE,
        ),
        (
            "const-fold-versioned native",
            ids(tcl_registry::pack_hooks::CONST_FOLD_VERSIONED_NATIVE),
            catalogue::CONST_FOLD_VERSIONED_NATIVE,
        ),
        (
            "taint-sink-gate native",
            ids(tcl_registry::pack_hooks::TAINT_SINK_GATE_NATIVE),
            catalogue::TAINT_SINK_GATE_NATIVE,
        ),
        (
            "context-gate native",
            ids(tcl_registry::pack_hooks::CONTEXT_GATE_NATIVE),
            catalogue::CONTEXT_GATE_NATIVE,
        ),
        (
            "literal-argument-validator native",
            ids(tcl_registry::pack_hooks::LITERAL_ARGUMENT_VALIDATOR_NATIVE),
            catalogue::LITERAL_ARGUMENT_VALIDATOR_NATIVE,
        ),
        (
            "clause-shape-check native",
            ids(tcl_registry::pack_hooks::CLAUSE_SHAPE_CHECK_NATIVE),
            catalogue::CLAUSE_SHAPE_CHECK_NATIVE,
        ),
        (
            "option-arity native",
            ids(tcl_registry::pack_hooks::OPTION_ARITY_NATIVE),
            catalogue::OPTION_ARITY_NATIVE,
        ),
        (
            "constraints native",
            ids(tcl_registry::pack_hooks::CONSTRAINTS_NATIVE),
            catalogue::CONSTRAINTS_NATIVE,
        ),
        (
            "state-transition-resolver native",
            ids(tcl_registry::pack_hooks::STATE_TRANSITION_RESOLVER_NATIVE),
            catalogue::STATE_TRANSITION_RESOLVER_NATIVE,
        ),
        (
            "semantics native",
            ids(tcl_registry::pack_hooks::SEMANTICS_NATIVE),
            catalogue::SEMANTICS_NATIVE,
        ),
        (
            "evaluate native",
            ids(tcl_registry::pack_hooks::EVALUATE_NATIVE),
            catalogue::EVALUATE_NATIVE,
        ),
        (
            "facts native",
            ids(tcl_registry::pack_hooks::FACTS_NATIVE),
            catalogue::FACTS_NATIVE,
        ),
    ] {
        assert_table_covers(what, &mine, catalogue);
    }
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
