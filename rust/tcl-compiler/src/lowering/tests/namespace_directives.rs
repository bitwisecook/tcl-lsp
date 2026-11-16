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

//! `namespace import` and `namespace export` directives recorded as module metadata.

use super::*;

#[test]
fn ns_import_recorded_with_context_namespace() {
    // ``namespace import ::tcltest::*`` at top-level
    // records (``::``, ``::tcltest::*``).
    let m = lower_to_ir("namespace import ::tcltest::*", &reg());
    assert_eq!(
        m.namespace_imports,
        vec![("::".to_string(), "::tcltest::*".to_string())]
    );
}

#[test]
fn ns_import_retains_relative_dependency_pattern() {
    let m = lower_to_ir("namespace import foo::*", &reg());
    assert_eq!(
        m.namespace_imports,
        vec![("::".to_owned(), "foo::*".to_owned())]
    );
}

#[test]
fn ns_import_dependencies_preserve_selected_literal_patterns_and_order() {
    let m = lower_to_ir("namespace import foo::* ::* ::::* ::foo::* foo::*", &reg());
    assert_eq!(
        m.namespace_imports,
        ["foo::*", "::*", "::::*", "::foo::*", "foo::*"]
            .into_iter()
            .map(|pattern| ("::".to_owned(), pattern.to_owned()))
            .collect::<Vec<_>>()
    );
    let shadowed = lower_to_ir(
        "proc namespace args {}; namespace import foo::* ::foo::*",
        &reg(),
    );
    assert_eq!(
        shadowed.namespace_imports,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn ns_import_handles_force_flag() {
    // ``-force`` is the documented option; the next word is
    // the pattern.
    let m = lower_to_ir("namespace import -force ::tcltest::*", &reg());
    assert_eq!(
        m.namespace_imports,
        vec![("::".to_string(), "::tcltest::*".to_string())]
    );
}

#[test]
fn ns_export_recorded() {
    let m = lower_to_ir("namespace eval ::tcltest { namespace export test }", &reg());
    // The export was inside a ``namespace eval`` body so the
    // context namespace is ``::tcltest``.
    assert!(
        m.namespace_exports
            .iter()
            .any(|(ns, pat)| ns == "::tcltest" && pat == "test")
    );
}

#[test]
fn namespace_directive_metadata_keeps_actual_availability_and_source_config() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = baseline.commands().project_for_profile(profile);
    let mut descriptor = registry.get("namespace").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(descriptor);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(current.commands())),
    );
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let make = |context: std::sync::Arc<tcl_registry::model::ContextRegistry>| {
        let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let mut lowerer =
            Lowerer::with_config(current.commands(), config).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(entry.options());
        lowerer.with_resolved_analysis_input(input)
    };
    let source = "namespace export exposed";
    let mut selected = make(std::sync::Arc::clone(&current));
    selected.lower(source);
    assert_eq!(
        selected.namespace_exports,
        [("::".to_owned(), "exposed".to_owned())]
    );
    let mut unavailable = make(older);
    unavailable.lower(source);
    assert_eq!(
        unavailable.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let mut missing = make(std::sync::Arc::clone(&current));
    missing.metadata_input = None;
    missing.dialect_context = None;
    missing.lower(source);
    assert_eq!(
        missing.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let logical_profile = tcl_dialect::DialectProfile::plain_tcl();
    let logical_input = crate::analyser::ResolvedAnalysisInput::new(
        logical_profile,
        logical_profile,
        std::sync::Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(logical_profile.grammar),
    );
    let mut missing_authoring =
        Lowerer::with_config(current.commands(), logical_input.lexer_config())
            .with_resolved_analysis_input(logical_input);
    assert!(missing_authoring.invocation_metadata_context().is_some());
    assert_eq!(
        missing_authoring.source_entry_origin,
        SourceEntryOrigin::Authoring
    );
    missing_authoring.metadata_input = None;
    missing_authoring.logical_source_input = None;
    missing_authoring.dialect_context = None;
    assert!(!missing_authoring.allows_standalone_registry_metadata());
    assert!(missing_authoring.invocation_metadata_context().is_none());
    missing_authoring.lower(source);
    assert_eq!(
        missing_authoring.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let mut stale = make(std::sync::Arc::clone(&current));
    stale.config.strict_quoting = !config.strict_quoting;
    stale.lower(source);
    assert_eq!(
        stale.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let mut foreign = make(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    foreign.lower(source);
    assert_eq!(
        foreign.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn namespace_directive_metadata_uses_frozen_handler_operands() {
    for source in [
        "set pattern {-c}; namespace export OLD; namespace export -clear $pattern -- x",
        "rename namespace ns; set pattern {-c}; ns export OLD; ns export -clear $pattern -- x",
    ] {
        let module = lower_to_ir(source, &reg());
        assert_eq!(
            module.namespace_exports,
            ["OLD", "-c", "--", "x"].map(|pattern| ("::".to_owned(), pattern.to_owned())),
            "{source}",
        );
    }
    let replaced = lower_to_ir("proc namespace args {}; namespace export x", &reg());
    assert_eq!(
        replaced.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn ns_import_in_dead_branch_suppressed() {
    // ``if {0} { namespace import ::evil::* }`` — the
    // import is inside a syntactically-dead branch so it must
    // NOT be recorded.
    let m = lower_to_ir("if {0} { namespace import ::evil::* }", &reg());
    assert_eq!(
        m.namespace_imports.len(),
        0,
        "imports inside dead if{{0}} branch must not be collected, got {:?}",
        m.namespace_imports,
    );
}

#[test]
fn ns_import_in_static_true_else_suppressed() {
    // ``if {1} { ... } else { namespace import ::evil::* }``
    // — the else branch is dead.
    let m = lower_to_ir(
        "if {1} { namespace import ::good::* } else { namespace import ::evil::* }",
        &reg(),
    );
    // Only ``::good::*`` recorded.
    assert_eq!(m.namespace_imports.len(), 1);
    assert_eq!(m.namespace_imports[0].1, "::good::*");
}
