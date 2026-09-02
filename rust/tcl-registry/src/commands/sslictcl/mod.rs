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

//! `SslicTcl` — the command pack for the `.sslictcl` TLS-assurance DSL.
//!
//! A `.sslictcl` document is a declarative statement of TLS facts — the
//! certificates, endpoints, trust programs, protocol and cipher catalogues,
//! and the policy that grades them. It is read from the canonical syntax tree
//! and **evaluated nowhere**: the loader never invokes an interpreter, and a
//! `predicate { … }` word is retained verbatim rather than run. This pack is
//! the registry half of that vocabulary, so authoring a document gets hover,
//! completion, signature help, semantic tokens, folding, and document symbols
//! from the same machinery every shipped Tcl command uses.
//!
//! The pack loads only under the `sslictcl` dialect profile — `.sslictcl`
//! files, a document whose mandatory `sslictcl VERSION` header is detected,
//! and anything a user pins with `# tcl-dialect: sslictcl` — so words as
//! generic as `chain`, `policy`, `status`, and `message` never reach an
//! ordinary Tcl document. Base Tcl stays loaded underneath: the grammar says
//! what is *not* an `SslicTcl` declaration.
//!
//! ## Open and closed blocks
//!
//! A block is **open** when a document may state members the loader does not
//! recognise (they are retained as forwards-compatibility notices) and
//! **closed** when it may not. `certificate`, `endpoint`, and `trust-program`
//! are open; every other block is closed. The distinction is a loader rule
//! about unknown *members*, not a registry gate: the pack declares the member
//! vocabulary of both kinds identically, and a consumer that wants to know
//! whether a word is declared asks the block's grammar.
//!
//! ## Two layers, and why both
//!
//! * Each statement word carries a [`CommandSpec`], which is what gives it
//!   hover, completion, and arity checking.
//! * Each *block* statement additionally carries a `definition_body` grammar
//!   naming the words legal inside it, which is what makes the vocabulary
//!   **context-sensitive**: `status` means something inside `protocol { … }`
//!   and `cipher { … }` and nothing inside `hsts { … }`, and the generic
//!   definition-body walker paints, folds, and recurses accordingly with no
//!   walker changes at all.
//!
//! A word that occurs in several blocks with one meaning (`protocols`,
//! `status`) is one spec; grammar membership provides the context.

use crate::arity::Arity;
use crate::hover::HoverSnippet;
use crate::spec::CommandSpec;
use crate::traits::Traits;
use tcl_dialect::model::SpecSurface;

mod blocks;
mod rows;
mod values;

/// The `source` line every `SslicTcl` statement's hover carries.
pub(crate) const SOURCE: &str = "SslicTcl (docs/design/sslictcl-vocabulary.md)";

/// Build the spec for an `SslicTcl` member row — a statement word that takes
/// operands and opens no block.
pub(crate) fn statement(
    name: &'static str,
    arity: Arity,
    summary: &'static str,
    synopsis: &'static [&'static str],
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SSLICTCL),
        arity,
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet,
            source: SOURCE,
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

/// Every `SslicTcl` statement word, as command specs.
#[must_use]
pub fn sslictcl_command_specs() -> Vec<CommandSpec> {
    let mut specs = blocks::specs();
    specs.extend(rows::specs());
    specs
}
