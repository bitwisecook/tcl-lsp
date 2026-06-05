//! `group` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "group ?-design_name name? ?-cell_name name? cell_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "group",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Group logic into a hierarchical block.",
            &["group ?-design_name name? ?-cell_name name? cell_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
