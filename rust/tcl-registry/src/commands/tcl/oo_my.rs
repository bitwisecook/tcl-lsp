//! `my` — call a method on the current object.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "my method ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "my",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "invoke a method on the current object",
            synopsis: &["my method ?arg ...?"],
            snippet: "The my command is used within the body of a method, constructor, or destructor to invoke a method on the current object.  It is equivalent to [self] method ?arg ...? but avoids the overhead of determining the object name.",
            source: "Tcl man page my.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
