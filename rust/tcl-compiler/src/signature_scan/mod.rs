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

//! Signature-only scan for background-indexed Tcl files.
//!
//! Walks
//! the segmented command stream of a Tcl source and extracts a
//! lightweight [`SignatureScanResult`] subset — proc / class
//! definitions, package requires, source targets, command aliases,
//! namespace imports, auto-path entries, and a flat command-invocation
//! list — without running the full analyser pipeline.
//!
//! Used by cross-file LSP features (workspace symbols, `package
//! require` resolution, command-usage counts) on every non-OPEN
//! document; the full analyser still runs on `didOpen` /
//! `didChange` so OPEN files are unaffected.
//!
//! The walker stays at the segmenter level (not the IR level) — the
//! whole point of this module is to be fast for background-indexed
//! files, so it deliberately avoids the lowering pass.
//!
//! Module layout:
//!
//! - [`types`] — public record types ([`SignatureScanResult`] +
//!   per-collection records) plus [`ParamDef`].
//! - [`arity`] — [`arity::arity_of`], the canonical parameter-list →
//!   argument-count-arity computation shared by every consumer of
//!   [`ParamDef`] (same-file, cross-file, and `TclOO` method arity
//!   checks).
//! - [`params`] — `parse_param_list` for the proc parameter-list arg.
//! - `ctx` (private) — internal scan state ([`ScanCtx`],
//!   `FactoryCandidate`, `ProcBodyInfo`, `FACTORY_SKIP_NONCOMMAND_HEADS`).
//! - `handlers` (private) — registry-dispatched semantic handlers
//!   (`handle_proc`, `handle_namespace_eval`, `handle_package_require`, …).
//! - `walker` (private) — top-level `scan` plus the body-recursion
//!   helpers (`maybe_recurse_body`, `handle_if` / `handle_catch` /
//!   `handle_try`) and the factory-body sub-walker
//!   (`scan_factory_candidates` / `scan_factory_structural`).
//! - `factory` (private) — second-pass factory-wrapper resolver
//!   (`is_factory_body`, `lookup_factory`, `resolve_factory_defs`).
//!
//! The public entry point is [`extract_signatures`].
//!
//! [`ParamDef`]: types::ParamDef

pub mod arity;
pub(crate) mod command_prefix;
mod ctx;
mod factory;
pub mod formal_count;
pub mod formal_parameters;
mod handlers;
pub(crate) mod name_value;
pub mod original_name;
pub mod params;
pub mod scope;
pub mod symbol_name;
pub mod types;
pub mod variable_name;
pub mod variable_symbol;
pub mod vendor_name;
pub mod vendor_variable;
mod walker;

use tcl_registry::CommandRegistry;

use ctx::ScanCtx;
pub use types::SignatureScanResult;

/// Extract a lightweight [`SignatureScanResult`] for a Tcl source.
///
/// The public entry point for the signature scanner. Runs the main
/// walker (with segmenter-level error recovery seeded from the
/// registry's command names), then resolves factory-wrapper
/// synthetic procs.
#[must_use]
pub fn extract_signatures(source: &str, registry: &CommandRegistry) -> SignatureScanResult {
    extract_signatures_in(
        source,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        registry,
        None,
    )
}

