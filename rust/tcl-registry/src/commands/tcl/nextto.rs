//! `nextto` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nextto",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "invoke a specific superclass implementation of a method",
            synopsis: &["nextto class ?arg ...?"],
            snippet: "The nextto command is like next but invokes a specific class's implementation of the current method rather than the next in the MRO.",
            source: "Tcl man page next.n",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "nextto class ?arg ...?",
        }],
        traits: Traits::LANGUAGE_KEYWORD,
        ..CommandSpec::DEFAULT
    }
}
