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

#[test]
fn unavailable_write_and_delete_fence_held_values_without_changing_standalone() {
    let registry = CommandRegistry::build_default();
    let source = "set x 2\nunset x\nputs $x\n";
    let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
        source,
        &registry,
        false,
        tcl_lexer::LexerConfig::default(),
    );
    let mut unavailable_cfg = unit.top_level.cfg.clone();
    unavailable_cfg.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let mut entry = ResolveContext::for_function("::top");
    entry.define_literal("x", "held-before-unknown-effects", &registry);
    let bindings = vec!["x".to_owned()];
    let unavailable = build_ssa_with_context_for_entry(
        &unavailable_cfg,
        &registry,
        tcl_lexer::LexerConfig::default(),
        entry.clone(),
        Some(&bindings),
    );
    let symbol = unavailable
        .var_symbol("x")
        .expect("the held entry binding is represented");

    for expected in ["set x 2", "unset x"] {
        let point = unavailable_cfg
            .blocks
            .iter()
            .find_map(|(&block, data)| {
                data.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        let span = statement.span().as_range();
                        (source
                            .get(span)
                            .is_some_and(|text| text.starts_with(expected)))
                        .then_some((block, index))
                    })
            })
            .unwrap_or_else(|| panic!("missing source operation {expected}"));
        let versions = unavailable
            .value_clobbers
            .get(&point.0)
            .and_then(|markers| markers.get(&point.1))
            .and_then(|versions| versions.get(&symbol))
            .copied()
            .unwrap_or_else(|| panic!("held value crosses {expected} without a fresh SSA key"));
        assert_ne!(
            versions.0, versions.1,
            "{expected} must poison the held value"
        );
    }

    let set_point = unavailable_cfg
        .blocks
        .iter()
        .find_map(|(&block, data)| {
            data.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    let span = statement.span().as_range();
                    source
                        .get(span)
                        .is_some_and(|text| text.starts_with("set x 2"))
                        .then_some((block, index))
                })
        })
        .unwrap();
    let after_unavailable_write =
        SsaSourceView::at_statement(&unavailable, set_point.0, set_point.1 + 1)
            .reaching_binding("x", &registry)
            .expect("the following point retains the variable lookup");
    assert_eq!(
        after_unavailable_write.version, None,
        "the stale held literal must not cross the unavailable write"
    );

    let standalone = build_ssa_with_context_for_entry(
        &unit.top_level.cfg,
        &registry,
        tcl_lexer::LexerConfig::default(),
        entry,
        Some(&bindings),
    );
    assert!(
        standalone.value_clobbers.is_empty(),
        "explicit Standalone keeps its selected catalogue transfer path"
    );
}

#[test]
fn unavailable_branch_command_fences_values_live_after_the_terminator() {
    let registry = CommandRegistry::build_default();
    let source = "if {[expr {[eval {set x 1}]}]} {}\nputs $x\n";
    let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
        source,
        &registry,
        false,
        tcl_lexer::LexerConfig::default(),
    );
    let mut cfg = unit.top_level.cfg.clone();
    cfg.metadata_context = crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let mut entry = ResolveContext::for_function("::top");
    entry.define_literal("x", "held-before-branch-command", &registry);
    let ssa = build_ssa_with_context_for_entry(
        &cfg,
        &registry,
        tcl_lexer::LexerConfig::default(),
        entry,
        Some(&["x".to_owned()]),
    );
    let symbol = ssa.var_symbol("x").expect("held entry cell is represented");
    let (block, data) = cfg
        .blocks
        .iter()
        .find(|(_, data)| {
            matches!(
                data.terminator.as_ref(),
                Some(crate::cfg::Terminator::Branch { .. })
            )
        })
        .expect("the command expression remains the original CFG branch");
    let (prior, fresh) = ssa
        .value_clobbers
        .get(block)
        .and_then(|markers| markers.get(&usize::MAX))
        .and_then(|versions| versions.get(&symbol))
        .copied()
        .expect("unavailable branch effects fence the outgoing value");
    assert_ne!(prior, fresh);
    assert_eq!(
        ssa.blocks[block].exit_versions.get(&symbol).copied(),
        Some(prior),
        "the condition still reads the pre-effect version"
    );
    assert!(data.terminator.is_some());
}

