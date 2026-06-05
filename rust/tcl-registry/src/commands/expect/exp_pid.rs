//! `exp_pid` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-i",
    takes_value: true,
    value_hint: "spawn_id",
    detail: "Query the specified spawn id.",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exp_pid ?-i spawn_id?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_pid",
        traits: Traits::PURE,
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the process id of a spawned process.",
            &["exp_pid ?-i spawn_id?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
