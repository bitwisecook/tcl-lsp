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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "return ?-code code? ?-level level? ?result?",
}];

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

/// Command spec for `return`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "return",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: &[
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
        ],
        hover: Some(HoverSnippet {
            summary: "Return from the current procedure/script with optional control-code metadata.",
            synopsis: &["return ?-code code? ?-level level? ?result?"],
            snippet: "Advanced forms can emulate `break`, `continue`, or custom return codes. In an iRules event body, `return` takes no arguments — it exits the current event invocation only.",
            source: "Tcl return(n); F5 return(1)",
            examples: "",
            return_value: "",
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
                    detail: "Exceptional return code: ok/error/return/break/continue, or an integer (5–0x3fffffff reserved for application use by convention, not enforced — tclsh 8.6.14 accepts any integer here).",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-level",
                    value: OptionValue::Takes(OptionArg {
                        integer: Some(IntegerDomain::Range(0, 2_147_483_647)),
                        hint: "level",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Stack levels up the code applies to (default 1). 0 means this `return` itself returns -code. Verified against tclsh 8.6.14: must be 0..=2147483647, a negative or larger value is a hard error (unlike -code's integer, which never errors in this range).",
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
                    detail: "Initial error stack (must be an even-sized list — verified against tclsh 8.6.14). Only meaningful with -code error.",
                    dialects: Some(DialectSet::TCL86_PLUS),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-options",
                    value: OptionValue::value("dict"),
                    detail: "Dictionary of additional option/value pairs, merged in as if each had been given directly.",
                    dialects: Some(DialectSet::TCL86_PLUS),
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        ..CommandSpec::DEFAULT
    }
}
