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

//! `tcltest::test` command.
use crate::prelude::*;

/// Dynamic arg-role resolver for the legacy positional form of
/// `tcltest::test name description ?constraints? body result`, where the body
/// is always the penultimate argument.
///
/// The option form (`test name description ?option value ...?`) needs no help
/// here: the `-setup` / `-body` / `-cleanup` options are modelled declaratively
/// as [`OptionValue::script()`], so the shared option-value machinery
/// (`arg_indices_for_role` / the semantic-token role pass) already recurses
/// their bodies. This resolver only owns the shape the option model cannot
/// express, and suppresses the positional guess when a body *option* is present
/// (there the trailing words are option values, not a positional body).
///
/// Without the positional `Body` role the analyser never descends into a legacy
/// `test` body, under-reporting every nested diagnostic.
fn test_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    const BODY_OPTIONS: [&str; 3] = ["-setup", "-body", "-cleanup"];
    let n = args.len();
    // Skip name (0) and description (1); scan option positions in *pairs* so a
    // value that is literally `-body`/`-setup`/`-cleanup` (e.g. `-result -body`)
    // is not misread as an option.
    let mut i = 2usize;
    while i + 1 < n {
        if BODY_OPTIONS.contains(&args[i]) {
            // Option form — the option-value model handles the bodies.
            return Vec::new();
        }
        i += 2;
    }
    // Legacy positional form: the body is the penultimate argument.
    if n >= 4
        && let Ok(idx) = u8::try_from(n - 2)
    {
        return vec![(idx, ArgRole::Body)];
    }
    Vec::new()
}

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-body",
        value: OptionValue::script(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-result",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-output",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-errorOutput",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-returnCodes",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-errorCode",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-match",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-setup",
        value: OptionValue::script(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-cleanup",
        value: OptionValue::script(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-constraints",
        value: OptionValue::value(""),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "test name description ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::test",
        dialects: None,
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Define and run a single test case.",
            synopsis: &[
                "tcltest::test name description ?option value ...?",
                "tcltest::test name description ?constraints? body result",
            ],
            snippet: "The primary command for defining tests.  Options include ``-body``, ``-result``, ``-output``, ``-errorOutput``, ``-returnCodes``, ``-match``, ``-setup``, ``-cleanup``, and ``-constraints``.",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        arg_role_resolver: Some(test_arg_roles),
        body_kind: BodyKind::Structural,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        is_namespace_exported: true,
        ..CommandSpec::DEFAULT
    }
}
