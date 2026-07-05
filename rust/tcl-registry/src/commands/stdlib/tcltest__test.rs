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

/// Dynamic arg-role resolver for `tcltest::test`.
///
/// The command has two shapes whose script bodies must be recursed into:
///
///   * `test name description ?option value ...?` — the values of the
///     `-setup` / `-body` / `-cleanup` options are Tcl scripts.
///   * `test name description ?constraints? body result` — the legacy
///     positional form, where the body is always the penultimate argument.
///
/// Without these `Body` roles the analyser never descends into a `test`
/// body, under-reporting every nested diagnostic (real `*.test` suites are
/// almost entirely composed of `test` blocks).
fn test_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    const BODY_OPTIONS: [&str; 3] = ["-setup", "-body", "-cleanup"];
    let mut roles: Vec<(u8, ArgRole)> = Vec::new();
    let n = args.len();
    // Skip name (0) and description (1); scan option/value *pairs*, examining
    // only the option positions (step by 2). Advancing by 1 would let an option
    // whose value is literally `-body`/`-setup`/`-cleanup` (e.g. `-result -body`)
    // be misread as an option and mark the following word as a body.
    let mut has_body_option = false;
    let mut i = 2usize;
    while i + 1 < n {
        if BODY_OPTIONS.contains(&args[i]) {
            if let Ok(idx) = u8::try_from(i + 1) {
                roles.push((idx, ArgRole::Body));
            }
            has_body_option = true;
        }
        i += 2;
    }
    // Legacy positional form: the body is the penultimate argument. Only
    // applies when no body-related options were used.
    if !has_body_option
        && n >= 4
        && let Ok(idx) = u8::try_from(n - 2)
    {
        roles.push((idx, ArgRole::Body));
    }
    roles
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
        value: OptionValue::value("script"),
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
        value: OptionValue::value("script"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-cleanup",
        value: OptionValue::value("script"),
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
