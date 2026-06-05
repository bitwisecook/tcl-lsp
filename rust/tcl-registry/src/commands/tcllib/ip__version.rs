//! `ip::version` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::version address",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::version",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the IP version of an address (4 or 6).",
            synopsis: &["ip::version address"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "4 or 6, or -1 if not a valid IP address.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
