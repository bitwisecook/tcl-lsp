//! `xsim` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "xsim ?-R? ?-input cmd_file? snapshot_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xsim",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Xcelium simulation on an elaborated snapshot.",
            &["xsim ?-R? ?-input cmd_file? snapshot_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
