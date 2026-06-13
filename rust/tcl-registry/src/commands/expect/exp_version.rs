//! `exp_version` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exp_version ?version?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_version",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
hover: Some(HoverSnippet {
            summary: "Query or require a minimum Expect version.",
            synopsis: &["exp_version ?version?"],
            snippet: "Without arguments, returns the current Expect version. With a version argument, raises an error if the running Expect is older than the specified version.",
            source: "Expect exp_version(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
