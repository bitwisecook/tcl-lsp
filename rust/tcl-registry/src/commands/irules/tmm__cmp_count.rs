//! `TMM::cmp_count` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides the active number of TMM instances running.",
            &["TMM::cmp_count"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
