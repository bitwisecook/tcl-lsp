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
use tcl_dialect::model::SpecSurface;

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
///
/// The test *name* (argument 0) is described by [`CommandSpec::defines_symbol`]
/// rather than an `ArgRole::Name` here, so the outline / symbol consumers read
/// it from one place; this resolver owns only the body shape.
fn test_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    const BODY_OPTIONS: [&str; 3] = ["-setup", "-body", "-cleanup"];
    let n = args.len();
    // Skip name (0) and description (1); scan option positions in *pairs* so a
    // value that is literally `-body`/`-setup`/`-cleanup` (e.g. `-result -body`)
    // is not misread as an option. In the option form the body roles come
    // from the option-value model (`arg_indices_for_role` derives Body for
    // the script-valued options), so this resolver contributes none — it
    // owns only the legacy positional shape.
    let _ = BODY_OPTIONS;
    let mut i = 2usize;
    while i + 1 < n {
        // ANY `-option` at an option position selects the option form —
        // `test n d -result r` must not fall through to the positional
        // branch and mis-mark the value as a body.
        if args[i].starts_with('-') {
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
    ..SideEffect::DEFAULT
}];

/// Built-in comparison modes for `-match` (a custom mode registered via
/// `tcltest::customMatch` is also accepted, so the set is *not* closed).
const MATCH_MODES: &[ArgValue] = &[
    ArgValue {
        value: "exact",
        detail: "actual result must equal the expected result exactly",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "glob",
        detail: "expected result is a glob pattern (string match)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "regexp",
        detail: "expected result is a regular expression",
        ..ArgValue::DEFAULT
    },
];

/// Symbolic return codes accepted by `-returnCodes` (the numeric forms
/// `0`-`4` are equivalent, so the set is *not* closed).
const RETURN_CODES: &[ArgValue] = &[
    ArgValue {
        value: "ok",
        detail: "TCL_OK (0)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "error",
        detail: "TCL_ERROR (1)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "return",
        detail: "TCL_RETURN (2)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "break",
        detail: "TCL_BREAK (3)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "continue",
        detail: "TCL_CONTINUE (4)",
        ..ArgValue::DEFAULT
    },
];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-body",
        value: OptionValue::script(),
        detail: "the Tcl script that makes up the body of the test",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-setup",
        value: OptionValue::script(),
        detail: "a script to run before the test body",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cleanup",
        value: OptionValue::script(),
        detail: "a script to run after the test body",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-constraints",
        value: OptionValue::value("constraintList"),
        detail: "constraints that must all be satisfied for the test to run",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-result",
        value: OptionValue::value("expected"),
        detail: "the expected result of the test body",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-output",
        value: OptionValue::value("expected"),
        detail: "expected data written to the output channel by the test body",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-errorOutput",
        value: OptionValue::value("expected"),
        detail: "expected data written to the error channel by the test body",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-returnCodes",
        value: OptionValue::enumerated(RETURN_CODES, false, "codeList"),
        detail: "acceptable return code(s) from the test body",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-errorCode",
        value: OptionValue::value("pattern"),
        detail: "expected -errorcode from the test body (tcltest 2.5+)",
        // Added in tcltest 2.5 (bundled with Tcl 8.6).  The `min_version` gate
        // only bites when a `package require tcltest <ver>` pins a floor; the
        // common unversioned `package require tcltest` leaves the floor
        // unknown, so also gate by Tcl core version (tcltest 2.5 ships with
        // 8.6) to hide the option under 8.4 / 8.5.
        lifecycle: Lifecycle::introduced_in("2.5"),
        surface: Some(SpecSurface::TCL86_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-match",
        value: OptionValue::enumerated(MATCH_MODES, false, "mode"),
        detail: "match algorithm comparing actual to expected result",
        ..OptionSpec::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "test name description ?option value ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::test",
        surface: Some(SpecSurface::ALL_TCL),
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
        // The test name (arg 0) and its description (arg 1) name a test case the
        // outline lists.  Every symbol consumer discovers it from this
        // descriptor — the argument index and category are data here, not a
        // `cmd == "test"` check in the analyser / signature scanner.
        defines_symbol: Some(SymbolDef::new(0, DefinedSymbolKind::Test).with_detail(1)),
        body_kind: BodyKind::Structural,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        is_namespace_exported: true,
        ..CommandSpec::DEFAULT
    }
}
