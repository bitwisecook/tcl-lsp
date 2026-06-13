//! `append_to_collection` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "append_to_collection collection ?-unique? objects",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "append_to_collection",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Append objects to a collection variable.",
            &["append_to_collection collection ?-unique? objects"],
            "F5",
        )),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    }
}
