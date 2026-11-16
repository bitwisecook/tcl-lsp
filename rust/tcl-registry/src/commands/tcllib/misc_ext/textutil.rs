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

//! The `textutil` umbrella package and its `adjust`, `patch`, `string`, `tabify` and `trim` sub-packages.

use super::{Arity, Row};

/// The `textutil` package.
pub(super) const TEXTUTIL_CMDS: &[Row] = &[(
    "textutil::expander",
    Arity::exact(1),
    &["textutil::expander expanderName"],
    "The command creates a new expander object with an associated Tcl command whose name is expanderName.",
)];

/// The `textutil::adjust` package.
///
/// `adjust` and `indent` are the package's two primary, most-used commands
/// (tcllib-2.0 `modules/textutil/adjust.tcl`); they are also re-exported as
/// the flattened `::textutil::adjust` / `::textutil::indent` aliases by the
/// `textutil` umbrella package (`textutil.tcl`'s `namespace import -force
/// adjust::adjust adjust::indent adjust::undent`) — see
/// `textutil__adjust.rs` / `textutil__indent.rs` for those, which are gated
/// on `required_package: "textutil"` (the umbrella), not this package.
/// Requiring `textutil::adjust` alone grants only these three-segment names
/// (confirmed against tclsh 9.0.4 + real tcllib-2.0: a
/// bare `::textutil::adjust` command exists only after `package require
/// textutil`, never after `package require textutil::adjust` alone).
pub(super) const TEXTUTIL__ADJUST_CMDS: &[Row] = &[
    (
        "textutil::adjust::adjust",
        Arity::at_least(1),
        &[
            "textutil::adjust::adjust string ?-length num? ?-justify left|right|center|plain? ?-hyphenate bool? ?-full bool? ?-strictlength bool?",
        ],
        "Adjust/reformat a paragraph of text to a given line length, with optional justification and hyphenation.",
    ),
    (
        "textutil::adjust::indent",
        Arity::new(2, 3),
        &["textutil::adjust::indent text prefix ?skip?"],
        "Indent each line of text by prefixing it with prefix, skipping the first skip lines.",
    ),
    (
        "textutil::adjust::readPatterns",
        Arity::exact(1),
        &["textutil::adjust::readPatterns filename"],
        "Loads the internal storage for hyphenation patterns with the contents of the file filename.",
    ),
    (
        "textutil::adjust::listPredefined",
        Arity::exact(0),
        &["textutil::adjust::listPredefined"],
        "This command returns a list containing the names of the hyphenation files coming with this package.",
    ),
    (
        "textutil::adjust::getPredefined",
        Arity::exact(1),
        &["textutil::adjust::getPredefined filename"],
        "Use this command to query the package for the full path name of the hyphenation file filename coming with the pac",
    ),
    (
        "textutil::adjust::undent",
        Arity::exact(1),
        &["textutil::adjust::undent string"],
        "The command computes the common prefix for all lines in string consisting solely out of whitespace, removes this ",
    ),
];

/// The `textutil::patch` package.
pub(super) const TEXTUTIL__PATCH_CMDS: &[Row] = &[(
    "textutil::patch::apply",
    Arity::exact(4),
    &["textutil::patch::apply basedirectory striplevel patch reportcmd"],
    "Applies the patch (text of the path, not file) to the files in the basedirectory using the specified striplevel.",
)];

/// The `textutil::string` package.
pub(super) const TEXTUTIL__STRING_CMDS: &[Row] = &[
    (
        "textutil::string::chop",
        Arity::exact(1),
        &["textutil::string::chop string"],
        "A convenience command.",
    ),
    (
        "textutil::string::tail",
        Arity::exact(1),
        &["textutil::string::tail string"],
        "A convenience command.",
    ),
    (
        "textutil::string::cap",
        Arity::exact(1),
        &["textutil::string::cap string"],
        "Capitalizes the first character of string and returns the modified string.",
    ),
    (
        "textutil::string::capEachWord",
        Arity::exact(1),
        &["textutil::string::capEachWord string"],
        "Capitalizes the first character of word of the string and returns the modified string.",
    ),
    (
        "textutil::string::uncap",
        Arity::exact(1),
        &["textutil::string::uncap string"],
        "The complementary operation to ::textutil::string::cap.",
    ),
    (
        "textutil::string::longestCommonPrefixList",
        Arity::exact(1),
        &["textutil::string::longestCommonPrefixList list"],
        "tcllib command.",
    ),
    (
        "textutil::string::longestCommonPrefix",
        Arity::at_least(0),
        &["textutil::string::longestCommonPrefix ?string...?"],
        "Returns the longest common prefix of the given strings.",
    ),
];

