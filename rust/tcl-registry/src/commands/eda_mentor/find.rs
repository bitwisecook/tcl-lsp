//! `find` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "find ?-recursive? ?-type type? ?-ports? ?-signals? ?-internal? pattern",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "find",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Find signals matching a pattern.",
            &["find ?-recursive? ?-type type? ?-ports? ?-signals? ?-internal? pattern"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
