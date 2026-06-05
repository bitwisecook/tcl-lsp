//! `html::html_entities` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "html::html_entities string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html::html_entities",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Replace special characters with HTML entities.",
            synopsis: &["html::html_entities string"],
            snippet: "",
            source: "tcllib html package",
            examples: "set safe [html::html_entities \"<b>text</b>\"]",
            return_value: "The entity-encoded string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
