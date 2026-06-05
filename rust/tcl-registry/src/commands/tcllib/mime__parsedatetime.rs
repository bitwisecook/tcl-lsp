//! `mime::parsedatetime` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::parsedatetime datestring property",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::parsedatetime",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Parse an RFC 2822 date/time string.",
            synopsis: &["mime::parsedatetime datestring property"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("mime"),
        required_package: Some("mime"),
        ..CommandSpec::DEFAULT
    }
}
