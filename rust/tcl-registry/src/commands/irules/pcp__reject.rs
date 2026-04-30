//! `PCP::reject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::reject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides the ability to cause a PCP reqeust to fail based on processing in the i",
            &["PCP::reject PCP_RESULT_CODE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
