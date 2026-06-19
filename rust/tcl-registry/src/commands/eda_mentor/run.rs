//! `run` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "run ?time_value? ?-all? ?-continue?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "run",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Advance simulation by specified time.",
            &["run ?time_value? ?-all? ?-continue?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
