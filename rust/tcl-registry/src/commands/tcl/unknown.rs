//! `unknown` — handle unknown commands.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unknown cmdName ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unknown",
        traits: Traits::CREATES_BARRIER,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Handle attempts to use non-existent commands",
    synopsis: &["unknown cmdName ?arg arg ...?", "unknown cmdName ?arg ...?"],
    snippet: "This command is invoked by the Tcl interpreter whenever a script tries to invoke a command that does not exist.",
    source: "Tcl man page unknown.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
