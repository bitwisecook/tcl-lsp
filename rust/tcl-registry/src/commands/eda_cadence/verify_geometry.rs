//! `verify_geometry` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "verify_geometry ?-error n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "verify_geometry",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Verify design geometry (DRC).",
            &["verify_geometry ?-error n?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
