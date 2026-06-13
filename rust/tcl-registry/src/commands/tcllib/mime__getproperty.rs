//! `mime::getproperty` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::getproperty token ?property?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getproperty",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Return a property of a MIME part.",
            synopsis: &["mime::getproperty token ?property?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "The property value or a key-value list of all properties.",
        }),
        forms: FORMS,
        tcllib_package: Some("mime"),
        required_package: Some("mime"),
        ..CommandSpec::DEFAULT
    }
}
