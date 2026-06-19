//! `write_netlist` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "write_netlist file_name ?-top_module_first?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_netlist",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a gate-level netlist.",
            &["write_netlist file_name ?-top_module_first?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
