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

//! `tcl::prefix` — prefix-matching helpers (added Tcl 8.6, TIP 265).
//!
//! Unlike its sibling ensemble `tcl::mathop` (fully registered in
//! `mathop.rs`/`mathop_generated.rs`), this ensemble had no `CommandSpec` at
//! all until this fix (issue #923 differential-audit finding idx 9, main
//! audit wave): `rust/tcl-vm/src/cmd_prefix.rs` registers and fully
//! implements it for the VM, but hover/completion/signature-help had
//! nothing to show — a call like `tcl::prefix match -message switch
//! $candidates $name` (the exact idiom `argparse.tcl` uses to resolve an
//! abbreviated switch name) drew no false diagnostic (no `CommandSpec`
//! means no arity/unknown-command check either), but also no
//! documentation.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "all",
        arity: Arity::exact(2),
        detail: "Return every element of table that begins with string.",
        synopsis: "tcl::prefix all table string",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "longest",
        arity: Arity::exact(2),
        detail: "Return the longest common prefix of every element of table that begins with string.",
        synopsis: "tcl::prefix longest table string",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "match",
        // Positional arity is `table string` only (2); the `-exact`/
        // `-message`/`-error` leading options (and the value word each of
        // the latter two consumes) are skipped by the generic
        // leading-option scan `check_simple_arity` already runs against
        // `options` below — not counted here, mirroring `interp create`'s
        // identical `-safe`/`--` precedent.
        arity: Arity::exact(2),
        detail: "Resolve string to the unique element of table it is an unambiguous prefix of.",
        synopsis: "tcl::prefix match ?-exact? ?-message string? ?-error options? table string",
        pure: true,
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-exact",
                    value: OptionValue::flag(),
                    detail: "Disable prefix matching — string must exactly equal a table element.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-message",
                    value: OptionValue::value("string"),
                    detail: "Replace \"option\" in the generated error message with this text.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-error",
                    value: OptionValue::value("options"),
                    detail: "Control error handling: {} returns \"\" on no match instead of erroring.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the `tcl::prefix` ensemble.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::prefix",
        traits: Traits::NOT_PROC_FACTORY,
        // Added in Tcl 8.6 (TIP 265).
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Prefix-matching utility commands.",
            synopsis: &[
                "tcl::prefix match ?-exact? ?-message string? ?-error options? table string",
                "tcl::prefix all table string",
                "tcl::prefix longest table string",
            ],
            snippet: "Resolves an abbreviated string against a table of allowed values, the same matching C Tcl uses internally to resolve abbreviated subcommand/option names.",
            source: "Tcl man page prefix.n",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_arity_ignores_leading_options() {
        // TN — the exact real-world call shape this finding was mined from
        // (`argparse.tcl`'s `tcl::prefix match -message switch $candidates
        // $name`) must not false-fire E002/E003: 2 positional args
        // (candidates, name) after the `-message switch` option pair is
        // skipped.
        let sub = SUBCOMMANDS.iter().find(|s| s.name == "match").unwrap();
        assert_eq!(
            sub.leading_option_word_count(&["-message", "switch", "$candidates", "$name"]),
            2
        );
    }

    #[test]
    fn bare_ensemble_requires_a_subcommand() {
        assert_eq!(
            spec().arity.min,
            1,
            "tcl::prefix alone must require a subcommand (E001)"
        );
    }
}
