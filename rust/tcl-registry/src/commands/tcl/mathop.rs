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

//! `tcl::mathop` — namespace of math operator commands.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tcl::mathop::op ?arg ...?",
    dialects: None,
}];

/// Command spec for the `tcl::mathop` namespace itself — not a callable
/// command (there is no bare `tcl::mathop` command in real Tcl; invoking
/// it errors `invalid command name "tcl::mathop"`). The individual
/// operator commands (`tcl::mathop::+`, `tcl::mathop::eq`, …, every
/// spelling) are registered separately, one `CommandSpec` each, in
/// `mathop_generated.rs`. This entry exists only to carry hover /
/// documentation content for the namespace segment itself.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::mathop",
        traits: Traits::PURE,
        // The operator-command namespace was added in Tcl 8.5 (TIP 174):
        // no tcl8.4/TclCmd/mathop.html manpage exists (404), and the 8.5
        // manpage is the first to document it. Present unchanged in shape
        // through 8.6, 9.0, and 9.1 — 9.0 additionally gains the
        // `lt`/`le`/`gt`/`ge` string-ordering operators (TIP 461), but
        // that is a fact about the individual operator commands in
        // `mathop_generated.rs`, not about the namespace's own
        // availability.
        //
        // `TCL85_PLUS` also correctly excludes iRules — but not via
        // `DialectProfile::operators_as_commands`: this spec carries no
        // `Traits::OPERATOR_COMMAND` (only the individual operator-command
        // spellings in `mathop_generated.rs` do), so `is_available` /
        // `spec_visible`'s `operators_as_commands` clause is vacuously
        // satisfied for it regardless of a profile's setting — it never
        // actually gates this entry either way. The real reason iRules is
        // excluded is the plain dialect-intersection rule: `f5-irules`'s
        // `availability_mask` is the bare `IRULES` bit with no Tcl-core
        // version bit unioned in, so it never intersects `TCL85_PLUS`.
        // Every other modelled dialect (iApps, tmsh, the five EDA vendor
        // shells, Expect, bpf) mixes a Tcl-core version bit >= 8.5 into its
        // `availability_mask` alongside its vendor bit, so that same plain
        // intersection resolves them available without needing to name
        // any of them explicitly.
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(0),
        // No single `TclType` describes the family: the arithmetic and
        // bit-wise operators return a number, but boolean negation (`!`)
        // and the comparison, equality, and list-membership operators
        // return a boolean (`1`/`0`) — see `return_value` below.
        return_type: None,
        forms: FORMS,
        hover: Some(HoverSnippet {
            summary: "Namespace of mathematical, comparison, bit-wise, and logical operators exposed as ordinary commands.",
            synopsis: &["tcl::mathop::op ?arg ...?"],
            snippet: "`tcl::mathop` is a namespace, not a command in its own right — there is no bare `tcl::mathop` command to call. Each operator (`+`, `-`, `==`, `eq`, `in`, …) is a separate command inside the namespace, invoked as `tcl::mathop::<op>` or `::tcl::mathop::<op>`. Every operator implements the same computation `expr` uses internally, but as an ordinary, eagerly-evaluated command: redefining, renaming, or deleting one has no effect on `expr`'s own behaviour, and neither does adding a new command to the namespace. Because a command call evaluates all of its arguments up front, the short-circuiting `&&`, `||`, and `?:` operators have no command form here. `namespace import ::tcl::mathop::*` or `namespace path {::tcl::mathop}` brings the bare operator names into scope. Tcl 8.5 and 8.6 provide `==`/`!=`, `eq`/`ne`, and `<`/`<=`/`>`/`>=`; Tcl 9.0 adds `lt`/`le`/`gt`/`ge` for pure UNICODE string ordering.",
            source: "Tcl mathop(n)",
            examples: "namespace path {::tcl::mathop ::tcl::mathfunc}\nset total [+ 1 2 3]\nset inRange [<= 1 $x 10]\nset found [in $needle $haystack]",
            return_value: "A number for the arithmetic and bit-wise operators (+ - * / % ** & | ^ ~ << >>); 1 or 0 (true or false) for boolean negation and the comparison, equality, and list-membership operators (! == != eq ne < <= > >= in ni, plus lt/le/gt/ge from Tcl 9.0).",
        }),
        ..CommandSpec::DEFAULT
    }
}
