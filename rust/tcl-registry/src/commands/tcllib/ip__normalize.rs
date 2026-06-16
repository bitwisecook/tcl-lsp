//! `ip::normalize` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::normalize address ?Ip4inIp6?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::normalize",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Normalise an IP address to its canonical form.",
            synopsis: &["ip::normalize address"],
            snippet: "",
            source: "tcllib ip package",
            examples: "set norm [ip::normalize 192.168.001.001]",
            return_value: "The normalised IP address string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}
