//! `TCP::proxybuffer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybuffer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets proxy buffer low and high thresholds.",
            &["TCP::proxybuffer ('auto' | (LOW HIGH))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
