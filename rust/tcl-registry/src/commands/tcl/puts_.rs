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

//! `puts` — write to a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "puts ?-nonewline? ?channelId? string",
    dialects: None,
}];

/// Dynamic arg-role resolver: `puts` takes one positional arg (`string`,
/// writing to stdout) or two (`channelId string`); the optional leading
/// `-nonewline` flag shifts which raw-arg index is the channel. Declaring
/// the channel position lets the taint sink's position-aware filter
/// (`sink_var_position_safe` in `tcl_compiler::taint`) recognise a tainted
/// channel handle as a non-dangerous argument generically, with no
/// `puts`-by-name check in the compiler.
fn puts_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let skip: u8 = u8::from(args.first() == Some(&"-nonewline"));
    if args.len() - usize::from(skip) >= 2 {
        vec![(skip, ArgRole::Channel)]
    } else {
        Vec::new()
    }
}

/// Command spec for `puts`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "puts",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::TAINT_SINK,
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(puts_arg_roles),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        options: const {
            &[OptionSpec {
                name: "-nonewline",
                value: OptionValue::flag(),
                detail: "Do not output a newline character.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Write text to a channel (stdout by default).",
            synopsis: &["puts ?-nonewline? ?channelId? string"],
            snippet: "Use `-nonewline` to suppress the trailing newline.",
            source: "Tcl puts(1)",
            examples: "",
            return_value: "",
        }),
        // Tainted data reaching `puts` output → T101.
        taint_output_sink: Some("T101"),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
