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

//! The outline of a **declaration document**.
//!
//! A declaration document is one whose dialect states facts rather than
//! running a script — today that is the `sslictcl` environment, whose
//! defining property is that nothing in a `.sslictcl` file is ever evaluated.
//! Its *blocks* are its structure: each block statement is a declared entity,
//! so it is an outline entry, and a nested block is a child of the one that
//! contains it. A script document's outline comes from the analyser's scope
//! tree instead (procs, classes, namespaces), which a declaration document
//! has none of.
//!
//! The walk is pure registry data. A block is a statement whose spec carries
//! a [`DefinitionBodyGrammar`], its name is the word the spec marks
//! [`ArgRole::Name`], and its body is the word marked [`ArgRole::Body`]. No
//! declaration is named here, and a word added to the pack appears in the
//! outline with no change to this file.
//!
//! Which *vocabulary* a position admits is a different question with its own
//! owner: [`crate::oo_body::definition_grammar_at`], which completion and the
//! token walk read, and which answers for every definition body rather than
//! only a declaration document's.
//!
//! [`DefinitionBodyGrammar`]: tcl_registry::definer::DefinitionBodyGrammar
//! [`ArgRole::Name`]: tcl_registry::arg_role::ArgRole::Name
//! [`ArgRole::Body`]: tcl_registry::arg_role::ArgRole::Body

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_dialect::DialectProfile;
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::arg_role::ArgRole;

/// Nesting cap for the block walk, mirroring the folding walker's own limit:
/// a hand-written declaration document nests three or four deep, and the cap
/// is defence against a pathological or generated one.
const MAX_BLOCK_DEPTH: u32 = 32;

/// One block declaration and the blocks nested inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDeclaration {
    /// The statement word that opens the block (`endpoint`, `hsts`, …).
    pub keyword: String,
    /// The declared name, when the statement's spec marks one
    /// ([`ArgRole::Name`]) and the document wrote a literal there.
    pub name: Option<String>,
    /// The whole statement, head through closing brace.
    pub range: Span,
    /// The head word — what an editor highlights in the breadcrumb.
    pub selection: Span,
    /// Blocks declared inside this one's body, in source order.
    pub children: Vec<BlockDeclaration>,
}

impl BlockDeclaration {
    /// The outline label: `endpoint /Common/www`, or just `grade` for a
    /// block that declares no name.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.name {
            Some(name) => format!("{} {name}", self.keyword),
            None => self.keyword.clone(),
        }
    }
}

/// Whether documents of `dialect` are declarations rather than scripts.
///
/// The same resolved-authoring-surface question
/// [`crate::sslictcl_diagnostics::applies_to`] asks, and deliberately the
/// same answer: the property both surfaces here depend on — that the document
/// is never evaluated — is precisely what makes the `SslicTcl` loader the
/// authority over it. When a second such environment appears, both read the
/// widened predicate rather than growing a list of dialects apiece.
#[must_use]
pub fn is_declaration_document(dialect: &DialectProfile) -> bool {
    crate::sslictcl_diagnostics::applies_to(dialect)
}

/// Every block declaration in `source`, in source order, nested.
///
/// Empty for a document of a dialect that is not a declaration document — a
/// script's outline is the analyser's, and this walk would double it.
#[must_use]
pub fn declarations(source: &str, dialect: &'static DialectProfile) -> Vec<BlockDeclaration> {
    if source.is_empty() || !is_declaration_document(dialect) {
        return Vec::new();
    }
    let registry = crate::registry_for_dialect_profile(dialect);
    // The profile in hand already carries the document's grammar; resolving
    // its name a second time is the drift the ingress settlement forbids
    // (`docs/design/dialect-profile-model.md` §2.5).
    let config = LexerConfig::from_grammar(dialect.grammar);
    collect(source, source, 0, 0, registry, &config)
}

