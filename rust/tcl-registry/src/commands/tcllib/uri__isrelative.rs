//! `uri::isrelative` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::isrelative uri",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::isrelative",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::Boolean),
        hover: Some(HoverSnippet {
            summary: "Test whether a URI is relative.",
            synopsis: &["uri::isrelative uri"],
            snippet: "",
            source: "tcllib uri package",
            examples: "",
            return_value: "1 if the URI is relative, 0 otherwise.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