/// Scan with an independently retained source binding inventory. Exact source
/// bytes, channel and complete lexer configuration must match before callback
/// lookup coordinates can be borrowed. A mismatched inventory is ignored.
/// This entry point never runs source binding analysis or enters a callback.
#[must_use]
pub fn extract_signatures_with_original_bindings(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> SignatureScanResult {
    let image = tcl_lexer::SourceImage::document(source);
    let original = bindings
        .matches_original_source_image(&image, config)
        .then_some(bindings);
    extract_signatures_in(source, config, registry, original)
}

fn extract_signatures_in(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> SignatureScanResult {
    let known_commands: std::collections::HashSet<&str> = registry.command_names().collect();
    // The registry carries the environment's resolved profile, so the scan
    // reads every word and segments every body under the document's own
    // grammar — an iRules `}{`, a Jim `$(…)`, an 8.4 `{*}` — rather than
    // re-deriving C Tcl's answer.
    let mut ctx = ScanCtx {
        registry: Some(registry),
        original_bindings: bindings,
        original_image: Some(tcl_lexer::SourceImage::document(source)),
        rules: tcl_syntax::word_rules::WordValueRules::from_config(&config),
        config,
        ..ScanCtx::default()
    };
    // Heads that match the factory-wrapper token shape but are not
    // factories: registry commands carrying `NOT_PROC_FACTORY` (using
    // any-spec semantics so a dialect-shadowed core head still counts)
    // plus the unregistered non-command heads.
    ctx.skip_heads = known_commands
        .iter()
        .filter(|name| registry.is_not_proc_factory(name))
        .map(|name| (*name).to_owned())
        .chain(
            ctx::FACTORY_SKIP_NONCOMMAND_HEADS
                .iter()
                .map(|h| (*h).to_owned()),
        )
        .collect();
    let root = scope::SignatureNamespaceScope::root(ctx.name_policy());
    walker::scan_in_context(source, None, &root, false, &known_commands, &mut ctx);
    factory::resolve_factory_defs(&mut ctx);
    ctx.result
}

#[cfg(test)]
mod original_callback_scan_tests {
    use super::types::SignatureCommandInvocation;
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationMode};

    fn inventory(
        source: &str,
        config: tcl_lexer::LexerConfig,
        dialect: tcl_registry::InvocationDialect,
        registry: &CommandRegistry,
    ) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn callback(result: &SignatureScanResult) -> &SignatureCommandInvocation {
        result
            .command_invocations
            .iter()
            .find(|invocation| invocation.callback_arity.is_some())
            .unwrap()
    }

    #[test]
    fn original_callback_scan_borrows_only_matching_inventory_scope() {
        // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(name)).unwrap(),
            );
            let registry = tcl_registry::model::ingress::static_context_for(name).commands();
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let source = "lsort -command {::cmp fixed} {b a}";
            let bindings = inventory(source, config, dialect, registry);
            let result =
                extract_signatures_with_original_bindings(source, config, registry, &bindings);
            let row = callback(&result);
            assert_eq!(
                row.original_name_input
                    .as_ref()
                    .unwrap_or_else(|| panic!("{name}: original callback input"))
                    .bytes(),
                b"::cmp",
                "{name}"
            );
            assert_eq!(
                row.original_lookup
                    .as_ref()
                    .unwrap_or_else(|| panic!("{name}: original callback lookup"))
                    .callback_lookup_scope(),
                Some(tcl_registry::ScriptLookupScope::InvokingFrame),
                "{name}"
            );
            assert_eq!(row.callback_baked_args, 1);
            assert!(!row.rename_safe);
            let binding = bindings.invocation_at_source("", 0);
            let mut tokens = binding.original_recorded_command_tokens().unwrap();
            tokens.source_binding = Some(binding.clone());
            let original = binding
                .original_callback_prefix(&tokens, 2, registry)
                .unwrap();
            assert_eq!(
                row.original_name_input.as_ref(),
                Some(original.name_input())
            );
            assert_eq!(row.original_lookup.as_ref(), original.lookup());
            let changed = source.replace("cmp", "bad");
            let mismatched =
                extract_signatures_with_original_bindings(&changed, config, registry, &bindings);
            assert!(callback(&mismatched).original_lookup.is_none());
            let mut other_config = config;
            other_config.expand_syntax = !other_config.expand_syntax;
            let mismatched = extract_signatures_with_original_bindings(
                source,
                other_config,
                registry,
                &bindings,
            );
            assert!(callback(&mismatched).original_lookup.is_none());
        }
    }

    #[test]
    fn original_callback_scan_respects_selected_installer_moves_and_terminal_barriers() {
        // naming.callback.lookup-scope-owner
        // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::context_for_profile(profile);
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        for (source, expected) in [
            (
                "rename lsort sorter; sorter -command {::cmp fixed} {b a}",
                true,
            ),
            (
                "interp alias {} sorter {} lsort; sorter -command {::cmp fixed} {b a}",
                true,
            ),
            (
                "proc lsort args {}; lsort -command {::cmp fixed} {b a}",
                false,
            ),
            ("rename lsort {}; lsort -command {::cmp fixed} {b a}", false),
        ] {
            let bindings = inventory(source, config, dialect, registry.commands());
            let result = extract_signatures_with_original_bindings(
                source,
                config,
                registry.commands(),
                &bindings,
            );
            let callbacks = result
                .command_invocations
                .iter()
                .filter(|row| row.name == "::cmp" && row.callback_arity.is_some())
                .collect::<Vec<_>>();
            assert_eq!(!callbacks.is_empty(), expected, "{source}");
            for row in callbacks {
                assert!(row.original_callback_prefix.is_some());
                assert!(row.argc.is_none() && !row.rename_safe);
            }
        }
    }

    #[test]
    fn original_callback_header_scan_keeps_readonly_lineage_without_scope() {
        // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        for source in [
            "lsort -command {::cmp fixed} {b a}",
            "trace add variable v write {::cmp fixed}",
        ] {
            let result = extract_signatures(source, &registry);
            let row = callback(&result);
            assert_eq!(row.original_name_input.as_ref().unwrap().bytes(), b"::cmp");
            assert!(row.original_lookup.is_none());
            assert!(
                row.original_name_input
                    .as_ref()
                    .unwrap()
                    .original_word_key()
                    .is_none()
            );
            assert!(!row.rename_safe);
        }
        let result = extract_signatures("trace add variable v write {$head fixed}", &registry);
        assert!(
            result
                .command_invocations
                .iter()
                .filter(|row| row.callback_arity.is_some())
                .all(|row| row.original_name_input.is_none() && row.original_lookup.is_none())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> SignatureScanResult {
        let registry = CommandRegistry::build_default();
        extract_signatures(src, &registry)
    }

    #[test]
    fn signatures_keep_literal_colon_names_and_namespace_components() {
        let result = run(
            "namespace eval : {proc p {} {}; proc : {} {}; namespace eval x {proc q {} {}}}; namespace eval a {proc : {} {}}",
        );
        assert_eq!(result.procs[":::::p"].name, "p");
        assert_eq!(result.procs["::::::"].name, ":");
        assert_eq!(result.procs["::a:::"].name, ":");
        assert_eq!(result.procs[":::::x::q"].name, "q");
        assert!(!result.procs.contains_key("::p"));
    }

    #[test]
    fn signature_publication_keys_match_six_native_namespace_controls() {
        let source = "namespace eval : {proc p {} {}; puts [namespace which -command p]}\nnamespace eval a:::b {proc q {} {}; puts [namespace which -command q]}";
        let mut engines: Vec<_> = tcl_test_support::required_tclshs(&tcl_dialect::TclVersion::ALL)
            .unwrap()
            .into_iter()
            .map(|interpreter| {
                (
                    format!("tcl{}", interpreter.version.version_string()),
                    interpreter.path,
                )
            })
            .collect();
        if let Some(jim) = tcl_test_support::locate_jimsh().unwrap() {
            engines.push(("jim".to_owned(), jim.path));
        }
        for (engine, path) in engines {
            let profile =
                crate::environment_ingress::resolve_environment(&engine).analyser_profile();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let result = extract_signatures(source, &registry);
            let outcome = tcl_test_support::run_script(&path, source.as_bytes()).unwrap();
            let actual = outcome.strict_text().unwrap();
            let keys: Vec<_> = actual.lines().collect();
            assert_eq!(keys.len(), 2, "{engine}");
            assert_eq!(result.procs.len(), 2, "{engine}");
            for key in keys {
                assert!(
                    result.procs.contains_key(key),
                    "{engine}: {key}, {:?}",
                    result.procs
                );
            }
        }
    }

    #[test]
    fn signature_names_keep_jim_flat_keys_and_old_c_creation_rejections() {
        let source = "namespace eval a:::b {proc p {} {}; rename p q:::r; alias alias:::name q:::r}; namespace eval : {proc : {} {}}";
        let jim = CommandRegistry::build_default().project_for_profile(
            crate::environment_ingress::resolve_environment("jim").analyser_profile(),
        );
        let result = extract_signatures(source, &jim);
        assert!(result.procs.contains_key("::a:::b::p"));
        assert!(result.renames.contains_key("::a:::b::q:::r"));
        assert!(result.command_aliases.contains_key("::alias:::name"));
        for version in ["tcl8.4", "tcl8.5"] {
            let registry = CommandRegistry::build_default()
                .project_for_profile(tcl_dialect::DialectProfile::find(version).unwrap());
            let result = extract_signatures(source, &registry);
            assert!(result.procs.contains_key("::a::b::p"));
            assert!(!result.procs.contains_key("::::::"));
        }
    }

    fn for_engine(engine: &str, source: &str) -> SignatureScanResult {
        let profile = crate::environment_ingress::resolve_environment(engine).analyser_profile();
        extract_signatures(
            source,
            &CommandRegistry::build_default().project_for_profile(profile),
        )
    }

    #[test]
    fn reported_colon_tail_preserves_selected_slot_and_body_scope() {
        use scope::SignatureNamespaceScope;
        use tcl_core_types::ByteNamespacePath;
        let source = "namespace eval N {proc :f {} {return [namespace current]}}";
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = for_engine(engine, source);
            let declaration = &result.procs["::N:::f"];
            let namespace = SignatureNamespaceScope::C(ByteNamespacePath::from_segments(["N"]));
            assert_eq!(declaration.body_namespace, namespace, "{engine}");
            assert_eq!(declaration.name, ":f", "{engine}");
            assert_eq!(declaration.source_spelling(), None, "{engine}");
            let policy = declaration.source_name.as_ref().unwrap().policy();
            assert_eq!(
                result.procedures_for_written_name(&namespace, ":f", Some(policy)),
                [declaration]
            );
            assert!(
                result
                    .procedures_for_written_name(&namespace, "::N:::f", Some(policy))
                    .is_empty()
            );
        }
        for engine in ["tcl8.4", "tcl8.5"] {
            assert!(
                for_engine(engine, source).procedure_declarations.is_empty(),
                "{engine}"
            );
        }
        let jim = for_engine("jim", source);
        let declaration = &jim.procs["::N:::f"];
        assert_eq!(
            declaration.body_namespace,
            SignatureNamespaceScope::Jim(b"N:".as_slice().into())
        );
    }

    #[test]
    fn equal_reports_keep_distinct_original_declaration_slots() {
        use scope::SignatureNamespaceScope;
        use tcl_core_types::ByteNamespacePath;
        let source = "namespace eval a: {namespace eval b {proc p {} {return FIRST}}}; namespace eval a {namespace eval :b {proc p {} {return SECOND}}}";
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = for_engine(engine, source);
            assert_eq!(result.procedure_declarations.len(), 2, "{engine}");
            assert!(!result.procs.contains_key("::a:::b::p"), "{engine}");
            assert_ne!(
                result.procedure_declarations[0].source_name,
                result.procedure_declarations[1].source_name
            );
            for (index, components) in [["a:", "b"], ["a", ":b"]].into_iter().enumerate() {
                let scope =
                    SignatureNamespaceScope::C(ByteNamespacePath::from_segments(components));
                let declaration = &result.procedure_declarations[index];
                assert_eq!(declaration.body_namespace, scope);
                let policy = declaration.source_name.as_ref().unwrap().policy();
                assert_eq!(
                    result.procedures_for_written_name(&scope, "p", Some(policy)),
                    [declaration]
                );
                assert_eq!(declaration.source_spelling(), None);
            }
        }
        let jim = for_engine("jim", source);
        assert_eq!(jim.procedure_declarations.len(), 2);
        assert_eq!(
            jim.procedure_declarations[0].source_name,
            jim.procedure_declarations[1].source_name
        );
        assert_eq!(
            jim.procs["::a:::b::p"].body_range,
            jim.procedure_declarations[1].body_range
        );
    }

    #[test]
    fn alias_target_report_keeps_original_slot_without_global_spelling() {
        use types::SignatureCommandAliasTarget;
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = for_engine(engine, "interp alias {} A {} :target");
            let target = &result.command_aliases["::A"].target;
            let policy = result.command_aliases["::A"]
                .source_name
                .as_ref()
                .unwrap()
                .policy();
            let selected = target.selected_global_name(Some(policy)).unwrap();
            assert!(selected.slot().namespace.is_root(), "{engine}");
            assert_eq!(selected.slot().simple.as_bytes(), b":target", "{engine}");
            assert_eq!(
                target.reported_global_key(Some(policy)).as_deref(),
                Some(":::target"),
                "{engine}"
            );
            assert_eq!(target.checked_global_key(Some(policy)), None, "{engine}");
            let absolute = SignatureCommandAliasTarget::WrittenGlobal(":::target".to_owned());
            assert_ne!(
                absolute.selected_global_name(Some(policy)),
                Some(selected),
                "{engine}"
            );
        }
    }

    #[test]
    fn jim_alias_keeps_caller_lookup_without_global_target_projection() {
        let result = for_engine("jim", "namespace eval N {alias alias:::name target baked}");
        let alias = &result.command_aliases["::alias:::name"];
        assert_eq!(
            alias.target,
            types::SignatureCommandAliasTarget::WrittenCaller("target".to_owned())
        );
        assert_eq!(alias.extras, ["baked"]);
        assert_eq!(
            alias.target.selected_global_name(
                alias
                    .source_name
                    .as_ref()
                    .map(scope::SignatureSourceCommand::policy)
            ),
            None
        );
        assert_eq!(
            alias.target.reported_global_key(
                alias
                    .source_name
                    .as_ref()
                    .map(scope::SignatureSourceCommand::policy)
            ),
            None
        );
        assert_eq!(
            alias.target.checked_global_key(
                alias
                    .source_name
                    .as_ref()
                    .map(scope::SignatureSourceCommand::policy)
            ),
            None
        );
        assert!(
            for_engine("jim", "interp alias {} alias:::name {} target")
                .command_aliases
                .is_empty()
        );
        assert!(
            for_engine("jim", "alias a $target")
                .command_aliases
                .is_empty()
        );
    }

    #[test]
    fn proc_only() {
        let r = run("proc foo {} {}");
        assert!(r.procs.contains_key("::foo"));
        assert_eq!(r.classes.len(), 0);
        assert_eq!(r.command_invocations.len(), 1);
    }

    #[test]
    fn opt_proc_records_real_args_only_arity_not_optlists_own_words() {
        // Background-scan half: this module feeds
        // cross-file arity checking (`arity::arity_of`), independently of
        // the full analyser's own `AnalyserHookId::OptProc` fix — a
        // generic `Traits::DEFINES_PROCEDURE` dispatch here would
        // otherwise reuse `handle_proc`'s literal
        // `parse_param_list(&texts[2])` and record `optlist`'s own
        // descriptor words as the arity-relevant params, misreporting a
        // cross-file caller's true (unconstrained) argument count.
        let r = run("tcl::OptProc greet {child -use -display} { return $child }");
        let proc = r.procs.get("::greet").expect("greet recorded");
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["args"],
            "{:?}",
            proc.params
        );
    }

    #[test]
    fn class_only() {
        let r = run("oo::class create MyCls { method foo {} {} }");
        assert!(r.classes.contains_key("::MyCls"));
        assert_eq!(r.procs.len(), 0);
    }

    #[test]
    fn namespace_eval_with_inner_proc() {
        let r = run("namespace eval ns { proc inner {} {} }");
        assert!(r.procs.contains_key("::ns::inner"));
    }

    #[test]
    fn package_require_and_source() {
        let r = run("package require Tcl 8.6\nsource /a/b.tcl");
        assert_eq!(r.package_requires.len(), 1);
        assert_eq!(r.package_requires[0].name, "Tcl");
        assert_eq!(r.source_targets.len(), 1);
        assert!(r.source_targets[0].is_literal);
    }

    #[test]
    fn interp_alias_local_recorded() {
        let r = run("interp alias {} myalias {} puts hello");
        assert!(r.command_aliases.contains_key("::myalias"));
    }

    #[test]
    fn rename_recorded_qualified_and_namespaced() {
        let r = run("rename puts my_puts\nnamespace eval ns { rename set my_set }");
        assert!(r.renames.contains_key("::my_puts"));
        assert_eq!(r.renames["::my_puts"].target, "puts");
        assert!(r.renames.contains_key("::ns::my_set"));
    }

    #[test]
    fn rename_to_empty_deletes_not_recorded() {
        let r = run("rename puts {}");
        assert_eq!(r.renames.len(), 0);
    }

    #[test]
    fn tcllib_factory_wrapper_emits_synthetic_proc() {
        let src = "
            proc DEFC {name args body} {
                proc $name $args $body
            }
            proc INIT {} {
                DEFC ChildA {x} {return 1}
                DEFC ChildB {y} {return 2}
            }
        ";
        let r = run(src);
        // The wrapper itself + the INIT proc are real.
        assert!(r.procs.contains_key("::DEFC"));
        assert!(r.procs.contains_key("::INIT"));
        // Synthetic factory-emitted procs should be present too.
        assert!(r.procs.contains_key("::ChildA"));
        assert!(r.procs.contains_key("::ChildB"));
        // Synthetic procs have empty params (per-call arg map is
        // not statically known).
        assert_eq!(
            r.procs["::ChildA"].params,
            [] as [crate::signature_scan::types::ParamDef; 0]
        );
    }
}

