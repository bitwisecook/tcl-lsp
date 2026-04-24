//! `RADIUS::avp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::avp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns or adds/changes/removes RADIUS attribute-value pairs.",
            &["RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
