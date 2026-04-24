//! `SSL::forward_proxy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::forward_proxy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves t",
            &["SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
