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

//! `::tcl::unsupported::corotype` — inspect a coroutine's suspension state.
//!
//! Added mid-series in the Tcl 8.6 line, not alongside `coroutine`/`yield`/
//! `yieldto` themselves: `generic/tclBasic.c`'s `CoroTypeObjCmd` and its
//! `::tcl::unsupported::corotype` registration are absent from the 8.6.0
//! release and from every 8.6.x release through 8.6.9, present from 8.6.10
//! onward. Requires coroutines, so it does not exist under Tcl 8.4 or 8.5 at
//! all — neither has a `coroutine.n` page (TIP 328 landed in 8.6). Carried
//! into Tcl 9.0 and 9.1 unchanged: `CoroTypeObjCmd`'s body is byte-for-byte
//! identical across the 8.6.16, 9.0.4, and 9.1b0 sources bar internal C-API
//! churn (`int`/`ClientData` becoming `Tcl_Size`/`TCL_UNUSED(void *)` in
//! 9.x), and the "coro type" cases in Tcl's own `tests/coroutine.test` are
//! byte-for-byte identical across all three releases too.
//!
//! Genuinely undocumented rather than merely "internal": none of the
//! 8.6/9.0/9.1 `coroutine.n` manual pages mention `corotype`, and it has no
//! manual page of its own on any version. It is exercised directly by Tcl's
//! own `tests/coroutine.test` suite (the core regression tests, not the
//! `tcltest` package/library), and is the type-classifying half of the
//! `coroinject`/`coroprobe` probe machinery — the source comment on the
//! active/suspended check reads "which matters when you're injecting a
//! probe".
//!
//! The VM registers only the fully-qualified spelling and relies on its
//! own generic bare→qualified namespace resolution (`ns_resolve_qualified`)
//! to reach it either way. `tcl-registry`'s lookup has no such generic
//! fallback in that direction (only qualified→bare, the opposite way), so —
//! matching the `tcl::process`/`tcl::zipfs`/`tcl::idna` sibling 9.0-era
//! additions — both spellings are registered here explicitly.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "::tcl::unsupported::corotype coroName",
    ..FormSpec::DEFAULT
}];

// Pure introspection: looks up the named coroutine's parked/running state
// and returns one of three literal strings, or errors — it never mutates
// anything. `InterpState` is the closest existing `SideEffectTarget` (no
// dedicated "coroutine table" variant exists), read-only.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    ..SideEffect::DEFAULT
}];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        // `cmd_corotype` (`tcl-vm/src/cmd_coro.rs`) is a flat hashmap lookup
        // with no eval fallback and no `Frame` of its own — genuinely
        // frame-free, like its list/string/set siblings on the audited
        // allow-list (`registry.rs`'s
        // `frameless_runtime_covers_the_audited_allow_list`, updated
        // alongside this spec).
        traits: Traits::FRAMELESS_RUNTIME,
        // TCL86_PLUS, not TCL90_PLUS: present (mid-series) in 8.6 and
        // unchanged in 9.0/9.1 — see the module doc comment for the exact
        // patch-level evidence. Not universal (`dialects: None`) either:
        // absent from 8.4/8.5 (no coroutines at all there), and — being a
        // standard-Tcl-core internal with no sibling registration anywhere
        // under `commands/irules|iapps|expect|tk|itcl|eda_*` — absent from
        // every non-Tcl-version dialect too, all of which fall outside
        // `TCL86_PLUS`. That same version gate is all that is needed to
        // exclude this command from iRules: `TCL86_PLUS` carries no
        // `IRULES` bit, so the spec never intersects iRules' bare `IRULES`
        // mask and is excluded on the version axis alone — no separate
        // per-dialect exclusion, and no disable list.
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(1, 1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Report whether a coroutine is active, or how it last suspended.",
            synopsis: &["::tcl::unsupported::corotype coroName"],
            snippet: "coroName must currently name a live coroutine — one created by coroutine and not yet finished; an ordinary command, a nonexistent name, or a coroutine that has already returned all raise \"can only get coroutine type of a coroutine\". The result is active while the coroutine is actually running — including when corotype is called on the current coroutine from inside its own body via [info coroutine], since there is no way to know what a running coroutine will do next. Once the coroutine is parked, the result instead names whichever command it is suspended at: yield or yieldto. This is the type-classifying half of the coroutine probe/inject machinery: coroprobe and coroinject both use it to decide how a resumption value should be delivered. It performs no I/O and touches no filesystem or process state, so — despite living in the ::tcl::unsupported namespace — it works unrestricted inside a safe interpreter.",
            source: "Tcl ::tcl::unsupported::corotype (undocumented; no manual page)",
            examples: "coroutine demo eval {\n    yield\n    yield \"phase 1\"\n    yieldto string cat \"phase 2\"\n    ::tcl::unsupported::corotype [info coroutine]\n}\nputs [demo]                              ;# -> phase 1\nputs [::tcl::unsupported::corotype demo] ;# -> yield (parked at the 2nd yield)\nputs [demo]                              ;# -> phase 2\nputs [::tcl::unsupported::corotype demo] ;# -> yieldto (parked at yieldto)\nputs [demo]                              ;# -> active (corotype ran on itself, mid-execution)",
            return_value: "active if coroName is currently running; otherwise yield or yieldto, naming whichever command it is suspended at.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

/// Command spec for the namespace-relative form `tcl::unsupported::corotype`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::unsupported::corotype")
}

/// Command spec for the fully-qualified form `::tcl::unsupported::corotype`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::unsupported::corotype")
}
