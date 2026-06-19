//! `dns::reset` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::reset token ?reason? ?message?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::reset",
        dialects: None,
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Reset a DNS query.",
            synopsis: &["dns::reset token ?reason? ?message?"],
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
