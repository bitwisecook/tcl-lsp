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

//! W116: a stub that shadows a registry command.

use super::*;

#[test]
fn w116_stub_shadow_uses_actual_selected_registry_and_exact_descriptor_data() {
    use std::sync::Arc;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    registry.insert(tcl_registry::CommandSpec {
        name: "overlay_stub_target",
        surface: registry.get("puts").unwrap().surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = Arc::new(
        tcl_registry::model::ingress::context_for_profile(profile)
            .with_command_store(Arc::new(registry)),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    for (name, expected) in [
        ("overlay_stub_target", true),
        ("::overlay_stub_target", true),
        (":overlay_stub_target", false),
        ("::::overlay_stub_target", false),
    ] {
        let source = format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub {name} {{value}}\n# tcl-lsp: stubs-end\n"
        );
        let result = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&source, "tcl8.6");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W116),
            expected,
            "{name}"
        );
    }
}
