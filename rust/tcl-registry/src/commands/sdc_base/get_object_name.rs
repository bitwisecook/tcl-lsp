//! `get_object_name` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_object_name object",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_object_name",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the name of an object.",
            &["get_object_name object"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
