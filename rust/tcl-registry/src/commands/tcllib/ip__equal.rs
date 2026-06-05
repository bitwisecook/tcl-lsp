//! `ip::equal` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::equal address1 address2",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::equal",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Test if two IP addresses or subnets are equal.",
            synopsis: &["ip::equal address1 address2"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "1 if equal, 0 otherwise.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
