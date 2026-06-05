//! `dns::name` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::name token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::name",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the domain name from a DNS query result.",
            synopsis: &["dns::name token"],
            snippet: "",
            source: "tcllib dns package",
            examples: "",
            return_value: "A list of domain names.",
        }),
        forms: FORMS,
        tcllib_package: Some("dns"),
        required_package: Some("dns"),
        ..CommandSpec::DEFAULT
    }
}
