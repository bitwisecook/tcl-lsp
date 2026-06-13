//! `delete_timing_netlist` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "delete_timing_netlist",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "delete_timing_netlist",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Delete the current timing netlist.",
            &["delete_timing_netlist"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
