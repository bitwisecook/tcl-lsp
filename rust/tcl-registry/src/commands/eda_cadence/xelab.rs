//! `xelab` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "xelab ?-access access_type? ?-top top_module? ?-snapshot snap_name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xelab",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Elaborate a design for Xcelium simulation.",
            &["xelab ?-access access_type? ?-top top_module? ?-snapshot snap_name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
