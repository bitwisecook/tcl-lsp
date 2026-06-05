//! `parity` command.
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
    synopsis: "parity ?-d | -i spawn_id? ?value?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "parity",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or query whether parity is retained on spawned process output.",
            &["parity ?-d | -i spawn_id? ?value?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
