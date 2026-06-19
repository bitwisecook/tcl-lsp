//! `add_endcap` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "add_endcap ?-pre_endcap cell? ?-post_endcap cell?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_endcap",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add endcap cells.",
            &["add_endcap ?-pre_endcap cell? ?-post_endcap cell?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
