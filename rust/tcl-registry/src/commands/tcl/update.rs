//! `update` — process pending events.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "update ?idletasks?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "update",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Process pending events and idle callbacks",
    synopsis: &["update ?idletasks?"],
    snippet: "This command is used to bring the application by entering the event loop repeatedly until all pending events (including idle callbacks) have been processed.",
    source: "Tcl man page update.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
