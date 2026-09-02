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
//!
//! ## One keyword rule: the grammar, never a global trait
//!
//! No spec in this pack carries `Traits::LANGUAGE_KEYWORD`. The token walker
//! paints a head as a keyword when it is a member of the *enclosing* grammar,
//! and falls back to the trait otherwise — so a spec carrying the trait is
//! painted as a valid keyword wherever it appears, which is exactly what the
//! context sensitivity exists to prevent: a misplaced `hostname` inside an
//! `hsts { … }`, or at the top level, would still look correct.
//!
//! Every word here is reachable as a grammar member, including the nine
//! top-level declarations and the `sslictcl` header — that is what
//! [`crate::definer::SSLICTCL_DOCUMENT_GRAMMAR`] is for. So the trait buys
//! nothing and costs the guarantee. Hover, completion, and signature help read
//! the spec's own fields and are unaffected.
//!
//! What each word carries instead is
//! [`Traits::DEFINITION_BODY_MEMBER_ONLY`](crate::Traits::DEFINITION_BODY_MEMBER_ONLY):
//! the spec has to exist for the word to hover, complete, and arity-check
//! where it *is* legal, and the flag is how it tells a consumer enumerating
//! the registry not to offer it at an open command position — inside a
//! retained `predicate`, say, where the vocabulary does not reach.

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
///
/// Deliberately **no** `Traits::LANGUAGE_KEYWORD`: see the module docs' "One
/// keyword rule" note. A row's keyword-ness is a fact about the block it sits
/// in, and the enclosing grammar already states it —
/// `DEFINITION_BODY_MEMBER_ONLY` is how the word says so.
pub(crate) fn statement(
    name: &'static str,
    arity: Arity,
    summary: &'static str,
    synopsis: &'static [&'static str],
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::DEFINITION_BODY_MEMBER_ONLY,
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
