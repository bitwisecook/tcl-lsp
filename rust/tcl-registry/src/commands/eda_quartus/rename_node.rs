//! `rename_node` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "rename_node -from old_name -to new_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rename_node",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Rename a node in an ECO change.",
            &["rename_node -from old_name -to new_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
