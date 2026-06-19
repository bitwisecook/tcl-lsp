//! `clock_opt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "clock_opt ?-effort high|medium|low?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clock_opt",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform clock tree synthesis and optimization.",
            &["clock_opt ?-effort high|medium|low?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
