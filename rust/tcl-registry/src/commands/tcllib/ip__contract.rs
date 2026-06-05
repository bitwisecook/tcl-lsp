//! `ip::contract` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::contract address",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::contract",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Contract an IPv6 address to its shortest form.",
            synopsis: &["ip::contract address"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "The contracted IPv6 address string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
