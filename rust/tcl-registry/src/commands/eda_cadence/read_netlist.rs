//! `read_netlist` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_netlist file_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_netlist",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read a gate-level netlist.",
            &["read_netlist file_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
