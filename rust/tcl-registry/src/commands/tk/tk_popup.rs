//! `tk_popup` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_popup menu x y ?entry?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_popup",
        dialects: None,
        arity: Arity::new(3, 4),
        hover: Some(HoverSnippet {
            summary: "Post a pop-up menu at the given screen coordinates.",
            synopsis: &["tk_popup menu x y ?entry?"],
            snippet: "",
            source: "Tk man page tk_popup.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
