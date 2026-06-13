//! `sha1::sha1` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "sha1::sha1 ?options? ?--? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha1::sha1",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Compute the SHA-1 hash of a string or file.",
            synopsis: &["sha1::sha1 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?"],
            snippet: "",
            source: "tcllib sha1 package",
            examples: "",
            return_value: "The SHA-1 hash as a hex or binary string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("sha1"),
        required_package: Some("sha1"),
        ..CommandSpec::DEFAULT
    }
}