#[cfg(test)]
mod original_native_scan_tests {
    use super::*;
    use scope::{SignatureSourceLookup, SignatureSourceNameInput};

    #[test]
    fn background_scan_keeps_opaque_slots_after_ui_maps_are_cleared() {
        let source = r"proc p\uD800 {} {return FIRST}
proc p\uD801 {} {return SECOND}
p\uD800
p\uD801";
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut result = extract_signatures(source, &registry);
        assert_eq!(result.procedure_declarations.len(), 2);
        let first = result.procedure_declarations[0]
            .source_name
            .as_ref()
            .unwrap();
        let second = result.procedure_declarations[1]
            .source_name
            .as_ref()
            .unwrap();
        assert_eq!(first.slot().simple.as_bytes(), b"p\xed\xa0\x80");
        assert_eq!(second.slot().simple.as_bytes(), b"p\xed\xa0\x81");
        assert_ne!(first.slot(), second.slot());
        let calls = result
            .command_invocations
            .iter()
            .filter(|invocation| {
                [
                    source.rfind(r"p\uD800").unwrap(),
                    source.rfind(r"p\uD801").unwrap(),
                ]
                .contains(&(invocation.range.start() as usize))
            })
            .filter_map(|invocation| invocation.original_name_input.clone())
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 2);
        result.procs.clear();
        let root = scope::SignatureNamespaceScope::root(Some(calls[0].policy()));
        let first = result.procedures_for_original_name(&calls[0], &root);
        let second = result.procedures_for_original_name(&calls[1], &root);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_ne!(first[0].name_range, second[0].name_range);
    }

    #[test]
    fn escaped_namespace_and_name_keep_complete_native_producers() {
        let source = r"namespace eval N\u0000tail {proc p\u0069ck {} {return BODY}; pick}";
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let result = extract_signatures(source, &registry);
        assert_eq!(result.procedure_declarations.len(), 1);
        let declaration = &result.procedure_declarations[0];
        let name = declaration.source_name.as_ref().unwrap();
        assert_eq!(
            name.slot().namespace.as_segments()[0].as_bytes(),
            b"N\xc0\x80tail"
        );
        assert_eq!(name.slot().simple.as_bytes(), b"pick");
        let call = result
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.range.start() == u32::try_from(source.rfind("pick").unwrap()).unwrap()
            })
            .unwrap();
        let input = call.original_name_input.as_ref().unwrap();
        let lookup =
            SignatureSourceLookup::from_input(declaration.body_namespace.clone(), input).unwrap();
        assert_eq!(lookup.candidates().unwrap()[0], *name.slot());
        assert_eq!(
            result
                .procedures_for_original_name(input, &declaration.body_namespace)
                .len(),
            1
        );
    }

    #[test]
    fn dynamic_and_expanded_heads_do_not_borrow_literal_word_identity() {
        let source = "$head\n{*}{p a}";
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let result = extract_signatures(source, &registry);
        assert!(
            result
                .command_invocations
                .iter()
                .all(|invocation| invocation.original_name_input.is_none())
        );
        let image = tcl_lexer::SourceImage::document("p");
        let line = scope::SignatureSourceNameValue::from_original_source_line(
            &image,
            0,
            tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar),
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
        )
        .unwrap();
        let lookup = SignatureSourceLookup::from_input(
            scope::SignatureNamespaceScope::root(Some(line.policy())),
            &SignatureSourceNameInput::OriginalValue(line),
        )
        .unwrap();
        assert!(lookup.original_name_key().is_none());
        assert!(lookup.original_name_input().is_some());
    }
}
