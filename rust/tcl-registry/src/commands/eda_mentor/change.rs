//! `change` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "change signal_name value",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "change",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Change the value of a VHDL signal or variable.",
            &["change signal_name value"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
