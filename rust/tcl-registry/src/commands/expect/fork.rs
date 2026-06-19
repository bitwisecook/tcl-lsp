//! `fork` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fork",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fork",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Fork the Expect process, returning 0 in the child and the child pid in the parent.",
            synopsis: &["fork"],
            snippet: "",
            source: "Expect fork(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
