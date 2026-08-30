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

//! `command` / `subcommand` — the two `SpecTcl` statements that open a
//! per-command declaration block.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-override",
    detail: "claim a command name a shipped spec already has",
    ..OptionSpec::DEFAULT
}];

const COMMAND_FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "command name ?-override? { declarations }",
    ..FormSpec::DEFAULT
}];

const SUBCOMMAND_FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "subcommand name { declarations }",
    ..FormSpec::DEFAULT
}];

/// `command NAME ?-override? { … }` — the block sits after the name and any
/// leading option word, so its index is 1 or 2.
///
/// A resolver rather than a fixed `arg_roles` pair for the same reason
/// `oo::class`'s is one: the body's index is a function of the call's own
/// words. The vocabulary it reads is the option table above, so adding a
/// second leading flag is an [`OptionSpec`] row, never an edit here.
fn command_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let leading = args
        .iter()
        .skip(1)
        .take_while(|word| OPTIONS.iter().any(|opt| opt.matches(word)))
        .count();
    let body = 1 + leading;
    if body < args.len() {
        vec![
            (0u8, ArgRole::Name),
            (u8::try_from(body).unwrap_or(u8::MAX), ArgRole::Body),
        ]
    } else {
        // A bare `command NAME` with no block: name the command and claim no
        // body rather than point one word past the end.
        Vec::new()
    }
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "command",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY | Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SPECTCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Declare one command in a SpecTcl pack.",
            synopsis: &["command name ?-override? { declarations }"],
            snippet: "Opens a per-command declaration block. Its statements are the DSL property words — one word per `CommandSpec` schema key, with a singular row statement (`option`, `arg`, `subcommand`, `form`, …) for every key that holds a list of rows. Without `-override`, a name a shipped spec already has is reported and shipped wins.",
            source: "SpecTcl (docs/design/spec-packs.md)",
            examples: "command with_var {\n    synopsis {with_var varName script}\n    arity 2\n    arg 0 -role VarWrite\n    arg 1 -role Body\n    traits {CREATES_SCOPE_ALIAS}\n}",
            return_value: "",
        }),
        forms: COMMAND_FORMS,
        options: OPTIONS,
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        arg_role_resolver: Some(command_arg_roles),
        body_kind: BodyKind::Structural,
        definition_body: Some(&crate::definer::SPECTCL_COMMAND_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}

/// `subcommand NAME { … }` — the same block, one level down. It reuses the
/// command grammar unchanged: the syntax memo's two coverage tables differ
/// only in which keys are *meaningful*, never in how a block is written, and
/// a loader checks key applicability anyway.
pub fn subcommand_spec() -> CommandSpec {
    CommandSpec {
        name: "subcommand",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY | Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SPECTCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Declare one subcommand of the enclosing command.",
            synopsis: &["subcommand name { declarations }"],
            snippet: "One block per subcommand. `arg` indices inside it are counted after the subcommand word — the same coordinates the registry uses. The one exception is `credential_arg`, whose W310 consumer counts the subcommand word itself as index 0 and which a loader therefore stores verbatim.",
            source: "SpecTcl (docs/design/spec-packs.md)",
            examples: "subcommand at { arity 1 ; detail {Get header name by index.} ; synopsis {HTTP::header at <index>} }",
            return_value: "",
        }),
        forms: SUBCOMMAND_FORMS,
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        definition_body: Some(&crate::definer::SPECTCL_COMMAND_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
