//! `textutil::trim` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::trim text ?regexp?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trim",
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Remove leading whitespace from each line of text.",
            synopsis: &["textutil::trim text ?regexp?"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "set trimmed [textutil::trim $text]",
            return_value: "The trimmed text.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}
