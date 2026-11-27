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

//! `subst` — perform Tcl substitutions on a string.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// `subst`'s embedded `[command]` substitutions can do literally anything
/// once evaluated, the same "unknowable statically" reasoning `eval` /
/// `apply` document on their own `Unknown` side effects — except `subst`
/// only evaluates a command when one is actually embedded in `string`
/// *and* command or variable substitution is enabled for this call (see
/// `subst_evaluates_code`/`taint_sink_gate` below), whereas `eval`
/// unconditionally does. `reads`/`writes` stay `true` rather than `eval`'s
/// `false`/`false` to match the sibling body-evaluating constructs that
/// keep an unknown-but-present effect (`if`, `for`, `foreach`, `catch`)
/// rather than a no-effect placeholder.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

use crate::substitution::{
    SUBST_NEGATED_FAMILY as NEGATED, SUBST_OPTIONS as OPTIONS, SUBST_POSITIVE_FAMILY as POSITIVE,
};

// Tcl 8.4, 8.5, 8.6, and 9.0 document exactly one SYNOPSIS line (the
// negative-switch form). Tcl 9.1 adds a second SYNOPSIS line for the new
// positive-switch form (see the OPTIONS comment above) — narrowed to
// `TCL91` here rather than inheriting the command's own unrestricted
// `surface: None`, mirroring how `return_.rs` narrows its `-level` form to
// `TCL85_PLUS`.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "subst ?-nobackslashes? ?-nocommands? ?-novariables? string",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "subst ?-backslashes? ?-commands? ?-variables? string",
        surface: Some(SpecSurface::TCL91),
        ..FormSpec::DEFAULT
    },
];

fn bare_template_effects(arguments: crate::InvocationArguments<'_>) -> crate::EffectFootprint {
    if arguments.exact_argv_len() != Some(1) {
        return crate::EffectFootprint::conservative_unknown_invocation();
    }
    crate::substitution::literal_template_effects(
        arguments,
        0,
        crate::substitution::SubstitutionKinds::ALL,
    )
}

const NATIVE_FORMS: &[crate::forms::CommandForm] = &[crate::forms::CommandForm {
    name: "template",
    arity: Arity::exact(1),
    side_effects: Some(crate::knowledge::Knowledge::Partial(&[])),
    world_effects: crate::knowledge::Knowledge::Partial(crate::WorldEffectDescriptor {
        resolver: Some(bare_template_effects),
        ..crate::WorldEffectDescriptor::EMPTY
    }),
    ..crate::forms::CommandForm::DEFAULT
}];
/// `subst`'s two switch families: the negated one (every release) starts
/// from every kind and the positive one (Tcl 9.1) from none, and both
/// accumulate — a switch may repeat.
const FAMILIES: &[OptionEffectFamily] = &[
    OptionEffectFamily {
        name: NEGATED,
        base: FamilyBase::AllOn,
        combine: FamilyCombine::Accumulate,
        surface: None,
    },
    OptionEffectFamily {
        name: POSITIVE,
        base: FamilyBase::AllOff,
        combine: FamilyCombine::Accumulate,
        surface: Some(SpecSurface::TCL91),
    },
];

/// The final `string` operand is never an option candidate (see the spec).
const RESERVED_TRAILING_WORDS: usize = 1;

/// tclsh 9.1b0's error for a call using both families.
const MIXED_FAMILIES: &str = "cannot combine positive and negative options";

const POSITIVE_SWITCHES: &[OptionTerm] = &[
    OptionTerm::Option("-backslashes"),
    OptionTerm::Option("-commands"),
    OptionTerm::Option("-variables"),
];

/// A negated switch forbids every positive one — the family exclusion as
/// three directional relations, since a relation's terms are one flat set
/// (a single `MutuallyExclusive` over all six would reject
/// `-nocommands -novariables`). Reported as W147 at the call site.
const fn forbids_positive(negated: &'static str) -> OptionRelation {
    OptionRelation {
        surface: Some(SpecSurface::TCL91),
        message: Some(MIXED_FAMILIES),
        ..Relation::forbids(OptionTerm::Option(negated), POSITIVE_SWITCHES)
    }
}

