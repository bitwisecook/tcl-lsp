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

fn build_tk_module(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::ir::Module,
) {
    let environment = tcl_registry::model::ingress::resolve_environment("tk");
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_resolved_analysis_input(input);
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    (context, module)
}

#[test]
fn interprocedural_option_summary_uses_selected_alias_target_receipt() {
    let source =
        "interp alias {} linkedEntry {} entry .e -textvariable; proc p {} {linkedEntry ::bound}";
    let (context, module) = build_tk_module(source);
    let registry = context.commands();
    let id = *module
        .procedures
        .keys()
        .next()
        .expect("one source procedure");
    let writes = tainted_global_writes(
        &module,
        registry,
        &global_instance_classes(&module, registry),
    );
    assert!(writes[&id].contains("::bound"));

    let ordinary_source = "proc p {} {entry .e -state normal}";
    let (context, ordinary) = build_tk_module(ordinary_source);
    let registry = context.commands();
    let id = *ordinary
        .procedures
        .keys()
        .next()
        .expect("one source procedure");
    let writes = tainted_global_writes(
        &ordinary,
        registry,
        &global_instance_classes(&ordinary, registry),
    );
    assert!(
        !writes
            .get(&id)
            .is_some_and(|names| names.contains("::normal"))
    );
}
