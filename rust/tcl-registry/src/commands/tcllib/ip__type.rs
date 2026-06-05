//! `ip::type` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::type address",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::type",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the type of an IP address.",
            synopsis: &["ip::type address"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "The address type string.",
        }),
        forms: FORMS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}
