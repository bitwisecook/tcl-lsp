//! `ASM::threat_campaign` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::threat_campaign",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the list of threat campaigns.",
            &["ASM::threat_campaign ( names | staged_names )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
