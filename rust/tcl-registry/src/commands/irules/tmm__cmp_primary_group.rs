//! `TMM::cmp_primary_group` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_primary_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `TMM::cmp_primary_group`.",
            &["TMM::cmp_primary_group"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
