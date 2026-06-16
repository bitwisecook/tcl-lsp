//! `bind` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "bind tag ?sequence? ?+??command?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bind",
        dialects: None,
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Arrange for X event bindings on windows or tags.",
            synopsis: &[
                "bind tag",
                "bind tag sequence",
                "bind tag sequence script",
                "bind tag sequence +script",
            ],
            snippet: "",
            source: "Tk man page bind.n",
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
