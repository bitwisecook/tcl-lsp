//! `remove_from_collection` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "remove_from_collection collection objects",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_from_collection",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Remove objects from a collection variable.",
            &["remove_from_collection collection objects"],
            "F5",
        )),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    }
}
