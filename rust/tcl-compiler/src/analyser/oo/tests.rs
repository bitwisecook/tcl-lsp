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
use crate::analyser::types::MemberRetractionRecord;

mod definition_aborts;
mod member_effects;
mod member_references;
mod member_visibility;
mod method_bodies;
mod retained_member_source;
mod snit_and_itcl;
mod unknown_proc;
mod wrapper_blocks;

/// The `TclOO` definition-body grammar the `apply_oo_subcommand` /
/// `extract_method_def` helpers read their member argument layout from —
/// the same `&'static` the analyser fetches from the registry at runtime.
fn tcloo() -> &'static DefinitionBodyGrammar {
    &tcl_registry::definer::TCLOO_GRAMMAR
}

fn class() -> ClassDef {
    ClassDef {
        name: "C".to_string(),
        qualified_name: "::C".to_string(),
        name_span: tcl_lexer::Span::new(0, 0),
        body_span: tcl_lexer::Span::new(0, 0),
        ..Default::default()
    }
}

/// The `W315` messages an analysis produced, in emission order — the
/// "this class definition cannot run" reports.
fn w315_messages(r: &super::super::types::AnalysisResult) -> Vec<String> {
    r.diagnostics
        .iter()
        .filter(|d| d.code == tcl_core_types::DiagCode::W315)
        .map(|d| d.message.clone())
        .collect()
}

fn has_w315(r: &super::super::types::AnalysisResult) -> bool {
    !w315_messages(r).is_empty()
}

fn tok(span: (u32, u32)) -> Token {
    Token::new(TokenType::Esc, tcl_lexer::Span::new(span.0, span.1))
}

fn str_tok(span: (u32, u32)) -> Token {
    Token {
        kind: TokenType::Str,
        span: tcl_lexer::Span::new(span.0, span.1),
        content_offset: 1,
        in_quote: false,
    }
}
