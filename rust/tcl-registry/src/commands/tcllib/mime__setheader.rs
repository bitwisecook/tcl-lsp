//! `mime::setheader` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::setheader token key value ?-mode mode?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::setheader",
        dialects: None,
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet {
            summary: "Set a header on a MIME token.",
            synopsis: &["mime::setheader token key value ?-mode mode?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("mime"),
        required_package: Some("mime"),
        ..CommandSpec::DEFAULT
    }
}
