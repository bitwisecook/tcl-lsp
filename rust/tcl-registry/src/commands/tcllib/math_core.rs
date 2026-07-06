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

//! tcllib `math` core packages — `math`, `math::fuzzy`, `math::roman`, and
//! `math::constants`.
//!
//! Public command surface from tcllib `modules/math/{math,fuzzy,roman,
//! constants}.man`.  (The large `math::statistics` package is registered
//! separately.)  Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a math package's command table: name, arity, synopsis, summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build pure command specs for a package from its table.
fn pure_rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            traits: Traits::PURE,
            arity,
            hover: Some(HoverSnippet::brief(
                summary,
                synopsis,
                "tcllib math package",
            )),
            tcllib_package: Some(pkg),
            required_package: Some(pkg),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// The pure numeric commands of the core `math` package (everything but the
/// non-deterministic `math::random`).
const MATH_CMDS: &[Row] = &[
    (
        "math::cov",
        Arity::at_least(2),
        &["math::cov value value ?{value ...}?"],
        "Compute the coefficient of variation of a set of values.",
    ),
    (
        "math::integrate",
        Arity::exact(1),
        &["math::integrate {list of xy value pairs}"],
        "Integrate a tabulated function given as x/y value pairs.",
    ),
    (
        "math::fibonacci",
        Arity::exact(1),
        &["math::fibonacci n"],
        "Return the n-th Fibonacci number.",
    ),
    (
        "math::max",
        Arity::at_least(1),
        &["math::max value ?{value ...}?"],
        "Return the maximum of a set of values.",
    ),
    (
        "math::mean",
        Arity::at_least(1),
        &["math::mean value ?{value ...}?"],
        "Return the arithmetic mean of a set of values.",
    ),
    (
        "math::min",
        Arity::at_least(1),
        &["math::min value ?{value ...}?"],
        "Return the minimum of a set of values.",
    ),
    (
        "math::product",
        Arity::at_least(1),
        &["math::product value ?{value ...}?"],
        "Return the product of a set of values.",
    ),
    (
        "math::sigma",
        Arity::at_least(2),
        &["math::sigma value value ?{value ...}?"],
        "Return the standard deviation of a set of values.",
    ),
    (
        "math::stats",
        Arity::at_least(2),
        &["math::stats value value ?{value ...}?"],
        "Return mean, standard deviation, and range of a set of values.",
    ),
    (
        "math::sum",
        Arity::at_least(1),
        &["math::sum value ?{value ...}?"],
        "Return the sum of a set of values.",
    ),
];

/// The `math::fuzzy` tolerant floating-point comparison commands.
const FUZZY_CMDS: &[Row] = &[
    (
        "math::fuzzy::teq",
        Arity::exact(2),
        &["math::fuzzy::teq value1 value2"],
        "Tolerant floating-point equality test.",
    ),
    (
        "math::fuzzy::tne",
        Arity::exact(2),
        &["math::fuzzy::tne value1 value2"],
        "Tolerant floating-point inequality test.",
    ),
    (
        "math::fuzzy::tge",
        Arity::exact(2),
        &["math::fuzzy::tge value1 value2"],
        "Tolerant floating-point greater-or-equal test.",
    ),
    (
        "math::fuzzy::tle",
        Arity::exact(2),
        &["math::fuzzy::tle value1 value2"],
        "Tolerant floating-point less-or-equal test.",
    ),
    (
        "math::fuzzy::tlt",
        Arity::exact(2),
        &["math::fuzzy::tlt value1 value2"],
        "Tolerant floating-point strictly-less test.",
    ),
    (
        "math::fuzzy::tgt",
        Arity::exact(2),
        &["math::fuzzy::tgt value1 value2"],
        "Tolerant floating-point strictly-greater test.",
    ),
    (
        "math::fuzzy::tfloor",
        Arity::exact(1),
        &["math::fuzzy::tfloor value"],
        "Tolerant floor of a floating-point value.",
    ),
    (
        "math::fuzzy::tceil",
        Arity::exact(1),
        &["math::fuzzy::tceil value"],
        "Tolerant ceiling of a floating-point value.",
    ),
    (
        "math::fuzzy::tround",
        Arity::exact(1),
        &["math::fuzzy::tround value"],
        "Tolerant rounding of a floating-point value.",
    ),
    (
        "math::fuzzy::troundn",
        Arity::exact(2),
        &["math::fuzzy::troundn value ndigits"],
        "Tolerant rounding of a value to ndigits decimals.",
    ),
];

/// The `math::roman` numeral commands.
const ROMAN_CMDS: &[Row] = &[
    (
        "math::roman::toroman",
        Arity::exact(1),
        &["math::roman::toroman i"],
        "Convert an integer to a Roman numeral string.",
    ),
    (
        "math::roman::tointeger",
        Arity::exact(1),
        &["math::roman::tointeger r"],
        "Convert a Roman numeral string to an integer.",
    ),
    (
        "math::roman::sort",
        Arity::exact(1),
        &["math::roman::sort list"],
        "Sort a list of Roman numerals into numeric order.",
    ),
    (
        "math::roman::expr",
        Arity::at_least(1),
        &["math::roman::expr args"],
        "Evaluate an expression whose operands are Roman numerals.",
    ),
];

/// All `math` core / `fuzzy` / `roman` / `constants` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = pure_rows("math", MATH_CMDS);
    // math::random draws pseudo-random numbers — not constant-foldable.
    specs.push(CommandSpec {
        name: "math::random",
        traits: Traits::empty(),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Return a pseudo-random number, optionally within a range.",
            synopsis: &["math::random ?value1? ?value2?"],
            snippet: "",
            source: "tcllib math package",
            examples: "",
            return_value: "A pseudo-random number in [0,1), [0,value1), or [value1,value2).",
        }),
        tcllib_package: Some("math"),
        required_package: Some("math"),
        ..CommandSpec::DEFAULT
    });
    specs.extend(pure_rows("math::fuzzy", FUZZY_CMDS));
    specs.extend(pure_rows("math::roman", ROMAN_CMDS));
    // math::constants imports named constants into the caller's namespace.
    specs.push(CommandSpec {
        name: "math::constants::constants",
        traits: Traits::empty(),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Import named mathematical constants (pi, e, …) into the caller.",
            &["math::constants::constants ?name ...?"],
            "tcllib math::constants package",
        )),
        tcllib_package: Some("math::constants"),
        required_package: Some("math::constants"),
        ..CommandSpec::DEFAULT
    });
    specs.push(CommandSpec {
        name: "math::constants::print-constants",
        traits: Traits::empty(),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Print the named mathematical constants and their values.",
            &["math::constants::print-constants ?name ...?"],
            "tcllib math::constants package",
        )),
        tcllib_package: Some("math::constants"),
        required_package: Some("math::constants"),
        ..CommandSpec::DEFAULT
    });
    specs
}
