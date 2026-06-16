//! `uri::split` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::split url ?defaultScheme?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::split",
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Split a URI into its component parts.",
            synopsis: &["uri::split url ?defaultScheme?"],
            snippet: "Parses the URI and returns a key-value list with scheme, host, port, path, query, fragment, etc.",
            source: "tcllib uri package",
            examples: "array set parts [uri::split $url]",
            return_value: "A key-value list of URI components.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("uri"),
        required_package: Some("uri"),
        ..CommandSpec::DEFAULT
    }
}
