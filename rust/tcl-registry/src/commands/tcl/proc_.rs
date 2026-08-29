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

//! `proc` — define a procedure.

use crate::hooks::LoweringHookId;
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// `proc name args body` — the synopsis, arity, and argument grammar are
/// byte-for-byte identical across the fetched Tcl 8.4, 8.5, 8.6, 9.0, and
/// 9.1 manpages (8.6, 9.0, and 9.1 — 9.1b0, the live beta — share
/// byte-identical DESCRIPTION text, differing only in incidental
/// line-wrapping and the version banner). No option/flag has ever existed
/// and no second form was ever added, so this is the command's only form,
/// unrestricted by dialect or version.
///
/// The DESCRIPTION prose itself grew across versions in ways that are
/// documentation clarification, not behaviour change: a paragraph noting
/// that a defaulted argument followed by a non-defaulted one becomes
/// de-facto required first appears in 8.5 (hedged there with "in 8.6 this
/// will be considered an error" — a threat 8.6 walked back to "though this
/// may change in a future version of Tcl"; it remains a non-error through
/// 9.1), and a sentence naming the body's execution namespace explicitly
/// first appears in 8.6. Neither is a new behaviour introduced at that
/// version — positional argument binding and namespace-scoped body
/// execution are both intrinsic to how `proc` has always worked — so both
/// are folded into the hover snippet below as plain current-truth prose
/// rather than as dialect gates.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "proc name args body",
    ..FormSpec::DEFAULT
}];

/// Command spec for `proc`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::INSTALLS_NAMED_DEFINITION
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NEVER_INLINE_BODY
            | Traits::IRULES_TOP_LEVEL_ONLY
            // …and the same "stored, not executed" fact the comment below
            // spells out, in the form a consumer can read: `DEFERS_BODY`
            // is what tells a static walk that an unreadable `body` word
            // costs it nothing about *this* call's completion (issue #1571).
            | Traits::DEFERS_BODY,
        // Deliberately no `TAINT_SINK` / `DYNAMIC_EVAL_BODY`: `body` is
        // *stored*, not executed, by this call. Unlike `eval` / `uplevel`
        // / `apply`, which run their tainted argument immediately as part
        // of the same call, a tainted `proc` body only becomes dangerous
        // if and when the newly-defined procedure is itself later
        // invoked — one more hop than either trait models. That is why
        // `proc` stays off both here even though it shares `apply`'s
        // exact `{args} body` grammar (see `commands/tcl/apply.rs`).
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        return_type: Some(TclType::String),
        lowering_hook: Some(LoweringHookId::Proc),
        // A `proc` body runs in the proc's own frame on each
        // call — never the caller's frame.  Stamping `Structural`
        // here lets generic `body_indices_to_skip` consumers (SSA,
        // dead-store, def-use) treat `proc` like every other
        // structural-body command without a string-match special
        // case.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Create a Tcl procedure.",
            synopsis: &["proc name args body"],
            snippet: "`args` is a list of formal-parameter specifiers, each either a bare name or a `{name default}` pair. Actual arguments bind positionally in order, so a defaulted parameter followed by a non-defaulted one becomes de-facto required — enough actual arguments must be supplied to reach every parameter that lacks a default. If the last formal parameter is named `args`, any actual arguments beyond the other formals are combined into a list (as `list` would build it) and bound to it, so the procedure accepts a variable number of arguments.\n\nEach call runs `body` in a fresh call frame: a local variable is created for every formal parameter, and the frame is discarded when the procedure returns. A bare name inside `body` resolves to that local frame; reaching a variable in the global namespace needs `global`, and reaching one in another namespace needs `variable`, `upvar`, or (Tcl 8.5+) `namespace upvar`. `body` executes in the namespace the procedure's name resolves in — its defining namespace, unless the procedure has since been renamed elsewhere with `rename`.\n\n`proc` itself always returns an empty string. The *procedure it defines* returns whatever an explicit `return` inside `body` specifies, or otherwise the value of the last command `body` executed; an error raised while running `body` propagates as that call's own error.",
            source: "Tcl proc(n)",
            examples: "proc greet {name {greeting Hello}} {\n    return \"$greeting, $name!\"\n}\ngreet Alice        ;# -> Hello, Alice!\ngreet Bob Hi       ;# -> Hi, Bob!\n\nproc sum args {\n    set total 0\n    foreach n $args { incr total $n }\n    return $total\n}\nsum 1 2 3          ;# -> 6",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Proc),

        world_effects: Some(WorldEffectDescriptor::EMPTY),
        // Declared once, by naming the stock descriptor (ledger C8).
        state_transitions: Some(crate::state_transition::command_binding::DEFINES_PROCEDURE),
        ..CommandSpec::DEFAULT
    }
}
