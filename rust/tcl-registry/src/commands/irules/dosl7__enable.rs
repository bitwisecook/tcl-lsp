//! `DOSL7::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables blocking and detection of DoS attacks according to the ASM security poli",
            &["DOSL7::enable (DOSL7_PROFILE_OBJ)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
