//! `TMM::cmp_groups` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_groups",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `TMM::cmp_groups`.",
            &["TMM::cmp_groups"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