#[test]
fn unavailable_return_command_fences_values_live_in_exception_handler() {
    let registry = CommandRegistry::build_default();
    let source = "try {return [eval {set x 1}]} on error {message options} {puts $x}\n";
    let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
        source,
        &registry,
        false,
        tcl_lexer::LexerConfig::default(),
    );
    let mut cfg = unit.top_level.cfg.clone();
    cfg.metadata_context = crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let mut entry = ResolveContext::for_function("::top");
    entry.define_literal("x", "held-before-return-command", &registry);
    let ssa = build_ssa_with_context_for_entry(
        &cfg,
        &registry,
        tcl_lexer::LexerConfig::default(),
        entry,
        Some(&["x".to_owned()]),
    );
    let symbol = ssa.var_symbol("x").expect("held entry cell is represented");
    let (block, _) = cfg
        .blocks
        .iter()
        .find(|(_, data)| {
            matches!(
                data.terminator.as_ref(),
                Some(crate::cfg::Terminator::Return { .. })
            )
        })
        .expect("the return command remains an original CFG terminator");
    assert!(
        !cfg.block_successors(*block).is_empty(),
        "the return retains its real exception-handler successor"
    );
    let (prior, fresh) = ssa
        .value_clobbers
        .get(block)
        .and_then(|markers| markers.get(&usize::MAX))
        .and_then(|versions| versions.get(&symbol))
        .copied()
        .expect("unavailable return effects fence the handler-visible value");
    assert_ne!(prior, fresh);
    assert_eq!(
        ssa.blocks[block].exit_versions.get(&symbol).copied(),
        Some(prior),
        "the return value reads before the handler-edge freshness barrier"
    );
}

#[test]
fn no_profile_standalone_keeps_legacy_registry_variable_roles() {
    let registry = CommandRegistry::build_default();
    assert!(registry.profile().is_none());
    let source = "set x\n";
    let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
        source,
        &registry,
        false,
        tcl_lexer::LexerConfig::default(),
    );
    let cfg = &unit.top_level.cfg;
    assert!(cfg.metadata_context.is_standalone());
    let statement = cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| {
            matches!(statement, Statement::Call { tokens: Some(tokens), .. }
                    if tokens.argv_texts.first().is_some_and(|name| name == "set"))
        })
        .expect("the standalone CFG retains the original set invocation");
    let selection = SsaInvocationContext::supplied_with_standalone_owner(
        &registry,
        cfg.metadata_context
            .metadata_context(&registry)
            .and_then(crate::registry_invocation::InvocationMetadataSelection::context),
        cfg.metadata_context.is_standalone(),
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: true,
            ..VarScanOptions::default()
        },
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        uses_of_classified_in_context(statement, &mut scanner, selection)
            .iter()
            .any(|(name, class)| name == "x" && *class == UseClass::Name),
        "explicit no-profile Standalone keeps registry variable-read roles"
    );

    let mut unavailable_cfg = cfg.clone();
    unavailable_cfg.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let unavailable = SsaInvocationContext::supplied_with_standalone_owner(
        &registry,
        unavailable_cfg
            .metadata_context
            .metadata_context(&registry)
            .and_then(crate::registry_invocation::InvocationMetadataSelection::context),
        unavailable_cfg.metadata_context.is_standalone(),
    );
    assert!(!unavailable.standalone);
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: true,
            ..VarScanOptions::default()
        },
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        uses_of_classified_in_context(statement, &mut scanner, unavailable)
            .iter()
            .all(|(name, _)| name != "x"),
        "unavailable owner cannot acquire Standalone variable-read roles"
    );
}
