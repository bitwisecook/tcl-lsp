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

//! Semantic tokens, end-to-end against the packaged server. A fresh document is
//! opened per case and `textDocument/semanticTokens/full` is requested once,
//! then the delta-encoded `data` is decoded and the legend (advertised in
//! `initialize`) maps type/modifier indices back to names — so a legend or
//! encoder drift in the shipped artifact is caught here.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

#[path = "semantic_tokens/command_roles.rs"]
mod command_roles;
#[path = "semantic_tokens/config_documents.rs"]
mod config_documents;
#[path = "semantic_tokens/lexical_tokens.rs"]
mod lexical_tokens;
#[path = "semantic_tokens/object_dispatch.rs"]
mod object_dispatch;
#[path = "semantic_tokens/script_walk.rs"]
mod script_walk;
#[path = "semantic_tokens/subtokens.rs"]
mod subtokens;

use subtokens::is_re_type;

/// A decoded token with its resolved type-name.
#[derive(Debug, Clone)]
struct TypedToken {
    line: i64,
    char: i64,
    length: i64,
    ttype: String,
    modifiers: i64,
}

/// The `tokenTypes` legend advertised in `initialize`.
fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("tokenTypes legend")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect()
}

/// The `tokenModifiers` legend advertised in `initialize`.
fn modifiers(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenModifiers"]
        .as_array()
        .expect("tokenModifiers legend")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect()
}

/// Decode `uri`'s semantic tokens, mapping each type index to its legend name.
fn typed(lsp: &mut Lsp, legend: &[String], uri: &str) -> Vec<TypedToken> {
    // Converged, not first-response: several tests here assert on the
    // *enriched* tier (regex-source retagging, user-class method resolution),
    // which the server may legitimately deliver a refresh late when the 40 ms
    // fast-path budget loses. Reading only the first response makes those
    // assertions a bet on how much CPU the machine had — see
    // `Lsp::semantic_tokens_settled`.
    let raw = lsp.semantic_tokens_settled(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .map(|t| TypedToken {
            line: t.line,
            char: t.char,
            length: t.length,
            ttype: legend[usize::try_from(t.ttype).unwrap()].clone(),
            modifiers: t.modifiers,
        })
        .collect()
}

/// Open `source` in a fresh URI, block until ready, return the URI.
fn open_doc(lsp: &mut Lsp, source: &str) -> String {
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, source);
    uri
}

/// The source text a decoded token covers (single-line tokens only).
fn covered<'a>(source: &'a str, tok: &TypedToken) -> &'a str {
    let line = source
        .split('\n')
        .nth(usize::try_from(tok.line).unwrap())
        .unwrap_or("");
    let start = usize::try_from(tok.char).unwrap();
    let end = usize::try_from(tok.char + tok.length).unwrap();
    // Slicing by UTF-16-ish char offset; the covered() cases here are all ASCII.
    line.get(start..end).unwrap_or("")
}

/// The bit mask for a modifier name.
fn modifier_bit(mods: &[String], name: &str) -> i64 {
    let idx = mods
        .iter()
        .position(|m| m == name)
        .expect("modifier in legend");
    1i64 << idx
}
