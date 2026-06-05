//! `vopt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vopt ?+acc? ?-o optimized_name? ?-debugdb? ?-L library? top_module",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vopt",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Optimize a compiled design for simulation.",
            &["vopt ?+acc? ?-o optimized_name? ?-debugdb? ?-L library? top_module"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
