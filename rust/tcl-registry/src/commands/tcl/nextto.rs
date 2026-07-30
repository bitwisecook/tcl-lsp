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

//! `nextto` — invoke a specific class's implementation of the current method.
use crate::prelude::*;

/// `nextto` invokes a method implementation the registry cannot see into —
/// an arbitrary, program-defined method body picked at runtime by the
/// target class name — the same "unknowable statically" placeholder idiom
/// `eval`/`uplevel`/`apply` use for their opaque bodies. Unlike those
/// commands (caught earlier by an `EVALUATES_CODE`/`CREATES_BARRIER` trait
/// check in `classify_side_effects`, which never actually reads their
/// declared `side_effects`), `nextto` carries neither trait, so this
/// declared entry is what the side-effect classifier consults directly —
/// stated explicitly rather than left to fall through to the conservative
/// unknown-write fallback.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// `next`/`nextto` are `TclOO` commands: the Tcl 8.4 and 8.5 `TclCmd`
/// manpage trees have no `next.n` page at all. Both
/// `tcl-lang.org/man/tcl8.{4,5}/TclCmd/next.html` (and `.htm`) return
/// HTTP 200 with title "URL Not Found" — the site's soft-404, confirmed
/// by a raw HTTP fetch (curl, not just an HTML-to-text summary): there
/// is no literal 404 status on this request, only body content marking
/// the page missing — and the 8.5 `TclCmd` contents index independently
/// lists no "next"/"nextto"/`oo::*` command. `TclOO` was folded into
/// core Tcl only at 8.6 (TIP 257). The `next.n` page's DESCRIPTION,
/// METHOD CHAIN, METHOD SEARCH ORDER, FILTERS, SEE ALSO, and KEYWORDS
/// sections are identical in substance across 8.6, 9.0, and 9.1 (raw
/// HTML diffed line-by-line, not just summarised): the only textual
/// deltas are the NAME line's "invoke"/"Invoke" capitalisation and the
/// SYNOPSIS's `package require TclOO` (8.6) vs `package require
/// tcl::oo` (9.0+) rename — neither changes `nextto`'s own calling
/// convention — so `nextto` carries a single, version-ungated form
/// under its command-level `TCL86_PLUS` gate.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "nextto class ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nextto",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Invoke a specific class's implementation of the currently executing method.",
            synopsis: &["nextto class ?arg ...?"],
            snippet: "Behaves like `next`, but instead of advancing to the next implementation in the method resolution order it jumps straight to the implementation provided by `class`. `class` must appear later than the currently executing step in the method's call chain — mixed-in classes count, not only true superclasses — and must supply a genuine, non-filter implementation there; otherwise `nextto` raises an error naming the class it could not find a further implementation for. Any trailing `arg` words are passed to that implementation in place of the current one's own arguments, and `nextto`'s result is whatever that implementation returns. Like `next`, it only exists inside a method, constructor, destructor, or filter body reached through TclOO dispatch — called anywhere else it is simply not a defined command.",
            source: "Tcl man page next.n",
            examples: "oo::class create Base {\n    method report {} { return Base }\n}\noo::class create Mid {\n    superclass Base\n    method report {} { return Mid+[next] }\n}\noo::class create Derived {\n    superclass Mid\n    method report {} { return Derived+[nextto Base] }\n}\n[Derived new] report\n# -> Derived+Base -- nextto jumps straight to Base, skipping Mid's implementation",
            return_value: "The result returned by `class`'s method implementation.",
        }),
        forms: FORMS,
        // The leading `class` word is a symbolic name, structurally marking
        // this form as carrying an explicit MRO-search-start class — how
        // the analyser's `queue_next_arity_candidate` tells `nextto` apart
        // from bare `next` without matching on the command name.
        arg_roles: &[(0, ArgRole::Name)],
        traits: Traits::LANGUAGE_KEYWORD
            .union(Traits::TCLOO_NEXT_CHAIN)
            .union(Traits::TCLOO_METHOD_CONTEXT)
            .union(Traits::TCLOO_REQUIRES_METHOD_FRAME),
        ..CommandSpec::DEFAULT
    }
}
