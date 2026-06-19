//! `toggle` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "toggle ?signal_name? ?-report?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "toggle",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report toggle coverage statistics.",
            &["toggle ?signal_name? ?-report?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
