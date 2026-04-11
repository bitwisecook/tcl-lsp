//! `NSH::service_index` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::service_index",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets/Get the Service Index for NSH.",
            &["NSH::service_index DIRECTION (NSH_SERVICE_IDX)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
