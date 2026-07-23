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

//! `return` — return from the current procedure or script.

use crate::hooks::{InlineCodegenHookId, LoweringHookId};
use crate::prelude::*;

// Tcl 8.4's SYNOPSIS is the single line `return ?-code code? ?-errorinfo
// info? ?-errorcode code? ?string?`: only -code, -errorinfo, and
// -errorcode are recognised, there is no generic "any option" mechanism,
// and the trailing argument is named `string`. Tcl 8.5 replaced this with
// three forms (`return ?result?` / `return ?-code code? ?result?` /
// `return ?option value ...? ?result?`), renamed the trailing argument to
// `result`, and added -level/-options as recognised options; -errorstack
// followed in 8.6 (see the OptionSpecs below for the exact per-option
// gates). That 8.5 shape is unchanged through 9.0 and 9.1 — 9.1's
// return.html is byte-for-byte identical to 9.0's once the doc-anchor
// line-number IDs are discounted.
const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        // -level is Tcl 8.5+ (see the per-option gate on the `-level`
        // OptionSpec below, and the 8.4 SYNOPSIS split immediately
        // below this entry) — this form's own `dialects` must say so
        // too rather than inheriting the command's unrestricted
        // `dialects: None`, or it would claim `-level` is legal
        // syntax in Tcl 8.4 and iRules (embedded Tcl 8.4.6), which it
        // is not: the 8.4 manpage's SYNOPSIS/body recognise only
        // -code/-errorinfo/-errorcode.
        synopsis: "return ?-code code? ?-level level? ?result?",
        dialects: Some(DialectSet::TCL85_PLUS),
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "return ?-code code? ?-errorinfo info? ?-errorcode code? ?string?",
        dialects: Some(DialectSet::TCL84),
    },
    // F5 `return(1)`: directly inside a `when EVENT { … }` body, `return`
    // takes no arguments — `return_context_gate` below enforces this
    // structurally; this entry makes the restricted synopsis queryable
    // (completion/hover) rather than only documented in prose. Outside an
    // event body (e.g. inside a `proc`) the gate doesn't fire, but iRules
    // still only ever exposes the Tcl-8.4-shaped form above, never the
    // fuller 8.5+ one: iRules is a genuine embedded Tcl 8.4.6 (its
    // `f5-irules` profile in `tcl-dialect/src/profile.rs` pins
    // signature_base/runtime_base/version_ceiling all to 8.4), so
    // -level/-options/-errorstack can never resolve there regardless of
    // context — their DialectSet gates can't intersect the bare IRULES
    // availability mask (`ProfileQueries::is_option_available`). This
    // entry narrows the *form*, not the command's own Tcl-version gating
    // — return itself stays universal (`dialects: None` below).
    FormSpec {
        kind: FormKind::Default,
        synopsis: "return",
        dialects: Some(DialectSet::IRULES),
    },
];

/// `-code`'s five symbolic completion codes, each paired with its
/// canonical integer equivalent (`return -code error …` ≡
/// `return -code 1 …`) — verified against real `tclsh` 8.6.14: `-code`
/// also accepts an arbitrary integer alongside these (see
/// [`OptionArity`]'s `integer` field on the `-code` [`OptionSpec`] below),
/// so this set is completion/hover metadata, not exhaustive validation.
const RETURN_CODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "ok",
        detail: "Normal completion (TCL_OK).",
        code: Some(0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "error",
        detail: "Error return (TCL_ERROR).",
        code: Some(1),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "return",
        detail: "Propagate TCL_RETURN.",
        code: Some(2),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "break",
        detail: "Propagate TCL_BREAK.",
        code: Some(3),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "continue",
        detail: "Propagate TCL_CONTINUE.",
        code: Some(4),
        ..ArgValue::DEFAULT
    },
];

/// Content check for `-errorstack`'s value: real `tclsh` 8.6.14 rejects an
/// odd-sized list ("forbidden odd-sized list for -errorstack") regardless
/// of arity — the word is always consumed (confirmed empirically: a
/// multi-bare-word `-errorstack a b c d` still only takes `a` as the
/// value), so this only ever needs to validate `args[start]`, never adjust
/// how many words are consumed.
fn errorstack_value(args: &[&str], start: usize) -> OptionValueOutcome {
    let Some(word) = args.get(start) else {
        return OptionValueOutcome {
            words: 0,
            invalid: Some("missing value for -errorstack"),
        };
    };
    let invalid = match tcl_syntax::list::split_list_raw(word) {
        Ok(elems) if elems.len() % 2 == 0 => None,
        Ok(_) => Some("value must be an even-sized list"),
        Err(_) => Some("value must be a valid Tcl list"),
    };
    OptionValueOutcome { words: 1, invalid }
}

/// iRules restricts `return` used directly inside a `when EVENT { … }`
/// body to the bare form (F5 `return(1)`: "Causes immediate exit from the
/// currently executing event in the currently executing iRule" — no
/// documented arguments). A `proc` — even one defined inside the same
/// `ltm rule` — is unaffected: procs "live outside an event" structurally
/// (`DevCentral`, "Advanced iRules: Getting Started with iRules
/// Procedures"), so `self.current_event` is naturally `None` inside one
/// and this gate never fires there.
fn return_context_gate(args: &[&str], in_event_body: bool) -> Option<&'static str> {
    (in_event_body && !args.is_empty()).then_some(
        "`return` takes no arguments directly inside an iRules event body; \
         wrap the call in a proc to use -code/-level/-errorcode/etc.",
    )
}

