//! `uri::geturl` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::FileIo,
        reads: true,
        writes: false,
        connection_side: ConnectionSide::None,
    },
    SideEffect {
        target: SideEffectTarget::NetworkIo,
        reads: true,
        writes: false,
        connection_side: ConnectionSide::None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::geturl url ?options...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::geturl",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Fetch the contents of a URI.",
            synopsis: &["uri::geturl url ?options...?"],
            snippet: "",
            source: "tcllib uri package",
            examples: "",
            return_value: "The fetched content.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("uri"),
        required_package: Some("uri"),
        ..CommandSpec::DEFAULT
    }
}
