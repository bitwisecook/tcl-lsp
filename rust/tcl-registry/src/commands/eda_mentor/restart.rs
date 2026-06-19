//! `restart` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "restart ?-force? ?-nowave? ?-nolist? ?-nolog? ?-nobreakpoint? ?-nokill?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "restart",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Restart the current simulation.",
            &["restart ?-force? ?-nowave? ?-nolist? ?-nolog? ?-nobreakpoint? ?-nokill?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