/// The `textutil::trim` package.
///
/// `trim`/`trimleft`/`trimright` are also re-exported as the flattened
/// `::textutil::trim` / `::textutil::trimleft` / `::textutil::trimright`
/// aliases by the `textutil` umbrella package — see `textutil__trim.rs` /
/// `textutil__trimleft.rs` / `textutil__trimright.rs` for those (gated on
/// `required_package: "textutil"`, not this package). Requiring
/// `textutil::trim` alone grants only these three-segment names (same
/// meta-package-flattening shape as `textutil::adjust`;
/// confirmed against tclsh 9.0.4 + real tcllib-2.0).
pub(super) const TEXTUTIL__TRIM_CMDS: &[Row] = &[
    (
        "textutil::trim::trim",
        Arity::new(1, 2),
        &["textutil::trim::trim string ?trim?"],
        "Removes any leading and trailing occurrences of the characters in trim (a regular expression, default whitespace) from string.",
    ),
    (
        "textutil::trim::trimleft",
        Arity::new(1, 2),
        &["textutil::trim::trimleft string ?trim?"],
        "Removes any leading occurrences of the characters in trim (a regular expression, default whitespace) from string.",
    ),
    (
        "textutil::trim::trimright",
        Arity::new(1, 2),
        &["textutil::trim::trimright string ?trim?"],
        "Removes any trailing occurrences of the characters in trim (a regular expression, default whitespace) from string.",
    ),
    (
        "textutil::trim::trimPrefix",
        Arity::exact(2),
        &["textutil::trim::trimPrefix string prefix"],
        "Removes the prefix from the beginning of string and returns the result.",
    ),
    (
        "textutil::trim::trimEmptyHeading",
        Arity::exact(1),
        &["textutil::trim::trimEmptyHeading string"],
        "Looks for empty lines (including lines consisting of only whitespace) at the beginning of the string and removes ",
    ),
];

/// The `textutil::tabify` package.
///
/// `tabify`/`untabify`/`tabify2`/`untabify2` are also re-exported as the
/// flattened `::textutil::tabify` / `::textutil::untabify` /
/// `::textutil::tabify2` / `::textutil::untabify2` aliases by the `textutil`
/// umbrella package — see `textutil__tabify.rs` / `textutil__untabify.rs` /
/// `textutil__tabify2.rs` / `textutil__untabify2.rs` for those (gated on
/// `required_package: "textutil"`, not this package). Requiring
/// `textutil::tabify` alone grants only these three-segment names (same
/// meta-package-flattening shape as `textutil::adjust`;
/// confirmed against tclsh 9.0.4 + real tcllib-2.0).
pub(super) const TEXTUTIL__TABIFY_CMDS: &[Row] = &[
    (
        "textutil::tabify::tabify",
        Arity::new(1, 2),
        &["textutil::tabify::tabify string ?num?"],
        "Replaces all appropriate occurrences of num spaces with a tab character in string.",
    ),
    (
        "textutil::tabify::untabify",
        Arity::new(1, 2),
        &["textutil::tabify::untabify string ?num?"],
        "Replaces all tabs in string with the appropriate number of spaces, assuming a tab size of num.",
    ),
    (
        "textutil::tabify::tabify2",
        Arity::new(1, 2),
        &["textutil::tabify::tabify2 string ?num?"],
        "Like tabify, but works line by line and only replaces spaces at true tab-stop-aligned positions.",
    ),
    (
        "textutil::tabify::untabify2",
        Arity::new(1, 2),
        &["textutil::tabify::untabify2 string ?num?"],
        "Like untabify, but works line by line.",
    ),
];