const RELATIONS: &[OptionRelation] = &[
    forbids_positive("-nobackslashes"),
    forbids_positive("-nocommands"),
    forbids_positive("-novariables"),
];

/// The template-word plan over this command's own switch table
/// (`docs/design/compiler/value-transfers.md` § *The template-word plan*).
static TEMPLATE: crate::value_transfer::template::TemplateSemantics =
    crate::value_transfer::template::TemplateSemantics::new(
        "template:subst",
        OPTIONS,
        FAMILIES,
        RESERVED_TRAILING_WORDS,
    );

/// Fold a literal `subst string`.
///
/// `subst` performs variable, command, and backslash substitution on
/// its string argument — even inside braces (`subst {$x}` substitutes
/// `$x`). The fold receives the evaluated template, so it accepts only
/// the bare `subst string` form whose string
/// carries **no** substitution: no `$`, `[`, or `\`.  Such a string is
/// its own `subst` result.  Anything with a substitution bails
/// (a stricter subset of what a fold with upstream resolution +
/// the `[command]` caller-bail could decide).  The option forms
/// (`-nobackslashes` / …) change which substitutions apply, so the
/// multi-arg form bails too.
pub(crate) fn fold_subst(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    if s.contains(['$', '[', '\\']) {
        return None;
    }
    Some((*s).to_owned())
}

