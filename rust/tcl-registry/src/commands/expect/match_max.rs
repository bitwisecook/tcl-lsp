//! `match_max` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-d",
        takes_value: false,
        value_hint: "",
        detail: "Set the default for all spawn ids.",
        dialects: None,
    },
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Set for the specified spawn id.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "match_max ?-d | -i spawn_id? ?size?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "match_max",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set or query the maximum match buffer size.",
            synopsis: &["match_max ?-d | -i spawn_id? ?size?"],
            snippet: "",
            source: "Expect match_max(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
