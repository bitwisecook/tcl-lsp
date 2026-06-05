//! `dns::resolve` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::resolve name ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::resolve",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Perform a DNS lookup.",
            synopsis: &[
                "dns::resolve name ?-type type? ?-class class? ?-server server? ?-timeout ms?",
            ],
            snippet: "",
            source: "tcllib dns package",
            examples: "set tok [dns::resolve www.example.com]",
            return_value: "A DNS query token.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("dns"),
        required_package: Some("dns"),
        ..CommandSpec::DEFAULT
    }
}
