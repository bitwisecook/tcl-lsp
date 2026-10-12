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
use tcl_dialect::model::Family;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::surface;

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
        // -level is Tcl 8.5+ (see the per-option gate on the `-level`
        // OptionSpec below, and the 8.4 SYNOPSIS split immediately
        // below this entry) — this form's own `dialects` must say so
        // too rather than inheriting the command's unrestricted
        // `surface: None`, or it would claim `-level` is legal
        // syntax in Tcl 8.4 and iRules (embedded Tcl 8.4.6), which it
        // is not: the 8.4 manpage's SYNOPSIS/body recognise only
        // -code/-errorinfo/-errorcode.
        synopsis: "return ?-code code? ?-level level? ?result?",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        // `TCL84 | IRULES`, not bare `TCL84`: these are separate
        // `SpecSurface` bits (iRules' own embedded-8.4.6 base doesn't
        // imply the `TCL84` bit), and this form — return used inside a
        // `proc`, not directly in a `when EVENT { … }` body — is exactly
        // what an iRules-defined proc uses: the full Tcl 8.4
        // -code/-errorinfo/-errorcode option set, unrestricted, since the
        // event-body-only bare-return restriction (see the `IRULES` form
        // below and `return_context_gate`) never fires inside a proc.
        // Without the explicit union here, this form would incorrectly
        // read as invisible to iRules even though it's the form every
        // iRules proc actually uses.
        synopsis: "return ?-code code? ?-errorinfo info? ?-errorcode code? ?string?",
        surface: Some(surface![
            SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.5"))]),
            SpecSurface::core(Family::F5Irules)
        ]),
        ..FormSpec::DEFAULT
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
    // context — their Tcl-version gates cannot admit the iRules point
    // (`ResolvedContext::option_available`). This
    // entry narrows the *form*, not the command's own Tcl-version gating
    // — return itself stays universal (`surface: None` below).
    FormSpec {
        synopsis: "return",
        surface: Some(SpecSurface::IRULES),
        ..FormSpec::DEFAULT
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
/// A single ordinary argv word is always the result, even when its value is
/// dynamic or starts with `-`. The native option parser requires a following
/// word before treating that spelling as an option. Expansion must retain an
/// independently known cardinality; ambiguous option mixtures stay unresolved.
fn return_layout_roles(
    args: crate::InvocationArguments<'_>,
    _options: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    match args.exact_argv_len()? {
        0 => Some(Vec::new()),
        1 => Some(vec![(0, ArgRole::Result)]),
        _ => args.literal_values().map(|_| Vec::new()),
    }
}

fn return_context_gate(
    args: crate::InvocationArguments<'_>,
    in_event_body: bool,
) -> Option<&'static str> {
    (in_event_body && args.exact_argv_len()? != 0).then_some(
        "`return` takes no arguments directly inside an iRules event body; \
         wrap the call in a proc to use -code/-level/-errorcode/etc.",
    )
}

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::EventControl,
    writes: true,
    surface: Some(SpecSurface::IRULES),
    ..SideEffect::DEFAULT
}];

const RETURN_STORAGE: &[crate::world_effect::StaticEffectAccess] = &[
    crate::world_effect::StaticEffectAccess::new(
        crate::world_effect::WorldStateDomain::InterpreterResult,
        crate::world_effect::EffectAccessMode::Write,
        crate::world_effect::StaticInterpreterScope::Current,
        crate::world_effect::StaticNamespaceScope::Any,
        crate::world_effect::StaticSubjectScope::Wildcard,
    ),
    crate::world_effect::StaticEffectAccess::new(
        crate::world_effect::WorldStateDomain::CompletionState,
        crate::world_effect::EffectAccessMode::Write,
        crate::world_effect::StaticInterpreterScope::Current,
        crate::world_effect::StaticNamespaceScope::Any,
        crate::world_effect::StaticSubjectScope::Wildcard,
    ),
];

fn return_state_effects(
    arguments: crate::InvocationArguments<'_>,
) -> crate::world_effect::EffectFootprint {
    match crate::registry::native_return_state_effect(arguments) {
        crate::completion_route::ReturnStateEffect::ResultAndCompletion => {
            crate::world_effect::EffectFootprint::default()
        }
        crate::completion_route::ReturnStateEffect::MayMaterialiseError => {
            crate::world_effect::EffectFootprint::conservative_unknown_invocation()
        }
    }
}

fn return_state_transitions(arguments: crate::InvocationArguments<'_>) -> crate::StateTransitions {
    match crate::registry::native_return_state_effect(arguments) {
        crate::completion_route::ReturnStateEffect::ResultAndCompletion => {
            crate::StateTransitions::default()
        }
        crate::completion_route::ReturnStateEffect::MayMaterialiseError => {
            crate::StateTransitions::unknown_invocation()
        }
    }
}

const RETURN_STATE_TRANSITIONS: crate::StateTransitionDescriptor =
    crate::StateTransitionDescriptor {
        resolver: Some(return_state_transitions),
        // Error-option publication may invoke observers before an abrupt return.
        commit: crate::StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        ..crate::StateTransitionDescriptor::EMPTY
    };

