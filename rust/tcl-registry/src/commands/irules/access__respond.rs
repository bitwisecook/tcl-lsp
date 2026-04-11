//! `ACCESS::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command generates new respond and automatically overrides the default respo",
            &["ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
