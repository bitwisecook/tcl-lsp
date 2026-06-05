//! `derive_clocks` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "derive_clocks ?-period period?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "derive_clocks",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Automatically derive clocks from the design.",
            &["derive_clocks ?-period period?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
