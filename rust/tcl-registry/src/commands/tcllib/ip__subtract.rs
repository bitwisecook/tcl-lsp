//! `ip::subtract` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::subtract addressList",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::subtract",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Subtract address ranges from a list of hosts.",
            synopsis: &["ip::subtract addressList"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "A list of remaining address ranges.",
        }),
        forms: FORMS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}
