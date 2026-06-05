//! `update_timing_netlist` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "update_timing_netlist",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "update_timing_netlist",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Update the timing netlist after changes.",
            &["update_timing_netlist"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
