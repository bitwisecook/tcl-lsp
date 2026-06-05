//! `remove_nulls` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-d",
        takes_value: false,
        value_hint: "",
        detail: "Set the default.",
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
    synopsis: "remove_nulls ?-d | -i spawn_id? ?0 | 1?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_nulls",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Control whether null bytes are removed from spawned process output.",
            &["remove_nulls ?-d | -i spawn_id? ?0 | 1?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
