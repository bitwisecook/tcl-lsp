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
use std::sync::Arc;

#[test]
fn factory_return_scalar_names_use_whole_original_words() {
    for (value, expected) in [
        ("${$literal}", Some("$literal")),
        ("${[literal]}", Some("[literal]")),
        ("${ with space }", Some(" with space ")),
        ("\"${ with space }\"", Some(" with space ")),
        ("{${$literal}}", None),
        ("\"${$literal} tail\"", None),
        ("$array(key)", None),
    ] {
        let source = format!("proc source_return {{}} {{return {value}}}");
        let fixture = crate::provider_fixtures::original_native_module(&source, "tcl8.6");
        let (_, module) = &*fixture;
        let mut procedures = module.procedures.values();
        let procedure = procedures.next().expect("actual source return body");
        assert!(procedures.next().is_none());
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .expect("actual retained return source words");
        let word = original_source_return_operand(module, module.resolved_registry(), tokens)
            .expect("actual selected original return operand");
        assert_eq!(
            returned_scalar_reference_name(word, module.native_lexer_config()).as_deref(),
            expected,
            "{source}"
        );
    }
}

fn assert_factory_source_metadata_refusals(
    analyser: &Analyser,
    cu: &mut crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
) {
    let original_profile = cu.ir_module.dialect_profile;
    cu.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::plain_tcl());
    assert!(
        analyser
            .compute_factory_object_ranges(cu, registry)
            .ranges
            .is_empty(),
        "changed actual Module profile cannot borrow the input"
    );
    cu.ir_module.dialect_profile = original_profile;
    let original_config = cu.ir_module.lexer_config;
    cu.ir_module.lexer_config.expand_syntax = !original_config.expand_syntax;
    assert!(
        analyser
            .compute_factory_object_ranges(cu, registry)
            .ranges
            .is_empty(),
        "changed actual Module syntax cannot borrow the input"
    );
    cu.ir_module.lexer_config = original_config;
    cu.source.push(' ');
    assert_eq!(
        analyser.compute_factory_object_ranges(cu, registry).ranges,
        [] as [(
            tcl_core_types::BodyFrameId,
            u32,
            u32,
            std::collections::HashSet<std::string::String>
        ); 0]
    );
}

#[test]
fn factory_may_advice_uses_actual_availability_and_exact_supplied_source() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These Registry declarations supply possible source result shapes,
    // never an actual factory implementation, allocation or return value.
    let source = "gated-store handle [possible-object]";
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "gated-store",
        surface: registry.get("dict").unwrap().surface,
        ..registry.get("set").unwrap().clone()
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "possible-object",
        return_type: Some(tcl_registry::TclType::Object),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let store = Arc::new(registry);
    for (environment, expected) in [("tcl8.4", false), ("tcl9.0", true)] {
        let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(Arc::clone(&store)),
        );
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let mut analyser = Analyser::new().with_resolved_input(input);
        let analysis = analyser.analyse(source, environment);
        let mut cu = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            Arc::clone(&context),
        );
        let advice = analyser.compute_factory_object_ranges(&cu, context.commands());
        assert_eq!(
            advice
                .ranges
                .iter()
                .any(|(_, _, _, names)| names.contains("handle")),
            expected,
            "{environment}"
        );
        let foreign = tcl_registry::CommandRegistry::build_default();
        assert_eq!(
            analyser.compute_factory_object_ranges(&cu, &foreign).ranges,
            [] as [(
                tcl_core_types::BodyFrameId,
                u32,
                u32,
                std::collections::HashSet<std::string::String>
            ); 0]
        );
        analyser.result.resolved_input = None;
        assert_eq!(
            analyser
                .compute_factory_object_ranges(&cu, context.commands())
                .ranges,
            [] as [(
                tcl_core_types::BodyFrameId,
                u32,
                u32,
                std::collections::HashSet<std::string::String>
            ); 0]
        );
        analyser.result = analysis.clone();
        cu.ir_module.source_metadata_input = None;
        assert_eq!(
            analyser
                .compute_factory_object_ranges(&cu, context.commands())
                .ranges,
            [] as [(
                tcl_core_types::BodyFrameId,
                u32,
                u32,
                std::collections::HashSet<std::string::String>
            ); 0]
        );
        cu.ir_module.source_metadata_input = analysis.resolved_input.clone();
        assert_factory_source_metadata_refusals(&analyser, &mut cu, context.commands());
    }
}
