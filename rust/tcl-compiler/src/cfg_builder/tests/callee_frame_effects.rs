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

//! Procedure summaries selected per body: alias targets, parameters and summary keys.

use super::*;

#[test]
fn distinct_body_summaries_preserve_conflicting_alias_targets() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Actual whole-module source and existing frame-summary producer.
    let module = lower_module(
        "namespace eval a {proc bind {} {upvar 1 left local; set local 1}}\n\
             namespace eval b {proc bind {} {upvar 1 right local; set local 2}}\n\
             proc forward {} {uplevel 1 [list bind]}",
    );
    let summaries = detect_upvar_procs(&module);
    assert_eq!(
        summaries[&summary_id(&module, "::a::bind")]
            .literal_targets
            .get("local")
            .map(String::as_str),
        Some("left")
    );
    assert_eq!(
        summaries[&summary_id(&module, "::b::bind")]
            .literal_targets
            .get("local")
            .map(String::as_str),
        Some("right")
    );
    assert_eq!(summaries.len(), module.procedures.len());
    let a = summary_id(&module, "::a::bind");
    let b = summary_id(&module, "::b::bind");
    assert_ne!(a, b);
    // The ambiguous constructed call belongs to forward's source receipt.
    let forwarded = &summaries[&summary_id(&module, "::forward")];
    assert!(forwarded.literal_targets.is_empty());
    assert!(forwarded.caller_frame_opaque_writes && forwarded.caller_frame_opaque_reads);
}

#[test]
fn summary_parameters_retain_each_original_signature() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Defaults, order and variadic position belong to the original header.
    let module = lower_module(
        "namespace eval a {proc p {{name left} args} {}; proc q {{name shared} args} {}; proc r {a b} {}; proc s {a args} {}}\n\
             namespace eval b {proc p {{name right} args} {}; proc q {{name shared} args} {}; proc r {b a} {}; proc s {args a} {}}",
    );
    let (_, params, _, _, _) = prepare_cfg_context(&module);
    assert_eq!(params.len(), 8);
    for label in ["::a::p", "::b::p", "::a::q", "::b::q"] {
        assert_eq!(
            params.get(&summary_id(&module, label)),
            Some(&vec!["name".into(), "args".into()])
        );
    }
    assert_eq!(
        params.get(&summary_id(&module, "::a::r")),
        Some(&vec!["a".into(), "b".into()])
    );
    assert_eq!(
        params.get(&summary_id(&module, "::b::r")),
        Some(&vec!["b".into(), "a".into()])
    );
    assert_eq!(
        params.get(&summary_id(&module, "::a::s")),
        Some(&vec!["a".into(), "args".into()])
    );
    assert_eq!(
        params.get(&summary_id(&module, "::b::s")),
        Some(&vec!["args".into(), "a".into()])
    );
    let original = module
        .procedures
        .get(&summary_id(&module, "::a::p"))
        .unwrap();
    let distinct = module
        .procedures
        .get(&summary_id(&module, "::b::p"))
        .unwrap();
    assert_ne!(original.params_raw, distinct.params_raw);
}

#[test]
fn body_summary_selection_declines_foreign_id_at_the_original_declaration() {
    let mut module = lower_module("proc writer {} {upvar 1 x local; set local 1}; writer");
    let foreign = lower_module("proc writer {} {upvar 1 x local; set local 1}; writer");
    let own_id = summary_id(&module, "::writer");
    let foreign_id = summary_id(&foreign, "::writer");
    assert_ne!(own_id, foreign_id);
    let body = module.procedures.get(&own_id).unwrap().clone();
    let call = module.top_level.statements.last().unwrap().clone();
    let original = module
        .top_level
        .retained_source_tokens_for_statement(module.top_level.statements.last().unwrap())
        .unwrap()
        .clone();
    let registry = module.resolved_registry().snapshot().shared_registry();
    let owners = OriginalProcedureSummaryOwners::for_module(&module, &registry);
    assert_eq!(owners.calls(&original, &registry).unwrap()[0].0, own_id);
    module.procedures.insert(foreign_id, body);
    let owners = OriginalProcedureSummaryOwners::for_module(&module, &registry);
    assert!(owners.calls(&original, &registry).is_none());
    let prepared = prepare_cfg_context_bundle(&module, &registry);
    let mut builder = CfgBuilder::new_with_upvars_and_classes(
        true,
        prepared.context.0.clone(),
        prepared.context.1.clone(),
        prepared.context.2.clone(),
        prepared.context.3.clone(),
        &registry,
        prepared.command_classes.clone(),
    )
    .with_procedure_summary_owners(prepared.context.4.clone());
    builder
        .command_binding_sites
        .extend(module.top_level.command_binding_sites.iter().cloned());
    assert!(builder.direct_upvar_effects(&call).opaque_arguments);
}

#[test]
fn summary_keys_decline_one_changed_member_and_foreign_module_source() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Removing original correspondence cannot leave a false unique winner.
    let module = lower_module(
        "namespace eval a {proc p {name} {upvar 1 $name local}}\n\
             namespace eval b {proc p {name} {upvar 1 $name local}}",
    );
    let mut changed = module.clone();
    {
        let id = fixture_id(&changed.procedures, "::b::p");
        changed.procedures.get_mut(&id)
    }
    .unwrap()
    .params_raw
    .push_str(" extra");
    let (summaries, params, _, _, _) = prepare_cfg_context(&changed);
    let unchanged = summary_id(&changed, "::a::p");
    let changed_id = summary_id(&changed, "::b::p");
    assert!(params.contains_key(&unchanged));
    assert!(!params.contains_key(&changed_id));
    assert!(summaries[&changed_id].literal_targets.is_empty());
    assert!(summaries[&changed_id].param_targets.is_empty());
    assert!(summaries[&changed_id].caller_frame_opaque_reads);
    let mut foreign = module;
    foreign.source = tcl_lexer::SourceImage::document("proc replacement {} {}");
    let (summaries, params, _, _, _) = prepare_cfg_context(&foreign);
    assert!(params.is_empty());
    assert!(
        summaries
            .values()
            .all(|summary| summary.literal_targets.is_empty()
                && summary.param_targets.is_empty()
                && summary.caller_frame_opaque_writes
                && summary.caller_frame_opaque_reads)
    );
}
