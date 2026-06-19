//! `ip::mask` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::mask address",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::mask",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the network mask for an address.",
            synopsis: &["ip::mask address"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "The network mask string.",
        }),
        forms: FORMS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}
