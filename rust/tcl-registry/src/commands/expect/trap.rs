//! `trap` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "trap ?command? ?signal ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trap",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Trap signals and execute a command when they occur.",
            synopsis: &[
                "trap ?command? ?signal ...?",
                "trap SIG_IGN SIGINT",
                "trap { puts caught } SIGTERM",
            ],
            snippet: "",
            source: "Expect trap(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}
