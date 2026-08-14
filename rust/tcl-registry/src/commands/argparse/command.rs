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

//! The `argparse` package — George Yashin's (georgtree) argument parser
//! (`package require argparse`).
//!
//! Models the *global* switches of the `argparse` command itself — the
//! switches before the definition list (e.g. `argparse -inline -pfirst
//! {def...}`).  The per-element switches inside a definition element
//! (`{-foo -required -default x}`) are consumed by `argparse` when it parses
//! the definition, not passed to `argparse` as command switches, so they are
//! kept as [`ELEMENT_SWITCHES`] data for definition-aware tooling rather than
//! command `OptionSpec` entries.

use crate::prelude::*;

/// Element-definition switches valid *inside* a definition element (not as
/// command switches).  Data for definition-aware completion / harvesting.
pub const ELEMENT_SWITCHES: &[&str] = &[
    "-alias",
    "-argument",
    "-boolean",
    "-catchall",
    "-default",
    "-enum",
    "-forbid",
    "-ignore",
    "-imply",
    "-keep",
    "-key",
    "-level",
    "-optional",
    "-parameter",
    "-pass",
    "-reciprocal",
    "-require",
    "-required",
    "-standalone",
    "-switch",
    "-upvar",
    "-validate",
    "-value",
    "-type",
    "-allow",
    "-help",
    "-errormsg",
    "-hsuppress",
];

/// Element-definition switches that take a following value.
pub const ELEMENT_SWITCHES_WITH_ARGS: &[&str] = &[
    "-alias",
    "-default",
    "-enum",
    "-forbid",
    "-imply",
    "-key",
    "-pass",
    "-require",
    "-validate",
    "-value",
    "-type",
    "-allow",
    "-help",
    "-errormsg",
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    // `argparse` sets variables in the caller's frame (or builds an
    // `-inline` dict); classify as interp-state mutation.
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

macro_rules! opt {
    ($name:literal, $takes:expr, $hint:literal, $detail:literal) => {
        OptionSpec {
            name: $name,
            value: if $takes {
                OptionValue::value($hint)
            } else {
                OptionValue::flag()
            },
            detail: $detail,
            dialects: None,
            aliases: &[],
            lifecycle: Lifecycle::UNSPECIFIED,
            min_abbrev: None,
        }
    };
}

const OPTIONS: &[OptionSpec] = &[
    // Boolean global switches.
    opt!(
        "-inline",
        false,
        "",
        "Return a result dict instead of setting caller variables."
    ),
    opt!(
        "-exact",
        false,
        "",
        "Require exact switch names; disable prefix matching."
    ),
    opt!(
        "-mixed",
        false,
        "",
        "Allow switches to appear after parameters."
    ),
    opt!(
        "-long",
        false,
        "",
        "Accept a double-dash (`--switch`) spelling for each switch."
    ),
    opt!(
        "-equalarg",
        false,
        "",
        "Accept `-switch=value` in addition to `-switch value`."
    ),
    opt!(
        "-normalize",
        false,
        "",
        "Normalise switch spellings and omitted switches in output."
    ),
    opt!(
        "-reciprocal",
        false,
        "",
        "Make each `-require` relationship symmetric."
    ),
    opt!(
        "-boolean",
        false,
        "",
        "Treat every switch as boolean (no argument)."
    ),
    opt!(
        "-keep",
        false,
        "",
        "Do not unset caller variables for omitted switches/parameters."
    ),
    opt!(
        "-pfirst",
        false,
        "",
        "Require parameters to precede switches."
    ),
    opt!(
        "-helpret",
        false,
        "",
        "Return the help text instead of raising it as an error."
    ),
    // Value-taking global switches.
    opt!(
        "-level",
        true,
        "level",
        "Frame level at which caller variables are set (default 1)."
    ),
    opt!(
        "-template",
        true,
        "template",
        "Template substituted into each output key name."
    ),
    opt!(
        "-pass",
        true,
        "name",
        "Collect unrecognised arguments into the named key."
    ),
    opt!(
        "-validate",
        true,
        "dict",
        "Named validation expressions shared by elements."
    ),
    opt!(
        "-enum",
        true,
        "dict",
        "Named enumeration lists shared by elements."
    ),
    opt!(
        "-help",
        true,
        "text",
        "Overall help text for the generated help message."
    ),
    opt!(
        "-helplevel",
        true,
        "level",
        "Frame level used when generating the help message."
    ),
    // Option terminator: argparse stops global-switch scanning at `--`.
    opt!(
        "--",
        false,
        "",
        "Marks the end of global switches; the next argument is the definition."
    ),
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "argparse ?switch ...? ?--? definition ?arguments?",
    ..FormSpec::DEFAULT
}];

/// The `argparse` command spec.
#[must_use]
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "argparse",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Parse and validate switches and parameters from an argument list.",
            synopsis: &["argparse ?switch ...? ?--? definition ?arguments?"],
            snippet: "Processes an argument definition list, then reads the caller's `args` (or an explicit argument list) against it, setting a variable (or building an `-inline` dict) for each recognised switch and parameter.",
            source: "argparse package (argparse.n)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        // `argparse` creates one local per recognised element **in the frame
        // that called it** — `argparse.tcl:964` runs
        // `uplevel 1 [list ::upvar … $val $key]` for `-upvar` elements and
        // `uplevel 1 [list set …]` for the rest.  The names come from the
        // definition list's own mini-language (`-key`/`-alias`/`-pass`
        // rewrite them, `-inline` suppresses them entirely), which this
        // analysis does not interpret, so the effect is an *opaque*
        // caller-frame injection: a consumer must widen rather than
        // enumerate.  tclsh 9.0.4 / 8.6.14: `proc p {args} {argparse {{a
        // -upvar} b c}; puts "$a $b $c"}` reads three locals nothing in `p`
        // ever assigns.
        frame_effect: Some(FrameEffectSpec {
            level_word: FrameLevelWord::None,
            layout: FrameArgLayout::OpaqueCallerVars,
        }),
        required_package: Some("argparse"),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argparse_is_gated_and_has_global_switches() {
        let s = spec();
        assert_eq!(s.required_package, Some("argparse"));
        let names = s.switch_names(None);
        for expected in [
            "-inline",
            "-mixed",
            "-pfirst",
            "-boolean",
            "-template",
            "--",
        ] {
            assert!(
                names.contains(&expected),
                "missing global switch {expected}"
            );
        }
    }

    #[test]
    fn value_taking_switches_are_marked() {
        let s = spec();
        let takes = |name: &str| {
            s.options
                .iter()
                .find(|o| o.name == name)
                .is_some_and(OptionSpec::takes_value)
        };
        for v in [
            "-level",
            "-template",
            "-pass",
            "-validate",
            "-enum",
            "-help",
            "-helplevel",
        ] {
            assert!(takes(v), "{v} should take a value");
        }
        assert!(!takes("-inline"));
        assert!(!takes("-mixed"));
    }
}
