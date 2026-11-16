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

fn current_source_unit(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Arc<CompilationUnit> {
    let registry = input.borrowed_context_registry().commands();
    Arc::new(
        CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
        .with_interprocedural(registry, Some(input.unit_profile())),
    )
}

#[test]
fn logical_source_correspondence_reuses_an_unchanged_body_after_an_unrelated_edit() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    for body in ["set x 11111", "puts 11111"] {
        let left = current_source_unit(
            &format!("proc a {{}} {{{body}}}\nproc b {{}} {{set y 22}}"),
            &input,
        );
        let right = current_source_unit(
            &format!("\nproc a {{}} {{{body}}}\nproc b {{}} {{set y 999999}}"),
            &input,
        );
        let left_id = *left.ir_module.procedures.unique_display("::a").unwrap().0;
        let right_id = *right.ir_module.procedures.unique_display("::a").unwrap().0;
        let a =
            BodyAnalysisCapsule::retain(Arc::clone(&left), left_id, context.commands()).unwrap();
        let b =
            BodyAnalysisCapsule::retain(Arc::clone(&right), right_id, context.commands()).unwrap();
        assert_ne!(left_id, right_id);
        assert!(a.has_closed_analysis_correspondence(), "source: {body}");
        assert!(!a.has_closed_native_correspondence());
        assert_eq!(a, b, "unchanged exact Logical body: {body}");
        assert!(std::ptr::eq(b.unit(), right.as_ref()));
        assert_eq!(b.body_id(), right_id);
    }
}

#[test]
fn original_document_target_keeps_its_actual_summary_dependencies() {
    let (context, actual) = unit("proc helper {} {return SAFE}\nproc caller {} {helper}");
    let caller = *actual
        .ir_module
        .procedures
        .unique_display("::caller")
        .unwrap()
        .0;
    let capsule = BodyAnalysisCapsule::retain(actual, caller, context.commands()).unwrap();
    assert!(!capsule.has_closed_analysis_correspondence());
}

fn unit(
    source: &str,
) -> (
    Arc<tcl_registry::model::ContextRegistry>,
    Arc<CompilationUnit>,
) {
    let (context, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    (context, Arc::new(unit))
}

#[test]
fn source_body_capsule_keeps_the_actual_module_and_function() {
    let (context, unit) = unit("proc p {argument} {return $argument}");
    let body = *unit.ir_module.procedures.keys().next().unwrap();
    let capsule = BodyAnalysisCapsule::retain(Arc::clone(&unit), body, context.commands())
        .expect("original declaration, actual module and actual body");
    assert!(std::ptr::eq(capsule.unit(), unit.as_ref()));
    assert!(std::ptr::eq(
        capsule.function(),
        std::ptr::from_ref(&unit.procedures[&body])
    ));
    assert_eq!(capsule.body_id(), body);
    assert_eq!(capsule, capsule.clone());
}

#[test]
fn retained_body_registry_keeps_the_actual_source_store() {
    let (context, unit) = unit("proc helper {} {return SAFE}\nproc caller {} {helper}");
    let body = *unit
        .ir_module
        .procedures
        .unique_display("::caller")
        .unwrap()
        .0;
    let capsule = BodyAnalysisCapsule::retain(Arc::clone(&unit), body, context.commands()).unwrap();
    assert!(!capsule.has_closed_analysis_correspondence());
    assert_eq!(capsule.registry_snapshot(), &context.commands().snapshot());
    assert!(Arc::ptr_eq(
        &capsule.registry_snapshot().shared_registry(),
        &context.commands().snapshot().shared_registry()
    ));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert!(BodyAnalysisCapsule::retain(unit, body, foreign.commands()).is_none());
}

#[test]
fn equal_source_and_display_do_not_replace_the_module_owner() {
    let source = "proc p {} {return VALUE}";
    let (context, left) = unit(source);
    let (_, right) = unit(source);
    let left_id = *left.ir_module.procedures.keys().next().unwrap();
    let right_id = *right.ir_module.procedures.keys().next().unwrap();
    assert_eq!(left_id.ordinal(), right_id.ordinal());
    assert_ne!(left_id, right_id);
    assert!(BodyAnalysisCapsule::retain(Arc::clone(&left), right_id, context.commands()).is_none());
    assert_ne!(
        BodyAnalysisCapsule::retain(left, left_id, context.commands()).unwrap(),
        BodyAnalysisCapsule::retain(right, right_id, context.commands()).unwrap(),
    );
}

#[test]
fn changed_source_registry_or_body_owner_withdraws_the_capsule() {
    let source = "proc p {} {return VALUE}";
    let (context, actual) = unit(source);
    let body = *actual.ir_module.procedures.keys().next().unwrap();
    let mut changed_source = (*actual).clone();
    changed_source.source = "proc p {} {return OTHER}".to_owned();
    assert!(
        BodyAnalysisCapsule::retain(Arc::new(changed_source), body, context.commands()).is_none()
    );
    let mut changed_function = (*actual).clone();
    changed_function.procedures.get_mut(&body).unwrap().body_id = None;
    assert!(
        BodyAnalysisCapsule::retain(Arc::new(changed_function), body, context.commands()).is_none()
    );
    let foreign = CommandRegistry::build_default();
    assert!(BodyAnalysisCapsule::retain(actual, body, &foreign).is_none());
}
