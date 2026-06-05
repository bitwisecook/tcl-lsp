//! `next` — call the next method in the chain.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "next ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "next",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::any(),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "invoke the next implementation of a method",
    synopsis: &["next ?arg ...?"],
    snippet: "The next command is used within the body of a method to call the next implementation of that method in the method resolution order (MRO). Arguments are passed to the next implementation.",
    source: "Tcl man page next.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
