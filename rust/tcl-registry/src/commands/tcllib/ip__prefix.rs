//! `ip::prefix` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::prefix address/mask",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::prefix",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the network prefix of an address/mask.",
            synopsis: &["ip::prefix address/mask"],
            snippet: "",
            source: "tcllib ip package",
            examples: "set net [ip::prefix 192.168.1.5/24]",
            return_value: "The network prefix address.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
