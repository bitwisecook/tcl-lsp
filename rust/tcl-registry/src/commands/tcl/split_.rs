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

//! `split` — split a string into a proper Tcl list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "split string ?splitChars?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "split",
        // `split`'s `dialects` group carries the `IRULES` bit explicitly
        // (`ALL_TCL.union(IRULES)`), deliberately, not by oversight: it is
        // a pure string/list-manipulation command, exactly the class iRules
        // keeps — the sandbox bans filesystem/process/interp commands
        // (`open`, `exec`, `file`, `glob`, `namespace`, …), never a
        // data-manipulation one — so it resolves under the bare `IRULES`
        // availability mask; and none of the irules/, expect/, tk/, itcl/,
        // iapps/, or eda_*/ command packs declares an override or an extra
        // form for it, so it resolves identically everywhere — the common
        // iRules idiom `split [HTTP::uri] "/"` relies on exactly this.
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        byte_array_effect: ByteArrayEffect::Coerces,
        const_fold: Some(crate::const_fold::fold_split),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        // `split string ?splitChars?` — synopsis, argument defaults, and
        // documented behaviour are byte-for-byte identical across the
        // split.n manual page in Tcl 8.4, 8.5, 8.6, 9.0, and 9.1: no
        // option or form was ever added, removed, or had its default
        // changed. (The only textual difference anywhere across the five
        // fetched pages is the illustrative EXAMPLES string shrinking from
        // "comp.lang.tcl.announce" to "comp.lang.tcl" from 8.6 onward — a
        // cosmetic doc edit, not a behavioural change.)
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Split a string into a proper Tcl list.",
            synopsis: &["split string ?splitChars?"],
            snippet: "Returns a list created by splitting string at each character that occurs in splitChars: every character in splitChars is its own delimiter (not a substring to match), so string is scanned once and each maximal run of non-delimiter characters becomes one list element. Adjacent delimiter characters, or a delimiter at the very start or end of string, produce empty-string elements. splitChars defaults to the standard whitespace characters (space, tab, newline, and carriage return) when omitted; passing an empty string instead splits string into its individual characters. Splitting an empty string always returns an empty list (zero elements), regardless of splitChars. The result is always a syntactically well-formed Tcl list — an element that needs braces or backslashes to round-trip (an embedded space, an unbalanced brace) is quoted accordingly — which makes split a common idiom for turning arbitrary delimited text, even text that is not itself a valid list, into one that is.",
            source: "Tcl split(n)",
            examples: "# Default splitChars is space/tab/newline/CR\nsplit \"the quick brown fox\"\n\n# Explicit delimiter set: split on comma or semicolon\nsplit \"a,b;c\" \",;\"\n\n# Parse a colon-separated /etc/passwd-style record\nlassign [split $line \":\"] userName password uid gid longName homeDir shell\n\n# Explode a string into its individual characters\nsplit \"abc\" {}\n\n# The empty string always yields an empty list, whatever splitChars is\nsplit \"\"",
            return_value: "A proper Tcl list: one element for each run of characters in string that lies between (or before the first / after the last) occurrence of a splitChars character. Adjacent or edge-of-string delimiters contribute empty-string elements; splitting the empty string always yields a list of zero elements.",
        }),
        forms: FORMS,
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}
