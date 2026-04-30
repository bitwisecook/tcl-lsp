//! `NSH::path_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::path_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set/Get the Path ID for NSH.",
            &["NSH::path_id DIRECTION (NSH_PATH_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
