//! `FLOW::idle_timeout` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::idle_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets/Gets the idle timeout on the flow",
            &["FLOW::idle_timeout (ANY_CHARS) (NONNEGATIVE_INTEGER)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
