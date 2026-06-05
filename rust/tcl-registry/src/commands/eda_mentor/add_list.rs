//! `add_list` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "add list ?-radix radix? signal_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_list",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add signals to the list window (add list).",
            &["add list ?-radix radix? signal_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
