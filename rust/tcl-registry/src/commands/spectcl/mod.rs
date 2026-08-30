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

//! `SpecTcl` — the command pack for the `.tclspec` spec-pack DSL itself.
//!
//! `spec-packs.md` calls for "a compiled-in command pack for `SpecTcl` itself —
//! `speclib`, `command`, `option`, `arg`, `subcommand`, the hook statements —
//! with its `definition_body` grammar, so authoring a pack gets highlighting,
//! completion, and misspelled-trait diagnostics from the same machinery it
//! configures."  This is that pack; the frozen syntax it describes is
//! `docs/design/spec-dsl-examples/README.md`, and the member vocabularies of
//! the nested blocks live beside every other definition-body grammar, in
//! [`crate::definer`].
//!
//! The pack loads only under the `spectcl` dialect profile — `.tclspec` files
//! and anything a user pins with `# tcl-dialect: spectcl` — so extremely
//! generic statement words (`command`, `option`, `arg`, `value`, `name`) never
//! reach an ordinary Tcl document.  Everything else about them is ordinary
//! registry data: they hover, complete, arity-check, and offer their flags
//! exactly as a shipped Tcl command does.
//!
//! ## Two layers, and why both
//!
//! * Each statement word carries a [`CommandSpec`], which is what gives it
//!   hover, completion, arity and option checking.
//! * Each *block* statement additionally carries a `definition_body` grammar
//!   naming the words legal inside it, which is what makes the vocabulary
//!   **context-sensitive**: `summary` means something inside `hover { … }`
//!   and nothing outside it, and the generic definition-body walker paints,
//!   folds, and recurses accordingly with no walker changes at all.
//!
//! Nesting needs no new machinery: the walker's rule is "recursing into the
//! body of a command that carries a grammar switches to that grammar", so a
//! `command` block inside a `speclib` block inside a `.tclspec` file lands in
//! the right vocabulary at every level, and a hook body — whose statement
//! carries no grammar — correctly drops back to ordinary Tcl.

use crate::arity::Arity;
use crate::hover::HoverSnippet;
use crate::spec::CommandSpec;
use crate::traits::Traits;
use tcl_dialect::model::SpecSurface;

mod arg;
mod blocks;
mod command;
mod hooks;
mod option;
mod properties;
mod rows;
mod speclib;

/// The `source` line every `SpecTcl` statement's hover carries.
pub(crate) const SOURCE: &str = "SpecTcl (docs/design/spec-packs.md)";

/// Build the spec for a `SpecTcl` statement word that is plain data — no block,
/// no options, nothing but a keyword and its operands.
///
/// The overwhelming majority of the DSL's vocabulary is this shape (one word
/// per `CommandSpec` / `SubCommand` schema key), so the *table* is the spec:
/// a new schema key becomes one row in [`properties`], never a new file.  The
/// richer statements — the ones with option tables or nested grammars — keep
/// their own modules.
pub(crate) fn statement(
    name: &'static str,
    arity: Arity,
    summary: &'static str,
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SPECTCL),
        arity,
        hover: Some(HoverSnippet {
            summary,
            synopsis: &[],
            snippet,
            source: SOURCE,
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

/// Every `SpecTcl` statement word, as command specs.
#[must_use]
pub fn spectcl_command_specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        speclib::spec(),
        command::spec(),
        command::subcommand_spec(),
        arg::spec(),
        option::spec(),
    ];
    specs.extend(blocks::specs());
    specs.extend(hooks::specs());
    specs.extend(rows::specs());
    specs.extend(properties::specs());
    specs
}
