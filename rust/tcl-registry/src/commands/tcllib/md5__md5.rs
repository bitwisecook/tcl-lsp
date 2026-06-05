//! `md5::md5` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "md5::md5 ?options? ?--? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "md5::md5",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Compute the MD5 hash of a string or file.",
            synopsis: &["md5::md5 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?"],
            snippet: "",
            source: "tcllib md5 package",
            examples: "",
            return_value: "The MD5 hash as a hex or binary string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("md5"),
        required_package: Some("md5"),
        ..CommandSpec::DEFAULT
    }
}
