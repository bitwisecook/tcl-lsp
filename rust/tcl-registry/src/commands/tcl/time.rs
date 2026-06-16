//! `time` — measure script execution time.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "time script ?count?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "time",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Time the execution of a script",
            synopsis: &["time script ?count?"],
            snippet: "This command will call the Tcl interpreter count times to evaluate script (or once if count is not specified).",
            source: "Tcl man page time.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
