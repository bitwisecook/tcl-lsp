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

use crate::interprocedural::test_body_identity::fixture_id;

mod cfg_lattice;
mod compilation_lookup_advice;
mod evaluated_bodies;
mod module_bindings;
mod mutation_summary;
mod source_walk;

use super::*;
use crate::compilation_unit::CompilationUnit;

fn analyse(src: &str) -> (CompilationUnit, CommandRegistry) {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(src, &reg, false);
    (cu, reg)
}

fn source_proof(
    source: &str,
    head: &str,
    dialect: tcl_native_model::InvocationDialect,
) -> SourceInvocationBinding {
    let registry = CommandRegistry::build_default();
    let binding = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..SourceAnalysisOptions::default()
        },
    );
    binding.invocation_at_source(head, u32::try_from(source.rfind(head).unwrap()).unwrap())
}

fn allocation_source(source: &str) -> SourceCommandBindings {
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &CommandRegistry::build_default(),
        SourceAnalysisOptions {
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            )),
            ..SourceAnalysisOptions::default()
        },
    )
}
