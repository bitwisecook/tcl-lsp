//! `sha2::sha256` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "sha2::sha256 ?options? ?--? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha2::sha256",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Compute the SHA-256 hash of a string or file.",
            synopsis: &[
                "sha2::sha256 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
            ],
            snippet: "",
            source: "tcllib sha2 package",
            examples: "",
            return_value: "The SHA-256 hash as a hex or binary string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
