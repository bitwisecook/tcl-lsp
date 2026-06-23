//! `timerate` — measure the rate of execution of a script.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
}];

/// Command spec for `timerate`.
///
/// Like its sibling `time`, the command takes a `command` argument plus
/// optional `time` / `max-count` positionals and leading `-direct`
/// / `-calibrate` / `-overhead` options, returning a measured-rate
/// summary string.  Arg 0 is a `BODY`, arg 1 an `INT` (shimmers),
/// the arity is unbounded, and there is an `UNKNOWN`-target read+write
/// side effect (the body runs arbitrary code).  Follows the tcl pack's
/// `dialects: None` convention for vanilla commands — iRules exclusion is
/// handled via the separate iRules pack, not per-command (sibling `time`
/// is `None` too).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timerate",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        // The body's position varies with the leading options, so we
        // don't pin a fixed BODY index beyond arg 0; the option /
        // positional mix makes the upper arity bound unbounded.
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Measure the rate of execution of a script.",
            synopsis: &[
                "timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
            ],
            snippet: "",
            source: "Tcl timerate(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
