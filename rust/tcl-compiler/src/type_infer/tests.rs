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
use crate::cfg::{Block, Function};
use crate::ir::Statement;
use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
use std::collections::HashSet;
use tcl_lexer::Span;

mod caller_safe_factories;
mod container_elements;
mod definition_types;
mod expression_types;
mod literals;
mod propagation;
mod scope_aliases;
mod source_metadata;

fn original_factory_tokens(
    source: &str,
    version: tcl_dialect::TclVersion,
    marker: &str,
    head: &str,
) -> crate::ir::CommandTokens {
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_profile_name()).commands();
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.rfind(marker).expect("original invocation")).unwrap();
    let point = bindings.invocation_at_source(head, offset);
    let mut tokens = point
        .original_recorded_command_tokens()
        .expect("original retained invocation vector");
    bindings.stamp_original_tokens(&mut tokens);
    tokens
}

fn standalone_factory_metadata(
    version: tcl_dialect::TclVersion,
) -> TypePropagationMetadata<'static> {
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    let context =
        tcl_registry::model::SemanticContext::for_environment(version.dialect_profile_name());
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_profile_name()).commands();
    TypePropagationMetadata {
        registry,
        context: Some(context.into()),
        config: tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        numbers: registry.numbers(),
    }
}

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.0").expect("native Tcl fixture"),
    )
}
