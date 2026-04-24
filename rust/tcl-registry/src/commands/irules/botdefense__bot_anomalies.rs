//! `BOTDEFENSE::bot_anomalies` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_anomalies",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the list of names of anomalies detected for the client that sent the cur",
            &["BOTDEFENSE::bot_anomalies"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
