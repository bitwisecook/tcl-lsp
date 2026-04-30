//! `TCP::snd_wnd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_wnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "The remote host's advertised receive window.",
            &["TCP::snd_wnd"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
