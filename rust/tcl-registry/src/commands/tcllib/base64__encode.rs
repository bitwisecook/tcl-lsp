//! `base64::encode` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "base64::encode",
        dialects: None,
        arity: Arity::new(1, 5),
        hover: Some(HoverSnippet {
            summary: "Encode binary data as a base64 string.",
            synopsis: &["base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data"],
            snippet: "",
            source: "tcllib base64 package",
            examples: "set encoded [base64::encode $binaryData]",
            return_value: "A base64-encoded string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("base64"),
        required_package: Some("base64"),
        ..CommandSpec::DEFAULT
    }
}