const RETURN_WORLD_EFFECTS: crate::WorldEffectDescriptor = crate::WorldEffectDescriptor {
    static_footprint: crate::world_effect::StaticEffectFootprint {
        accesses: RETURN_STORAGE,
        callback: crate::world_effect::CallbackEffect::NONE,
    },
    resolver: Some(return_state_effects),
    dynamic_fallback: crate::world_effect::WorldEffectDynamicFallback::Declared(
        crate::world_effect::StaticEffectFootprint::EMPTY,
    ),
    ..crate::WorldEffectDescriptor::EMPTY
};

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
        native_result: Some(crate::native_result::NativeResultContract::ReturnResult),
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Return,
            operation: crate::SemanticOperationId::StructuredLowering(
                crate::hooks::LoweringHookId::Return,
            ),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("return"),
        // Present and unrestricted: its `dialects` group carries the
        // `IRULES` bit explicitly (`ALL_TCL.union(IRULES)`), so it resolves
        // under the bare `IRULES` availability mask — a pure control-flow
        // primitive with no filesystem/process/network access, so every
        // dialect that hosts a real Tcl core (irules, iapps, tmsh, the EDA
        // shells, expect, tk, itcl) carries it unmodified. Its *legal
        // option set* still narrows per dialect/version through the
        // individual OptionSpecs' own `dialects` gates below (enforced
        // generically by `ProfileQueries::is_option_available`), and its
        // *argument shape* narrows further inside an iRules event body
        // (see `FORMS` / `return_context_gate`) — neither of which is a
        // whole-command dialect gate.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::any(),
        arg_role_layout_resolver: Some(return_layout_roles),
        arg_role_resolver_roles: &[ArgRole::Result],
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        world_effects: Some(RETURN_WORLD_EFFECTS),
        state_transitions: Some(RETURN_STATE_TRANSITIONS),
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
        native_lowering: Some(NativeLowering::Structured(LoweringHookId::Return)),
        inline_codegen_hook: Some(InlineCodegenHookId::Return),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::completion::RETURN),
        forms: FORMS,
        context_gate: Some(return_context_gate),
        // Native return scans option/value pairs while preserving the last
        // word as a possible result, including an unknown singleton value.
        reserved_trailing_words: 1,
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
                    surface: Some(SpecSurface::TCL85_PLUS),
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
                    surface: Some(SpecSurface::TCL86_PLUS),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-options",
                    value: OptionValue::value("dict"),
                    detail: "Dictionary of additional option/value pairs, merged in as if each had been given directly.",
                    surface: Some(SpecSurface::TCL85_PLUS),
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use crate::{ArgRole, CommandRegistry, InvocationArguments, InvocationDialect, InvocationWord};

    #[test]
    fn sole_return_result_roles_match_all_six_native_option_spelling_controls() {
        let registry = CommandRegistry::build_default();
        let dialects = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(InvocationDialect::for_version)
            .chain([InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment("jim").unit_profile(),
            )]);
        for dialect in dialects {
            for value in [
                InvocationWord::Dynamic,
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("-level"),
                InvocationWord::Literal("-bad"),
                InvocationWord::Literal("-1"),
            ] {
                let arguments = [value];
                let resolution = registry.resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        InvocationWord::Literal("return"),
                        &arguments,
                    )
                    .with_dialect(dialect),
                    dialect.authoring_query(),
                );
                let resolved = resolution.resolved().expect("selected native return");
                for facts in [resolved.facts(), resolved.facts_after_success()] {
                    assert!(facts.arg_roles_complete, "{dialect:?} {value:?}");
                    assert_eq!(facts.arg_roles, vec![(0, ArgRole::Result)]);
                }
                assert_eq!(
                    registry.arg_indices_for_role_words(
                        "return",
                        InvocationArguments::structured(&[value]).with_dialect(dialect),
                        ArgRole::Result
                    ),
                    Some(vec![0]),
                    "{dialect:?} {value:?}"
                );
            }
        }
    }

    #[test]
    fn return_option_pairs_keep_their_value_even_at_the_last_argv_position() {
        let registry = CommandRegistry::build_default();
        let dialects = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(|version| (Some(version), InvocationDialect::for_version(version)))
            .chain([(
                None,
                InvocationDialect::of_profile(
                    crate::model::ingress::resolve_environment("jim").unit_profile(),
                ),
            )]);
        for (version, dialect) in dialects {
            for arguments in [
                vec!["-code", "ok"],
                vec!["-code", "ok", "RESULT"],
                vec!["-level", "0"],
                vec!["-level", "0", "RESULT"],
            ] {
                if arguments[0] == "-level" && version == Some(tcl_dialect::TclVersion::V8_4) {
                    continue;
                }
                let values = arguments
                    .iter()
                    .map(|value| InvocationWord::Literal(value))
                    .collect::<Vec<_>>();
                let resolution = registry.resolve_structured_invocation(
                    crate::InvocationWords::structured(InvocationWord::Literal("return"), &values)
                        .with_dialect(dialect),
                    dialect.authoring_query(),
                );
                let resolved = resolution.resolved().expect("selected native option pair");
                for facts in [resolved.facts(), resolved.facts_after_success()] {
                    assert!(facts.arg_roles_complete, "{version:?} {arguments:?}");
                    assert!(
                        !facts
                            .arg_roles
                            .iter()
                            .any(|&(index, role)| index == 1 && role == ArgRole::Result)
                    );
                }
                assert_eq!(
                    resolved.semantics.options.leading_word_count(
                        InvocationArguments::structured(&values).with_dialect(dialect)
                    ),
                    Some(2)
                );
            }
        }
    }

    #[test]
    fn return_result_roles_withdraw_for_unknown_expansion_and_option_mixtures() {
        let registry = CommandRegistry::build_default();
        for words in [
            vec![InvocationWord::Expanded],
            vec![InvocationWord::Dynamic, InvocationWord::Dynamic],
        ] {
            assert!(
                registry
                    .arg_indices_for_role_words(
                        "return",
                        InvocationArguments::structured(&words),
                        ArgRole::Result
                    )
                    .is_none()
            );
        }
        assert_eq!(
            registry.arg_indices_for_role_words(
                "return",
                InvocationArguments::literals(&["-code", "error"]),
                ArgRole::Result
            ),
            Some(Vec::new())
        );
    }
}
