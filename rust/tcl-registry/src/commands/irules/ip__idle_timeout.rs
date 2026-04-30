//! `IP::idle_timeout` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::idle_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the idle timeout value.",
            &["IP::idle_timeout (TIMEOUT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
