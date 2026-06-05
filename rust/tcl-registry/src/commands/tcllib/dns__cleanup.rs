//! `dns::cleanup` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dns::cleanup token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::cleanup",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Clean up resources associated with a DNS query.",
            synopsis: &["dns::cleanup token"],
            snippet: "",
            source: "tcllib dns package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
