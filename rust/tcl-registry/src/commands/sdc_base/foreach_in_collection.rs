//! `foreach_in_collection` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "foreach_in_collection var collection body",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreach_in_collection",
        traits: Traits::CONTROL_FLOW
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Iterate over objects in a collection.",
            &["foreach_in_collection var collection body"],
            "F5",
        )),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}
