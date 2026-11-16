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

use super::*;
use crate::analyser::ResolvedAnalysisInput;

fn context(class_grammar: bool) -> Arc<tcl_registry::model::ContextRegistry> {
    let mut store = CommandRegistry::build_default();
    let spec = if class_grammar {
        store.get("oo::class").unwrap().clone()
    } else {
        tcl_registry::CommandSpec::DEFAULT
    };
    store.insert(tcl_registry::CommandSpec {
        name: "forge",
        ..spec
    });
    Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl").with_command_store(Arc::new(store)),
    )
}

fn unit(source: &str, context: &Arc<tcl_registry::model::ContextRegistry>) -> CompilationUnit {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    )
}

#[test]
fn genuine_cu_roster_uses_selected_custom_renamed_and_direct_aliased_definers() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let context = context(true);
    for source in [
        "forge create Empty {}",
        "rename forge moved; moved create Empty {}",
        "interp alias {} build {} forge; build create Empty {}",
        "namespace eval ns {forge create Empty {}}",
    ] {
        assert!(
            !source.contains("class") && !source.contains("oo::") && !source.contains("snit::")
        );
        let unit = unit(source, &context);
        let expected = if source.starts_with("namespace") {
            "::ns::Empty"
        } else {
            "::Empty"
        };
        let facts = unit
            .source_class_facts
            .as_ref()
            .expect("genuine produced roster");
        assert!(
            facts
                .for_module(&unit.ir_module, context.commands())
                .unwrap()
                .contains(expected),
            "{source}"
        );
        assert!(
            unit.methods.is_empty(),
            "the empty class has no method-derived roster"
        );
        let actual = collect_known_classes(source, &unit.ir_module, context.commands()).unwrap();
        let bindings = unit
            .ir_module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .positioned_for_module(&unit.ir_module, context.commands())
            .unwrap();
        let expected_scan: HashSet<_> =
            crate::signature_scan::extract_signatures_with_original_bindings(
                source,
                unit.ir_module.lexer_config,
                context.commands(),
                bindings,
            )
            .class_declarations
            .into_iter()
            .filter_map(|declaration| declaration.source_spelling())
            .collect();
        assert_eq!(actual, expected_scan);
    }
}

#[test]
fn selected_roster_preserves_changed_definition_and_captured_prefix_refusals() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let selected = context(true);
    for source in [
        "proc forge args {}; forge create Empty {}",
        "rename forge {}; forge create Empty {}",
        "interp alias {} build {} forge create; build Empty {}",
    ] {
        let unit = unit(source, &selected);
        assert!(
            !collect_known_classes(source, &unit.ir_module, selected.commands())
                .unwrap()
                .contains("::Empty"),
            "{source}"
        );
    }
    let other = context(false);
    let source = "forge create Empty {}";
    let unit = unit(source, &other);
    assert!(
        collect_known_classes(source, &unit.ir_module, other.commands())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn class_source_producer_withdraws_foreign_missing_and_changed_full_owners() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let context = context(true);
    let source = "forge create Empty {}";
    let unit = unit(source, &context);
    assert!(collect_known_classes(source, &unit.ir_module, context.commands()).is_some());
    assert!(
        collect_known_classes("forge create Other {}", &unit.ir_module, context.commands())
            .is_none()
    );
    assert!(
        collect_known_classes(source, &unit.ir_module, self::context(false).commands()).is_none()
    );
    for axis in 0..4 {
        let mut module = unit.ir_module.clone();
        match axis {
            0 => module.retained_source_bindings = None,
            1 => module.lexer_config.strict_quoting ^= true,
            2 => module.source_metadata_input = None,
            _ => {
                module.source =
                    tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes()));
            }
        }
        assert!(
            collect_known_classes(source, &module, context.commands()).is_none(),
            "axis {axis}"
        );
    }
    assert!(
        collect_known_classes("", &self::unit("", &context).ir_module, context.commands())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn retained_roster_projection_shares_empty_custom_and_selected_source_classes() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let context = context(true);
    for source in [
        "forge create Empty {}",
        "rename forge moved; moved create Empty {}",
        "interp alias {} build {} forge; build create Empty {}",
    ] {
        let unit = unit(source, &context);
        assert!(unit.ir_module.methods.is_empty());
        let names = unit
            .retained_source_class_names(context.commands())
            .unwrap();
        assert_eq!(names, &HashSet::from(["::Empty".to_owned()]));
        assert!(std::ptr::eq(
            names,
            unit.source_class_facts
                .as_ref()
                .unwrap()
                .for_module(&unit.ir_module, context.commands())
                .unwrap()
        ));
    }
    let empty = unit("", &context);
    assert!(
        empty
            .retained_source_class_names(context.commands())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn retained_roster_projection_withdraws_changed_cu_module_input_and_store() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let context = context(true);
    let source = "forge create Empty {}";
    let original = unit(source, &context);
    assert!(
        original
            .retained_source_class_names(context.commands())
            .is_some()
    );
    for axis in 0..8 {
        let mut changed = original.clone();
        match axis {
            0 => changed.source = "forge create Other {}".to_owned(),
            1 => {
                changed.ir_module.source =
                    tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes()));
            }
            2 => changed.ir_module.lexer_config.strict_quoting ^= true,
            3 => changed.ir_module.source_metadata_input = None,
            4 => changed.ir_module.source_entry.unknown_entry = true,
            5 => changed.ir_module.retained_source_bindings = None,
            6 => changed.source_class_facts = None,
            _ => {
                changed.source_class_facts =
                    unit("forge create Other {}", &context).source_class_facts;
            }
        }
        assert!(
            changed
                .retained_source_class_names(context.commands())
                .is_none(),
            "axis {axis}"
        );
    }
    assert!(
        original
            .retained_source_class_names(self::context(false).commands())
            .is_none()
    );
}

#[test]
fn source_ssa_missing_entry_never_infers_activation_from_the_report_name() {
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser
        .prepare_analysis_input(tcl_dialect::DialectProfile::plain_tcl().name)
        .unwrap();
    let registry = input.borrowed_context_registry().commands();
    let source = "set value TEXT; puts $value";
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let mut cfg = unit.top_level.cfg.clone();
    cfg.command_binding_sites = Vec::new();
    cfg.namespace_context = None;
    cfg.name = "::invented::procedure".to_owned();
    let entry = function_source_entry(&cfg, None);
    assert_eq!(
        entry.frame_kind,
        crate::var_resolve::VariableFrameKind::Unknown
    );
    assert!(entry.activation.is_none());
    assert!(!entry.namespace_known);
    let first = crate::ssa::build_ssa_with_config(&cfg, registry, input.lexer_config());
    cfg.name = "::top".to_owned();
    assert_eq!(function_source_entry(&cfg, None), entry);
    let second = crate::ssa::build_ssa_with_config(&cfg, registry, input.lexer_config());
    assert_eq!(first.cell_keys(), second.cell_keys());
    cfg.metadata_context = crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    assert_eq!(
        function_source_entry(&cfg, None).frame_kind,
        crate::var_resolve::VariableFrameKind::Unknown
    );
    let standalone = crate::cfg::Function::new("::standalone::model", "entry");
    let compatibility = function_source_entry(&standalone, None);
    assert_eq!(
        compatibility.frame_kind,
        crate::var_resolve::VariableFrameKind::Local
    );
    assert_eq!(
        compatibility.activation.as_deref(),
        Some("::standalone::model")
    );
}
