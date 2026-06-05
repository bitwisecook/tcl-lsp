//! `ip::is` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::is class address",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::is",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        return_type: Some(TclType::Boolean),
        hover: Some(HoverSnippet {
            summary: "Test whether a value is a valid IP address of the given class.",
            synopsis: &["ip::is class address"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "1 if the address matches the class, 0 otherwise.",
        }),
        forms: FORMS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}
