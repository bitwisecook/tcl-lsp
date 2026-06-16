//! `dns::wait` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::wait token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::wait",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Wait for a DNS query to complete.",
            synopsis: &["dns::wait token"],
            snippet: "",
            source: "tcllib dns package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("dns"),
        required_package: Some("dns"),
        ..CommandSpec::DEFAULT
    }
}
