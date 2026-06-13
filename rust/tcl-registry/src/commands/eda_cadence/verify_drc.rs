//! `verify_drc` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "verify_drc ?-limit n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "verify_drc",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run design rule checking.",
            &["verify_drc ?-limit n?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
