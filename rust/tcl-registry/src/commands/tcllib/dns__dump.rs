//! `dns::dump` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::dump token ?channel?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::dump",
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Dump the contents of a DNS query token for debugging.",
            synopsis: &["dns::dump token ?channel?"],
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