/// Whether a `subst` call — given its argument words, excluding the
/// command name — can run a script, the hazard T100 warns about for this
/// command. Command substitution runs `[cmd]` directly, and variable
/// substitution runs the `[cmd]` in an array index: `subst -nocommands
/// {$a([cmd])}` runs `cmd` under tclsh 8.4 to 9.1 and Jim, as does 9.1's
/// `subst -variables`. Only backslash substitution alone runs nothing.
/// Drives `CommandSpec::taint_sink_gate`.
///
/// One projection of [`crate::option_effect::substitution_kinds`] over this
/// command's own switch table, which owns both families and the
/// unreadable-call answer, so the taint gate and the consumers of
/// [`crate::CommandRegistry::substitutions_performed`] can never disagree
/// about the same call.
fn subst_evaluates_code(args: &[&str]) -> bool {
    let kinds = crate::option_effect::substitution_kinds(
        OPTIONS,
        FAMILIES,
        InvocationArguments::literals(args),
        RESERVED_TRAILING_WORDS,
        None,
    );
    kinds.commands || kinds.variables
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "subst",
        body_execution: Some(crate::body_execution::BodyExecutionSpec::SubstitutionTemplate),
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::SubstitutionTemplate,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("subst"),
        // Present and unrestricted: its `dialects` group carries the
        // `IRULES` bit explicitly (`ALL_TCL.union(IRULES)`), so it resolves
        // under the bare `IRULES` availability mask; every dialect hosting
        // a real embedded Tcl core (irules, iapps, tmsh, the EDA shells,
        // expect, tk, itcl) carries this command unmodified. Its legal
        // *option*
        // set still narrows per Tcl version through each OptionSpec's own
        // `dialects` gate above (enforced generically by
        // `ProfileQueries::is_option_available` together with each
        // profile's `version_ceiling`), which is why the 9.1-only positive
        // switches can never resolve under any of those 8.x-pinned
        // embedded cores even though the command itself is universal.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        byte_array_effect: ByteArrayEffect::Coerces,
        traits: Traits::TAINT_SINK | Traits::IS_UNESCAPE | Traits::PERFORMS_SUBSTITUTION,
        // Which of the three substitutions a call actually runs is decided by
        // the switches above — each option row's effect, in its family — so
        // the trait alone would tell a consumer only that *some* substitution
        // happens.
        option_effect_families: FAMILIES,
        option_relations: RELATIONS,
        // Exactly one trailing `string` is mandatory; 0 or more recognised
        // switch words may precede it with no fixed ceiling (a switch may
        // legally repeat — `subst -nocommands -nocommands $s` is valid,
        // confirmed live on `tclsh` 8.6.14). Identical arity shape in all
        // five fetched manpages, so `at_least(1)` with no upper bound is
        // correct for every version.
        arity: Arity::at_least(1),
        // That one mandatory trailing `string` is never an option candidate,
        // whatever its shape. `Tcl_SubstObjCmd` (generic/tclCmdAH.c) scans
        // switches only while `i < objc - 1`, so the last word is the operand
        // even when it is spelled like a switch: tclsh 8.6.18 prints
        // `-commands` for `puts [subst -commands]` rather than rejecting a
        // 9.1-only option, and rejects `subst -- -nocommands` with `bad
        // option "--"` because `subst` has no `--` terminator to fall back on.
        reserved_trailing_words: RESERVED_TRAILING_WORDS,
        return_type: Some(TclType::String),
        const_fold: Some(fold_subst),
        hover: Some(HoverSnippet {
            summary: "Perform backslash, command, and variable substitutions on a string.",
            synopsis: &[
                "subst ?-nobackslashes? ?-nocommands? ?-novariables? string",
                "subst ?-backslashes? ?-commands? ?-variables? string",
            ],
            snippet: "Applies the same variable, command, and backslash substitution rules Tcl itself uses when parsing a command word, but a second time: string is substituted once by the Tcl parser when the call to subst is itself parsed, then again by subst at runtime. This is what lets `subst {$x}` substitute `$x` even though the enclosing braces would normally suppress substitution.\n\nAll three substitution kinds run by default. -nobackslashes, -nocommands, and -novariables (every Tcl version) each turn off exactly the substitution they name; command substitution is the kind that executes code, but disabling it alone is not enough. One kind can still trigger another even when disabled: with -novariables, a `[command]` still expands any $variable inside it, since evaluating the command requires it; with -nocommands, a `$name([b])` array reference still runs the `[b]` inside its index, since resolving the variable requires it.\n\nTcl 9.1 adds inverted, positive-only switches -backslashes/-commands/-variables: giving any one of these turns every substitution off by default and enables only the ones named, so `subst -variables $s` is equivalent to `subst -nobackslashes -nocommands $s`. The two switch families cannot be combined in one call.\n\nA break inside an embedded command or variable index stops substitution immediately, keeping only the text produced so far; continue substitutes an empty string for that one embedded piece and keeps going; return (or any other completion code) substitutes the returned value for it. Any other error propagates as subst's own error.\n\n**Security**: unless both -nocommands and -novariables are given (or, on 9.1, only -backslashes), a `[cmd]` in string is executed as Tcl, directly or inside an array index such as `$a([cmd])`, making subst a code-injection sink for untrusted input. For untrusted templates prefer `[string map]` / `[format]`.",
            source: "Tcl subst(n)",
            examples: "set name Alice\nputs [subst {Hello, $name! Today is [clock format [clock seconds] -format %A].}]\n\n# Trusted template, no command substitution\nset template {Welcome, $user!}\nputs [subst -nocommands $template]\n\n# Tcl 9.1: enable only variable substitution\nputs [subst -variables {$name < [expr {1+1}] > \\n}]",
            return_value: "The fully-substituted string: break truncates it at the point of the exception, continue/return substitute their value for just that one embedded command or variable index, and any other error becomes subst's own error.",
        }),
        forms: FORMS,
        command_forms: NATIVE_FORMS,
        options: OPTIONS,
        side_effects: crate::knowledge::Knowledge::declared(SIDE_EFFECTS),
        taint_sink_gate: Some(subst_evaluates_code),
        semantics: SemanticsDeclaration::Declared(&TEMPLATE),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::{fold_subst, subst_evaluates_code};

    #[test]
    fn subst_literal_effects_preserve_enabled_substitutions() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = crate::model::ingress::static_context_for(profile);
            let dialect =
                crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
            for (word, closed) in [
                (crate::InvocationWord::Literal("hello"), true),
                (crate::InvocationWord::Literal("a\\nb"), true),
                (crate::InvocationWord::Literal("$value"), false),
                (crate::InvocationWord::Literal("[operation]"), false),
                (crate::InvocationWord::Literal("$array([operation])"), false),
                (crate::InvocationWord::Dynamic, false),
                (crate::InvocationWord::Expanded, false),
            ] {
                let words = [word];
                let invocation = crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("subst"),
                    &words,
                )
                .with_dialect(dialect);
                let resolved = context
                    .commands()
                    .resolve_structured_invocation(invocation, dialect.authoring_query())
                    .resolved()
                    .unwrap();
                assert_eq!(
                    !resolved.effects().requires_world_barrier(),
                    closed,
                    "{profile}: {word:?}",
                );
            }
        }
    }

    #[test]
    fn subst_evaluates_code_default_is_true() {
        assert!(subst_evaluates_code(&["$tainted"]));
    }

    #[test]
    fn subst_evaluates_code_until_commands_and_variables_are_both_off() {
        // `$a([cmd])` runs `cmd` while variables are substituted.
        assert!(subst_evaluates_code(&["-nocommands", "$tainted"]));
        assert!(subst_evaluates_code(&[
            "-nocommands",
            "-nobackslashes",
            "$tainted"
        ]));
        assert!(!subst_evaluates_code(&[
            "-nocommands",
            "-novariables",
            "$tainted"
        ]));
        assert!(!subst_evaluates_code(&[
            "-novariables",
            "-nocommands",
            "$tainted"
        ]));
    }

    #[test]
    fn subst_evaluates_code_legacy_other_negatives_leave_commands_on() {
        assert!(subst_evaluates_code(&["-novariables", "$tainted"]));
        assert!(subst_evaluates_code(&["-nobackslashes", "$tainted"]));
    }

    #[test]
    fn subst_evaluates_code_positive_form_needs_commands_or_variables() {
        // Tcl 9.1 positive form: everything defaults off.
        assert!(subst_evaluates_code(&["-variables", "$tainted"]));
        assert!(!subst_evaluates_code(&["-backslashes", "$tainted"]));
        assert!(subst_evaluates_code(&["-commands", "$tainted"]));
        assert!(subst_evaluates_code(&[
            "-variables",
            "-commands",
            "$tainted"
        ]));
    }

    #[test]
    fn subst_evaluates_code_stops_scanning_at_the_string_operand() {
        // Only flags *before* the string operand matter — a `-nocommands`-
        // shaped word after it (an arity-error call shape) is the string
        // argument's own trailing content, not a switch, and must not
        // suppress the sink.
        assert!(subst_evaluates_code(&["$x", "-nocommands"]));
    }

    /// A call with no operand at all is an arity error, and one the resolver
    /// cannot read: its conservative "every kind" answer keeps the sink live
    /// rather than letting a malformed call suppress T100.
    #[test]
    fn subst_evaluates_code_stays_on_for_an_operand_less_call() {
        assert!(subst_evaluates_code(&["-nocommands"]));
    }

    #[test]
    fn subst_folds_only_substitution_free_strings() {
        // A plain `subst string` is its own
        // result; anything with $ / [ / \ bails (sound subset).
        assert_eq!(fold_subst(&["hello"]).as_deref(), Some("hello"));
        assert_eq!(fold_subst(&["a b c"]).as_deref(), Some("a b c"));
        assert_eq!(fold_subst(&["$x"]), None, "variable substitution bails");
        assert_eq!(fold_subst(&["[cmd]"]), None, "command substitution bails");
        assert_eq!(fold_subst(&["a\\nb"]), None, "backslash bails");
        assert_eq!(
            fold_subst(&["-novariables", "x"]),
            None,
            "option form bails"
        );
    }
}
