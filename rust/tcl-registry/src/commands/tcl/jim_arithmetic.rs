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

//! Jim's global arithmetic commands `+ - * /`.
//!
//! Defined in `jim.c` (`Jim_AddCoreCommand` and its siblings) with the
//! minimum and maximum word counts below. Every operand is read as a wide
//! integer, and the rest of the call as doubles from the first operand that
//! is not one; a single operand of `-` negates and of `/` reciprocates (a
//! double); integer `/` by zero is an error.
use crate::prelude::*;

macro_rules! arithmetic {
    ($name:literal, $minimum:literal, $synopsis:literal, $summary:literal) => {
        CommandSpec {
            name: $name,
            native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
                grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
                operation: crate::SemanticOperationId::Invoke,
                body: crate::native_compilation::NativeBodyCompilation::Direct,
            }),
            surface: Some(tcl_dialect::surface![
                tcl_dialect::model::SpecSurface::core_in(
                    tcl_dialect::model::Family::Jim,
                    &[("0.84", None)]
                )
            ]),
            runtime_backing: RuntimeBacking::shipped($name),
            hover: Some(HoverSnippet {
                summary: $summary,
                synopsis: &[$synopsis],
                snippet: $summary,
                source: "https://jim.tcl.tk/home/doc/trunk/jim_tcl.txt#_math_commands",
                examples: "",
                return_value: "Returns an integer, or a double once any operand is not an integer.",
            }),
            arity: Arity::at_least($minimum),
            return_type: Some(TclType::Numeric),
            traits: Traits::OPERATOR_COMMAND,
            forms: &[FormSpec {
                synopsis: $synopsis,
                ..FormSpec::DEFAULT
            }],
            ..CommandSpec::DEFAULT
        }
    };
}

/// The four commands, in `jim.c`'s order.
pub fn specs() -> [CommandSpec; 4] {
    [
        arithmetic!(
            "*",
            0,
            "* ?number ...?",
            "Multiply the numbers; the product of none is 1."
        ),
        arithmetic!(
            "+",
            0,
            "+ ?number ...?",
            "Add the numbers; the sum of none is 0."
        ),
        arithmetic!(
            "-",
            1,
            "- number ?number ...?",
            "Subtract the later numbers from the first, or negate a single number."
        ),
        arithmetic!(
            "/",
            1,
            "/ number ?number ...?",
            "Divide the first number by the later ones, or take the reciprocal of a single number."
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandRegistry;

    /// `jim.c` registers `*` and `+` with 0 or more words and `-` and `/` with
    /// at least 1: Jim 0.84's own `{"-", Jim_SubCoreCommand, 1, -1, "number
    /// ?number ...?"}` table row, which is the `wrong # args` text jimsh prints
    /// for a bare `-`.
    #[test]
    fn the_arities_are_jims_registered_counts() {
        let registry = CommandRegistry::build_default();
        for (name, minimum) in [("*", 0), ("+", 0), ("-", 1), ("/", 1)] {
            let spec = registry
                .get(name)
                .unwrap_or_else(|| panic!("{name} is specified"));
            assert_eq!(spec.arity, Arity::at_least(minimum), "{name}");
        }
    }
}
