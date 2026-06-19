//! `fblocked` — test whether the last input operation exhausted all available input.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fblocked channel",
}];

/// Command spec for `fblocked`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fblocked",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Test whether the last input operation exhausted all available input",
            synopsis: &["fblocked channel"],
            snippet: "The fblocked command has been superceded by the chan blocked command which supports the same syntax and options.",
            source: "Tcl man page fblocked.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