const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::InterpState,
        reads: false,
        writes: true,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
    SideEffect {
        target: SideEffectTarget::EventControl,
        reads: false,
        writes: true,
        connection_side: ConnectionSide::None,
        dialects: Some(DialectSet::IRULES),
    },
];

/// Command spec for `return`.
///
/// -level and -options were added in Tcl 8.5 — absent from Tcl 8.4's
/// single-line SYNOPSIS and body text, which recognise only -code,
/// -errorinfo, and -errorcode. -errorstack followed in Tcl 8.6. Tcl 9.0's
/// manpage newly documents (without newly enforcing) that -code's integer
/// values 5–0x3fffffff are reserved for application use; the 8.4-8.6
/// manpages give the plain "value must be an integer" wording with no
/// such range. Tcl 9.1's return.html is byte-for-byte identical to 9.0's
/// (bar the doc-anchor line-number IDs) — no 9.1-specific delta exists
/// for this command.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "return",
        // Present, unrestricted, and not in any dialect's
        // `disabled_commands` list (only `f5-irules` has a non-empty one,
        // and `return` isn't on it — checked against
        // `tcl-dialect/src/profile.rs`) — a pure control-flow primitive
        // with no filesystem/process/network access, so every dialect
        // that hosts a real Tcl core (irules, iapps, tmsh, the EDA
        // shells, expect, tk, itcl) carries it unmodified. Its *legal
        // option set* still narrows per dialect/version through the
        // individual OptionSpecs' own `dialects` gates below (enforced
        // generically by `ProfileQueries::is_option_available`), and its
        // *argument shape* narrows further inside an iRules event body
        // (see `FORMS` / `return_context_gate`) — neither of which is a
        // whole-command dialect gate.
        dialects: None,
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Return from the current procedure/script with optional control-code metadata.",
            synopsis: &[
                "return ?result?",
                "return ?-code code? ?result?",
                "return ?option value ...? ?result?",
            ],
            snippet: "With no options, immediately returns from the current procedure — or, inside a script evaluated by source, stops evaluating that script — with an empty result unless result is given. -code sets an exceptional completion code instead of the default ok; -errorinfo and -errorcode (plus -errorstack from Tcl 8.6) are honoured only when -code is error, each seeding the matching global (errorInfo/errorCode) and defaulting to Tcl's own trace or \"NONE\" when omitted. -level (Tcl 8.5+, default 1) counts call-stack levels up the code applies to; -level 0 makes this return itself complete with code immediately, which is how interp alias builds break/continue-alike commands. -options (Tcl 8.5+) merges a dict of option/value pairs in as if each had been passed directly — typically the options dict a catch just captured, to re-raise a caught error unchanged. Tcl 8.4 recognises only -code, -errorinfo, and -errorcode. In an iRules event body, return takes no arguments at all and exits only the current event invocation; outside an event body iRules still exposes just that 8.4-shaped option set, since its embedded core is Tcl 8.4.6.",
            source: "Tcl return(n); F5 return(1)",
            examples: "proc safeDiv {a b} {\n    if {$b == 0} {\n        return -code error \"cannot divide by zero\"\n    }\n    return [expr {$a / $b}]\n}\n\n# Re-throw a caught error unchanged, preserving errorInfo/errorCode (Tcl 8.5+)\nif {[catch {doWork} result options]} {\n    return -options $options $result\n}\n\n# Build a break-alike via -level 0 (Tcl 8.5+)\ninterp alias {} Break {} return -level 0 -code break",
            return_value: "The result string that becomes the enclosing procedure's result (or, inside a script evaluated by source, that script's result). With -code other than the default ok, result instead becomes the payload of the resulting exceptional completion — e.g. the message text of a -code error, or the value a catch reports when it traps this return.",
        }),
        lowering_hook: Some(LoweringHookId::Return),
        inline_codegen_hook: Some(InlineCodegenHookId::Return),
        forms: FORMS,
        context_gate: Some(return_context_gate),
        options: const {
            &[
                OptionSpec {
                    name: "-code",
                    value: OptionValue::Takes(OptionArg {
                        values: RETURN_CODE_VALUES,
                        closed: true,
                        integer: Some(IntegerDomain::Any),
                        hint: "code",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Exceptional return code: ok/error/return/break/continue, or an integer (5–0x3fffffff reserved for application use by convention, not enforced).",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-level",
                    value: OptionValue::Takes(OptionArg {
                        integer: Some(IntegerDomain::Range(0, 2_147_483_647)),
                        hint: "level",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Stack levels up the code applies to (default 1). 0 means this `return` itself returns -code. Must be 0..=2147483647; a negative or larger value is a hard error (unlike -code's integer, which never errors in this range).",
                    dialects: Some(DialectSet::TCL85_PLUS),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-errorcode",
                    value: OptionValue::value("list"),
                    detail: "Additional error info, merged into the errorCode global. Only meaningful with -code error; defaults to \"NONE\" when omitted there.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-errorinfo",
                    value: OptionValue::value("info"),
                    detail: "Initial stack trace, merged into the errorInfo global. Only meaningful with -code error; Tcl supplies its own default when omitted there.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-errorstack",
                    value: OptionValue::Takes(OptionArg {
                        arity: OptionArity::Hook(errorstack_value),
                        hint: "list",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Initial error stack (must be an even-sized list). Only meaningful with -code error.",
                    dialects: Some(DialectSet::TCL86_PLUS),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-options",
                    value: OptionValue::value("dict"),
                    detail: "Dictionary of additional option/value pairs, merged in as if each had been given directly.",
                    dialects: Some(DialectSet::TCL85_PLUS),
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        ..CommandSpec::DEFAULT
    }
}
