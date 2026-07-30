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

//! `self` — query the current method invocation's context.
use crate::prelude::*;

/// `self` is a `TclOO` command: like `next` (`next.rs`'s survey documents
/// the same underlying fact), Tcl 8.4 and 8.5 have no `self.n` manpage at
/// all — `tcl-lang.org/man/tcl8.{4,5}/TclCmd/self.html` both return the
/// site's soft-404 ("URL Not Found", HTTP 200) rather than real content,
/// confirmed with a raw HTTP fetch rather than just an HTML-to-text
/// summary — `TclOO` was folded into core Tcl only at 8.6 (TIP 257). The
/// self.n DESCRIPTION and per-subcommand text are identical in substance
/// across 8.6.18, 9.0.4, and 9.1b0 (raw HTML diffed line-by-line): the only
/// deltas are the SYNOPSIS's `package require TclOO` (8.6) vs `package
/// require tcl::oo` (9.0+) rename — which does not change `self`'s calling
/// convention — and one behavioural addition documented on the `call`
/// value below. So `self` carries a single, version-ungated form under its
/// command-level `TCL86_PLUS` gate.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "self ?subcommand?",
    dialects: None,
}];

/// `self subcommand`'s nine legal words, each a bare keyword taking no
/// further arguments of its own — deliberately modelled as a closed
/// [`ArgValue`] set (`arg_values` + `closed_value_args`) rather than as
/// [`crate::spec::SubCommand`] entries: `self`'s own top-level `arity` is
/// `0..=1` (a bare `self` is a legal, extremely common call, equivalent to
/// `self object`), and `tcl_compiler::type_infer::return_type_for_command`
/// special-cases *any* command with a non-empty `subcommands` table to
/// require a matched subcommand word before it will consult a return type
/// at all — falling back to `overdefined` rather than the command-level
/// `return_type` when no word is present. Populating `subcommands` here
/// would silently regress static type inference for the common bare-`self`
/// call from `String` to unknown. `arg_values`/`closed_value_args` carries
/// no such trap (`args.get(i)` in both `return_type_for_command` and the
/// W127 emitter cleanly skips an absent index), so it is the correct
/// mechanism for this specific "optional closed positional word" shape —
/// see `HTTP::version`'s identically-shaped optional first argument
/// (`irules/http__version.rs`) for the same pattern outside `TclOO`.
///
/// All nine words are present, under the same synopsis, in every fetched
/// version (8.6.18, 9.0.4, 9.1b0), so none carries its own `min_tcl`. One
/// behavioural (not structural) delta: `call`'s first list element
/// additionally labels a private-method implementation `private` instead
/// of `method` from Tcl 9.0 on — present in the 9.0.4 and 9.1b0 text,
/// absent from 8.6.18's — captured as version-qualified prose in that
/// entry's `detail` rather than a version gate, since `self call` itself
/// is offered unchanged across 8.6-9.1. This project's own bytecode VM
/// (`tcl-vm/src/cmd_oo.rs::cmd_self`) currently only implements
/// `object`/`namespace`/`class`/`method`/`call`/`target`, erroring on
/// `caller`/`filter`/`next` — a VM-backing gap, not a reason to
/// under-document those three here: the registry models the real Tcl
/// command per self.n regardless of this project's own execution coverage.
const SELF_SUBCOMMAND_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "call",
        detail: "Two-element list describing the current method's implementation chain: the first element matches `info object call`'s report for this call (`<constructor>`/`<destructor>` for those bodies; from Tcl 9.0, a private-method implementation is reported as `private` instead of `method`), and the second element is the zero-based index of the implementation currently executing.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "caller",
        detail: "When the method was itself invoked from inside another object's method, a three-element list: the declaring object or class of that calling method, the object it was invoked on, and its name (`<constructor>`/`<destructor>` for those bodies).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "class",
        detail: "The class that defines the currently executing method implementation. Changes as `next` advances along the method chain; fails if the implementation was defined directly on an object rather than a class — use `info object class [self object]` for the current object's own class.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "filter",
        detail: "Inside a filter implementation, a three-element list: the object or class that declared the filter (which may differ from the one providing this particular implementation), either `object` or `class` depending on which, and the filter's name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "method",
        detail: "The name of the currently executing method (`<constructor>`/`<destructor>` for those bodies).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "namespace",
        detail: "The object's unique per-instance namespace.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "next",
        detail: "When the call chain continues past the current implementation (the point where `next` would invoke a further implementation), a two-element list naming the class or object that declares the next part of the chain and its method name. The empty string when the current implementation is the last one in the chain.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "object",
        detail: "The name of the object the method was invoked on. Also what a bare self, with no subcommand, returns.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "target",
        detail: "Inside a filter implementation, a two-element list naming the declarer of the method being filtered and its actual name.",
        ..ArgValue::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "self",
        traits: Traits::PURE
            | Traits::LANGUAGE_KEYWORD
            | Traits::TCLOO_INTROSPECTION
            | Traits::TCLOO_METHOD_CONTEXT
            | Traits::TCLOO_REQUIRES_METHOD_FRAME,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        arg_values: &[(0, SELF_SUBCOMMAND_VALUES)],
        closed_value_args: &[0],
        hover: Some(HoverSnippet {
            summary: "query information about the current method invocation",
            synopsis: &["self ?subcommand?"],
            snippet: "The self command is used within the body of a method, constructor, or destructor to introspect how it is currently being called; used anywhere else, self is simply not a defined command. Called with no subcommand it behaves like self object. self object, self method, self class, and self namespace return the current object, the running method's name, the class that defined it, and the object's instance namespace respectively; self class fails when the implementation was defined directly on an object rather than a class. self call and self next describe the method's position in its call chain — the sequence of implementations built from the object's mixins, class, and superclasses that next walks along — while self filter and self target are meaningful only inside a filter implementation, and self caller only when the method was itself invoked from inside another object's method.",
            source: "Tcl man page self.n",
            examples: "oo::class create Greeter {\n    method hello {} {\n        puts \"[self] ([self class]) running [self method]\"\n    }\n}\nGreeter create g\ng hello                    ;# -> ::g (::Greeter) running hello",
            return_value: "Depends on the subcommand: self object, self method, self class, and self namespace each return a single name; self call, self caller, self filter, self next, and self target each return a two- or three-element list (self next returns the empty string once the chain is exhausted). Omitting the subcommand returns the same value as self object.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
