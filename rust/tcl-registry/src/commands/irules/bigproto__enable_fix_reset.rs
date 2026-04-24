//! `BIGPROTO::enable_fix_reset` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BIGPROTO::enable_fix_reset",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable or Disable Reset of FIX Protocol Connections",
            &["BIGPROTO::enable_fix_reset BOOLEAN"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
