//! `mime::getbody` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::getbody token ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getbody",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Return the body of a MIME part.",
            synopsis: &["mime::getbody token ?-decode? ?-command cmdprefix?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "The body content.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
