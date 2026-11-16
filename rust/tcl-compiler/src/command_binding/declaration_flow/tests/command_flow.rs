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

//! Lifecycle subcommands schedule their original phase scripts.

use super::*;

#[test]
fn lifecycle_subcommand_schedules_its_original_phase_scripts() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let mut descriptor = registry.get("tcltest::test").unwrap().clone();
    descriptor.name = "offset-lifecycle";
    descriptor.required_package = None;
    descriptor.subcommands = Box::leak(Box::new([tcl_registry::SubCommand {
        name: "run",
        arity: descriptor.arity,
        arg_role_resolver: descriptor.arg_role_resolver,
        arg_role_resolver_roles: descriptor.arg_role_resolver_roles,
        body_kind: descriptor.body_kind,
        body_execution: descriptor.body_execution,
        completion: descriptor.completion,
        ..tcl_registry::SubCommand::DEFAULT
    }]));
    registry.insert(descriptor);
    let loader = crate::lowering::stock_body_provider_loader(
        tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
        None,
    )
    .unwrap();
    let wrapper =
        "offset-lifecycle run n d -cleanup {puts CLEANUP} -body {puts BODY} -setup {puts SETUP}";
    let source = format!("package require tcltest\nproc p {{}} {{{wrapper}}}");
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let bindings = super::super::super::SourceCommandBindings::analyse_with_options(
        &source,
        config,
        &registry,
        super::super::super::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            trusted_package_loaders: std::slice::from_ref(&loader),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.find(wrapper).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(wrapper, offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(&source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let mut builder = Builder {
        inventory,
        registry: &registry,
        config,
        dialect: tcl_registry::InvocationDialect::of_profile(profile),
        entry: &frame,
        purpose: DeclarationFlowPurpose::Reads,
        representation_realm: None,
        representation_loops: Vec::new(),
        current_representation_body: None,
        nodes: Vec::new(),
        jobs: Vec::new(),
        conditional_overwrites: Vec::new(),
        read_sites: BTreeSet::new(),
    };
    let (selected, lifecycle) = builder.selection(&tokens, site).unwrap();
    let lifecycle = lifecycle.unwrap();
    assert_eq!(lifecycle.invocation.argument_offset, 1);
    let after = builder.placeholder();
    builder.lifecycle_flow(site, &selected, &lifecycle, after, Exits::all(after));
    let scripts = builder
        .jobs
        .iter()
        .map(|job| job.source.text.try_text().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(scripts, ["puts CLEANUP", "puts BODY", "puts SETUP"]);
}
