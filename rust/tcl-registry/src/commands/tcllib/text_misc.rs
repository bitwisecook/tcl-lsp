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

//! tcllib text packages — `soundex`, `stringprep`, and `unicode`.
//!
//! Public command surface from tcllib `modules/soundex/soundex.man` and
//! `modules/stringprep/{stringprep,unicode}.man`.  The `stringprep` module
//! provides the `stringprep` (RFC 3454) and `unicode` (normalisation)
//! packages.  Requires Tcl 8.5+.

use crate::prelude::*;

/// Unicode normalisation forms accepted by `unicode::normalize` /
/// `unicode::normalizeS`.
const NORMAL_FORMS: &[ArgValue] = &[
    ArgValue {
        value: "D",
        detail: "Canonical decomposition (NFD).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "C",
        detail: "Canonical decomposition then composition (NFC).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "KD",
        detail: "Compatibility decomposition (NFKD).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "KC",
        detail: "Compatibility decomposition then composition (NFKC).",
        ..ArgValue::DEFAULT
    },
];

/// Options for `stringprep::register`.
const REGISTER_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-mapping",
        value: OptionValue::value("list"),
        detail: "Character mapping tables to apply.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-normalization",
        value: OptionValue::value("form"),
        detail: "The Unicode normalisation form for the profile.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-prohibited",
        value: OptionValue::value("list"),
        detail: "Ranges of prohibited characters.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-prohibitedList",
        value: OptionValue::value("list"),
        detail: "An explicit list of prohibited characters.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-prohibitedCommand",
        value: OptionValue::command_prefix("command"),
        detail: "A command that decides whether a character is prohibited.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-prohibitedBidi",
        value: OptionValue::boolean(),
        detail: "Whether to apply the bidirectional-text checks.",
        ..OptionSpec::DEFAULT
    },
];

/// A `unicode` normalisation command taking a `form` then a list.
fn normalize(
    name: &'static str,
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    const FORM_OPT: &[ArgValue] = NORMAL_FORMS;
    CommandSpec {
        name,
        traits: Traits::PURE,
        arity: Arity::exact(2),
        arg_values: &[(0, FORM_OPT)],
        closed_value_args: &[0],
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet: "",
            source: "tcllib unicode package",
            examples: "",
            return_value: "The normalised value.",
        }),
        tcllib_package: Some("unicode"),
        required_package: Some("unicode"),
        ..CommandSpec::DEFAULT
    }
}

/// The `soundex` package — a single `soundex::knuth` command.
fn soundex_cmd() -> CommandSpec {
    CommandSpec {
        name: "soundex::knuth",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute the Knuth-variant Soundex code of a string.",
            synopsis: &["soundex::knuth string"],
            snippet: "",
            source: "tcllib soundex package",
            examples: "",
            return_value: "The four-character Soundex code.",
        }),
        tcllib_package: Some("soundex"),
        required_package: Some("soundex"),
        ..CommandSpec::DEFAULT
    }
}

/// The `stringprep` package — `register`, `stringprep`, and `compare`.
fn stringprep_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "stringprep::register",
            traits: Traits::empty(),
            arity: Arity::at_least(1),
            options: REGISTER_OPTS,
            hover: Some(HoverSnippet {
                summary: "Register a named stringprep (RFC 3454) profile.",
                synopsis: &[
                    "stringprep::register profile ?-mapping list? ?-normalization form? ?-prohibited list? ?-prohibitedList list? ?-prohibitedCommand command? ?-prohibitedBidi boolean?",
                ],
                snippet: "",
                source: "tcllib stringprep package",
                examples: "",
                return_value: "",
            }),
            tcllib_package: Some("stringprep"),
            required_package: Some("stringprep"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "stringprep::stringprep",
            traits: Traits::PURE,
            arity: Arity::exact(2),
            hover: Some(HoverSnippet {
                summary: "Prepare a string according to a registered stringprep profile.",
                synopsis: &["stringprep::stringprep profile string"],
                snippet: "",
                source: "tcllib stringprep package",
                examples: "",
                return_value: "The prepared string.",
            }),
            tcllib_package: Some("stringprep"),
            required_package: Some("stringprep"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "stringprep::compare",
            traits: Traits::PURE,
            arity: Arity::exact(3),
            hover: Some(HoverSnippet {
                summary: "Compare two strings after applying a stringprep profile.",
                synopsis: &["stringprep::compare profile string1 string2"],
                snippet: "",
                source: "tcllib stringprep package",
                examples: "",
                return_value: "0 if the prepared strings are equal, non-zero otherwise.",
            }),
            tcllib_package: Some("stringprep"),
            required_package: Some("stringprep"),
            ..CommandSpec::DEFAULT
        },
    ]
}

/// The `unicode` package — code-point conversion and normalisation.
fn unicode_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "unicode::fromstring",
            traits: Traits::PURE,
            arity: Arity::exact(1),
            hover: Some(HoverSnippet {
                summary: "Convert a Tcl string to a list of Unicode code points.",
                synopsis: &["unicode::fromstring string"],
                snippet: "",
                source: "tcllib unicode package",
                examples: "",
                return_value: "A list of integer Unicode code points.",
            }),
            tcllib_package: Some("unicode"),
            required_package: Some("unicode"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "unicode::tostring",
            traits: Traits::PURE,
            arity: Arity::exact(1),
            hover: Some(HoverSnippet {
                summary: "Convert a list of Unicode code points to a Tcl string.",
                synopsis: &["unicode::tostring uclist"],
                snippet: "",
                source: "tcllib unicode package",
                examples: "",
                return_value: "The reconstructed Tcl string.",
            }),
            tcllib_package: Some("unicode"),
            required_package: Some("unicode"),
            ..CommandSpec::DEFAULT
        },
        normalize(
            "unicode::normalize",
            &["unicode::normalize form uclist"],
            "Normalise a list of Unicode code points to the given form.",
        ),
        normalize(
            "unicode::normalizeS",
            &["unicode::normalizeS form string"],
            "Normalise a Tcl string to the given Unicode form.",
        ),
    ]
}

/// All `soundex` / `stringprep` / `unicode` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = vec![soundex_cmd()];
    specs.extend(stringprep_specs());
    specs.extend(unicode_specs());
    specs
}
