//! `dns::address` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::address token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::address",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the IP addresses from a DNS query result.",
            synopsis: &["dns::address token"],
            snippet: "",
            source: "tcllib dns package",
            examples: "",
            return_value: "A list of IP addresses.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
