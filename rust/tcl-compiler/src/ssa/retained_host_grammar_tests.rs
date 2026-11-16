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
fn public_ssa_reborrows_the_supplied_host_grammar_for_return_reads() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let registry = context.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("jim");
    assert_ne!(
        config.normalized(),
        tcl_lexer::LexerConfig::for_profile(registry.profile()).normalized()
    );
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    let source = "return $($::value)";
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let cfg = &unit.top_level.cfg;
    assert_eq!(cfg.metadata_context.source_analysis_input(), Some(&input));
    assert!(cfg.blocks.values().any(|block| matches!(
        &block.terminator,
        Some(crate::cfg::Terminator::Return { value: Some(value), .. }) if value == "$($::value)"
    )));
    let explicit = build_ssa_with_config(cfg, registry, config);
    assert!(
        explicit
            .point_symbols
            .values()
            .any(|symbols| symbols.contains_key("::value"))
    );
    assert_eq!(build_ssa(cfg, registry), explicit);
}