/// The body-word slice of a `{ … }` argument token, in original-source
/// coordinates: `(inner_text, absolute_start)`.
///
/// The lexer's `Str` span starts at the opening `{` and, for a closed
/// non-empty body, ends just before the closing `}`; an empty `{}` clamps
/// onto the closer, which must be trimmed off before re-lexing (issue #527).
fn body_slice<'a>(original: &'a str, token: &tcl_lexer::Token) -> Option<(&'a str, u32)> {
    let content_start = token.span.start() as usize + token.content_offset as usize;
    let raw_end = token.span.end() as usize;
    let content_end = if raw_end > content_start
        && raw_end - content_start == 1
        && original.as_bytes().get(raw_end - 1) == Some(&b'}')
    {
        content_start
    } else {
        raw_end
    };
    if content_end <= content_start {
        return None;
    }
    let start = u32::try_from(content_start).ok()?;
    Some((original.get(content_start..content_end)?, start))
}

/// One statement's block facts, when it opens one: its grammar and the
/// argument indices holding its name and its body.
struct BlockShape {
    name_index: Option<usize>,
    body_index: usize,
}

fn block_shape(
    head: &str,
    args: &[&str],
    registry: &'static CommandRegistry,
) -> Option<BlockShape> {
    crate::oo_body::outer_definition_grammar(head, args, registry)?;
    let body_index = *registry
        .arg_indices_for_role(head, args, ArgRole::Body)
        .first()?;
    let name_index = registry
        .arg_indices_for_role(head, args, ArgRole::Name)
        .first()
        .copied();
    Some(BlockShape {
        name_index,
        body_index,
    })
}

fn collect(
    original: &str,
    body: &str,
    base_offset: u32,
    depth: u32,
    registry: &'static CommandRegistry,
    config: &LexerConfig,
) -> Vec<BlockDeclaration> {
    if depth >= MAX_BLOCK_DEPTH {
        return Vec::new();
    }
    let mut out = Vec::new();
    for cmd in segment_commands_with_offset_and_config(body, base_offset, config.at_depth(depth)) {
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        let Some(shape) = block_shape(cmd.name(), &args, registry) else {
            continue;
        };
        let tokens = cmd.arg_tokens();
        let Some(&body_token) = tokens.get(shape.body_index) else {
            continue;
        };
        if body_token.kind != TokenType::Str {
            continue;
        }
        let children = body_slice(original, &body_token).map_or_else(Vec::new, |(inner, start)| {
            collect(original, inner, start, depth + 1, registry, config)
        });
        let Some(&head_token) = cmd.argv.first() else {
            continue;
        };
        out.push(BlockDeclaration {
            keyword: cmd.name().to_owned(),
            name: shape
                .name_index
                .and_then(|index| args.get(index))
                .filter(|name| !name.is_empty())
                .map(|name| (*name).to_owned()),
            range: cmd.span,
            selection: head_token.span,
            children,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_for_dialect;

    const DOC: &str = "sslictcl 1\n\
                       endpoint /Common/www {\n\
                       \x20   hostname www.example.test\n\
                       \x20   hsts {\n\
                       \x20       enabled true\n\
                       \x20   }\n\
                       }\n\
                       policy corporate {\n\
                       \x20   check modern {\n\
                       \x20       severity error\n\
                       \x20   }\n\
                       \x20   grade {\n\
                       \x20       minimum A\n\
                       \x20   }\n\
                       }\n";

    fn sslictcl() -> &'static DialectProfile {
        profile_for_dialect("sslictcl")
    }

    #[test]
    fn every_block_is_an_outline_entry_under_the_block_that_contains_it() {
        let outline = declarations(DOC, sslictcl());
        let labels: Vec<String> = outline.iter().map(BlockDeclaration::label).collect();
        assert_eq!(labels, vec!["endpoint /Common/www", "policy corporate"]);
        assert_eq!(
            outline[0]
                .children
                .iter()
                .map(BlockDeclaration::label)
                .collect::<Vec<_>>(),
            vec!["hsts"],
        );
        assert_eq!(
            outline[1]
                .children
                .iter()
                .map(BlockDeclaration::label)
                .collect::<Vec<_>>(),
            vec!["check modern", "grade"],
        );
    }

    #[test]
    fn a_script_document_has_no_block_outline() {
        let script = "oo::class create Greeter {\n    method hi {} { return 1 }\n}\n";
        assert!(declarations(script, profile_for_dialect("tcl9.0")).is_empty());
    }
}
