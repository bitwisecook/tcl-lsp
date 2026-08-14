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

//! `next` — invoke the next implementation of the currently executing method.
use crate::prelude::*;

/// `next` is a `TclOO` command: Tcl 8.4 and 8.5 have no `next.n` manpage at
/// all — `tcl-lang.org/man/tcl8.{4,5}/TclCmd/next.html` (and the `.htm`
/// swap) both return HTTP 200 for the site's soft-404 ("URL Not Found"),
/// confirmed with a raw HTTP fetch rather than just an HTML-to-text
/// summary, and the 8.5 `TclCmd` contents index independently lists no
/// `next`/`nextto`/`oo::*` command — `TclOO` was folded into core Tcl only
/// at 8.6 (TIP 257). The `next.n` page's DESCRIPTION, THE METHOD CHAIN,
/// METHOD SEARCH ORDER, FILTERS, and EXAMPLES sections are identical in
/// substance across 8.6, 9.0, and 9.1 (raw HTML diffed line-by-line, not
/// just summarised): the only textual deltas are the NAME line's
/// "invoke"/"Invoke" capitalisation and the SYNOPSIS's `package require
/// TclOO` (8.6) vs `package require tcl::oo` (9.0+) rename — neither
/// changes `next`'s own calling convention — so `next` carries a single,
/// version-ungated form under its command-level `TCL86_PLUS` gate. See
/// `nextto.rs`, which documents the identical finding for its sibling
/// command on the same manpage.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "next ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// `next` invokes a method implementation the registry cannot see into —
/// an arbitrary, program-defined method body picked at runtime by the
/// object's mixins, class, and superclasses — the same "unknowable
/// statically" placeholder idiom `eval`/`uplevel`/`apply` use for their
/// opaque bodies. Unlike those commands (caught earlier by an
/// `EVALUATES_CODE`/`CREATES_BARRIER` trait check in
/// `classify_side_effects`, which never actually reads their declared
/// `side_effects`), `next` carries neither trait, so this declared entry
/// is what the side-effect classifier consults directly — stated
/// explicitly rather than left to fall through to the conservative
/// unknown-write fallback. Mirrors `nextto.rs`'s identical declaration for
/// its sibling command.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "next",
        traits: Traits::LANGUAGE_KEYWORD
            .union(Traits::TCLOO_NEXT_CHAIN)
            .union(Traits::TCLOO_METHOD_CONTEXT)
            .union(Traits::TCLOO_REQUIRES_METHOD_FRAME),
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Invoke the next implementation of the currently executing method.",
            synopsis: &["next ?arg ...?"],
            snippet: "The next command is used within the body of a method, constructor, destructor, or filter to call the next implementation of the same call along the method resolution order (MRO) — the chain built from the object's mixins, its class's mixins, the object itself, its class, and the class's superclasses. Its result is the result of that next implementation; if none remains further along the chain, next raises an error (\"no next method implementation\", or the constructor/destructor equivalent inside those bodies). Arguments given to next replace, rather than extend, the arguments the current implementation itself received — a bare `next` with no arguments calls onward with zero arguments; use `next {*}$args` to forward the current arguments unchanged. A filter body typically calls next to let execution proceed toward the real method implementation, since filters sit at the front of the chain. Like `self` and `my`, next only exists inside a body reached through TclOO dispatch — called anywhere else it is simply not a defined command.",
            source: "Tcl man page next.n",
            examples: "oo::class create Base {\n    method greet {who} { return \"Base: hello $who\" }\n}\noo::class create Derived {\n    superclass Base\n    method greet {who} {\n        return \"Derived: [next $who]\"\n    }\n}\n[Derived new] greet World\n# -> Derived: Base: hello World",
            return_value: "The result returned by the next implementation in the method chain.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
