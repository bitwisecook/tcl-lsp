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

//! `my` — call a method on the current object.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "my method ?arg ...?",
}];

/// Every argument after the `variable` subcommand word is a variable name to
/// link (`my variable a b c` links all of `a`, `b`, `c`), so — unlike a
/// fixed-position `arg_roles` entry — this must be a resolver: the SSA
/// def-extraction generic in `lowering/mod.rs::lower_default` (via
/// `CommandRegistry::arg_indices_for_role`) calls it with the sub-relative
/// args (everything after `variable`) and offsets the returned indices by
/// `+1` for the subcommand word automatically.
fn my_variable_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (0..args.len())
        .filter_map(|i| u8::try_from(i).ok())
        .map(|i| (i, ArgRole::VarWrite))
        .collect()
}

/// `my` forwards to an arbitrary method name, but `my variable ?name ...?`
/// is `TclOO`'s own reserved dispatch — the per-object-namespace analogue of
/// the top-level `variable` command (links each `name` to the object's
/// private-namespace storage, same as `oo::define CLASS { variable name }`
/// does for every method body). Modelled as the one recognised subcommand so
/// [`Traits::CREATES_SCOPE_ALIAS`]'s per-subcommand sibling
/// (`creates_scope_alias`) picks it up generically — the compiler's
/// `is_scope_alias_call` widens its defs to `Overdefined` exactly like
/// `global`/`variable`/`upvar`/`namespace upvar`, so a variable linked this
/// way (whose true intrep may have been set by a *different* method) is not
/// misreported as a local shimmer. `allow_unknown_subcommands` keeps every
/// other `my <method>` form dispatching freely instead of tripping W001 —
/// the analyser cannot know a class's user-defined method names statically,
/// so only `variable` (and its unique-prefix abbreviations) is validated.
const SUBCOMMANDS: &[SubCommand] = &[SubCommand {
    name: "variable",
    arity: Arity::at_least(1),
    detail: "Link the named object-instance variable(s) into the current method's local scope.",
    synopsis: "my variable ?name ...?",
    return_type: Some(TclType::String),
    arg_role_resolver: Some(my_variable_arg_roles),
    creates_scope_alias: true,
    dialects: Some(DialectSet::TCL86_PLUS),
    ..SubCommand::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "my",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        subcommands: SUBCOMMANDS,
        allow_unknown_subcommands: true,
        hover: Some(HoverSnippet {
            summary: "invoke a method on the current object",
            synopsis: &["my method ?arg ...?"],
            snippet: "The my command is used within the body of a method, constructor, or destructor to invoke a method on the current object.  It is equivalent to [self] method ?arg ...? but avoids the overhead of determining the object name.",
            source: "Tcl man page my.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
