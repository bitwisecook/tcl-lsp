//! `PEM::flow` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::flow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "PEM iRule command for flow features, including transacitonal and eval.",
            &["PEM::flow transactional disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
