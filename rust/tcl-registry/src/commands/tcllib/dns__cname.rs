//! `dns::cname` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::cname token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::cname",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the canonical name from a DNS query result.",
            synopsis: &["dns::cname token"],
            snippet: "",
            source: "tcllib dns package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("dns"),
        required_package: Some("dns"),
        ..CommandSpec::DEFAULT
    }
}
