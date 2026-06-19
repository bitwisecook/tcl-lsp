//! `dns::status` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::status token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::status",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the status of a DNS query.",
            synopsis: &["dns::status token"],
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
