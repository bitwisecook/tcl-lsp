//! `disconnect` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "disconnect",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "disconnect",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(0),
hover: Some(HoverSnippet {
            summary: "Disconnect the process from the controlling terminal (daemonise).",
            synopsis: &["disconnect"],
            snippet: "Disconnects the forked process from the terminal. Typically used after ``fork`` in the child process.",
            source: "Expect disconnect(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
