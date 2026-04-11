//! `TMM::cmp_group` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the number (0-x) of the group of the CPU executing the rule.",
            &["TMM::cmp_group"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
