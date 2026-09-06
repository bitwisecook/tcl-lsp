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

//! `my` — invoke a method on the current object.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "my methodName ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// Every argument after the `variable` subcommand word is a variable name to
/// link (`my variable a b c` links all of `a`, `b`, `c`), so — unlike a
/// fixed-position `arg_roles` entry — this must be a resolver: the SSA
/// def-extraction generic in `lowering/mod.rs::lower_default` (via
/// `CommandRegistry::arg_indices_for_role`) calls it with the sub-relative
/// args (everything after `variable`) and offsets the returned indices by
/// `+1` for the subcommand word automatically. An empty `args` (bare `my
/// variable`) yields an empty `Vec` here, which is correct — zero names is
/// a legal no-op, not an error; see the `arity` comment below.
fn my_variable_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (0..args.len())
        .filter_map(|i| u8::try_from(i).ok())
        .map(|i| (i, ArgRole::VarWrite))
        .collect()
}

/// `my` forwards to an arbitrary method name, but `my variable ?varName
/// ...?` is `TclOO`'s own reserved dispatch — the per-object-namespace
/// analogue of the top-level `variable` command (links each `varName` to
/// the object's private-namespace storage, same as `oo::define CLASS {
/// variable name }` does for every method body). Modelled as the one
/// recognised subcommand so [`Traits::CREATES_SCOPE_ALIAS`]'s
/// per-subcommand sibling (`creates_scope_alias`) picks it up generically —
/// the compiler's `is_scope_alias_call` widens its defs to `Overdefined`
/// exactly like `global`/`variable`/`upvar`/`namespace upvar`, so a
/// variable linked this way (whose true intrep may have been set by a
/// *different* method) is not misreported as a local shimmer.
/// `allow_unknown_subcommands` keeps every other `my <method>` form
/// dispatching freely instead of tripping W001 — the analyser cannot know a
/// class's user-defined method names statically, so only `variable` (and
/// its unique-prefix abbreviations) is validated.
const SUBCOMMANDS: &[SubCommand] = &[SubCommand {
    name: "variable",
    // `obj variable ?varName ...?` (object.n's NON-EXPORTED METHODS list)
    // brackets the whole `varName ...` group together — zero or more
    // names, not "at least one". Confirmed in the reference C
    // implementation (`TclOO_Object_LinkVar`, generic/tclOOBasic.c): the
    // only arity check is `objc - Tcl_ObjectContextSkippedArgs(context) <
    // 0`, never tripped by a zero-name call, so bare `my variable` is a
    // legal no-op returning "". This project's own VM agrees
    // (`tcl-vm/src/cmd_oo.rs::builtin_method`'s "variable" arm loops over
    // `args` with no emptiness check). Wording is identical in object.n
    // across 8.6, 9.0, and 9.1 — my.n's own page only shows the EXAMPLES
    // usage, not the arity contract. (The same C function also rejects a
    // varName that resolves to an array element, "name refers to an
    // element in an array" / errorCode {TCL UPVAR LOCAL_ELEMENT}, on top
    // of the documented "::"-separator restriction below; neither this
    // registry's static spec nor this project's VM currently enforces the
    // array-element case.)
    arity: Arity::any(),
    detail: "Link each named object-instance variable into the current method's local scope; a bare `my variable` (no names) is a legal no-op. Each varName must be unqualified — no :: namespace separator.",
    synopsis: "my variable ?varName ...?",
    return_type: Some(TclType::String),
    arg_role_resolver: Some(my_variable_arg_roles),
    arg_role_resolver_roles: &[ArgRole::VarWrite],
    creates_scope_alias: true,
    side_effects: &[SideEffect {
        target: SideEffectTarget::Variable,
        writes: true,
        ..SideEffect::DEFAULT
    }],
    surface: Some(SpecSurface::TCL86_PLUS),
    ..SubCommand::DEFAULT
}];

/// `my` invokes a method implementation the registry cannot see into — an
/// arbitrary, program-defined method body supplied at runtime by whichever
/// mixin, class, or object actually provides `methodName` — the same
/// "unknowable statically" placeholder idiom `eval`/`uplevel`/`apply` use
/// for their opaque bodies. `my`'s own `TclOO`-dispatch siblings `oo_next.rs`
/// and `nextto.rs` document the identical situation for the same reason:
/// none of the three carry `Traits::EVALUATES_CODE` or
/// `Traits::CREATES_BARRIER`, so without a declared entry here
/// `classify_side_effects` (`tcl-compiler/src/side_effects.rs`) would
/// reach `my` only via the conservative unknown-write *fallback* at the
/// very end of that function — stated explicitly instead, matching
/// `next`/`nextto` (and `timerate`, audited the same way in
/// `registry.rs`'s `timerate_registered_with_body_and_int_hint` test).
/// This entry governs only the general `my methodName ...` case:
/// `dialect_side_effect_hints` checks a matched subcommand's own
/// `side_effects` first, so `my variable ...` is still classified by
/// `SUBCOMMANDS`'s own more precise `Variable`-write entry above, never by
/// this `Unknown` one.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// `my` is the one family member that is **not**
/// [`Traits::TCLOO_REQUIRES_METHOD_FRAME`].
///
/// It is not an `::oo::Helpers` member at all — `namespace which -command
/// my` answers `::oo::ObjN::my`, a command in the *object's own*
/// namespace — and a class is itself an object, so it works in a Tcl 9
/// class `initialise` / `initialize` body as well as in a method. tclsh
/// 9.0.4, inside `oo::class create ::P { initialize { … } }`:
///
/// ```text
/// my: which='::oo::Obj20::my'   my new -> ::oo::Obj22      (dispatches on the class object)
/// self / link / classvariable / next / nextto  ->  … may only be called from inside a method
/// ```
///
/// It still carries [`Traits::TCLOO_METHOD_CONTEXT`], because outside any
/// object frame there is no `my` to call at all (top level: `invalid
/// command name "my"`).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "my",
        traits: Traits::LANGUAGE_KEYWORD
            | Traits::TCLOO_SELF_DISPATCH
            | Traits::TCLOO_METHOD_CONTEXT,
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        subcommands: SUBCOMMANDS,
        allow_unknown_subcommands: true,
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "invoke any method, including non-exported ones, of the current object",
            synopsis: &["my methodName ?arg ...?"],
            snippet: "The my command is used within the body of a method, constructor, or destructor to invoke any method the current object supports, including ones that are not exported — unlike calling the object's own command directly (e.g. [self object] methodName), which only reaches exported methods. In Tcl 9.0+, my can also reach a declared private method of the object or class — a visibility tier stricter than plain non-export — when called from another method defined by that same object or class. Each object has its own my command, living in its instance namespace; a saved reference to it keeps invoking the correct object even if that object is later renamed. A common idiom is `namespace code {my Callback}`, which packages a call to a non-exported method as a command prefix usable from outside the object — for example as a variable-trace or event-handler callback.",
            source: "Tcl my(n)",
            examples: "oo::class create Counter {\n    method count {} {\n        my variable total\n        puts [incr total]\n    }\n}\nset c [Counter new]\n$c count                  ;# prints 1\n$c count                  ;# prints 2\n$c count                  ;# prints 3",
            return_value: "The result of the invoked method.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
